mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header::CONTENT_TYPE},
};
use common::{TestApp, TestSession};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use terrier_server::{audit::Scope, audited_automatic};
use uuid::Uuid;

async fn seed(app: &TestApp, scope: Scope, sql: String) -> anyhow::Result<()> {
    audited_automatic!(&app.db, scope, "Project write test fixture", |tx| {
        tx.execute_unprepared(&sql).await?;
        Ok(())
    })
    .await
    .map_err(|error| anyhow::anyhow!("{error:?}"))
}

async fn send(
    app: &TestApp,
    session: Option<&TestSession>,
    method: Method,
    path: &str,
    body: &str,
) -> anyhow::Result<(StatusCode, Value)> {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header(CONTENT_TYPE, "application/json");
    let request = match session {
        Some(session) => session.authorize(request),
        None => request,
    };
    let response = app.send(request.body(Body::from(body.to_owned()))?).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await?;
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)?
    };
    Ok((status, body))
}

async fn post(
    app: &TestApp,
    session: &TestSession,
    body: Value,
) -> anyhow::Result<(StatusCode, Value)> {
    send(
        app,
        Some(session),
        Method::POST,
        "/api/hackathons/alpha/projects",
        &body.to_string(),
    )
    .await
}

async fn patch(
    app: &TestApp,
    session: &TestSession,
    project: &str,
    body: Value,
) -> anyhow::Result<(StatusCode, Value)> {
    send(
        app,
        Some(session),
        Method::PATCH,
        &format!("/api/hackathons/alpha/projects/{project}"),
        &body.to_string(),
    )
    .await
}

async fn beta_hackathon(app: &TestApp) -> anyhow::Result<Uuid> {
    let row = app
        .db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM hackathon WHERE slug='beta'",
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("beta hackathon is missing"))?;
    Ok(row.try_get_by_index(0)?)
}

async fn count(app: &TestApp, sql: &str) -> anyhow::Result<i64> {
    let row = app
        .db
        .query_one_raw(Statement::from_string(DbBackend::Postgres, sql))
        .await?
        .ok_or_else(|| anyhow::anyhow!("no row for {sql}"))?;
    Ok(row.try_get_by_index(0)?)
}

fn ids(tracks: &Value) -> Vec<String> {
    tracks
        .as_array()
        .unwrap()
        .iter()
        .map(|track| track["id"].as_str().unwrap().to_owned())
        .collect()
}

/// Hackathon `alpha` has an active event, an inactive event, active tracks "Apex" and "Zeta",
/// an inactive track, and two teams, `team` and `other_team`. Hackathon `beta` has its own event and track. Nobody is eligible to submit until [`World::participant`] is called.
struct World {
    alpha: Uuid,
    event: Uuid,
    inactive_event: Uuid,
    beta_event: Uuid,
    apex: Uuid,
    zeta: Uuid,
    inactive_track: Uuid,
    beta_track: Uuid,
    team: Uuid,
    other_team: Uuid,
}

