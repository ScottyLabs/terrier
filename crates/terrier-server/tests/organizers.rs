mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use common::TestApp;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::{Value, json};
use terrier_server::{
    audit::Scope,
    entities::{
        hackathon, organizer,
        sea_orm_active_enums::{Capability, HackathonRole, JudgeAvailability},
        user_hackathon_role,
    },
};
use uuid::Uuid;

#[tokio::test]
async fn adding_organizer_requires_authentication() -> anyhow::Result<()> {
    let app = TestApp::new().await?;

    let response = app
        .send(Request::post("/api/hackathons/test-hackathon/organizers").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let error: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;

    assert_eq!(error["code"], "unauthenticated");

    app.close().await
}

#[tokio::test]
async fn adding_organizer_requires_existing_hackathon() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;

    let response = app
        .send(
            member
                .authorize(Request::post(
                    "/api/hackathons/nonexistent-hackathon/organizers",
                ))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let error: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;

    assert_eq!(error["code"], "not_found");

    app.close().await
}

async fn create_test_hackathon(app: &TestApp) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();

    let application_deadline =
        sea_orm::prelude::DateTimeWithTimeZone::parse_from_rfc3339("2027-01-01T00:00:00Z")?;
    let team_formation_deadline =
        sea_orm::prelude::DateTimeWithTimeZone::parse_from_rfc3339("2027-01-02T00:00:00Z")?;
    let submission_deadline =
        sea_orm::prelude::DateTimeWithTimeZone::parse_from_rfc3339("2027-01-03T00:00:00Z")?;

    terrier_server::audited_automatic!(
        &app.db,
        Scope::Global,
        "Create hackathon test fixture",
        |db| {
            hackathon::ActiveModel {
                id: Set(id),
                slug: Set("test-hackathon".into()),
                name: Set("Test Hackathon".into()),
                description: Set("Test event".into()),
                is_active: Set(true),
                max_team_size: Set(4),
                max_participants: Set(100),
                location: Set("Pittsburgh".into()),
                contact_email: Set("test@example.com".into()),
                application_schema: Set(serde_json::json!({})),
                data: Set(serde_json::json!({})),
                completed_at: Set(None),
                max_teams_per_table: Set(2),
                application_deadline: Set(application_deadline),
                team_formation_deadline: Set(team_formation_deadline),
                submission_deadline: Set(submission_deadline),
                project_schema: Set(serde_json::json!({})),
                directory_schema: Set(serde_json::json!({})),
                directory_fields: Set(serde_json::json!([])),
                reimbursement_categories: Set(serde_json::json!({})),
                sponsors: Set(serde_json::json!({})),
                version: Set(1),
                file_fields: Set(serde_json::json!({})),
            }
            .insert(db)
            .await?;

            Ok(())
        }
    )
    .await
    .map_err(|error| anyhow::anyhow!("Failed to create test hackathon: {error:?}"))?;

    Ok(id)
}

#[tokio::test]
async fn can_create_test_hackathon() -> anyhow::Result<()> {
    let app = TestApp::new().await?;

    create_test_hackathon(&app).await?;

    app.close().await
}

async fn create_test_admin(app: &TestApp, hackathon_id: Uuid, user_id: Uuid) -> anyhow::Result<()> {
    terrier_server::audited_automatic!(
        &app.db,
        Scope::HackathonExclusive(hackathon_id),
        "Create test hackathon administrator",
        |db| {
            user_hackathon_role::ActiveModel {
                user_id: Set(user_id),
                hackathon_id: Set(hackathon_id),
                role: Set(HackathonRole::Organizer),
                profile_data: Set(json!({})),
                directory_opt_in: Set(false),
                is_active: Set(true),
                judge_availability: Set(JudgeAvailability::Available),
                version: Set(1),
            }
            .insert(db)
            .await?;

            organizer::ActiveModel {
                user_id: Set(user_id),
                hackathon_id: Set(hackathon_id),
                capabilities: Set(vec![Capability::HackathonAdmin]),
                is_active: Set(true),
                version: Set(1),
            }
            .insert(db)
            .await?;

            Ok(())
        }
    )
    .await
    .map_err(|error| anyhow::anyhow!("Failed to create test admin: {error:?}"))?;

    Ok(())
}

#[tokio::test]
async fn admin_can_add_organizer() -> anyhow::Result<()> {
    let app = TestApp::new().await?;

    let admin = app.login("admin").await?;
    let target = app.login("target").await?;

    let hackathon_id = create_test_hackathon(&app).await?;
    create_test_admin(&app, hackathon_id, admin.user_id).await?;

    let response = app
        .send(
            admin
                .authorize(Request::post("/api/hackathons/test-hackathon/organizers"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "user_id": target.user_id.to_string(),
                        "capabilities": ["judging_management"]
                    })
                    .to_string(),
                ))?,
        )
        .await?;

    let status = response.status();
    let body = to_bytes(response.into_body(), 4096).await?;

    assert_eq!(
        status,
        StatusCode::CREATED,
        "Unexpected response: {}",
        String::from_utf8_lossy(&body)
    );

    let role = user_hackathon_role::Entity::find_by_id((target.user_id, hackathon_id))
        .one(&app.db)
        .await?
        .expect("Missing user hackathon role");

    assert_eq!(role.role, HackathonRole::Organizer);
    assert!(role.is_active);

    let organizer = organizer::Entity::find_by_id((target.user_id, hackathon_id))
        .one(&app.db)
        .await?
        .expect("Missing organizer");

    assert!(organizer.is_active);
    assert_eq!(organizer.capabilities, vec![Capability::JudgingManagement]);

    app.close().await
}

