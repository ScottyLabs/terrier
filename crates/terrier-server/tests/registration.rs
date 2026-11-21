mod common;

use std::collections::BTreeSet;

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use common::{TestApp, TestSession};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use terrier_server::{audit::Scope, error::ApiError};
use uuid::Uuid;

/// Writes fixture rows through an automatic audited transaction, as the triggers require.
async fn fixture(app: &TestApp, scope: Scope, sql: String) -> anyhow::Result<()> {
    terrier_server::audit::automatic(&app.db, scope, "Integration test fixture", |tx| {
        Box::pin(async move {
            tx.execute_unprepared(&sql)
                .await
                .map_err(|error| ApiError::Internal(error.into()))?;
            Ok(())
        })
    })
    .await
    .map_err(|error| anyhow::anyhow!("{error:?}"))
}

/// The fixture form: one optional text question, `school`.
const FORM: &str =
    r#"{"fields":[{"id":"school","label":"School","type":"text","required":false}]}"#;

/// Creates an active hackathon whose application deadline is `deadline` from now.
async fn hackathon(app: &TestApp, slug: &str, deadline: &str) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    fixture(app, Scope::Global, format!(
        "INSERT INTO hackathon(id,slug,name,description,is_active,max_team_size,max_participants,location,contact_email,application_schema,data,directory_fields,application_deadline,team_formation_deadline,submission_deadline)
            VALUES('{id}','{slug}','Fixture','Fixture',true,4,100,'Campus','fixture@example.com','{FORM}','{{}}','[\"school\"]',clock_timestamp()+interval '{deadline}',clock_timestamp()+interval '2 days',clock_timestamp()+interval '3 days')"
    )).await?;
    Ok(id)
}

async fn organizer(
    app: &TestApp,
    hackathon: Uuid,
    user: Uuid,
    capabilities: &str,
) -> anyhow::Result<()> {
    fixture(app, Scope::HackathonExclusive(hackathon), format!(
        "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) VALUES('{user}','{hackathon}','organizer');
         INSERT INTO organizer(user_id,hackathon_id,capabilities) VALUES('{user}','{hackathon}',ARRAY[{capabilities}]::capability[])"
    )).await
}

/// Adds a hacker whose application has been accepted and confirmed.
async fn attendee(
    app: &TestApp,
    hackathon: Uuid,
    user: Uuid,
    opt_in: bool,
    profile: Value,
) -> anyhow::Result<()> {
    fixture(app, Scope::HackathonExclusive(hackathon), format!(
        "INSERT INTO user_hackathon_role(user_id,hackathon_id,role,directory_opt_in,profile_data) VALUES('{user}','{hackathon}','hacker',{opt_in},'{profile}')"
    )).await?;
    fixture(app, Scope::Hackathon(hackathon), format!(
        "INSERT INTO application(id,user_id,hackathon_id,status,data,submitted_at,attendance_status)
            VALUES(gen_random_uuid(),'{user}','{hackathon}','accepted','{{\"school\":\"CMU\"}}',clock_timestamp(),'confirmed')"
    )).await
}

async fn send(
    app: &TestApp,
    session: Option<&TestSession>,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> anyhow::Result<(StatusCode, Value)> {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(session) = session {
        request = session.authorize(request);
    }
    let request = match body {
        Some(body) => request
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))?,
        None => request.body(Body::empty())?,
    };
    let response = app.send(request).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20).await?;
    Ok((status, serde_json::from_slice(&bytes)?))
}

async fn put(
    app: &TestApp,
    session: &TestSession,
    path: &str,
    body: Value,
) -> anyhow::Result<(StatusCode, Value)> {
    send(app, Some(session), Method::PUT, path, Some(body)).await
}

async fn get(
    app: &TestApp,
    session: Option<&TestSession>,
    path: &str,
) -> anyhow::Result<(StatusCode, Value)> {
    send(app, session, Method::GET, path, None).await
}

fn user_ids(body: &Value) -> BTreeSet<String> {
    body["applications"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|application| application["user_id"].as_str().map(str::to_owned))
        .collect()
}

fn ids<const N: usize>(users: [Uuid; N]) -> BTreeSet<String> {
    users.iter().map(Uuid::to_string).collect()
}

async fn scalar(app: &TestApp, sql: &str) -> anyhow::Result<i64> {
    Ok(app
        .db
        .query_one_raw(Statement::from_string(DbBackend::Postgres, sql))
        .await?
        .ok_or_else(|| anyhow::anyhow!("missing scalar"))?
        .try_get("", "value")?)
}