impl World {
    async fn new(app: &TestApp) -> anyhow::Result<Self> {
        let [
            alpha,
            beta,
            event,
            inactive_event,
            beta_event,
            apex,
            zeta,
            inactive_track,
            beta_track,
            team,
            other_team,
        ] = std::array::from_fn(|_| Uuid::new_v4());
        let mut sql = String::new();
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
        let mut leaders = String::new();
        for team in [team, other_team] {
            let leader = Uuid::new_v4();
            sql.push_str(&format!(
                "INSERT INTO \"user\"(id,email,display_name,oidc_issuer,oidc_subject) \
                 VALUES('{leader}','{leader}@example.com','Leader','https://example.com',\
                 '{leader}');"
            ));
            leaders.push_str(&format!(
                "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
                 VALUES('{leader}','{alpha}','hacker');\
                 INSERT INTO team_member(team_id,user_id,hackathon_id,status,is_leader,\
                 requested_at,joined_at) \
                 VALUES('{team}','{leader}','{alpha}','active',true,now(),now());"
            ));
        }
        seed(app, Scope::Global, sql).await?;
        seed(
            app,
            Scope::HackathonExclusive(alpha),
            format!(
                "INSERT INTO event(id,hackathon_id,name,description,start_time,end_time,location,\
                 is_active) VALUES('{event}','{alpha}','Opening','Opening',now(),\
                 now()+interval '1 hour','Campus',true),('{inactive_event}','{alpha}','Cancelled',\
                 'Cancelled',now(),now()+interval '1 hour','Campus',false);\
                 INSERT INTO prize_track(id,hackathon_id,name,description,is_judging_started,\
                 judging_method,is_active) \
                 VALUES('{zeta}','{alpha}','Zeta','Zeta',false,'scoring',true),\
                 ('{apex}','{alpha}','Apex','Apex',false,'scoring',true),\
                 ('{inactive_track}','{alpha}','Retired','Retired',false,'scoring',false);\
                 INSERT INTO team(id,hackathon_id,name) \
                 VALUES('{team}','{alpha}','Team'),('{other_team}','{alpha}','Other');{leaders}"
            ),
        )
        .await?;
        seed(
            app,
            Scope::HackathonExclusive(beta),
            format!(
                "INSERT INTO event(id,hackathon_id,name,description,start_time,end_time,location) \
                 VALUES('{beta_event}','{beta}','Beta','Beta',now(),now()+interval '1 hour',\
                 'Campus');\
                 INSERT INTO prize_track(id,hackathon_id,name,description,is_judging_started,\
                 judging_method) VALUES('{beta_track}','{beta}','Beta','Beta',false,'scoring');"
            ),
        )
        .await?;
        Ok(Self {
            alpha,
            event,
            inactive_event,
            beta_event,
            apex,
            zeta,
            inactive_track,
            beta_track,
            team,
            other_team,
        })
    }

    /// Makes `user` a hacker with confirmed attendance, a valid check-in at the active event and an active membership in `team`. The first member of a team becomes its leader.
    async fn participant(&self, app: &TestApp, user: Uuid, team: Uuid) -> anyhow::Result<()> {
        let Self { alpha, event, .. } = self;
        seed(
            app,
            Scope::HackathonExclusive(*alpha),
            format!(
                "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
                 VALUES('{user}','{alpha}','hacker');"
            ),
        )
        .await?;
        seed(
            app,
            Scope::Hackathon(*alpha),
            format!(
                "INSERT INTO application(id,user_id,hackathon_id,status,data,draft_data,\
                 submitted_at,attendance_status) \
                 VALUES('{}','{user}','{alpha}','accepted','{{}}','{{}}',now(),'confirmed');\
                 INSERT INTO event_check_in(id,event_id,user_id,check_in_time,checked_in_by) \
                 VALUES('{}','{event}','{user}',now(),'{user}');\
                 SELECT id FROM team WHERE id='{team}' FOR UPDATE;\
                 INSERT INTO team_member(team_id,user_id,hackathon_id,status,is_leader,\
                 requested_at,joined_at) \
                 VALUES('{team}','{user}','{alpha}','active',false,now(),now());",
                Uuid::new_v4(),
                Uuid::new_v4()
            ),
        )
        .await
    }

    fn new_project(&self) -> Value {
        json!({
            "event_id": self.event,
            "data": {"title": "Terrier"},
            "track_ids": [self.zeta, self.apex],
        })
    }

    async fn set_deadline(&self, app: &TestApp, interval: &str) -> anyhow::Result<()> {
        seed(
            app,
            Scope::HackathonExclusive(self.alpha),
            format!(
                "UPDATE hackathon SET submission_deadline=clock_timestamp()+interval '{interval}' \
                 WHERE id='{}';",
                self.alpha
            ),
        )
        .await
    }

    async fn projects(&self, app: &TestApp) -> anyhow::Result<i64> {
        count(app, "SELECT count(*) FROM project").await
    }
}

