#![expect(
    clippy::tests_outside_test_module,
    reason = "Cargo compiles this integration test target only as a test"
)]

#[cfg(test)]
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

async fn rename(
    app: &TestApp,
    session: &common::TestSession,
    user_id: uuid::Uuid,
    body: &str,
) -> anyhow::Result<(StatusCode, Value)> {
    let response = app
        .send(
            session
                .authorize(Request::patch(format!("/api/users/{user_id}")))
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))?,
        )
        .await?;
    let status = response.status();
    Ok((
        status,
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?,
    ))
}

#[tokio::test]
async fn signed_in_user_can_rename_themselves() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let (status, profile) = rename(
        &app,
        &member,
        member.user_id,
        r#"{"display_name":"  Scotty  "}"#,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        profile,
        json!({"id": member.user_id, "email": "member@example.com", "display_name": "Scotty"})
    );
    let again = app.login("member").await?;
    let response = app
        .send(
            again
                .authorize(Request::get(format!("/api/users/{}", again.user_id)))
                .body(Body::empty())?,
        )
        .await?;
    let profile: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    assert_eq!(profile["display_name"], "Scotty");
    app.close().await
}

#[tokio::test]
async fn profile_updates_validate_input_and_ownership() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let other = app.login("other").await?;
    let long = format!(r#"{{"display_name":"{}"}}"#, "a".repeat(101));
    for (user_id, body, expected, code) in [
        (
            other.user_id,
            r#"{"display_name":"Taken"}"#,
            StatusCode::FORBIDDEN,
            "forbidden",
        ),
        (
            member.user_id,
            r#"{"display_name":"   "}"#,
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
        ),
        (
            member.user_id,
            long.as_str(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
        ),
        (
            member.user_id,
            r#"{"email":"new@example.com"}"#,
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            member.user_id,
            "not json",
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
    ] {
        let (status, error) = rename(&app, &member, user_id, body).await?;
        assert_eq!(status, expected, "{body}");
        assert_eq!(error["code"], code, "{body}");
    }
    let response = app
        .send(
            Request::patch(format!("/api/users/{}", member.user_id))
                .header("cookie", &member.cookie)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"display_name":"No token"}"#))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let (_, profile) = rename(&app, &other, other.user_id, "{}").await?;
    assert_eq!(profile["display_name"], "other");
    app.close().await
}
