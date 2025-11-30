use axum::{Json, extract::State, http::StatusCode};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    AppState,
    auth::extractors::HackathonRole,
    entities::{applications, prelude::Applications},
};

#[derive(Serialize, ToSchema)]
pub struct ApplicationResponse {
    pub form_data: serde_json::Value,
    pub status: String,
    pub submitted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Get the current user's application for a hackathon
#[utoipa::path(
    get,
    path = "/hackathons/{slug}/application",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "User's application", body = ApplicationResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "No access to this hackathon"),
        (status = 404, description = "No application found")
    ),
    tag = "Applications"
)]
pub async fn get_application(
    State(state): State<AppState>,
    role: HackathonRole,
) -> Result<Json<ApplicationResponse>, StatusCode> {
    let application = Applications::find()
        .filter(applications::Column::UserId.eq(role.user_id))
        .filter(applications::Column::HackathonId.eq(role.hackathon_id))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ApplicationResponse {
        form_data: application.form_data,
        status: application.status,
        submitted_at: application.submitted_at,
        created_at: application.created_at,
        updated_at: application.updated_at,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct SaveApplicationRequest {
    pub form_data: serde_json::Value,
}

#[derive(Serialize, ToSchema)]
pub struct SaveApplicationResponse {
    pub success: bool,
    pub status: String,
    pub updated_at: NaiveDateTime,
}

/// Save or update the current user's application (auto-save/draft)
#[utoipa::path(
    put,
    path = "/hackathons/{slug}/application",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = SaveApplicationRequest,
    responses(
        (status = 200, description = "Application saved", body = SaveApplicationResponse),
        (status = 400, description = "Cannot modify submitted application"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "No access to this hackathon")
    ),
    tag = "Applications"
)]
pub async fn save_application(
    State(state): State<AppState>,
    role: HackathonRole,
    Json(payload): Json<SaveApplicationRequest>,
) -> Result<Json<SaveApplicationResponse>, StatusCode> {
    let now = Utc::now().naive_utc();

    // Check if application already exists
    let existing = Applications::find()
        .filter(applications::Column::UserId.eq(role.user_id))
        .filter(applications::Column::HackathonId.eq(role.hackathon_id))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match existing {
        Some(app) => {
            // Don't allow editing applications under review
            if app.status == "under_review" {
                return Err(StatusCode::BAD_REQUEST);
            }

            // Update existing application (revert to draft if it was submitted)
            let mut active: applications::ActiveModel = app.into();
            active.form_data = Set(payload.form_data);
            active.status = Set("draft".to_string()); // Always set back to draft on edit
            active.updated_at = Set(now);

            let updated = active
                .update(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(Json(SaveApplicationResponse {
                success: true,
                status: updated.status,
                updated_at: updated.updated_at,
            }))
        }
        None => {
            // Create new application
            let new_application = applications::ActiveModel {
                user_id: Set(role.user_id),
                hackathon_id: Set(role.hackathon_id),
                form_data: Set(payload.form_data),
                status: Set("draft".to_string()),
                submitted_at: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };

            let inserted = new_application
                .insert(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(Json(SaveApplicationResponse {
                success: true,
                status: inserted.status,
                updated_at: inserted.updated_at,
            }))
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct SubmitApplicationResponse {
    pub success: bool,
    pub submitted_at: NaiveDateTime,
}

/// Submit the current user's application
#[utoipa::path(
    post,
    path = "/hackathons/{slug}/application/submit",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Application submitted", body = SubmitApplicationResponse),
        (status = 400, description = "Application already submitted or no draft found"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "No access to this hackathon"),
        (status = 404, description = "No application found to submit")
    ),
    tag = "Applications"
)]
pub async fn submit_application(
    State(state): State<AppState>,
    role: HackathonRole,
) -> Result<Json<SubmitApplicationResponse>, StatusCode> {
    let now = Utc::now().naive_utc();

    // Find existing application
    let existing = Applications::find()
        .filter(applications::Column::UserId.eq(role.user_id))
        .filter(applications::Column::HackathonId.eq(role.hackathon_id))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Don't allow re-submitting if already submitted or under review
    if existing.status == "submitted" || existing.status == "under_review" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update to submitted status
    let mut active: applications::ActiveModel = existing.into();
    active.status = Set("submitted".to_string());
    active.submitted_at = Set(Some(now));
    active.updated_at = Set(now);

    active
        .update(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SubmitApplicationResponse {
        success: true,
        submitted_at: now,
    }))
}
