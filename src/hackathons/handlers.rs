use axum::{Json, extract::State, http::StatusCode};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use serde::Serialize;

use crate::{
    AppState,
    auth::extractors::{HackathonRole, RequireGlobalAdmin},
    entities::{hackathons, prelude::*},
    types::HackathonInfo,
};

/// List all active hackathons
pub async fn list_public_hackathons(
    State(state): State<AppState>,
) -> Result<Json<Vec<HackathonInfo>>, StatusCode> {
    let hackathons = Hackathons::find()
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
                is_active: h.is_active,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct CreateHackathonRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub start_date: String,
    pub end_date: String,
}

/// Create a new hackathon
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

    if existing.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create hackathon
    let hackathon = hackathons::ActiveModel {
        name: Set(req.name),
        slug: Set(req.slug),
        description: Set(req.description),
        start_date: Set(
            NaiveDateTime::parse_from_str(&req.start_date, "%Y-%m-%dT%H:%M:%S%.3fZ")
                .or_else(|_| NaiveDateTime::parse_from_str(&req.start_date, "%Y-%m-%dT%H:%M:%S"))
                .map_err(|_| StatusCode::BAD_REQUEST)?,
        ),
        end_date: Set(
            NaiveDateTime::parse_from_str(&req.end_date, "%Y-%m-%dT%H:%M:%S%.3fZ")
                .or_else(|_| NaiveDateTime::parse_from_str(&req.end_date, "%Y-%m-%dT%H:%M:%S"))
                .map_err(|_| StatusCode::BAD_REQUEST)?,
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
            is_active: result.is_active,
        }),
    ))
}

#[derive(Serialize)]
pub struct UserRoleResponse {
    pub role: String,
}

/// Get user's role for a specific hackathon
pub async fn get_user_role(role: HackathonRole) -> Result<Json<UserRoleResponse>, StatusCode> {
    Ok(Json(UserRoleResponse { role: role.role }))
}
