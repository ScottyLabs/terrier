use std::{future::Future, time::Duration};

use axum::{
    extract::{Request, State},
    http::header::COOKIE,
    middleware::Next,
    response::Response,
};
use fred::{
    prelude::*,
    types::{Expiration, SetOptions},
};
use tower_sessions::{cookie::Cookie, session::Id};

use crate::error::ApiError;

const LEASE: Duration = Duration::from_secs(60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const LOCK_WAIT: Duration = Duration::from_secs(5);

pub trait SessionLockStore: Clone + Send + Sync + 'static {
    fn acquire(
        &self,
        key: &str,
        owner: &str,
    ) -> impl Future<Output = Result<bool, ApiError>> + Send;
    fn release(&self, key: &str, owner: &str) -> impl Future<Output = Result<(), ApiError>> + Send;
}

impl SessionLockStore for Pool {
    async fn acquire(&self, key: &str, owner: &str) -> Result<bool, ApiError> {
        let result: Option<String> = self
            .set(
                key,
                owner,
                Some(Expiration::EX(LEASE.as_secs() as i64)),
                Some(SetOptions::NX),
                false,
            )
            .await
            .map_err(|_| internal("failed to acquire session lock"))?;
        Ok(result.is_some())
    }

    async fn release(&self, key: &str, owner: &str) -> Result<(), ApiError> {
        let removed: i64 = self.eval(
            "if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('del', KEYS[1]) else return 0 end",
            vec![key], vec![owner],
        ).await.map_err(|_| internal("failed to release session lock"))?;
        if removed != 1 {
            return Err(internal(
                "session lock expired before the request completed",
            ));
        }
        Ok(())
    }
}

/// Must wrap the session manager, including its response-time session save.
pub async fn serialize<S: SessionLockStore>(
    State(store): State<S>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let id = request
        .headers()
        .get_all(COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .flat_map(|header| header.split(';'))
        .filter_map(|cookie| Cookie::parse_encoded(cookie.to_owned()).ok())
        .rfind(|cookie| cookie.name() == super::SESSION_COOKIE)
        .and_then(|cookie| cookie.value().parse::<Id>().ok());
    let Some(id) = id else {
        return Ok(next.run(request).await);
    };
    let key = format!("terrier:session-lock:{id}");
    let owner = uuid::Uuid::new_v4().to_string();
    tokio::time::timeout(LOCK_WAIT, async {
        while !store.acquire(&key, &owner).await? {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        Ok::<_, ApiError>(())
    })
    .await
    .map_err(|_| internal("session is busy; retry the request"))??;

    // The request deadline precedes the lease expiry so another replica cannot load stale credentials while this request is still saving the session.
    let response = tokio::time::timeout(REQUEST_TIMEOUT, next.run(request))
        .await
        .map_err(|_| internal("session request timed out"));
    let released = store.release(&key, &owner).await;
    match response {
        Ok(response) => {
            released?;
            Ok(response)
        }
        Err(error) => {
            if released.is_err() {
                tracing::error!("failed to release session lock after request timeout");
            }
            Err(error)
        }
    }
}

fn internal(message: &'static str) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(message))
}

#[cfg(test)]
#[path = "../../../tests/common/locks.rs"]
mod test_locks;
#[cfg(test)]
pub(crate) use test_locks::MemoryLocks;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
    };
    use tower::ServiceExt;

    #[tokio::test(start_paused = true)]
    async fn request_timeout_releases_the_lock() {
        let store = MemoryLocks::default();
        let id = Id::default();
        let app = Router::new()
            .route(
                "/",
                get(|| async {
                    tokio::time::sleep(REQUEST_TIMEOUT + Duration::from_secs(1)).await;
                }),
            )
            .layer(middleware::from_fn_with_state(
                store.clone(),
                serialize::<MemoryLocks>,
            ));
        let response = app
            .oneshot(
                Request::get("/")
                    .header(COOKIE, format!("terrier_session={id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            store
                .acquire(&format!("terrier:session-lock:{id}"), "next")
                .await
                .unwrap()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn lock_contention_is_bounded_and_does_not_run_the_handler() {
        async fn unexpected_handler() -> StatusCode {
            panic!("handler ran without a lock");
        }

        let store = MemoryLocks::default();
        let id = Id::default();
        let key = format!("terrier:session-lock:{id}");
        assert!(store.acquire(&key, "other").await.unwrap());
        let app = Router::new().route("/", get(unexpected_handler)).layer(
            middleware::from_fn_with_state(store.clone(), serialize::<MemoryLocks>),
        );
        let response = app
            .oneshot(
                Request::get("/")
                    .header(COOKIE, format!("terrier_session={id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        store.release(&key, "other").await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires Valkey and VALKEY_URL"]
    async fn valkey_locks_exclude_other_owners_and_release_atomically() -> anyhow::Result<()> {
        let (_, first) = crate::auth::session::connect(&std::env::var("VALKEY_URL")?).await?;
        let (_, second) = crate::auth::session::connect(&std::env::var("VALKEY_URL")?).await?;
        let key = format!("terrier:session-lock:{}", Id::default());
        assert!(
            SessionLockStore::acquire(&first, &key, "first")
                .await
                .unwrap()
        );
        assert!(
            !SessionLockStore::acquire(&second, &key, "second")
                .await
                .unwrap()
        );
        assert!(second.release(&key, "second").await.is_err());
        first
            .release(&key, "first")
            .await
            .map_err(|_| anyhow::anyhow!("release failed"))?;
        assert!(
            SessionLockStore::acquire(&second, &key, "second")
                .await
                .unwrap()
        );
        second
            .release(&key, "second")
            .await
            .map_err(|_| anyhow::anyhow!("release failed"))?;
        Ok(())
    }
}
