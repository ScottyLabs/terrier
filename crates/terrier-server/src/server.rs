use std::{path::Path, sync::Arc};

use anyhow::Context;
use axum::Router;
use terrier_server::auth::{config::AuthConfig, middleware::application_routes, oidc, session};
use terrier_server::state::AppState;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

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

fn router(assets: Option<&Path>, api_routes: Router<AppState>) -> Router<AppState> {
    let (router, mut api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes!(health))
        .split_for_parts();
    api.merge(oidc::AuthApiDoc::openapi());
    api.merge(terrier_server::profile::ProfileApiDoc::openapi());

    let mut app = router
        .merge(api_routes)
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", api));

    if let Some(assets) = assets {
        app = app.fallback_service(tower_http::services::ServeDir::new(assets).fallback(
            tower_http::services::ServeFile::new(assets.join("index.html")),
        ));
    }

    app
}

pub async fn run() -> anyhow::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("{host}:{port}");

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let config = Arc::new(AuthConfig::from_env()?);
    let valkey_url = std::env::var("VALKEY_URL").context("VALKEY_URL must be set")?;
    let state = AppState::connect(database_url).await?;
    let sessions = session::connect(&valkey_url).await?;
    let client = oidc::client(&config).await?;
    let metadata_client = client.clone();
    let application = application_routes(
        terrier_server::profile::routes(),
        state.clone(),
        config.clone(),
        client.clone(),
    );
    let auth = oidc::routes(config.clone(), client, state.clone())
        .nest("/api", application)
        .layer(session::layer(sessions, config.secure_cookies()))
        .layer(axum::middleware::from_fn(session::error_response));

    let assets = Path::new("assets");
    let app = router(assets.exists().then_some(assets), auth)
        .with_state(state)
        .layer(TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
            tracing::info_span!("request", method = %request.method(), path = request.uri().path())
        }));

    tracing::info!("listening on {addr}");
    let listener = TcpListener::bind(&addr).await?;
    let metadata_refresh = tokio::spawn(oidc::refresh_metadata(
        metadata_client,
        axum_oidc::openidconnect::IssuerUrl::new(config.issuer.to_string())?,
    ));
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(terrier_common::shutdown_signal())
        .await;
    metadata_refresh.abort();
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

    fn router(assets: Option<&Path>) -> axum::Router {
        super::router(
            assets,
            axum::Router::new().nest(
                "/api",
                axum::Router::new()
                    .fallback(|| async { terrier_server::error::ApiError::NotFound }),
            ),
        )
        .with_state(AppState {
            db: sea_orm::DatabaseConnection::Disconnected,
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
    async fn openapi_includes_health() {
        let response = router(None)
            .oneshot(Request::get("/openapi.json").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let body = std::str::from_utf8(&body).unwrap();
        assert!(body.contains("\"/api/health\""));
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
    async fn swagger_ui_is_available() {
        let response = router(None)
            .oneshot(Request::get("/swagger-ui/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "text/html");
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