#[tokio::test]
async fn applicant_saves_a_draft_then_submits_once() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    hackathon(&app, "tartanhacks", "1 day").await?;
    let path = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );

    let (status, _) = get(&app, Some(&hacker), &path).await?;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, draft) = put(&app, &hacker, &path, json!({"data": {"school": "CM"}})).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(draft["status"], Value::Null);
    assert_eq!(draft["draft_data"], json!({"school": "CM"}));
    assert_eq!(draft["data"], json!({}));
    assert_eq!(draft["submitted_at"], Value::Null);
    let role = scalar(&app, &format!(
        "SELECT count(*) AS value FROM user_hackathon_role WHERE user_id='{}' AND role='hacker' AND is_active",
        hacker.user_id
    )).await?;
    assert_eq!(role, 1);

    let (status, submitted) = put(
        &app,
        &hacker,
        &path,
        json!({"data": {"school": "CMU"}, "submit": true}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(submitted["status"], "pending");
    assert_eq!(submitted["data"], json!({"school": "CMU"}));
    assert_eq!(submitted["draft_data"], json!({"school": "CMU"}));
    assert!(submitted["submitted_at"].is_string());

    let revisions = "SELECT count(*) AS value FROM audit_event WHERE resource_type='application'";
    let before = scalar(&app, revisions).await?;
    let (status, retried) = put(
        &app,
        &hacker,
        &path,
        json!({"data": {"school": "CMU"}, "submit": true}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(retried["submitted_at"], submitted["submitted_at"]);
    assert_eq!(scalar(&app, revisions).await?, before);

    let (status, autosaved) = put(
        &app,
        &hacker,
        &path,
        json!({"data": {"school": "Carnegie Mellon"}}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(autosaved["data"], json!({"school": "CMU"}));
    assert_eq!(
        autosaved["draft_data"],
        json!({"school": "Carnegie Mellon"})
    );
    assert_eq!(autosaved["submitted_at"], submitted["submitted_at"]);

    let (status, read) = get(&app, Some(&hacker), &path).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read, autosaved);
    app.close().await
}

#[tokio::test]
async fn application_writes_enforce_ownership_and_input() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let other = app.login("other").await?;
    hackathon(&app, "open", "1 day").await?;
    hackathon(&app, "closed", "-1 minute").await?;
    let own = format!("/api/hackathons/open/applications/{}", hacker.user_id);
    let body = json!({"data": {"school": "CMU"}, "submit": true});

    for (session, path, body, expected, code) in [
        (
            Some(&hacker),
            format!("/api/hackathons/open/applications/{}", other.user_id),
            body.clone(),
            StatusCode::FORBIDDEN,
            "forbidden",
        ),
        (
            None,
            own.clone(),
            body.clone(),
            StatusCode::UNAUTHORIZED,
            "unauthenticated",
        ),
        (
            Some(&hacker),
            "/api/hackathons/open/applications/not-a-uuid".into(),
            body.clone(),
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            Some(&hacker),
            format!("/api/hackathons/missing/applications/{}", hacker.user_id),
            body.clone(),
            StatusCode::NOT_FOUND,
            "not_found",
        ),
        (
            Some(&hacker),
            format!("/api/hackathons/closed/applications/{}", hacker.user_id),
            body.clone(),
            StatusCode::CONFLICT,
            "conflict",
        ),
        (
            Some(&hacker),
            own.clone(),
            json!({"data": ["school"]}),
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
        ),
        (
            Some(&hacker),
            own.clone(),
            json!({"data": {}, "status": "accepted"}),
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
    ] {
        let (status, error) = send(&app, session, Method::PUT, &path, Some(body.clone())).await?;
        assert_eq!(status, expected, "{path} {body}");
        assert_eq!(error["code"], code, "{path} {body}");
    }
    assert_eq!(
        scalar(&app, "SELECT count(*) AS value FROM application").await?,
        0
    );

    app.close().await
}

#[tokio::test]
async fn decided_applications_are_read_only() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let id = hackathon(&app, "decided", "1 day").await?;
    let path = format!("/api/hackathons/decided/applications/{}", hacker.user_id);
    put(
        &app,
        &hacker,
        &path,
        json!({"data": {"school": "CMU"}, "submit": true}),
    )
    .await?;
    fixture(
        &app,
        Scope::Hackathon(id),
        format!(
            "UPDATE application SET status='rejected' WHERE user_id='{}'",
            hacker.user_id
        ),
    )
    .await?;
    let (status, error) = put(&app, &hacker, &path, json!({"data": {"school": "Changed"}})).await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["code"], "conflict");
    let (_, read) = get(&app, Some(&hacker), &path).await?;
    assert_eq!(read["status"], "rejected");
    assert_eq!(read["draft_data"], json!({"school": "CMU"}));
    app.close().await
}

#[tokio::test]
async fn reviewers_see_applications_by_capability() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let manager = app.login("manager").await?;
    let staff = app.login("staff").await?;
    let drafter = app.login("drafter").await?;
    let pending = app.login("pending").await?;
    let confirmed = app.login("confirmed").await?;
    let id = hackathon(&app, "review", "1 day").await?;
    let elsewhere = hackathon(&app, "elsewhere", "1 day").await?;
    organizer(&app, id, manager.user_id, "'application_management'").await?;
    organizer(&app, id, staff.user_id, "").await?;
    organizer(&app, elsewhere, drafter.user_id, "'hackathon_admin'").await?;
    attendee(&app, id, confirmed.user_id, false, json!({})).await?;
    for (session, submit) in [(&drafter, false), (&pending, true)] {
        let path = format!("/api/hackathons/review/applications/{}", session.user_id);
        let (status, _) = put(
            &app,
            session,
            &path,
            json!({"data": {"school": "CMU"}, "submit": submit}),
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
    }

    let list = "/api/hackathons/review/applications";
    let (status, body) = get(&app, Some(&manager), list).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(user_ids(&body), ids([pending.user_id, confirmed.user_id]));
    assert!(body["applications"][0].get("draft_data").is_none());

    let (_, body) = get(&app, Some(&manager), &format!("{list}?status=pending")).await?;
    assert_eq!(user_ids(&body), ids([pending.user_id]));

    let (status, body) = get(&app, Some(&staff), list).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(user_ids(&body), ids([confirmed.user_id]));

    for (path, expected) in [
        (format!("{list}/{}", pending.user_id), StatusCode::OK),
        (format!("{list}/{}", drafter.user_id), StatusCode::NOT_FOUND),
    ] {
        let (status, _) = get(&app, Some(&manager), &path).await?;
        assert_eq!(status, expected, "{path}");
    }
    for (path, expected) in [
        (format!("{list}/{}", confirmed.user_id), StatusCode::OK),
        (format!("{list}/{}", pending.user_id), StatusCode::NOT_FOUND),
    ] {
        let (status, _) = get(&app, Some(&staff), &path).await?;
        assert_eq!(status, expected, "{path}");
    }

    for (session, path, expected) in [
        (&pending, list.to_owned(), StatusCode::FORBIDDEN),
        (
            &pending,
            format!("{list}/{}", confirmed.user_id),
            StatusCode::FORBIDDEN,
        ),
        (&drafter, list.to_owned(), StatusCode::FORBIDDEN),
        (
            &manager,
            format!("{list}?status=maybe"),
            StatusCode::BAD_REQUEST,
        ),
        (
            &manager,
            "/api/hackathons/elsewhere/applications".to_owned(),
            StatusCode::FORBIDDEN,
        ),
    ] {
        let (status, _) = get(&app, Some(session), &path).await?;
        assert_eq!(status, expected, "{path}");
    }
    app.close().await
}

#[tokio::test]
async fn directory_shows_opted_in_participants_to_attendees() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let visible = app.login("visible").await?;
    let hidden = app.login("hidden").await?;
    let applicant = app.login("applicant").await?;
    let staff = app.login("staff").await?;
    let id = hackathon(&app, "directory", "1 day").await?;
    attendee(
        &app,
        id,
        visible.user_id,
        true,
        json!({"school": "CMU", "phone": "555-0100"}),
    )
    .await?;
    attendee(&app, id, hidden.user_id, false, json!({"school": "Pitt"})).await?;
    organizer(&app, id, staff.user_id, "").await?;
    put(
        &app,
        &applicant,
        &format!(
            "/api/hackathons/directory/applications/{}",
            applicant.user_id
        ),
        json!({"data": {}, "submit": true}),
    )
    .await?;

    let path = "/api/hackathons/directory/users";
    let expected = json!({"users": [
        {"user_id": visible.user_id, "display_name": "visible", "profile": {"school": "CMU"}}
    ]});
    for session in [&hidden, &staff] {
        let (status, body) = get(&app, Some(session), path).await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, expected);
    }
    for (session, path, expected) in [
        (Some(&applicant), path, StatusCode::FORBIDDEN),
        (None, path, StatusCode::UNAUTHORIZED),
        (
            Some(&visible),
            "/api/hackathons/missing/users",
            StatusCode::NOT_FOUND,
        ),
    ] {
        let (status, _) = get(&app, session, path).await?;
        assert_eq!(status, expected, "{path}");
    }
    app.close().await
}

