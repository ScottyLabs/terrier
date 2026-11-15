mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use common::TestApp;
use serde_json::Value;

#[tokio::test]
async fn sessions_use_real_csrf_checks_and_logout() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let again = app.login("member").await?;
    assert_eq!(member.user_id, again.user_id);
    let response = app
        .send(
            Request::post("/api/auth/logout")
                .header("cookie", &member.cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .send(
            member
                .authorize(Request::post("/api/auth/logout"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = app
        .send(
            member
                .authorize(Request::get(format!("/api/users/{}", member.user_id)))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    // Signing out one session must not sign out another session for the same user.
    let response = app
        .send(
            again
                .authorize(Request::get(format!("/api/users/{}", again.user_id)))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    app.close().await
}

#[tokio::test]
async fn fixtures_isolate_users_and_sessions() -> anyhow::Result<()> {
    let first = TestApp::new().await?;
    let second = TestApp::new().await?;
    let first_user = first.login("member").await?;
    let second_user = second.login("member").await?;
    assert_ne!(first_user.user_id, second_user.user_id);
    let response = second
        .send(
            first_user
                .authorize(Request::get(format!("/api/users/{}", first_user.user_id)))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    first.close().await?;
    second.close().await
}

#[tokio::test]
async fn database_failure_returns_an_error() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    app.db.clone().close().await?;
    let response = app
        .send(
            member
                .authorize(Request::get(format!("/api/users/{}", member.user_id)))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let error: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    assert_eq!(error["code"], "internal_error");
    app.close().await
}
