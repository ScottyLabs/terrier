mod attributes;
mod config;
mod discovery;
mod error;
mod idp;
mod session;
mod sp;
mod state;

use axum::Router;
use config::Config;
use state::AppState;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let config = Config::from_env()?;
    let addr = format!("{}:{}", config.host, config.port);
    let state = Arc::new(AppState::new(config)?);

    tokio::spawn(session::session_cleanup_task(state.sessions.clone()));
    tokio::spawn(discovery::federation_index::federation_index_task(
        state.federation_index.clone(),
    ));

    let app = Router::new()
        .nest("/saml", idp::router())
        .nest("/sp", sp::router())
        .route("/discovery", axum::routing::get(discovery::discovery_page))
        .route(
            "/discovery",
            axum::routing::post(discovery::discovery_submit),
        )
        .route(
            "/api/entities/search",
            axum::routing::get(discovery::search_entities),
        )
        .with_state(state)
        .fallback_service(tower_http::services::ServeDir::new("static"))
        .layer(TraceLayer::new_for_http());

    tracing::info!("listening on {addr}");
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(terrier_common::shutdown_signal())
        .await?;

    Ok(())
}