#[tokio::test]
async fn ordinary_user_cannot_add_organizer() -> anyhow::Result<()> {
    let app = TestApp::new().await?;

    let member = app.login("member").await?;
    let target = app.login("target").await?;

    let hackathon_id = create_test_hackathon(&app).await?;

    let response = app
        .send(
            member
                .authorize(Request::post("/api/hackathons/test-hackathon/organizers"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "user_id": target.user_id.to_string(),
                        "capabilities": ["judging_management"]
                    })
                    .to_string(),
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let organizer = organizer::Entity::find_by_id((target.user_id, hackathon_id))
        .one(&app.db)
        .await?;

    assert!(organizer.is_none());

    app.close().await
}

#[tokio::test]
async fn cannot_add_same_organizer_twice() -> anyhow::Result<()> {
    let app = TestApp::new().await?;

    let admin = app.login("admin").await?;
    let target = app.login("target").await?;

    let hackathon_id = create_test_hackathon(&app).await?;
    create_test_admin(&app, hackathon_id, admin.user_id).await?;

    // First request succeeds; second request should be rejected.
    for expected_status in [StatusCode::CREATED, StatusCode::BAD_REQUEST] {
        let response = app
            .send(
                admin
                    .authorize(Request::post("/api/hackathons/test-hackathon/organizers"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "user_id": target.user_id.to_string(),
                            "capabilities": ["judging_management"]
                        })
                        .to_string(),
                    ))?,
            )
            .await?;

        assert_eq!(response.status(), expected_status);
    }

    // Verify the original organizer record is unchanged.
    let organizer = organizer::Entity::find_by_id((target.user_id, hackathon_id))
        .one(&app.db)
        .await?
        .expect("Missing organizer");

    assert!(organizer.is_active);
    assert_eq!(organizer.capabilities, vec![Capability::JudgingManagement]);

    app.close().await
}

#[tokio::test]
async fn admin_can_deactivate_organizer() -> anyhow::Result<()> {
    let app = TestApp::new().await?;

    let admin = app.login("admin").await?;
    let target = app.login("target").await?;

    let hackathon_id = create_test_hackathon(&app).await?;

    // Give both users organizer roles.
    create_test_admin(&app, hackathon_id, admin.user_id).await?;
    create_test_admin(&app, hackathon_id, target.user_id).await?;

    // Admin deactivates the target organizer.
    let response = app
        .send(
            admin
                .authorize(Request::patch(format!(
                    "/api/hackathons/test-hackathon/organizers/{}",
                    target.user_id
                )))
                .header("content-type", "application/json")
                .body(Body::from(json!({"is_active": false}).to_string()))?,
        )
        .await?;

    let status = response.status();
    let body = to_bytes(response.into_body(), 4096).await?;

    assert_eq!(
        status,
        StatusCode::NO_CONTENT,
        "Unexpected response: {}",
        String::from_utf8_lossy(&body)
    );

    // Verify organizer record is inactive.
    let organizer = organizer::Entity::find_by_id((target.user_id, hackathon_id))
        .one(&app.db)
        .await?
        .expect("Missing organizer");

    assert!(!organizer.is_active);

    // Verify the corresponding hackathon role is inactive.
    let role = user_hackathon_role::Entity::find_by_id((target.user_id, hackathon_id))
        .one(&app.db)
        .await?
        .expect("Missing hackathon role");

    assert!(!role.is_active);

    // Verify the administrator remains active.
    let admin_organizer = organizer::Entity::find_by_id((admin.user_id, hackathon_id))
        .one(&app.db)
        .await?
        .expect("Missing administrator");

    assert!(admin_organizer.is_active);

    app.close().await
}

#[tokio::test]
async fn ordinary_user_cannot_deactivate_organizer() -> anyhow::Result<()> {
    let app = TestApp::new().await?;

    let member = app.login("member").await?;
    let target = app.login("target").await?;

    let hackathon_id = create_test_hackathon(&app).await?;
    create_test_admin(&app, hackathon_id, target.user_id).await?;

    let response = app
        .send(
            member
                .authorize(Request::patch(format!(
                    "/api/hackathons/test-hackathon/organizers/{}",
                    target.user_id
                )))
                .header("content-type", "application/json")
                .body(Body::from(json!({"is_active": false}).to_string()))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let organizer = organizer::Entity::find_by_id((target.user_id, hackathon_id))
        .one(&app.db)
        .await?
        .expect("Missing organizer");

    assert!(organizer.is_active);

    app.close().await
}
