use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use axum::{Extension, Json, Router, routing::get};
use terrier_server::auth::{config::AuthConfig, middleware::application_routes, oidc, session};
use terrier_server::{state::AppState, storage::Storage};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(info(
    title = "Terrier API",
    description = "Hackathon management platform",
    license(name = "AGPL-3.0-or-later"),
))]
struct ApiDoc;

#[utoipa::path(get, path = "/api/health", responses((status = OK, body = str)))]
async fn health() -> &'static str {
    "ok"
}

fn router(
    assets: Option<&Path>,
    api_routes: Router<AppState>,
    application_api: utoipa::openapi::OpenApi,
) -> Router<AppState> {
    let (router, mut api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes!(health))
        .split_for_parts();
    api.merge(oidc::AuthApiDoc::openapi());
    api.merge(application_api);

    let spec = api.clone();
    let mut app = router
        .merge(api_routes)
        .route(
            "/openapi.json",
            get(move || {
                let spec = spec.clone();
                async move { Json(spec) }
            }),
        )
        .merge(Scalar::with_url("/scalar", api));

    if let Some(assets) = assets {
        app = app.fallback_service(tower_http::services::ServeDir::new(assets).fallback(
            tower_http::services::ServeFile::new(assets.join("index.html")),
        ));
    }

    app
}

fn static_assets(configured: Option<OsString>) -> anyhow::Result<Option<PathBuf>> {
    let explicit = configured.is_some();
    let path = PathBuf::from(configured.unwrap_or_else(|| "assets".into()));
    if !explicit && !path.exists() {
        return Ok(None);
    }
    anyhow::ensure!(
        !path.as_os_str().is_empty() && path.join("index.html").is_file(),
        "Static assets directory {} must contain index.html",
        path.display()
    );
    Ok(Some(path))
}

pub async fn run() -> anyhow::Result<()> {
    let assets = static_assets(std::env::var_os("STATIC_DIR"))?;
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("{host}:{port}");

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let config = Arc::new(AuthConfig::from_env()?);
    let valkey_url = std::env::var("VALKEY_URL").context("VALKEY_URL must be set")?;
    let storage = Storage::from_env()?;
    storage.check().await?;
    let state = AppState::connect(database_url).await?;
    let (sessions, locks) = session::connect(&valkey_url).await?;
    let client = oidc::client(&config).await?;
    let metadata_client = client.clone();
    let (routes, application_api) = terrier_server::routes::routes();
    let application = application_routes(routes, state.clone(), config.clone(), client.clone());
    let auth = oidc::routes(config.clone(), client, state.clone())
        .nest(
            "/api",
            application.merge(terrier_server::hackathons::public_routes()),
        )
        .layer(session::layer(sessions, config.secure_cookies()))
        .layer(axum::middleware::from_fn_with_state(
            locks,
            session::lock::serialize::<fred::prelude::Pool>,
        ))
        .layer(axum::middleware::from_fn(session::error_response));

    let db = state.db.clone();
    let app = router(assets.as_deref(), auth, application_api)
        .with_state(state)
        .layer(Extension(storage.clone()))
        .layer(TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
            tracing::info_span!("request", method = %request.method(), path = request.uri().path())
        }));

    tracing::info!("listening on {addr}");
    let listener = TcpListener::bind(&addr).await?;
    let metadata_refresh = tokio::spawn(oidc::refresh_metadata(metadata_client));
    let upload_cleanup = tokio::spawn(terrier_server::files::remove_unused_uploads_hourly(
        db, storage,
    ));
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(terrier_common::shutdown_signal())
        .await;
    metadata_refresh.abort();
    upload_cleanup.abort();
    result?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::CONTENT_TYPE},
    };
    use tower::ServiceExt;

    use super::AppState;

    #[test]
    fn configured_static_assets_require_index() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let assets = fixtures.join("assets");
        assert_eq!(
            super::static_assets(Some(assets.clone().into_os_string())).unwrap(),
            Some(assets)
        );
        for invalid in [
            fixtures.clone(),
            fixtures.join("missing"),
            Path::new("").into(),
        ] {
            super::static_assets(Some(invalid.into_os_string())).expect_err("invalid asset path");
        }
    }

    fn router(assets: Option<&Path>) -> axum::Router {
        super::router(
            assets,
            axum::Router::new().nest(
                "/api",
                axum::Router::new()
                    .fallback(|| async { terrier_server::error::ApiError::NotFound }),
            ),
            terrier_server::routes::routes().1,
        )
        .with_state(AppState {
            db: sea_orm::DatabaseConnection::default(),
        })
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let response = router(None)
            .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(body.as_ref(), b"ok");
    }

    #[tokio::test]
    async fn openapi_includes_registered_routes() {
        let response = router(None)
            .oneshot(Request::get("/openapi.json").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let body = std::str::from_utf8(&body).unwrap();
        assert!(body.contains("\"/api/health\""));
        assert!(body.contains("\"/api/users/{user_id}\""));
    }

    #[tokio::test]
    async fn unknown_api_route_returns_not_found_without_assets() {
        let response = router(None)
            .oneshot(Request::get("/api/missing").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn scalar_is_available() {
        let response = router(None)
            .oneshot(Request::get("/scalar").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers()[CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
    }

    #[tokio::test]
    async fn static_files_and_client_routes_serve_assets() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assets");
        let app = router(Some(&assets));

        for path in ["/index.html", "/dashboard"] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK, "{path}");
            let body = to_bytes(response.into_body(), 1024).await.unwrap();
            assert_eq!(
                body.as_ref(),
                include_bytes!("../tests/fixtures/assets/index.html")
            );
        }

        let missing = app
            .clone()
            .oneshot(Request::get("/api/missing").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert_eq!(missing.headers()[CONTENT_TYPE], "application/json");

        let response = app
            .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(body.as_ref(), b"ok");
    }
}
