mod server;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        // The upstream callback span records the authorization code and state.
        .with(tracing_subscriber::filter::filter_fn(|metadata| {
            metadata.target() != "axum_oidc::handler"
        }))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    server::run().await
}
