use std::sync::Arc;

use anyhow::{Context, ensure};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{
        Request, StatusCode,
        header::{COOKIE, LOCATION, SET_COOKIE},
        request::Builder,
    },
    middleware,
    response::Response,
};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};
use serde_json::Value;
use terrier_server::{
    auth::{
        config::AuthConfig, middleware::application_routes, oidc, session,
        session::lock::SessionLockStore,
    },
    error::ApiError,
    state::AppState,
};
use tower::ServiceExt;
use tower_sessions::MemoryStore;
use uuid::Uuid;

mod locks;
mod provider;

pub struct TestApp {
    pub db: DatabaseConnection,
    app: Router,
    provider: Arc<provider::Provider>,
    task: tokio::task::JoinHandle<()>,
    admin: DatabaseConnection,
    schema: String,
}

pub struct TestSession {
    pub user_id: Uuid,
    pub cookie: String,
    pub csrf_token: String,
}

impl TestSession {
    /// Adds the session cookie and CSRF headers to a request.
    pub fn authorize(&self, request: Builder) -> Builder {
        request
            .header(COOKIE, &self.cookie)
            .header("origin", "https://terrier.example")
            .header("x-csrf-token", &self.csrf_token)
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TestApp {
    pub async fn new() -> anyhow::Result<Self> {
        let url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let admin = Database::connect(url.clone()).await?;
        let schema = format!("route_test_{}", Uuid::new_v4().simple());
        admin
            .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
            .await?;
        let mut options = ConnectOptions::new(url);
        options.set_schema_search_path(&schema);
        let state = AppState::connect(options).await?;
        let (provider, task) = provider::Provider::start().await;
        let config = Arc::new(AuthConfig {
            app_url: "https://terrier.example/".parse()?,
            issuer: provider.url.parse()?,
            relay_url: Some("https://relay.example/callback".parse()?),
            client_id: "terrier".into(),
            client_secret: provider::CLIENT_SECRET.into(),
            instance_admin_subjects: ["instance-admin".into()].into(),
        });
        let client = oidc::client(&config).await?;
        let application = application_routes(
            terrier_server::routes::routes().0,
            state.clone(),
            config.clone(),
            client.clone(),
        );
        let app = oidc::routes(config, client, state.clone())
            .nest(
                "/api",
                application.merge(terrier_server::hackathons::public_routes()),
            )
            .layer(session::layer(MemoryStore::default(), true))
            .layer(middleware::from_fn_with_state(
                locks::MemoryLocks::default(),
                session::lock::serialize::<locks::MemoryLocks>,
            ))
            .layer(middleware::from_fn(session::error_response))
            .with_state(state.clone());
        Ok(Self {
            db: state.db,
            app,
            provider,
            task,
            admin,
            schema,
        })
    }

    pub async fn send(&self, request: Request<Body>) -> anyhow::Result<Response> {
        Ok(self.app.clone().oneshot(request).await?)
    }

    /// Signs in through the real login and callback routes using a local OIDC provider.
    /// Distinct subjects create distinct users; repeated subjects sign in as the same user.
    pub async fn login(&self, subject: &str) -> anyhow::Result<TestSession> {
        let response = self
            .send(Request::get("/api/auth/login").body(Body::empty())?)
            .await?;
        ensure!(
            response.status() == StatusCode::SEE_OTHER,
            "login: {response:?}"
        );
        let pending = cookie(&response)?;
        let location: url::Url = response.headers()[LOCATION].to_str()?.parse()?;
        let query: std::collections::HashMap<_, _> = location.query_pairs().into_owned().collect();
        {
            let mut exchange = self.provider.exchange.lock().unwrap();
            exchange.nonce = query["nonce"].clone();
            exchange.challenge = query["code_challenge"].clone();
            exchange.redirect_uri = query["redirect_uri"].clone();
            exchange.overrides.insert("sub".into(), subject.into());
            exchange
                .overrides
                .insert("email".into(), format!("{subject}@example.com").into());
            exchange.overrides.insert("name".into(), subject.into());
        }
        let response = self
            .send(
                Request::get(format!(
                    "/api/auth/callback?code=test-code&state={}",
                    query["state"]
                ))
                .header(COOKIE, pending)
                .body(Body::empty())?,
            )
            .await?;
        ensure!(
            response.status() == StatusCode::SEE_OTHER,
            "callback: {response:?}"
        );
        let cookie = cookie(&response)?;
        let response = self
            .send(
                Request::get("/api/auth/status")
                    .header(COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        ensure!(
            response.status() == StatusCode::OK,
            "auth status: {response:?}"
        );
        let status: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
        Ok(TestSession {
            user_id: status["user"]["id"]
                .as_str()
                .context("missing user ID")?
                .parse()?,
            cookie,
            csrf_token: status["csrf_token"]
                .as_str()
                .context("missing CSRF token")?
                .into(),
        })
    }

    pub async fn close(self) -> anyhow::Result<()> {
        self.db.clone().close().await?;
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .await?;
        self.admin.clone().close().await?;
        Ok(())
    }
}

fn cookie(response: &Response) -> anyhow::Result<String> {
    Ok(response
        .headers()
        .get(SET_COOKIE)
        .context("missing session cookie")?
        .to_str()?
        .split(';')
        .next()
        .context("empty session cookie")?
        .into())
}
