use axum::{Json, Router, extract::State, routing::get};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

use crate::{
    auth::policy::{Auth, ProjectReader},
    entities::{project_track_sub, sea_orm_active_enums::ProjectStatus},
    error::ApiError,
    state::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(read_project))]
pub struct ProjectsApiDoc;

#[derive(Serialize, ToSchema)]
struct Project {
    id: String,
    team_id: String,
    event_id: String,
    data: serde_json::Value,
    status: &'static str,
    submitted_at: Option<String>,
    track_ids: Vec<String>,
    version: i32,
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
    let project = &auth.data;
    let track_ids = project_track_sub::Entity::find()
        .filter(project_track_sub::Column::ProjectId.eq(project.id))
        .filter(project_track_sub::Column::IsActive.eq(true))
        .all(&state.db)
        .await?
        .into_iter()
        .map(|entry| entry.track_id.to_string())
        .collect();
    Ok(Json(Project {
        id: project.id.to_string(),
        team_id: project.team_id.to_string(),
        event_id: project.event_id.to_string(),
        data: project.data.clone(),
        status: match project.status {
            ProjectStatus::Draft => "draft",
            ProjectStatus::Submitted => "submitted",
        },
        submitted_at: project.submitted_at.map(|time| time.to_rfc3339()),
        track_ids,
        version: project.version,
    }))
}