#[tokio::test]
async fn checked_in_members_create_draft_projects() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;

    let (status, created) = post(&app, &member, world.new_project()).await?;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["event_id"], json!(world.event.to_string()));
    assert_eq!(
        created["team"],
        json!({"id": world.team.to_string(), "name": "Team"})
    );
    assert_eq!(created["data"], json!({"title": "Terrier"}));
    assert_eq!(created["status"], "draft");
    assert_eq!(created["submitted_at"], Value::Null);
    assert_eq!(
        ids(&created["tracks"]),
        [world.apex.to_string(), world.zeta.to_string()],
        "tracks are ordered by name"
    );
    assert_eq!(
        created["version"],
        json!(3),
        "each track entry revises the project"
    );

    let id = created["id"].as_str().unwrap();
    let (status, read) = send(
        &app,
        Some(&member),
        Method::GET,
        &format!("/api/hackathons/alpha/projects/{id}"),
        "",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read, created, "the response matches a later read");
    assert_eq!(
        count(
            &app,
            &format!(
                "SELECT count(*) FROM audit_event WHERE resource_type='project' \
                 AND resource_key->>'id'='{id}' AND actor_user_id='{}'",
                member.user_id
            )
        )
        .await?,
        3,
        "creation and both track entries are audited as the caller"
    );

    app.close().await
}

#[tokio::test]
async fn projects_can_be_submitted_on_creation() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;

    let mut body = world.new_project();
    body["status"] = json!("submitted");
    body["track_ids"] = json!([]);
    let (status, created) = post(&app, &member, body).await?;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["status"], "submitted");
    assert!(created["submitted_at"].is_string());
    assert_eq!(created["tracks"], json!([]));

    app.close().await
}

#[tokio::test]
async fn project_creation_requires_a_session_and_csrf() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    let body = world.new_project().to_string();

    let (status, error) = send(
        &app,
        None,
        Method::POST,
        "/api/hackathons/alpha/projects",
        &body,
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(error["code"], "unauthenticated");

    let request = Request::post("/api/hackathons/alpha/projects")
        .header(CONTENT_TYPE, "application/json")
        .header(axum::http::header::COOKIE, &member.cookie)
        .header("origin", "https://terrier.example")
        .body(Body::from(body))?;
    assert_eq!(
        app.send(request).await?.status(),
        StatusCode::FORBIDDEN,
        "writes without a CSRF token are rejected"
    );
    assert_eq!(world.projects(&app).await?, 0);

    app.close().await
}

