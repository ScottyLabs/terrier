use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
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

#[utoipa::path(get, path = "/health", responses((status = OK, body = str)))]
async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("{host}:{port}");

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes!(health))
        .split_for_parts();

    let mut app = router
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", api))
        .layer(TraceLayer::new_for_http());

    if std::path::Path::new("assets").exists() {
        app = app.fallback_service(
            tower_http::services::ServeDir::new("assets")
                .fallback(tower_http::services::ServeFile::new("assets/index.html")),
        );
    }

    tracing::info!("listening on {addr}");
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(terrier_common::shutdown_signal())
        .await?;

    Ok(())
}
