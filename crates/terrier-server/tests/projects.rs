mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    response::Response,
};
use common::{TestApp, TestSession};
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use terrier_server::{audit::Scope, audited_automatic};
use uuid::Uuid;

async fn seed(app: &TestApp, scope: Scope, sql: String) -> anyhow::Result<()> {
    audited_automatic!(&app.db, scope, "Project route test fixture", |tx| {
        tx.execute_unprepared(&sql).await?;
        Ok(())
    })
    .await
    .map_err(|error| anyhow::anyhow!("{error:?}"))
}

async fn get(app: &TestApp, session: Option<&TestSession>, path: &str) -> anyhow::Result<Response> {
    let request = Request::get(path);
    let request = match session {
        Some(session) => session.authorize(request),
        None => request,
    };
    app.send(request.body(Body::empty())?).await
}

async fn status(
    app: &TestApp,
    session: Option<&TestSession>,
    path: &str,
) -> anyhow::Result<StatusCode> {
    Ok(get(app, session, path).await?.status())
}

#[tokio::test]
async fn project_reads_enforce_team_organizer_and_hackathon_scope() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let caller = app.login("caller").await?;
    let session = Some(&caller);
    let caller_id = caller.user_id;

    let [
        alpha,
        beta,
        event,
        team,
        other_team,
        project,
        other_project,
        track,
        owner,
        other_owner,
    ] = std::array::from_fn(|_| Uuid::new_v4());
    let mut sql = String::new();
    for id in [owner, other_owner] {
        sql.push_str(&format!(
            "INSERT INTO \"user\"(id,email,display_name,oidc_issuer,oidc_subject) \
             VALUES('{id}','{id}@example.com','Owner','https://example.com','{id}');"
        ));
    }
    for (id, slug) in [(alpha, "alpha"), (beta, "beta")] {
        sql.push_str(&format!(
            "INSERT INTO hackathon(id,slug,name,description,is_active,max_team_size,\
             max_participants,location,contact_email,application_schema,data,\
             application_deadline,team_formation_deadline,submission_deadline) \
             VALUES('{id}','{slug}','Fixture','Fixture',true,4,100,'Campus',\
             'fixture@example.com','{{}}','{{}}',clock_timestamp()+interval '1 day',\
             clock_timestamp()+interval '2 days',clock_timestamp()+interval '3 days');"
        ));
    }
    seed(&app, Scope::Global, sql).await?;

    let mut sql = format!(
        "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
         VALUES('{owner}','{alpha}','hacker'),('{other_owner}','{alpha}','hacker'),\
         ('{caller_id}','{alpha}','hacker');\
         INSERT INTO event(id,hackathon_id,name,description,start_time,end_time,location) \
         VALUES('{event}','{alpha}','Expo','Expo',now(),now()+interval '1 hour','Campus');\
         INSERT INTO prize_track(id,hackathon_id,name,description,is_judging_started,\
         judging_method) VALUES('{track}','{alpha}','Track','Track',false,'scoring');"
    );
    for (team, leader) in [(team, owner), (other_team, other_owner)] {
        sql.push_str(&format!(
            "INSERT INTO team(id,hackathon_id,name) VALUES('{team}','{alpha}','{team}');\
             INSERT INTO team_member(team_id,user_id,hackathon_id,status,is_leader,\
             requested_at,joined_at) \
             VALUES('{team}','{leader}','{alpha}','active',true,now(),now());"
        ));
    }
    seed(&app, Scope::HackathonExclusive(alpha), sql).await?;
    seed(
        &app,
        Scope::Hackathon(alpha),
        format!(
            "INSERT INTO project(id,hackathon_id,event_id,team_id,data) \
             VALUES('{project}','{alpha}','{event}','{team}','{{}}'),\
             ('{other_project}','{alpha}','{event}','{other_team}','{{}}');\
             INSERT INTO project_track_sub(project_id,track_id,hackathon_id) \
             VALUES('{project}','{track}','{alpha}');"
        ),
    )
    .await?;

    let path = format!("/api/hackathons/alpha/projects/{project}");
    let other_path = format!("/api/hackathons/alpha/projects/{other_project}");
    assert_eq!(status(&app, None, &path).await?, StatusCode::UNAUTHORIZED);
    assert_eq!(
        status(&app, session, "/api/hackathons/alpha/projects/not-a-uuid").await?,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(status(&app, session, &path).await?, StatusCode::FORBIDDEN);
    for missing in [
        format!("/api/hackathons/beta/projects/{project}"),
        format!("/api/hackathons/unknown/projects/{project}"),
        format!("/api/hackathons/alpha/projects/{}", Uuid::new_v4()),
    ] {
        assert_eq!(
            status(&app, session, &missing).await?,
            StatusCode::NOT_FOUND
        );
    }

    seed(
        &app,
        Scope::Hackathon(alpha),
        format!(
            "SELECT id FROM team WHERE id='{team}' FOR UPDATE;\
             INSERT INTO team_member(team_id,user_id,hackathon_id,status,is_leader,\
             requested_at,joined_at) \
             VALUES('{team}','{caller_id}','{alpha}','active',false,now(),now());"
        ),
    )
    .await?;
    let response = get(&app, session, &path).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    assert_eq!(body["id"], json!(project.to_string()));
    assert_eq!(body["team_id"], json!(team.to_string()));
    assert_eq!(body["event_id"], json!(event.to_string()));
    assert_eq!(body["data"], json!({}));
    assert_eq!(body["status"], "draft");
    assert_eq!(body["submitted_at"], Value::Null);
    assert_eq!(body["track_ids"], json!([track.to_string()]));
    assert_eq!(
        status(&app, session, &other_path).await?,
        StatusCode::FORBIDDEN
    );

    seed(
        &app,
        Scope::HackathonExclusive(alpha),
        format!(
            "UPDATE user_hackathon_role SET role='organizer' \
             WHERE user_id='{caller_id}' AND hackathon_id='{alpha}';"
        ),
    )
    .await?;
    assert_eq!(status(&app, session, &other_path).await?, StatusCode::OK);

    app.close().await
}