#[tokio::test]
async fn project_creation_requires_every_eligibility_condition() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    let World {
        alpha,
        event,
        inactive_event,
        beta_event,
        team,
        ..
    } = world;
    let user = member.user_id;

    let cases: [(&str, String, String); 8] = [
        (
            "pending attendance",
            "UPDATE application SET attendance_status='pending'".into(),
            "UPDATE application SET attendance_status='confirmed'".into(),
        ),
        (
            "declined attendance",
            "UPDATE application SET attendance_status='declined'".into(),
            "UPDATE application SET attendance_status='confirmed'".into(),
        ),
        (
            "inactive application",
            "UPDATE application SET is_active=false".into(),
            "UPDATE application SET is_active=true".into(),
        ),
        (
            "invalid check-in",
            "UPDATE event_check_in SET is_valid=false".into(),
            "UPDATE event_check_in SET is_valid=true".into(),
        ),
        (
            "check-in at an inactive event",
            format!(
                "UPDATE event_check_in SET is_valid=false WHERE event_id='{event}';\
                     INSERT INTO event_check_in(id,event_id,user_id,check_in_time,checked_in_by) \
                     VALUES('{}','{inactive_event}','{user}',now(),'{user}')",
                Uuid::new_v4()
            ),
            format!(
                "UPDATE event_check_in SET is_valid=false WHERE event_id='{inactive_event}';\
                     UPDATE event_check_in SET is_valid=true WHERE event_id='{event}'"
            ),
        ),
        (
            "pending team request",
            format!(
                "SELECT id FROM team WHERE id='{team}' FOR UPDATE;\
                     UPDATE team_member SET status='pending', joined_at=NULL \
                     WHERE user_id='{user}'"
            ),
            format!(
                "SELECT id FROM team WHERE id='{team}' FOR UPDATE;\
                     UPDATE team_member SET status='active', joined_at=now() \
                     WHERE user_id='{user}'"
            ),
        ),
        (
            "removed from the team",
            format!(
                "SELECT id FROM team WHERE id='{team}' FOR UPDATE;\
                     UPDATE team_member SET status='removed', ended_at=now() \
                     WHERE user_id='{user}'"
            ),
            format!(
                "SELECT id FROM team WHERE id='{team}' FOR UPDATE;\
                     UPDATE team_member SET status='active', ended_at=NULL \
                     WHERE user_id='{user}'"
            ),
        ),
        (
            "inactive role",
            format!("UPDATE user_hackathon_role SET is_active=false WHERE user_id='{user}'"),
            format!("UPDATE user_hackathon_role SET is_active=true WHERE user_id='{user}'"),
        ),
    ];
    for (case, revoke, restore) in cases {
        let scope = Scope::HackathonExclusive(alpha);
        seed(&app, scope, format!("{revoke};"))
            .await
            .map_err(|error| error.context(case))?;
        let (status, error) = post(&app, &member, world.new_project()).await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "{case}: {error}");
        assert_eq!(error["code"], "forbidden", "{case}");
        audited_automatic!(&app.db, scope, "Restore test fixture", |tx| {
            tx.execute_unprepared(&format!("{restore};")).await?;
            Ok(())
        })
        .await
        .map_err(|error| anyhow::anyhow!("{case}: {error:?}"))?;
    }

    let outsider = app.login("outsider").await?;
    seed(
        &app,
        Scope::HackathonExclusive(alpha),
        format!(
            "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
             VALUES('{}','{alpha}','hacker');",
            outsider.user_id
        ),
    )
    .await?;
    assert_eq!(
        post(&app, &outsider, world.new_project()).await?.0,
        StatusCode::FORBIDDEN,
        "no application"
    );

    let elsewhere = app.login("elsewhere").await?;
    world
        .participant(&app, elsewhere.user_id, world.other_team)
        .await?;
    seed(
        &app,
        Scope::Hackathon(alpha),
        format!(
            "UPDATE event_check_in SET is_valid=false WHERE user_id='{}';",
            elsewhere.user_id
        ),
    )
    .await?;
    let beta_id = beta_hackathon(&app).await?;
    seed(
        &app,
        Scope::HackathonExclusive(beta_id),
        format!(
            "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
             VALUES('{0}','{beta_id}','hacker');\
             INSERT INTO event_check_in(id,event_id,user_id,check_in_time,checked_in_by) \
             VALUES('{1}','{beta_event}','{0}',now(),'{0}');",
            elsewhere.user_id,
            Uuid::new_v4()
        ),
    )
    .await?;
    assert_eq!(
        post(&app, &elsewhere, world.new_project()).await?.0,
        StatusCode::FORBIDDEN,
        "a check-in at another hackathon does not count"
    );

    assert_eq!(world.projects(&app).await?, 0);
    assert_eq!(
        post(&app, &member, world.new_project()).await?.0,
        StatusCode::CREATED,
        "restoring every condition allows submission"
    );

    app.close().await
}

#[tokio::test]
async fn project_creation_is_scoped_to_active_hackathons() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    let body = world.new_project().to_string();

    let (status, _) = send(
        &app,
        Some(&member),
        Method::POST,
        "/api/hackathons/unknown/projects",
        &body,
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &app,
        Some(&member),
        Method::POST,
        "/api/hackathons/beta/projects",
        &body,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "alpha eligibility does not apply to beta"
    );

    seed(
        &app,
        Scope::HackathonExclusive(world.alpha),
        format!(
            "UPDATE hackathon SET is_active=false WHERE id='{}';",
            world.alpha
        ),
    )
    .await?;
    assert_eq!(
        post(&app, &member, world.new_project()).await?.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(world.projects(&app).await?, 0);

    app.close().await
}

