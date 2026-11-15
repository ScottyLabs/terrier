mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use common::TestApp;
use serde_json::{Value, json};

#[tokio::test]
async fn signed_in_user_can_read_their_profile() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let response = app
        .send(
            member
                .authorize(Request::get(format!("/api/users/{}", member.user_id)))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let profile: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    assert_eq!(
        profile,
        json!({"id": member.user_id, "email": "member@example.com", "display_name": "member"})
    );
    app.close().await
}

#[tokio::test]
async fn profile_requests_enforce_ownership_and_validate_ids() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let other = app.login("other").await?;
    for (path, expected, code) in [
        (
            format!("/api/users/{}", other.user_id),
            StatusCode::FORBIDDEN,
            "forbidden",
        ),
        (
            "/api/users/not-a-uuid".into(),
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
    ] {
        let response = app
            .send(member.authorize(Request::get(path)).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), expected);
        let error: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
        assert_eq!(error["code"], code);
    }
    let response = app
        .send(Request::get(format!("/api/users/{}", member.user_id)).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let error: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    assert_eq!(error["code"], "unauthenticated");
    app.close().await
}
