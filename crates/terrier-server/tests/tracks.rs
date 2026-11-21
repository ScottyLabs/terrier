mod common;

use anyhow::Context;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::CONTENT_TYPE},
    response::Response,
};
use common::{TestApp, TestSession};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
};
use serde_json::{Value, json};
use terrier_server::{
    audit::{self, Scope},
    entities::prize_track,
};
use uuid::Uuid;

async fn fixture_write(db: &DatabaseConnection, scope: Scope, sql: String) -> anyhow::Result<()> {
    audit::automatic(db, scope, "Track route test fixture", |tx| {
        Box::pin(async move {
            tx.execute_unprepared(&sql).await?;
            Ok(())
        })
    })
    .await
    .map_err(|error| anyhow::anyhow!("{error:?}"))
}

async fn create_hackathon(
    app: &TestApp,
    slug: &str,
    known_orgs: &[Uuid],
    associated_orgs: &[Uuid],
) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    let sponsors: serde_json::Map<String, Value> = associated_orgs
        .iter()
        .map(|org_id| (org_id.to_string(), json!({})))
        .collect();
    let sponsors = Value::Object(sponsors);

    let mut sql = String::new();
    for org_id in known_orgs {
        sql.push_str(&format!(
            "INSERT INTO sponsor_org(id,name,description) \
             VALUES('{org_id}','Sponsor','Sponsor');"
        ));
    }
    sql.push_str(&format!(
        r#"INSERT INTO hackathon
        (id,slug,name,description,is_active,max_team_size,max_participants,
         location,contact_email,application_schema,data,
         application_deadline,team_formation_deadline,submission_deadline,sponsors)
        VALUES
        ('{id}','{slug}','Fixture','Fixture',true,4,100,
         'Campus','test@example.com','{{}}','{{}}',
         clock_timestamp()+interval '1 day',
         clock_timestamp()+interval '2 days',
         clock_timestamp()+interval '3 days',
         '{sponsors}'::jsonb);"#
    ));

    fixture_write(&app.db, Scope::Global, sql).await?;
    Ok(id)
}

async fn grant_organizer(
    app: &TestApp,
    user_id: Uuid,
    hackathon_id: Uuid,
    capability: &str,
) -> anyhow::Result<()> {
    fixture_write(
        &app.db,
        Scope::HackathonExclusive(hackathon_id),
        format!(
            "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
             VALUES('{user_id}','{hackathon_id}','organizer'); \
             INSERT INTO organizer(user_id,hackathon_id,capabilities) \
             VALUES('{user_id}','{hackathon_id}', \
                    ARRAY['{capability}']::capability[]);"
        ),
    )
    .await
}

async fn grant_sponsor(
    app: &TestApp,
    user_id: Uuid,
    hackathon_id: Uuid,
    org_id: Uuid,
) -> anyhow::Result<()> {
    fixture_write(
        &app.db,
        Scope::HackathonExclusive(hackathon_id),
        format!(
            "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
             VALUES('{user_id}','{hackathon_id}','sponsor'); \
             INSERT INTO sponsor(user_id,org_id,hackathon_id) \
             VALUES('{user_id}','{org_id}','{hackathon_id}');"
        ),
    )
    .await
}

async fn post_track(
    app: &TestApp,
    session: &TestSession,
    slug: &str,
    body: Value,
) -> anyhow::Result<Response> {
    app.send(
        session
            .authorize(Request::post(format!("/api/hackathons/{slug}/tracks")))
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))?,
    )
    .await
}

async fn assert_error(
    response: Response,
    expected_status: StatusCode,
    expected_code: &str,
) -> anyhow::Result<()> {
    assert_eq!(response.status(), expected_status);
    let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    assert_eq!(body["code"], expected_code);
    Ok(())
}

#[tokio::test]
async fn admin_creates_sponsored_and_unsponsored_tracks() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let admin = app.login("admin").await?;
    let org_id = Uuid::new_v4();
    let hackathon_id = create_hackathon(&app, "demo", &[org_id], &[org_id]).await?;
    grant_organizer(&app, admin.user_id, hackathon_id, "hackathon_admin").await?;

    let response = post_track(
        &app,
        &admin,
        "demo",
        json!({
            "name": " Best App ",
            "description": "A prize track",
            "judging_method": "scoring",
            "sponsor_org_id": org_id.to_string()
        }),
    )
    .await?;
    assert_eq!(response.status(), StatusCode::CREATED);

    let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    assert_eq!(body["name"], json!("Best App"));
    assert_eq!(body["description"], json!("A prize track"));
    assert_eq!(body["judging_method"], json!("scoring"));
    assert_eq!(body["sponsor_org_id"], json!(org_id.to_string()));
    assert_eq!(body["version"], json!(1));

    let track_id: Uuid = body["id"].as_str().context("missing track ID")?.parse()?;
    let saved = prize_track::Entity::find_by_id(track_id)
        .one(&app.db)
        .await?
        .context("track was not saved")?;
    assert_eq!(saved.hackathon_id, hackathon_id);
    assert_eq!(saved.sponsor_org_id, Some(org_id));
    assert!(!saved.is_judging_started);
    assert!(saved.is_active);
    assert_eq!(saved.criteria, json!({}));
    assert_eq!(saved.eligibility, json!({}));

    let unsponsored = post_track(
        &app,
        &admin,
        "demo",
        json!({
            "name": "General prize",
            "description": "",
            "judging_method": "expo"
        }),
    )
    .await?;
    assert_eq!(unsponsored.status(), StatusCode::CREATED);

    let count = prize_track::Entity::find()
        .filter(prize_track::Column::HackathonId.eq(hackathon_id))
        .count(&app.db)
        .await?;
    assert_eq!(count, 2);

    app.close().await
}