#[tokio::test]
async fn teams_create_only_one_project() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let teammate = app.login("teammate").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    world
        .participant(&app, teammate.user_id, world.team)
        .await?;

    assert_eq!(
        post(&app, &member, world.new_project()).await?.0,
        StatusCode::CREATED
    );
    for session in [&member, &teammate] {
        let (status, error) = post(&app, session, world.new_project()).await?;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(error["code"], "conflict");
    }
    assert_eq!(world.projects(&app).await?, 1);

    app.close().await
}

#[tokio::test]
async fn simultaneous_creations_produce_one_project() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let teammate = app.login("teammate").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    world
        .participant(&app, teammate.user_id, world.team)
        .await?;

    let (first, second) = tokio::join!(
        post(&app, &member, world.new_project()),
        post(&app, &teammate, world.new_project())
    );
    let mut statuses = [first?.0, second?.0];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(world.projects(&app).await?, 1);
    assert_eq!(
        count(&app, "SELECT count(*) FROM project_track_sub").await?,
        2,
        "the losing request leaves no track entries"
    );

    app.close().await
}

#[tokio::test]
async fn project_creation_closes_at_the_deadline() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    world.set_deadline(&app, "-1 second").await?;

    let (status, error) = post(&app, &member, world.new_project()).await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["message"], "The submission deadline has passed.");
    assert_eq!(world.projects(&app).await?, 0);

    app.close().await
}

#[tokio::test]
async fn project_creation_validates_references_and_fields() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;

    for (case, change, fields) in [
        (
            "other hackathon's event",
            json!({"event_id": world.beta_event}),
            vec!["event_id"],
        ),
        (
            "inactive event",
            json!({"event_id": world.inactive_event}),
            vec!["event_id"],
        ),
        (
            "unknown event",
            json!({"event_id": Uuid::new_v4()}),
            vec!["event_id"],
        ),
        (
            "other hackathon's track",
            json!({"track_ids": [world.beta_track]}),
            vec!["track_ids"],
        ),
        (
            "inactive track",
            json!({"track_ids": [world.apex, world.inactive_track]}),
            vec!["track_ids"],
        ),
        (
            "unknown track",
            json!({"track_ids": [Uuid::new_v4()]}),
            vec!["track_ids"],
        ),
        (
            "duplicate track",
            json!({"track_ids": [world.apex, world.apex]}),
            vec!["track_ids"],
        ),
        ("data array", json!({"data": []}), vec!["data"]),
        ("data string", json!({"data": "text"}), vec!["data"]),
        (
            "several invalid fields",
            json!({"data": 1, "track_ids": [world.apex, world.apex]}),
            vec!["data", "track_ids"],
        ),
        (
            "invalid references together",
            json!({"event_id": world.beta_event, "track_ids": [world.beta_track]}),
            vec!["event_id", "track_ids"],
        ),
    ] {
        let mut body = world.new_project();
        for (key, value) in change.as_object().unwrap() {
            body[key] = value.clone();
        }
        let (status, error) = post(&app, &member, body).await?;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{case}: {error}");
        assert_eq!(error["code"], "validation_failed", "{case}");
        let mut keys: Vec<_> = error["fields"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        keys.sort();
        assert_eq!(keys, fields, "{case}");
    }

    let event = world.event;
    for (case, body) in [
        ("malformed JSON", "{".to_owned()),
        (
            "missing event",
            json!({"data": {}, "track_ids": []}).to_string(),
        ),
        (
            "missing tracks",
            json!({"event_id": event, "data": {}}).to_string(),
        ),
        (
            "event not a UUID",
            json!({"event_id": "x", "data": {}, "track_ids": []}).to_string(),
        ),
        (
            "unknown status",
            json!({"event_id": event, "data": {}, "track_ids": [], "status": "final"}).to_string(),
        ),
        (
            "unknown field",
            json!({"event_id": event, "data": {}, "track_ids": [], "team_id": world.other_team})
                .to_string(),
        ),
    ] {
        let (status, error) = send(
            &app,
            Some(&member),
            Method::POST,
            "/api/hackathons/alpha/projects",
            &body,
        )
        .await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{case}");
        assert_eq!(error["code"], "bad_request", "{case}");
    }
    assert_eq!(world.projects(&app).await?, 0);

    app.close().await
}

