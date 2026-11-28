mod server;

use anyhow::Context;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(error) = dotenvy::dotenv()
        && !error.not_found()
    {
        match error {
            dotenvy::Error::Io(error) => return Err(error).context("read .env"),
            _ => anyhow::bail!("invalid .env contents"),
        }
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    server::run().await
}
