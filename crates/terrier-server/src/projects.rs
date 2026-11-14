use anyhow::anyhow;
use axum::{Json, Router, extract::State, routing::get};
use sea_orm::EntityTrait;
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

use crate::{
    auth::policy::{Auth, ProjectReader},
    entities::{sea_orm_active_enums::ProjectStatus, team},
    error::ApiError,
    state::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(read_project))]
pub struct ProjectsApiDoc;

#[derive(Serialize, ToSchema)]
struct Project {
    id: String,
    event_id: String,
    team: Team,
    tracks: Vec<Track>,
    data: serde_json::Value,
    status: &'static str,
    submitted_at: Option<String>,
    version: i32,
}

#[derive(Serialize, ToSchema)]
struct Team {
    id: String,
    name: String,
}

#[derive(Serialize, ToSchema)]
struct Track {
    id: String,
    name: String,
}

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/hackathons/{slug}/projects/{project_id}",
        get(read_project),
    )
}

#[utoipa::path(get, path = "/api/hackathons/{slug}/projects/{project_id}", tag = "projects",
    params(("slug" = String, Path, description = "Hackathon slug"),
        ("project_id" = String, Path, description = "Project UUID")),
    responses((status = 200, body = Project),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn read_project(
    auth: Auth<ProjectReader>,
    State(state): State<AppState>,
) -> Result<Json<Project>, ApiError> {
    let project = &auth.data.project;
    let team = team::Entity::find_by_id(project.team_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::Internal(anyhow!("project {} has no team", project.id)))?;
    Ok(Json(Project {
        id: project.id.to_string(),
        event_id: project.event_id.to_string(),
        team: Team {
            id: team.id.to_string(),
            name: team.name,
        },
        tracks: auth
            .data
            .tracks
            .iter()
            .map(|track| Track {
                id: track.id.to_string(),
                name: track.name.clone(),
            })
            .collect(),
        data: project.data.clone(),
        status: match project.status {
            ProjectStatus::Draft => "draft",
            ProjectStatus::Submitted => "submitted",
        },
        submitted_at: project.submitted_at.map(|time| time.to_rfc3339()),
        version: project.version,
    }))
}