#[tokio::test]
async fn existing_applicant_can_review_without_losing_membership() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let id = hackathon(&app, "registration", "1 day").await?;
    let admin = app.login("admin").await?;
    let applicant = app.login("applicant").await?;
    organizer(&app, id, admin.user_id, "'hackathon_admin'").await?;
    let application = format!(
        "/api/hackathons/registration/applications/{}",
        applicant.user_id
    );
    let (status, _) = put(
        &app,
        &applicant,
        &application,
        json!({"data": {}, "submit": true}),
    )
    .await?;
    assert!(status.is_success());

    let organizers = "/api/hackathons/registration/organizers";
    let response = app
        .send(
            admin
                .authorize(Request::post(organizers))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "user_id": applicant.user_id,
                        "capabilities": ["application_management"]
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CREATED);
    let listing = "/api/hackathons/registration/applications";
    assert_eq!(
        get(&app, Some(&applicant), listing).await?.0,
        StatusCode::OK
    );

    let response = app
        .send(
            admin
                .authorize(Request::patch(format!(
                    "{organizers}/{}",
                    applicant.user_id
                )))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"is_active":false}"#))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        get(&app, Some(&applicant), listing).await?.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(&app, Some(&applicant), &application).await?.0,
        StatusCode::OK
    );
    app.close().await
}