#[tokio::test]
async fn sponsor_can_create_only_for_own_associated_organization() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let sponsor = app.login("sponsor").await?;
    let own_org = Uuid::new_v4();
    let other_org = Uuid::new_v4();
    let hackathon_id =
        create_hackathon(&app, "demo", &[own_org, other_org], &[own_org, other_org]).await?;
    grant_sponsor(&app, sponsor.user_id, hackathon_id, own_org).await?;

    let own = post_track(
        &app,
        &sponsor,
        "demo",
        json!({
            "name": "Own prize",
            "description": "",
            "judging_method": "expo",
            "sponsor_org_id": own_org.to_string()
        }),
    )
    .await?;
    assert_eq!(own.status(), StatusCode::CREATED);

    for sponsor_org_id in [Some(other_org.to_string()), None] {
        let denied = post_track(
            &app,
            &sponsor,
            "demo",
            json!({
                "name": "Not allowed",
                "description": "",
                "judging_method": "expo",
                "sponsor_org_id": sponsor_org_id
            }),
        )
        .await?;
        assert_error(denied, StatusCode::FORBIDDEN, "forbidden").await?;
    }

    let count = prize_track::Entity::find()
        .filter(prize_track::Column::HackathonId.eq(hackathon_id))
        .count(&app.db)
        .await?;
    assert_eq!(count, 1);

    app.close().await
}

#[tokio::test]
async fn creation_rejects_insufficient_permission_and_bad_input() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let admin = app.login("admin").await?;
    let judging_manager = app.login("judging-manager").await?;
    let org_id = Uuid::new_v4();
    let hackathon_id = create_hackathon(&app, "demo", &[org_id], &[]).await?;
    create_hackathon(&app, "other", &[], &[]).await?;
    grant_organizer(&app, admin.user_id, hackathon_id, "hackathon_admin").await?;
    grant_organizer(
        &app,
        judging_manager.user_id,
        hackathon_id,
        "judging_management",
    )
    .await?;

    let judging_manager_request = post_track(
        &app,
        &judging_manager,
        "demo",
        json!({
            "name": "Prize",
            "description": "",
            "judging_method": "scoring"
        }),
    )
    .await?;
    assert_error(judging_manager_request, StatusCode::FORBIDDEN, "forbidden").await?;

    let blank_name = post_track(
        &app,
        &admin,
        "demo",
        json!({
            "name": "   ",
            "description": "",
            "judging_method": "scoring"
        }),
    )
    .await?;
    assert_error(
        blank_name,
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_failed",
    )
    .await?;

    let malformed_org = post_track(
        &app,
        &admin,
        "demo",
        json!({
            "name": "Prize",
            "description": "",
            "judging_method": "scoring",
            "sponsor_org_id": "not-a-uuid"
        }),
    )
    .await?;
    assert_error(malformed_org, StatusCode::BAD_REQUEST, "bad_request").await?;

    let unassociated_org = post_track(
        &app,
        &admin,
        "demo",
        json!({
            "name": "Prize",
            "description": "",
            "judging_method": "scoring",
            "sponsor_org_id": org_id.to_string()
        }),
    )
    .await?;
    assert_error(unassociated_org, StatusCode::FORBIDDEN, "forbidden").await?;

    let other_hackathon = post_track(
        &app,
        &admin,
        "other",
        json!({
            "name": "Prize",
            "description": "",
            "judging_method": "scoring"
        }),
    )
    .await?;
    assert_error(other_hackathon, StatusCode::FORBIDDEN, "forbidden").await?;

    let missing_hackathon = post_track(
        &app,
        &admin,
        "missing",
        json!({
            "name": "Prize",
            "description": "",
            "judging_method": "scoring"
        }),
    )
    .await?;
    assert_error(missing_hackathon, StatusCode::NOT_FOUND, "not_found").await?;

    let signed_out = app
        .send(
            Request::post("/api/hackathons/demo/tracks")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Prize",
                        "description": "",
                        "judging_method": "scoring"
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    assert_error(signed_out, StatusCode::UNAUTHORIZED, "unauthenticated").await?;

    let count = prize_track::Entity::find()
        .filter(prize_track::Column::HackathonId.eq(hackathon_id))
        .count(&app.db)
        .await?;
    assert_eq!(count, 0);

    app.close().await
}

