use std::time::Duration;

use crate::error::ErrorResponse;
use anyhow::Context;
use axum::{
    Json,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use fred::prelude::{ClientLike, Config, Pool};
use tower_sessions::{
    Expiry, SessionManagerLayer,
    cookie::{SameSite, time},
};
use tower_sessions_redis_store::RedisStore;

pub mod lock;

pub const OIDC_KEY: &str = "oidc_v2";
pub const SESSION_COOKIE: &str = "terrier_session";

pub fn layer<S: tower_sessions::SessionStore>(store: S, secure: bool) -> SessionManagerLayer<S> {
    SessionManagerLayer::new(store)
        .with_name(SESSION_COOKIE)
        .with_path("/")
        .with_http_only(true)
        .with_secure(secure)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(48)))
        .with_always_save(true)
}

pub async fn connect(url: &str) -> anyhow::Result<(RedisStore<Pool>, Pool)> {
    let config = Config::from_url(url).context("invalid VALKEY_URL")?;
    let performance = fred::types::config::PerformanceConfig {
        default_command_timeout: Duration::from_secs(5),
        ..Default::default()
    };
    let pool = Pool::new(config, Some(performance), None, None, 1)
        .context("failed to configure Valkey sessions")?;
    pool.connect();
    tokio::time::timeout(Duration::from_secs(10), pool.wait_for_connect())
        .await
        .context("timed out connecting to Valkey")?
        .context("failed to connect to Valkey")?;
    Ok((RedisStore::new(pool.clone()), pool))
}

pub async fn error_response(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    // tower-sessions logs save failures but returns an empty 500 response.
    if response.status() == StatusCode::INTERNAL_SERVER_ERROR
        && !response
            .headers()
            .contains_key(axum::http::header::CONTENT_TYPE)
    {
        response = (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "internal_error",
                message: "An unexpected error occurred.".into(),
                fields: Default::default(),
            }),
        )
            .into_response();
    }
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::Request,
        routing::get,
    };
    use tower::ServiceExt;
    use tower_sessions::Session;

    #[tokio::test]
    async fn unavailable_valkey_rejects_startup_and_session_writes() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let url = format!("redis://{address}");
        assert!(connect(&url).await.is_err());
        let config = Config::from_url(&url).unwrap();
        let performance = fred::types::config::PerformanceConfig {
            default_command_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let pool = Pool::new(config, Some(performance), None, None, 1).unwrap();
        pool.connect();
        assert!(pool.wait_for_connect().await.is_err());
        let app = Router::new()
            .route(
                "/",
                get(|session: Session| async move {
                    session.insert("value", true).await.unwrap();
                    StatusCode::OK
                }),
            )
            .layer(layer(RedisStore::new(pool), false))
            .layer(axum::middleware::from_fn(error_response));
        let response = tokio::time::timeout(
            Duration::from_secs(2),
            app.oneshot(Request::get("/").body(Body::empty()).unwrap()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["code"],
            "internal_error"
        );
    }
}
