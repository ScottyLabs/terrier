use saml_proxy::config::Config;
use saml_proxy::state::AppState;
use std::sync::Arc;
use tokio::net::TcpListener;
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

    tokio::spawn(saml_proxy::session::session_cleanup_task(
        state.sessions.clone(),
    ));
    tokio::spawn(
        saml_proxy::discovery::federation_index::federation_index_task(
            state.federation_index.clone(),
        ),
    );

    let app = saml_proxy::app(state, "static");

    tracing::info!("listening on {addr}");
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(terrier_common::shutdown_signal())
        .await?;

    Ok(())
}