#[tokio::test]
async fn track_edit_checks_revision_and_deactivation_permission() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let admin = app.login("admin").await?;
    let sponsor = app.login("sponsor").await?;
    let org_id = Uuid::new_v4();
    let hackathon_id = create_hackathon(&app, "demo", &[org_id], &[org_id]).await?;
    grant_organizer(&app, admin.user_id, hackathon_id, "hackathon_admin").await?;
    grant_sponsor(&app, sponsor.user_id, hackathon_id, org_id).await?;
    let created = post_track(
        &app,
        &sponsor,
        "demo",
        json!({
            "name": "Sponsor prize", "description": "Old", "judging_method": "expo",
            "sponsor_org_id": org_id.to_string()
        }),
    )
    .await?;
    assert_eq!(created.status(), StatusCode::CREATED);
    let body: Value = serde_json::from_slice(&to_bytes(created.into_body(), 4096).await?)?;
    let track_id = body["id"].as_str().context("missing ID")?;
    let path = format!("/api/hackathons/demo/tracks/{track_id}");
    let edit = app
        .send(
            sponsor
                .authorize(Request::patch(&path))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"version": 1, "description": "New"}).to_string(),
                ))?,
        )
        .await?;
    assert_eq!(edit.status(), StatusCode::OK);
    let changed: Value = serde_json::from_slice(&to_bytes(edit.into_body(), 4096).await?)?;
    assert_eq!(changed["version"], 2);
    assert_eq!(changed["description"], "New");
    let stale = app
        .send(
            sponsor
                .authorize(Request::patch(&path))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"version": 1, "name": "Stale"}).to_string(),
                ))?,
        )
        .await?;
    assert_error(stale, StatusCode::CONFLICT, "conflict").await?;
    let denied = app
        .send(
            sponsor
                .authorize(Request::patch(&path))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"version": 2, "is_active": false}).to_string(),
                ))?,
        )
        .await?;
    assert_error(denied, StatusCode::FORBIDDEN, "forbidden").await?;
    let removed = app
        .send(
            admin
                .authorize(Request::patch(&path))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"version": 2, "is_active": false}).to_string(),
                ))?,
        )
        .await?;
    assert_eq!(removed.status(), StatusCode::OK);
    let saved = prize_track::Entity::find_by_id(Uuid::parse_str(track_id)?)
        .one(&app.db)
        .await?
        .context("missing track")?;
    assert!(!saved.is_active);
    app.close().await
}

