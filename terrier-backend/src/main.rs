use axum::{
    BoxError, Router,
    error_handling::HandleErrorLayer,
    http::Uri,
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use axum_oidc::{
    EmptyAdditionalClaims, OidcAuthLayer, OidcClient, OidcLoginLayer, error::MiddlewareError,
    handle_oidc_redirect,
};
use sea_orm::DatabaseConnection;
use tokio::signal;
use tower::ServiceBuilder;
use tower_sessions::{
    Expiry, MemoryStore, SessionManagerLayer,
    cookie::{SameSite, time::Duration},
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod auth;
mod config;
mod docs;
mod entities;
mod hackathons;

use config::Config;
use docs::ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
}

pub async fn create_app(app_state: AppState) -> Result<Router, BoxError> {
    // Set up session management
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

    let oidc_login_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
            tracing::error!("OIDC Login error: {:?}", e);
            e.into_response()
        }))
        .layer(OidcLoginLayer::<EmptyAdditionalClaims>::new());

    let config = app_state.config.clone();
    let redirect_url = format!("{}/auth/callback", config.api_url);

    let oidc_client = OidcClient::<EmptyAdditionalClaims>::builder()
        .with_default_http_client()
        .with_redirect_url(Uri::try_from(redirect_url).expect("valid API_URL"))
        .with_client_id(config.oidc_client_id)
        .with_client_secret(config.oidc_client_secret)
        .with_scopes(["openid", "email", "profile"].into_iter())
        .discover(config.oidc_issuer)
        .await
        .unwrap()
        .build();

    let oidc_auth_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
            tracing::error!("OIDC Auth error: {:?}", e);
            e.into_response()
        }))
        .layer(OidcAuthLayer::new(oidc_client));

    let router = Router::new()
        // Protected routes
        .route(
            "/api/hackathons/{slug}/role",
            get(hackathons::handlers::get_user_role),
        )
        .route(
            "/api/hackathons",
            post(hackathons::handlers::create_hackathon),
        )
        .route("/api/auth/login", get(auth::handlers::login))
        .route("/api/auth/logout", get(auth::handlers::logout))
        // OIDC authentication layer
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::middleware::sync_user_middleware,
        ))
        .layer(oidc_login_service)
        // Public routes
        .route(
            "/api/hackathons/public",
            get(hackathons::handlers::list_public_hackathons),
        )
        .route("/api/auth/status", get(auth::handlers::status))
        .route(
            "/api/auth/callback",
            get(handle_oidc_redirect::<EmptyAdditionalClaims>),
        )
        .route(
            "/api/",
            get(|| async { "Visit /api/swagger for API documentation" }),
        )
        .route("/api/health", get(|| async { "OK" }))
        .merge(SwaggerUi::new("/api/swagger").url("/api/openapi.json", ApiDoc::openapi()))
        // Middleware layers
        .layer(oidc_auth_service)
        .layer(session_layer)
        .with_state(app_state);

    Ok(router)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(
            |_| "axum_oidc=debug,tower_sessions=debug,sqlx=warn,tower_http=debug".into(),
        )))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    let config = Config::from_env()?;
    let app_state = AppState {
        db: sea_orm::Database::connect(&config.database_url).await?,
        config,
    };

    let app = create_app(app_state).await.unwrap();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    println!("Listening on http://0.0.0.0:3000");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