/// Replaces the hackathon's application form.
async fn set_form(app: &TestApp, hackathon: Uuid, form: Value) -> anyhow::Result<()> {
    fixture(
        app,
        Scope::HackathonExclusive(hackathon),
        format!("UPDATE hackathon SET application_schema='{form}' WHERE id='{hackathon}'"),
    )
    .await
}

fn error_fields(error: &Value) -> BTreeSet<String> {
    error["fields"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(key, _)| key.clone())
        .collect()
}

#[tokio::test]
async fn answers_are_checked_against_the_form() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let id = hackathon(&app, "tartanhacks", "1 day").await?;
    set_form(
        &app,
        id,
        json!({"fields": [
            {"id": "school", "label": "School", "type": "text", "required": true},
            {"id": "year", "label": "Graduation year", "type": "number", "required": true},
            {"id": "shirt", "label": "Shirt size", "type": "single_choice", "required": false,
             "options": [{"id": "s", "label": "Small"}, {"id": "m", "label": "Medium"}]},
            {"id": "conduct", "label": "I agree to the code of conduct", "type": "checkbox",
             "required": true}
        ]}),
    )
    .await?;
    let path = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );

    let (status, error) = put(
        &app,
        &hacker,
        &path,
        json!({"data": {"school": 1, "shirt": "xl", "github": "ada"}}),
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["code"], "validation_failed");
    assert_eq!(
        error_fields(&error),
        ["data.github", "data.school", "data.shirt"]
            .map(str::to_owned)
            .into()
    );
    assert_eq!(
        scalar(
            &app,
            "SELECT (SELECT count(*) FROM application) + (SELECT count(*) FROM user_hackathon_role) AS value"
        )
        .await?,
        0,
        "a rejected first save creates neither the application nor the membership"
    );

    let (status, draft) = put(&app, &hacker, &path, json!({"data": {"school": "CMU"}})).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "drafts may leave required questions blank"
    );
    assert_eq!(draft["draft_data"], json!({"school": "CMU"}));

    let (status, error) = put(
        &app,
        &hacker,
        &path,
        json!({"data": {"school": "CMU", "conduct": false}, "submit": true}),
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        error_fields(&error),
        ["data.conduct", "data.year"].map(str::to_owned).into()
    );
    let (_, unchanged) = get(&app, Some(&hacker), &path).await?;
    assert_eq!(
        unchanged["status"],
        Value::Null,
        "a rejected submission stays a draft"
    );
    assert_eq!(unchanged["draft_data"], json!({"school": "CMU"}));

    let answers = json!({"school": "CMU", "year": 2027, "shirt": "m", "conduct": true});
    let (status, submitted) = put(
        &app,
        &hacker,
        &path,
        json!({"data": answers, "submit": true}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{submitted}");
    assert_eq!(submitted["status"], "pending");
    assert_eq!(submitted["data"], answers);

    app.close().await
}

#[tokio::test]
async fn read_only_applications_report_conflicts_before_answer_errors() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let id = hackathon(&app, "tartanhacks", "1 day").await?;
    let path = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );
    let (status, _) = put(
        &app,
        &hacker,
        &path,
        json!({"data": {"school": "CMU"}, "submit": true}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    fixture(
        &app,
        Scope::Hackathon(id),
        format!(
            "UPDATE application SET status='rejected' WHERE user_id='{}'",
            hacker.user_id
        ),
    )
    .await?;

    let (status, error) = put(&app, &hacker, &path, json!({"data": {"unknown": true}})).await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        error["message"],
        "This application can no longer be edited."
    );

    app.close().await
}

#[tokio::test]
async fn an_invalid_stored_form_fails_loudly() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let id = hackathon(&app, "tartanhacks", "1 day").await?;
    set_form(
        &app,
        id,
        json!({"fields": [{"id": "school", "type": "essay"}]}),
    )
    .await?;
    let path = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );

    let (status, error) = put(&app, &hacker, &path, json!({"data": {}})).await?;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(error["code"], "internal_error");
    assert_eq!(
        scalar(&app, "SELECT count(*) AS value FROM application").await?,
        0
    );

    app.close().await
}
