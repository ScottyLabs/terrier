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
    audited_automatic!(&app.db, scope, "Project list test fixture", |tx| {
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

async fn read(app: &TestApp, session: &TestSession, path: &str) -> anyhow::Result<Value> {
    let response = get(app, Some(session), path).await?;
    assert_eq!(response.status(), StatusCode::OK, "{path}");
    Ok(serde_json::from_slice(
        &to_bytes(response.into_body(), 65536).await?,
    )?)
}

/// Hackathon `alpha` holds three teams, named so that name order differs from creation order:
/// - "Bravo" has a draft project entered in the open track.
/// - "Alpha" has a submitted project entered in the open and sponsored tracks.
/// - "Charlie" has no project.
///
/// Hackathon `beta` holds one team with a submitted project in its own track. The caller is a hacker in alpha with no team.
struct World {
    alpha: Uuid,
    caller: Uuid,
    organization: Uuid,
    open_track: Uuid,
    sponsored_track: Uuid,
    beta_track: Uuid,
    bravo: Uuid,
    alpha_team: Uuid,
    charlie: Uuid,
    beta_team: Uuid,
    draft: Uuid,
    submitted: Uuid,
}

impl World {
    async fn new(app: &TestApp, caller: Uuid) -> anyhow::Result<Self> {
        let [
            alpha,
            beta,
            organization,
            open_track,
            sponsored_track,
            beta_track,
            bravo,
            alpha_team,
            charlie,
            beta_team,
            draft,
            submitted,
            beta_project,
            event,
            beta_event,
        ] = std::array::from_fn(|_| Uuid::new_v4());
        let teams = [
            (alpha, bravo, "Bravo"),
            (alpha, alpha_team, "Alpha"),
            (alpha, charlie, "Charlie"),
            (beta, beta_team, "Beta"),
        ]
        .map(|(hackathon, team, name)| (hackathon, team, name, Uuid::new_v4()));
        let mut sql = format!(
            "INSERT INTO sponsor_org(id,name,description) \
             VALUES('{organization}','Sponsor','Sponsor');"
        );
        for (_, _, _, leader) in teams {
            sql.push_str(&format!(
                "INSERT INTO \"user\"(id,email,display_name,oidc_issuer,oidc_subject) \
                 VALUES('{leader}','{leader}@example.com','Leader','https://example.com',\
                 '{leader}');"
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
        seed(app, Scope::Global, sql).await?;

        for (hackathon, event) in [(alpha, event), (beta, beta_event)] {
            let mut sql = format!(
                "INSERT INTO event(id,hackathon_id,name,description,start_time,end_time,location) \
                 VALUES('{event}','{hackathon}','Expo','Expo',now(),now()+interval '1 hour',\
                 'Campus');"
            );
            for (_, team, name, leader) in teams.iter().filter(|team| team.0 == hackathon) {
                sql.push_str(&format!(
                    "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
                     VALUES('{leader}','{hackathon}','hacker');\
                     INSERT INTO team(id,hackathon_id,name) \
                     VALUES('{team}','{hackathon}','{name}');\
                     INSERT INTO team_member(team_id,user_id,hackathon_id,status,is_leader,\
                     requested_at,joined_at) \
                     VALUES('{team}','{leader}','{hackathon}','active',true,now(),now());"
                ));
            }
            seed(app, Scope::HackathonExclusive(hackathon), sql).await?;
        }
        seed(
            app,
            Scope::HackathonExclusive(alpha),
            format!(
                "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
                 VALUES('{caller}','{alpha}','hacker');\
                 INSERT INTO prize_track(id,hackathon_id,sponsor_org_id,name,description,\
                 is_judging_started,judging_method) \
                 VALUES('{open_track}','{alpha}',NULL,'Open','Open',false,'scoring'),\
                 ('{sponsored_track}','{alpha}','{organization}','Sponsored','Sponsored',false,\
                 'scoring');"
            ),
        )
        .await?;
        seed(
            app,
            Scope::HackathonExclusive(beta),
            format!(
                "INSERT INTO prize_track(id,hackathon_id,name,description,is_judging_started,\
                 judging_method) VALUES('{beta_track}','{beta}','Beta','Beta',false,'scoring');"
            ),
        )
        .await?;
        seed(
            app,
            Scope::Hackathon(alpha),
            format!(
                "INSERT INTO project(id,hackathon_id,event_id,team_id,data) \
                 VALUES('{draft}','{alpha}','{event}','{bravo}','{{\"title\":\"Draft\"}}');\
                 INSERT INTO project(id,hackathon_id,event_id,team_id,data,status,submitted_at) \
                 VALUES('{submitted}','{alpha}','{event}','{alpha_team}',\
                 '{{\"title\":\"Submitted\"}}','submitted',clock_timestamp());\
                 INSERT INTO project_track_sub(project_id,track_id,hackathon_id) \
                 VALUES('{draft}','{open_track}','{alpha}'),\
                 ('{submitted}','{open_track}','{alpha}'),\
                 ('{submitted}','{sponsored_track}','{alpha}');"
            ),
        )
        .await?;
        seed(
            app,
            Scope::Hackathon(beta),
            format!(
                "INSERT INTO project(id,hackathon_id,event_id,team_id,data,status,submitted_at) \
                 VALUES('{beta_project}','{beta}','{beta_event}','{beta_team}','{{}}',\
                 'submitted',clock_timestamp());\
                 INSERT INTO project_track_sub(project_id,track_id,hackathon_id) \
                 VALUES('{beta_project}','{beta_track}','{beta}');"
            ),
        )
        .await?;
        Ok(Self {
            alpha,
            caller,
            organization,
            open_track,
            sponsored_track,
            beta_track,
            bravo,
            alpha_team,
            charlie,
            beta_team,
            draft,
            submitted,
        })
    }

    fn team_path(&self, team: Uuid) -> String {
        format!("/api/hackathons/alpha/teams/{team}/projects")
    }

    fn project_path(&self, project: Uuid) -> String {
        format!("/api/hackathons/alpha/projects/{project}")
    }

    async fn join(&self, app: &TestApp, team: Uuid) -> anyhow::Result<()> {
        let Self { alpha, caller, .. } = self;
        seed(
            app,
            Scope::Hackathon(*alpha),
            format!(
                "SELECT id FROM team WHERE id='{team}' FOR UPDATE;\
                 INSERT INTO team_member(team_id,user_id,hackathon_id,status,is_leader,\
                 requested_at,joined_at) \
                 VALUES('{team}','{caller}','{alpha}','active',false,now(),now());"
            ),
        )
        .await
    }

    async fn set_role(&self, app: &TestApp, role: &str) -> anyhow::Result<()> {
        let Self { alpha, caller, .. } = self;
        seed(
            app,
            Scope::HackathonExclusive(*alpha),
            format!(
                "UPDATE user_hackathon_role SET role='{role}' \
                 WHERE user_id='{caller}' AND hackathon_id='{alpha}';"
            ),
        )
        .await
    }

    /// Makes the caller an organizer in alpha with the given capabilities.
    async fn organize(&self, app: &TestApp, capabilities: &[&str]) -> anyhow::Result<()> {
        let Self { alpha, caller, .. } = self;
        self.set_role(app, "organizer").await?;
        let capabilities = capabilities
            .iter()
            .map(|capability| format!("'{capability}'"))
            .collect::<Vec<_>>()
            .join(",");
        seed(
            app,
            Scope::HackathonExclusive(*alpha),
            format!(
                "INSERT INTO organizer(user_id,hackathon_id,capabilities) \
                 VALUES('{caller}','{alpha}',ARRAY[{capabilities}]::capability[]) \
                 ON CONFLICT (user_id,hackathon_id) \
                 DO UPDATE SET capabilities=EXCLUDED.capabilities, is_active=true;"
            ),
        )
        .await
    }
}

#[tokio::test]
async fn team_project_lists_validate_scope_and_session() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let caller = app.login("caller").await?;
    let session = Some(&caller);
    let world = World::new(&app, caller.user_id).await?;
    world.organize(&app, &[]).await?;

    assert_eq!(
        status(&app, None, &world.team_path(world.bravo)).await?,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status(
            &app,
            session,
            "/api/hackathons/alpha/teams/not-a-uuid/projects"
        )
        .await?,
        StatusCode::BAD_REQUEST
    );
    for missing in [
        format!("/api/hackathons/beta/teams/{}/projects", world.bravo),
        format!("/api/hackathons/unknown/teams/{}/projects", world.bravo),
        world.team_path(world.beta_team),
        world.team_path(Uuid::new_v4()),
    ] {
        assert_eq!(
            status(&app, session, &missing).await?,
            StatusCode::NOT_FOUND,
            "{missing}"
        );
    }

    seed(
        &app,
        Scope::HackathonExclusive(world.alpha),
        format!(
            "SELECT id FROM team WHERE id='{team}' FOR UPDATE;\
             UPDATE team_member SET status='removed', is_leader=false, ended_at=now() \
             WHERE team_id='{team}';\
             UPDATE team SET is_active=false WHERE id='{team}';",
            team = world.bravo
        ),
    )
    .await?;
    assert_eq!(
        status(&app, session, &world.team_path(world.bravo)).await?,
        StatusCode::NOT_FOUND,
        "inactive teams are hidden"
    );

    app.close().await
}

#[tokio::test]
async fn team_members_list_their_draft_or_an_empty_list() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let caller = app.login("caller").await?;
    let session = Some(&caller);
    let world = World::new(&app, caller.user_id).await?;

    for team in [world.bravo, world.charlie] {
        assert_eq!(
            status(&app, session, &world.team_path(team)).await?,
            StatusCode::FORBIDDEN,
            "outsiders cannot list a team's project"
        );
    }

    world.join(&app, world.bravo).await?;
    let listed = read(&app, &caller, &world.team_path(world.bravo)).await?;
    let single = read(&app, &caller, &world.project_path(world.draft)).await?;
    assert_eq!(listed, json!([single]));
    assert_eq!(listed[0]["status"], "draft");
    assert_eq!(listed[0]["team"]["name"], "Bravo");
    assert_eq!(
        status(&app, session, &world.team_path(world.alpha_team)).await?,
        StatusCode::FORBIDDEN
    );

    let other = app.login("other").await?;
    let other_world = World {
        caller: other.user_id,
        ..world
    };
    seed(
        &app,
        Scope::HackathonExclusive(other_world.alpha),
        format!(
            "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) \
                 VALUES('{}','{}','hacker');",
            other.user_id, other_world.alpha
        ),
    )
    .await?;
    other_world.join(&app, other_world.charlie).await?;
    assert_eq!(
        read(&app, &other, &other_world.team_path(other_world.charlie)).await?,
        json!([])
    );

    app.close().await
}

#[tokio::test]
async fn organizers_list_any_team_project() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let caller = app.login("caller").await?;
    let world = World::new(&app, caller.user_id).await?;
    world.set_role(&app, "organizer").await?;

    let listed = read(&app, &caller, &world.team_path(world.bravo)).await?;
    assert_eq!(listed[0]["id"], json!(world.draft.to_string()));
    assert_eq!(
        read(&app, &caller, &world.team_path(world.charlie)).await?,
        json!([])
    );

    seed(
        &app,
        Scope::HackathonExclusive(world.alpha),
        format!(
            "UPDATE user_hackathon_role SET is_active=false \
                 WHERE user_id='{}' AND hackathon_id='{}';",
            world.caller, world.alpha
        ),
    )
    .await?;
    for team in [world.bravo, world.charlie] {
        assert_eq!(
            status(&app, Some(&caller), &world.team_path(team)).await?,
            StatusCode::FORBIDDEN,
            "inactive organizers lose access"
        );
    }

    app.close().await
}

#[tokio::test]
async fn sponsors_and_judges_list_only_submitted_projects_they_can_read() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let caller = app.login("caller").await?;
    let session = Some(&caller);
    let world = World::new(&app, caller.user_id).await?;
    let World {
        alpha,
        caller: caller_id,
        organization,
        ..
    } = world;

    world.set_role(&app, "sponsor").await?;
    seed(
        &app,
        Scope::HackathonExclusive(alpha),
        format!(
            "INSERT INTO sponsor(user_id,org_id,hackathon_id) \
                 VALUES('{caller_id}','{organization}','{alpha}');"
        ),
    )
    .await?;
    let listed = read(&app, &caller, &world.team_path(world.alpha_team)).await?;
    assert_eq!(listed[0]["id"], json!(world.submitted.to_string()));
    for team in [world.bravo, world.charlie] {
        assert_eq!(
            status(&app, session, &world.team_path(team)).await?,
            StatusCode::FORBIDDEN,
            "sponsors cannot list drafts or teams without a project"
        );
    }

    seed(
        &app,
        Scope::HackathonExclusive(alpha),
        format!(
            "UPDATE sponsor SET is_active=false WHERE user_id='{caller_id}';\
                 UPDATE user_hackathon_role SET role='judge' \
                 WHERE user_id='{caller_id}' AND hackathon_id='{alpha}';\
                 INSERT INTO judge_track_assign(user_id,track_id,hackathon_id,manually_granted) \
                 VALUES('{caller_id}','{}','{alpha}',true);",
            world.open_track
        ),
    )
    .await?;
    assert_eq!(
        status(&app, session, &world.team_path(world.alpha_team)).await?,
        StatusCode::FORBIDDEN,
        "a track grant without an assignment does not grant access"
    );
    seed(
        &app,
        Scope::Hackathon(alpha),
        format!(
            "INSERT INTO evaluation(id,track_id,project_id,judge_id,hackathon_id,\
                 judging_method,status,created_at) \
                 VALUES('{}','{}','{}','{caller_id}','{alpha}','scoring','pending',now());",
            Uuid::new_v4(),
            world.open_track,
            world.submitted
        ),
    )
    .await?;
    let listed = read(&app, &caller, &world.team_path(world.alpha_team)).await?;
    assert_eq!(listed[0]["id"], json!(world.submitted.to_string()));
    for team in [world.bravo, world.charlie] {
        assert_eq!(
            status(&app, session, &world.team_path(team)).await?,
            StatusCode::FORBIDDEN,
            "judges cannot list drafts or teams without a project"
        );
    }

    app.close().await
}

#[tokio::test]
async fn project_lists_require_judging_management_or_admin() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let caller = app.login("caller").await?;
    let session = Some(&caller);
    let world = World::new(&app, caller.user_id).await?;
    let path = "/api/hackathons/alpha/projects";

    assert_eq!(status(&app, None, path).await?, StatusCode::UNAUTHORIZED);
    assert_eq!(status(&app, session, path).await?, StatusCode::FORBIDDEN);
    world.set_role(&app, "organizer").await?;
    assert_eq!(
        status(&app, session, path).await?,
        StatusCode::FORBIDDEN,
        "organizers need a management capability"
    );
    world
        .organize(&app, &["application_management", "sponsorship_management"])
        .await?;
    assert_eq!(status(&app, session, path).await?, StatusCode::FORBIDDEN);

    for capability in ["judging_management", "hackathon_admin"] {
        world.organize(&app, &[capability]).await?;
        assert_eq!(
            status(&app, session, path).await?,
            StatusCode::OK,
            "{capability}"
        );
    }

    seed(
        &app,
        Scope::HackathonExclusive(world.alpha),
        format!(
            "UPDATE organizer SET is_active=false \
                 WHERE user_id='{}' AND hackathon_id='{}';",
            world.caller, world.alpha
        ),
    )
    .await?;
    assert_eq!(
        status(&app, session, path).await?,
        StatusCode::FORBIDDEN,
        "inactive organizer grants do not count"
    );

    world.organize(&app, &["judging_management"]).await?;
    seed(
        &app,
        Scope::HackathonExclusive(world.alpha),
        format!(
            "UPDATE user_hackathon_role SET is_active=false \
                 WHERE user_id='{}' AND hackathon_id='{}';",
            world.caller, world.alpha
        ),
    )
    .await?;
    assert_eq!(
        status(&app, session, path).await?,
        StatusCode::FORBIDDEN,
        "capabilities require an active organizer role"
    );
    assert_eq!(
        status(&app, session, "/api/hackathons/beta/projects").await?,
        StatusCode::FORBIDDEN,
        "capabilities in alpha do not apply to beta"
    );

    app.close().await
}

