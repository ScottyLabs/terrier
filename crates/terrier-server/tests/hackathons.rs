mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use common::TestApp;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use terrier_server::entities::organizer;

#[tokio::test]
async fn admin_can_create_configure_and_deactivate_a_hackathon() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let admin = app.login("instance-admin").await?;
    let member = app.login("ordinary-member").await?;
    let collection = "/api/hackathons";
    let resource = "/api/hackathons/test-hackathon";
    let mut settings = json!({
        "name": "Test Hackathon",
        "description": "A test event",
        "location": "Campus",
        "contact_email": "events@example.com",
        "max_team_size": 4,
        "max_participants": 120,
        "application_deadline": "2027-01-01T00:00:00Z",
        "team_formation_deadline": "2027-01-02T00:00:00Z",
        "submission_deadline": "2027-01-03T00:00:00Z",
        "application_schema": {"fields": []}
    });
    let input = json!({"slug": "test-hackathon", "settings": settings});
    let forbidden = app
        .send(
            member
                .authorize(Request::post(collection))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(input.to_string()))?,
        )
        .await?;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    for expected in [StatusCode::CREATED, StatusCode::CONFLICT] {
        let response = app
            .send(
                admin
                    .authorize(Request::post(collection))
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(input.to_string()))?,
            )
            .await?;
        assert_eq!(response.status(), expected);
    }

    assert!(
        organizer::Entity::find()
            .filter(organizer::Column::UserId.eq(admin.user_id))
            .one(&app.db)
            .await?
            .is_some()
    );

    let response = app
        .send(Request::get(resource).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let public: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    assert_eq!(public["settings"]["name"], "Test Hackathon");
    let version = &public["version"];
    settings["name"] = json!("Updated Hackathon");
    let update = json!({"version": version, "settings": settings});

    for (actor, expected) in [
        (&member, StatusCode::FORBIDDEN),
        (&admin, StatusCode::OK),
        (&admin, StatusCode::CONFLICT),
    ] {
        let response = app
            .send(
                actor
                    .authorize(Request::patch(resource))
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(update.to_string()))?,
            )
            .await?;
        assert_eq!(response.status(), expected);
        if expected == StatusCode::OK {
            let saved: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
            assert_eq!(saved["settings"]["name"], "Updated Hackathon");
            assert_ne!(saved["version"], *version);
        }
    }
    let response = app
        .send(Request::get(resource).body(Body::empty())?)
        .await?;
    let current: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    let response = app
        .send(
            admin
                .authorize(Request::patch(resource))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"version": current["version"], "is_active": false}).to_string(),
                ))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .send(Request::get(resource).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    app.close().await
}

#[tokio::test]
async fn setup_rejects_malformed_application_forms() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let admin = app.login("instance-admin").await?;
    let settings = |form: Value| {
        json!({
            "name": "Test Hackathon",
            "description": "A test event",
            "location": "Campus",
            "contact_email": "events@example.com",
            "max_team_size": 4,
            "max_participants": 120,
            "application_deadline": "2027-01-01T00:00:00Z",
            "team_formation_deadline": "2027-01-02T00:00:00Z",
            "submission_deadline": "2027-01-03T00:00:00Z",
            "application_schema": form
        })
    };
    let create = |slug: &str, form: Value| {
        admin
            .authorize(Request::post("/api/hackathons"))
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({"slug": slug, "settings": settings(form)}).to_string(),
            ))
    };

    for (form, field) in [
        (json!([]), "settings.application_schema"),
        (
            json!({"fields": [{"id": "school", "label": "School", "type": "essay", "required": true}]}),
            "settings.application_schema",
        ),
        (
            json!({"fields": [
                {"id": "school", "label": "School", "type": "text", "required": true},
                {"id": "school", "label": "College", "type": "text", "required": false}
            ]}),
            "settings.application_schema.fields[1].id",
        ),
        (
            json!({"fields": [{"id": "shirt", "label": "Shirt", "type": "single_choice", "required": true}]}),
            "settings.application_schema.fields[0].options",
        ),
    ] {
        let response = app.send(create("malformed", form.clone())?).await?;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{form}"
        );
        let error: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
        assert!(error["fields"].get(field).is_some(), "{form}: {error}");
    }

    let form = json!({"fields": [
        {"id": "school", "label": "School", "type": "text", "required": true},
        {"id": "shirt", "label": "Shirt size", "type": "single_choice", "required": false,
         "options": [{"id": "s", "label": "Small"}]}
    ]});
    let response = app.send(create("valid-form", form.clone())?).await?;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = app
        .send(Request::get("/api/hackathons/valid-form").body(Body::empty())?)
        .await?;
    let public: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    assert_eq!(public["settings"]["application_schema"], form);

    app.close().await
}
