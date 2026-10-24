use axum::{
    Extension, Json, Router,
    extract::{FromRequestParts, Path},
    http::{StatusCode, request::Parts},
    routing::get,
};
use serde::Serialize;
use slac::{Authorized, Policy};

// `Clone` lets Axum copy this demo user into requests; `Serialize` enables JSON output.
#[derive(Clone, Serialize)]
struct User {
    id: u32,
    name: &'static str,
}

struct IsProfileOwner;

// `()` means this policy needs no shared application state.
impl Policy<()> for IsProfileOwner {
    // The checked user becomes the handler's `Authorized::data`.
    type Output = User;
    // Returning an HTTP status rejects the request without running the handler.
    type Error = StatusCode;

    async fn check(parts: &mut Parts, state: &()) -> Result<User, StatusCode> {
        // Extensions hold request context, looked up by Rust type. Retrieve the
        // `User` attached below; `let Extension(user)` unwraps it into `user`.
        let Extension(user) = Extension::<User>::from_request_parts(parts, state)
            .await
            // `map_err` turns a missing extension into 401; `?` returns that error early.
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        // `Path::<u32>` parses `{user_id}` from the URL as an unsigned integer.
        let Path(user_id) = Path::<u32>::from_request_parts(parts, state)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        if user.id != user_id {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(user)
    }
}

// Axum runs the policy through this extractor before entering the handler.
// `data: user` names the checked output `user`; `..` ignores the other fields.
// For another protected route, choose its policy here and use that policy's output.
async fn profile(Authorized { data: user, .. }: Authorized<IsProfileOwner>) -> Json<User> {
    Json(user)
}

fn app() -> Router {
    Router::new()
        // Connect GET requests at this path to the `profile` handler.
        .route("/users/{user_id}", get(profile))
        // A layer adds request processing around these routes. This one attaches
        // a fixed `User` to each request; it does not authenticate anyone.
        // Real authentication middleware would attach the verified caller instead.
        .layer(Extension(User {
            id: 1,
            name: "Alex",
        }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
    axum::serve(listener, app()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn profile_requires_ownership() {
        for (path, expected) in [
            ("/users/1", StatusCode::OK),
            ("/users/2", StatusCode::FORBIDDEN),
            ("/users/invalid", StatusCode::BAD_REQUEST),
        ] {
            let response = app()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
            if expected == StatusCode::OK {
                assert_eq!(
                    to_bytes(response.into_body(), 1024).await.unwrap(),
                    r#"{"id":1,"name":"Alex"}"#
                );
            }
        }
    }

    #[tokio::test]
    async fn missing_user_is_unauthorized() {
        let response = Router::new()
            .route("/users/{user_id}", get(profile))
            .oneshot(
                Request::builder()
                    .uri("/users/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