/// Creates the team's draft project as `member` and returns its response.
async fn draft(app: &TestApp, world: &World, member: &TestSession) -> anyhow::Result<Value> {
    let (status, created) = post(app, member, world.new_project()).await?;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    Ok(created)
}

#[tokio::test]
async fn members_edit_data_with_the_current_version() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let teammate = app.login("teammate").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    world
        .participant(&app, teammate.user_id, world.team)
        .await?;
    let created = draft(&app, &world, &member).await?;
    let id = created["id"].as_str().unwrap();

    let (status, edited) = patch(
        &app,
        &teammate,
        id,
        json!({"version": created["version"], "data": {"title": "Renamed"}}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{edited}");
    assert_eq!(edited["data"], json!({"title": "Renamed"}));
    assert_eq!(
        edited["version"],
        json!(created["version"].as_i64().unwrap() + 1)
    );
    assert_eq!(edited["tracks"], created["tracks"], "tracks are unchanged");
    assert_eq!(edited["status"], "draft");

    let (status, error) = patch(
        &app,
        &member,
        id,
        json!({"version": created["version"], "data": {"title": "Overwrite"}}),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["code"], "stale_revision");
    let (_, unchanged) = send(
        &app,
        Some(&member),
        Method::GET,
        &format!("/api/hackathons/alpha/projects/{id}"),
        "",
    )
    .await?;
    assert_eq!(unchanged, edited, "a stale edit changes nothing");

    let (status, same) = patch(
        &app,
        &member,
        id,
        json!({"version": edited["version"], "data": {"title": "Renamed"}}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        same["version"], edited["version"],
        "an identical edit is not a new revision"
    );

    app.close().await
}

#[tokio::test]
async fn simultaneous_edits_of_one_version_apply_once() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let teammate = app.login("teammate").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    world
        .participant(&app, teammate.user_id, world.team)
        .await?;
    let created = draft(&app, &world, &member).await?;
    let id = created["id"].as_str().unwrap();
    let version = &created["version"];

    let (first, second) = tokio::join!(
        patch(
            &app,
            &member,
            id,
            json!({"version": version, "data": {"by": "member"}})
        ),
        patch(
            &app,
            &teammate,
            id,
            json!({"version": version, "data": {"by": "teammate"}})
        )
    );
    let (first, second) = (first?, second?);
    let mut statuses = [first.0, second.0];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    let (winner, loser) = if first.0 == StatusCode::OK {
        (first.1, second.1)
    } else {
        (second.1, first.1)
    };
    assert_eq!(loser["code"], "stale_revision");
    assert_eq!(winner["version"], json!(version.as_i64().unwrap() + 1));

    app.close().await
}

#[tokio::test]
async fn members_change_tracks_and_reenter_withdrawn_ones() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    let created = draft(&app, &world, &member).await?;
    let id = created["id"].as_str().unwrap();

    let (status, edited) = patch(
        &app,
        &member,
        id,
        json!({"version": created["version"], "track_ids": [world.zeta]}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{edited}");
    assert_eq!(ids(&edited["tracks"]), [world.zeta.to_string()]);

    let (status, edited) = patch(
        &app,
        &member,
        id,
        json!({"version": edited["version"], "track_ids": [world.apex, world.zeta]}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{edited}");
    assert_eq!(
        ids(&edited["tracks"]),
        [world.apex.to_string(), world.zeta.to_string()]
    );
    assert_eq!(
        count(&app, "SELECT count(*) FROM project_track_sub").await?,
        2,
        "re-entering a track restores its original entry"
    );
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM audit_event WHERE resource_type='project_track_sub' \
             AND action='restored' AND reason IS NOT NULL"
        )
        .await?,
        1
    );

    let (status, edited) = patch(
        &app,
        &member,
        id,
        json!({"version": edited["version"], "track_ids": []}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(edited["tracks"], json!([]));

    app.close().await
}

#[tokio::test]
async fn drafts_are_submitted_once_and_stay_editable_until_the_deadline() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    let created = draft(&app, &world, &member).await?;
    let id = created["id"].as_str().unwrap();

    let (status, submitted) = patch(
        &app,
        &member,
        id,
        json!({"version": created["version"], "status": "submitted"}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{submitted}");
    assert_eq!(submitted["status"], "submitted");
    let submitted_at = submitted["submitted_at"].clone();
    assert!(submitted_at.is_string());

    let (status, error) = patch(
        &app,
        &member,
        id,
        json!({"version": submitted["version"], "status": "draft"}),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["code"], "conflict");

    let (status, edited) = patch(
        &app,
        &member,
        id,
        json!({"version": submitted["version"], "data": {"title": "Final"}, "status": "submitted"}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "submitted projects stay editable");
    assert_eq!(edited["status"], "submitted");
    assert_eq!(
        edited["submitted_at"], submitted_at,
        "resubmitting keeps the first time"
    );

    world.set_deadline(&app, "-1 second").await?;
    let (status, error) = patch(
        &app,
        &member,
        id,
        json!({"version": edited["version"], "data": {"title": "Late"}}),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["message"], "The submission deadline has passed.");
    let (_, unchanged) = send(
        &app,
        Some(&member),
        Method::GET,
        &format!("/api/hackathons/alpha/projects/{id}"),
        "",
    )
    .await?;
    assert_eq!(unchanged["data"], json!({"title": "Final"}));

    app.close().await
}

#[tokio::test]
async fn only_eligible_members_of_the_owning_team_edit() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let teammate = app.login("teammate").await?;
    let rival = app.login("rival").await?;
    let organizer = app.login("organizer").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    world
        .participant(&app, teammate.user_id, world.team)
        .await?;
    world
        .participant(&app, rival.user_id, world.other_team)
        .await?;
    seed(
        &app,
        Scope::HackathonExclusive(world.alpha),
        format!(
            "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
             VALUES('{}','{}','organizer');",
            organizer.user_id, world.alpha
        ),
    )
    .await?;
    let created = draft(&app, &world, &member).await?;
    let id = created["id"].as_str().unwrap();
    let change = json!({"version": created["version"], "data": {"title": "Hijacked"}});

    for (who, session) in [("rival", &rival), ("organizer", &organizer)] {
        let (status, _) = patch(&app, session, id, change.clone()).await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "{who}");
    }
    seed(
        &app,
        Scope::Hackathon(world.alpha),
        format!(
            "UPDATE event_check_in SET is_valid=false WHERE user_id='{}';",
            teammate.user_id
        ),
    )
    .await?;
    assert_eq!(
        patch(&app, &teammate, id, change.clone()).await?.0,
        StatusCode::FORBIDDEN,
        "teammates must be checked in"
    );
    let (status, edited) = patch(
        &app,
        &member,
        id,
        json!({"version": created["version"], "data": {"title": "Ours"}}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "one checked-in member is enough");
    assert_eq!(edited["data"], json!({"title": "Ours"}));

    let (status, _) = send(
        &app,
        None,
        Method::PATCH,
        &format!("/api/hackathons/alpha/projects/{id}"),
        &change.to_string(),
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    app.close().await
}

#[tokio::test]
async fn project_edits_validate_scope_and_input() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;
    let created = draft(&app, &world, &member).await?;
    let id = created["id"].as_str().unwrap();
    let version = created["version"].clone();

    for path in [
        format!("/api/hackathons/beta/projects/{id}"),
        format!("/api/hackathons/alpha/projects/{}", Uuid::new_v4()),
    ] {
        let body = json!({"version": version, "data": {}}).to_string();
        let (status, _) = send(&app, Some(&member), Method::PATCH, &path, &body).await?;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }

    for (case, body) in [
        ("project ID not a UUID", None),
        ("malformed JSON", Some("{".to_owned())),
        ("missing version", Some(json!({"data": {}}).to_string())),
        (
            "nothing to change",
            Some(json!({"version": version}).to_string()),
        ),
        (
            "unknown field",
            Some(json!({"version": version, "event_id": world.event}).to_string()),
        ),
        (
            "unknown status",
            Some(json!({"version": version, "status": "withdrawn"}).to_string()),
        ),
    ] {
        let (path, body) = match body {
            None => (
                "/api/hackathons/alpha/projects/not-a-uuid".to_owned(),
                json!({"version": version, "data": {}}).to_string(),
            ),
            Some(body) => (format!("/api/hackathons/alpha/projects/{id}"), body),
        };
        let (status, error) = send(&app, Some(&member), Method::PATCH, &path, &body).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{case}: {error}");
    }

    for (case, change, field) in [
        ("data array", json!({"data": []}), "data"),
        (
            "duplicate track",
            json!({"track_ids": [world.apex, world.apex]}),
            "track_ids",
        ),
        (
            "inactive track",
            json!({"track_ids": [world.inactive_track]}),
            "track_ids",
        ),
        (
            "other hackathon's track",
            json!({"track_ids": [world.beta_track]}),
            "track_ids",
        ),
    ] {
        let mut body = change.clone();
        body["version"] = version.clone();
        let (status, error) = patch(&app, &member, id, body).await?;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{case}: {error}");
        assert!(error["fields"].get(field).is_some(), "{case}: {error}");
    }

    let (_, unchanged) = send(
        &app,
        Some(&member),
        Method::GET,
        &format!("/api/hackathons/alpha/projects/{id}"),
        "",
    )
    .await?;
    assert_eq!(unchanged, created, "rejected edits change nothing");

    app.close().await
}

/// Runs `write` while another transaction holds the team row lock and checks that it waits.
/// The holder runs `change` as an automatic audited write before releasing the lock.
async fn waits_for_team_lock(
    app: &TestApp,
    world: &World,
    change: &str,
    write: impl std::future::Future<Output = anyhow::Result<(StatusCode, Value)>>,
) -> anyhow::Result<(StatusCode, Value)> {
    use sea_orm::TransactionTrait;
    use std::sync::atomic::{AtomicBool, Ordering};

    let holder = app.db.begin().await?;
    holder
        .execute_unprepared(&format!(
            "SELECT set_config('terrier.automatic','true',true),\
             set_config('terrier.audit_reason','Project write lock test',true),\
             set_config('terrier.hackathon_id','{}',true);\
             SELECT id FROM team WHERE id='{}' FOR UPDATE;",
            world.alpha, world.team
        ))
        .await?;
    let finished = AtomicBool::new(false);
    let (result, released) = tokio::join!(
        async {
            let result = write.await;
            finished.store(true, Ordering::SeqCst);
            result
        },
        async {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let waited = !finished.load(Ordering::SeqCst);
            holder.execute_unprepared(change).await?;
            holder.commit().await?;
            Ok::<_, anyhow::Error>(waited)
        }
    );
    assert!(
        released?,
        "the write finished while the team row was locked"
    );
    result
}

#[tokio::test]
async fn writes_lock_the_team_and_recheck_eligibility() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let world = World::new(&app).await?;
    world.participant(&app, member.user_id, world.team).await?;

    let (status, created) = waits_for_team_lock(
        &app,
        &world,
        "SELECT 1",
        post(&app, &member, world.new_project()),
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let id = created["id"].as_str().unwrap();
    let (status, edited) = waits_for_team_lock(
        &app,
        &world,
        "SELECT 1",
        patch(
            &app,
            &member,
            id,
            json!({"version": created["version"], "data": {"a": 1}}),
        ),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{edited}");

    let revoke = format!(
        "UPDATE event_check_in SET is_valid=false WHERE user_id='{}'",
        member.user_id
    );
    let (status, _) = waits_for_team_lock(
        &app,
        &world,
        &revoke,
        patch(
            &app,
            &member,
            id,
            json!({"version": edited["version"], "data": {"a": 2}}),
        ),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "eligibility lost while waiting for the lock is caught"
    );
    let (_, unchanged) = send(
        &app,
        Some(&member),
        Method::GET,
        &format!("/api/hackathons/alpha/projects/{id}"),
        "",
    )
    .await?;
    assert_eq!(unchanged["data"], json!({"a": 1}));

    app.close().await
}