#[tokio::test]
async fn project_lists_cover_one_hackathon_ordered_by_team_name() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let caller = app.login("caller").await?;
    let world = World::new(&app, caller.user_id).await?;
    world.organize(&app, &["judging_management"]).await?;

    let listed = read(&app, &caller, "/api/hackathons/alpha/projects").await?;
    let submitted = read(&app, &caller, &world.project_path(world.submitted)).await?;
    let draft = read(&app, &caller, &world.project_path(world.draft)).await?;
    assert_eq!(listed, json!([submitted, draft]));
    assert_eq!(
        listed[0]["tracks"],
        json!([
            {"id": world.open_track.to_string(), "name": "Open"},
            {"id": world.sponsored_track.to_string(), "name": "Sponsored"},
        ])
    );
    assert_eq!(listed[1]["data"], json!({"title": "Draft"}));

    seed(
        &app,
        Scope::HackathonExclusive(world.alpha),
        format!(
            "UPDATE prize_track SET is_active=false WHERE id='{}';",
            world.sponsored_track
        ),
    )
    .await?;
    let listed = read(&app, &caller, "/api/hackathons/alpha/projects").await?;
    assert_eq!(
        listed[0]["tracks"],
        json!([{"id": world.open_track.to_string(), "name": "Open"}]),
        "inactive tracks are omitted"
    );

    app.close().await
}

