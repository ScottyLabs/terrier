use axum::{Json, extract::State, http::StatusCode};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    AppState,
    auth::extractors::{HackathonRole, RequireGlobalAdmin},
    entities::{hackathons, prelude::*},
};

#[derive(Serialize, ToSchema)]
pub struct HackathonInfo {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
}

/// List all active hackathons
#[utoipa::path(
    get,
    path = "/hackathons/public",
    responses(
        (status = 200, description = "List of active hackathons", body = Vec<HackathonInfo>)
    ),
    tag = "Hackathons"
)]
pub async fn list_public_hackathons(
    State(state): State<AppState>,
) -> Result<Json<Vec<HackathonInfo>>, StatusCode> {
    let hackathons = Hackathons::find()
        // .filter(hackathons::Column::IsActive.eq(true))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        hackathons
            .into_iter()
            .map(|h| HackathonInfo {
                id: h.id,
                name: h.name,
                slug: h.slug,
                description: h.description,
                start_date: h.start_date,
                end_date: h.end_date,
            })
            .collect(),
    ))
}

#[derive(Serialize, ToSchema)]
pub struct UserRoleResponse {
    pub role: String,
}

/// Get user's role for a specific hackathon
#[utoipa::path(
    get,
    path = "/hackathons/{slug}/role",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "User's role in this hackathon", body = UserRoleResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "No access to this hackathon"),
        (status = 404, description = "Hackathon not found")
    ),
    tag = "Hackathons"
)]
pub async fn get_user_role(role: HackathonRole) -> Result<Json<UserRoleResponse>, StatusCode> {
    Ok(Json(UserRoleResponse { role: role.role }))
}

#[derive(Deserialize, ToSchema)]
pub struct CreateHackathonRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub start_date: String,
    pub end_date: String,
}

/// Create a new hackathon
#[utoipa::path(
    post,
    path = "/hackathons",
    request_body = CreateHackathonRequest,
    responses(
        (status = 201, description = "Hackathon created successfully", body = HackathonInfo),
        (status = 400, description = "Invalid request or slug already exists"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not a global admin"),
    ),
    tag = "Hackathons"
)]
pub async fn create_hackathon(
    _admin: RequireGlobalAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateHackathonRequest>,
) -> Result<(StatusCode, Json<HackathonInfo>), StatusCode> {
    // Check if slug already exists
    let existing = Hackathons::find()
        .filter(hackathons::Column::Slug.eq(&req.slug))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    println!("{:?}", existing);

    if existing.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create hackathon
    let hackathon = hackathons::ActiveModel {
        name: Set(req.name),
        slug: Set(req.slug),
        description: Set(req.description),
        start_date: Set(
            NaiveDateTime::parse_from_str(&req.start_date, "%Y-%m-%dT%H:%M:%S%.3fZ").unwrap(),
        ),
        end_date: Set(
            NaiveDateTime::parse_from_str(&req.end_date, "%Y-%m-%dT%H:%M:%S%.3fZ").unwrap(),
        ),
        is_active: Set(false),
        ..Default::default()
    };

    let result = hackathon
        .insert(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(HackathonInfo {
            id: result.id,
            name: result.name,
            slug: result.slug,
            description: result.description,
            start_date: result.start_date,
            end_date: result.end_date,
        }),
    ))
}