#[tokio::test]
async fn result_reads_keep_scheduled_data_private_and_public_view_small() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let admin = app.login("admin").await?;
    let outsider = app.login("outsider").await?;
    let hacker = app.login("hacker").await?;
    let hackathon_id = create_hackathon(&app, "demo", &[], &[]).await?;
    grant_organizer(&app, admin.user_id, hackathon_id, "hackathon_admin").await?;
    let created = post_track(
        &app,
        &admin,
        "demo",
        json!({
            "name": "General prize", "description": "", "judging_method": "scoring"
        }),
    )
    .await?;
    let body: Value = serde_json::from_slice(&to_bytes(created.into_body(), 4096).await?)?;
    let track_id = body["id"].as_str().context("missing ID")?;
    let path = format!("/api/hackathons/demo/tracks/{track_id}/results");
    let event_id = Uuid::new_v4();
    let team_id = Uuid::new_v4();
    let project_id = Uuid::new_v4();
    fixture_write(&app.db, Scope::HackathonExclusive(hackathon_id), format!(
        "INSERT INTO event(id,hackathon_id,name,description,start_time,end_time,location) \
         VALUES('{event_id}','{hackathon_id}','Expo','',now(),now()+interval '1 hour','Campus'); \
         INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
         VALUES('{}','{hackathon_id}','hacker'); \
         INSERT INTO team(id,hackathon_id,name) VALUES('{team_id}','{hackathon_id}','Team'); \
         INSERT INTO team_member(team_id,user_id,hackathon_id,status,is_leader,requested_at,joined_at) \
         VALUES('{team_id}','{}','{hackathon_id}','active',true,now(),now()); \
         INSERT INTO project(id,hackathon_id,event_id,team_id,data,status,submitted_at) \
         VALUES('{project_id}','{hackathon_id}','{event_id}','{team_id}','{{}}','submitted',now()); \
         INSERT INTO project_track_sub(project_id,track_id,hackathon_id) \
         VALUES('{project_id}','{track_id}','{hackathon_id}');", hacker.user_id, hacker.user_id
    )).await?;
    let before = app.send(Request::get(&path).body(Body::empty())?).await?;
    assert_error(before, StatusCode::UNAUTHORIZED, "unauthenticated").await?;
    let private = app
        .send(admin.authorize(Request::get(&path)).body(Body::empty())?)
        .await?;
    assert_eq!(private.status(), StatusCode::OK);
    let internal: Value = serde_json::from_slice(&to_bytes(private.into_body(), 4096).await?)?;
    assert_eq!(
        internal["projects"][0]["project_id"],
        project_id.to_string()
    );
    let one = app
        .send(
            admin
                .authorize(Request::get(format!("{path}/{project_id}")))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(one.status(), StatusCode::OK);
    let project: Value = serde_json::from_slice(&to_bytes(one.into_body(), 4096).await?)?;
    assert_eq!(project["project_id"], project_id.to_string());
    assert_eq!(project["evaluations"], json!([]));
    let invalid = app
        .send(
            admin
                .authorize(Request::post(&path))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"version": 1, "winners": [{"project_id": Uuid::new_v4().to_string()}]})
                        .to_string(),
                ))?,
        )
        .await?;
    assert_error(invalid, StatusCode::BAD_REQUEST, "bad_request").await?;
    let future = app
        .send(
            admin
                .authorize(Request::post(&path))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"version": 1, "winners": [{"project_id": project_id.to_string()}], "release_at": "2099-01-01T00:00:00Z"})
                        .to_string(),
                ))?,
        )
        .await?;
    assert_eq!(future.status(), StatusCode::OK);
    let early = app.send(Request::get(&path).body(Body::empty())?).await?;
    assert_error(early, StatusCode::UNAUTHORIZED, "unauthenticated").await?;
    let denied = app
        .send(
            outsider
                .authorize(Request::get(format!("{path}/{}", Uuid::new_v4())))
                .body(Body::empty())?,
        )
        .await?;
    assert_error(denied, StatusCode::FORBIDDEN, "forbidden").await?;
    let id = Uuid::parse_str(track_id)?;
    let current = prize_track::Entity::find_by_id(id)
        .one(&app.db)
        .await?
        .context("missing track")?;
    assert_eq!(current.winners, json!([]));
    assert_eq!(
        current.published_winners,
        Some(json!([{"project_id": project_id.to_string()}]))
    );
    fixture_write(&app.db, Scope::HackathonExclusive(hackathon_id), format!(
        "UPDATE prize_track SET release_at=clock_timestamp()-interval '1 second' WHERE id='{id}' AND version={}", current.version
    )).await?;
    let public = app.send(Request::get(&path).body(Body::empty())?).await?;
    assert_eq!(public.status(), StatusCode::OK);
    let released: Value = serde_json::from_slice(&to_bytes(public.into_body(), 4096).await?)?;
    assert_eq!(
        released["winners"],
        json!([{"project_id": project_id.to_string()}])
    );
    assert!(released.get("projects").is_none());
    app.close().await
}

#[tokio::test]
async fn sponsor_reads_only_own_track_and_cannot_publish() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let sponsor = app.login("sponsor").await?;
    let other = app.login("other-sponsor").await?;
    let own_org = Uuid::new_v4();
    let other_org = Uuid::new_v4();
    let hackathon_id =
        create_hackathon(&app, "demo", &[own_org, other_org], &[own_org, other_org]).await?;
    grant_sponsor(&app, sponsor.user_id, hackathon_id, own_org).await?;
    grant_sponsor(&app, other.user_id, hackathon_id, other_org).await?;
    let created = post_track(
        &app,
        &sponsor,
        "demo",
        json!({
            "name": "Own prize", "description": "", "judging_method": "expo",
            "sponsor_org_id": own_org.to_string()
        }),
    )
    .await?;
    let body: Value = serde_json::from_slice(&to_bytes(created.into_body(), 4096).await?)?;
    let path = format!(
        "/api/hackathons/demo/tracks/{}/results",
        body["id"].as_str().context("missing ID")?
    );
    let own = app
        .send(sponsor.authorize(Request::get(&path)).body(Body::empty())?)
        .await?;
    assert_eq!(own.status(), StatusCode::OK);
    let denied = app
        .send(other.authorize(Request::get(&path)).body(Body::empty())?)
        .await?;
    assert_error(denied, StatusCode::FORBIDDEN, "forbidden").await?;
    let publish = app
        .send(
            sponsor
                .authorize(Request::post(&path))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"version": 1, "winners": []}).to_string()))?,
        )
        .await?;
    assert_error(publish, StatusCode::FORBIDDEN, "forbidden").await?;
    app.close().await
}