#[tokio::test]
async fn project_lists_filter_by_track() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let caller = app.login("caller").await?;
    let session = Some(&caller);
    let world = World::new(&app, caller.user_id).await?;
    world.organize(&app, &["judging_management"]).await?;
    let ids = |listed: &Value| -> Vec<String> {
        listed
            .as_array()
            .unwrap()
            .iter()
            .map(|project| project["id"].as_str().unwrap().to_owned())
            .collect()
    };
    let filtered = |track: Uuid| format!("/api/hackathons/alpha/projects?track_id={track}");

    assert_eq!(
        ids(&read(&app, &caller, &filtered(world.open_track)).await?),
        [world.submitted.to_string(), world.draft.to_string()]
    );
    assert_eq!(
        ids(&read(&app, &caller, &filtered(world.sponsored_track)).await?),
        [world.submitted.to_string()]
    );

    seed(
        &app,
        Scope::Hackathon(world.alpha),
        format!(
            "UPDATE project_track_sub SET is_active=false \
                 WHERE project_id='{}' AND track_id='{}';",
            world.submitted, world.sponsored_track
        ),
    )
    .await?;
    assert_eq!(
        read(&app, &caller, &filtered(world.sponsored_track)).await?,
        json!([]),
        "withdrawn track entries are excluded"
    );

    seed(
        &app,
        Scope::HackathonExclusive(world.alpha),
        format!(
            "UPDATE prize_track SET is_active=false WHERE id='{}';",
            world.sponsored_track
        ),
    )
    .await?;
    for (path, expected) in [
        (filtered(world.sponsored_track), StatusCode::NOT_FOUND),
        (filtered(world.beta_track), StatusCode::NOT_FOUND),
        (filtered(Uuid::new_v4()), StatusCode::NOT_FOUND),
        (
            "/api/hackathons/alpha/projects?track_id=not-a-uuid".into(),
            StatusCode::BAD_REQUEST,
        ),
        (
            format!("/api/hackathons/alpha/projects?track={}", world.open_track),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_eq!(status(&app, session, &path).await?, expected, "{path}");
    }

    app.close().await
}

#[tokio::test]
async fn inactive_hackathons_hide_project_lists() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let caller = app.login("caller").await?;
    let session = Some(&caller);
    let world = World::new(&app, caller.user_id).await?;
    world.organize(&app, &["hackathon_admin"]).await?;
    seed(
        &app,
        Scope::HackathonExclusive(world.alpha),
        format!(
            "UPDATE hackathon SET is_active=false WHERE id='{}';",
            world.alpha
        ),
    )
    .await?;
    for path in [
        "/api/hackathons/alpha/projects".into(),
        world.team_path(world.bravo),
    ] {
        assert_eq!(
            status(&app, session, &path).await?,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }

    app.close().await
}
