use std::collections::HashMap;

use anyhow::anyhow;
use axum::{
    Json, Router,
    extract::{Query, State, rejection::QueryRejection},
    routing::get,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use crate::{
    auth::policy::{Auth, ProjectManager, ProjectReader, TeamProjectReader},
    entities::{
        prize_track, project, project_track_sub, sea_orm_active_enums::ProjectStatus, team,
    },
    error::ApiError,
    state::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(read_project, list_team_projects, list_projects))]
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

#[derive(Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
struct ProjectFilter {
    /// Only list projects entered in this prize track (UUID)
    #[param(value_type = Option<String>)]
    track_id: Option<Uuid>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/hackathons/{slug}/projects", get(list_projects))
        .route(
            "/hackathons/{slug}/projects/{project_id}",
            get(read_project),
        )
        .route(
            "/hackathons/{slug}/teams/{team_id}/projects",
            get(list_team_projects),
        )
}

fn project_response(
    project: &project::Model,
    team: &team::Model,
    tracks: &[prize_track::Model],
) -> Project {
    Project {
        id: project.id.to_string(),
        event_id: project.event_id.to_string(),
        team: Team {
            id: team.id.to_string(),
            name: team.name.clone(),
        },
        tracks: tracks
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
    }
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
    Ok(Json(project_response(project, &team, &auth.data.tracks)))
}

#[utoipa::path(get, path = "/api/hackathons/{slug}/teams/{team_id}/projects", tag = "projects",
    params(("slug" = String, Path, description = "Hackathon slug"),
        ("team_id" = String, Path, description = "Team UUID")),
    responses((status = 200, body = [Project], description = "The team's project, or an empty list if it has none"),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn list_team_projects(auth: Auth<TeamProjectReader>) -> Json<Vec<Project>> {
    let team = &auth.data.team;
    Json(
        auth.data
            .project
            .iter()
            .map(|readable| project_response(&readable.project, team, &readable.tracks))
            .collect(),
    )
}

#[utoipa::path(get, path = "/api/hackathons/{slug}/projects", tag = "projects",
    params(("slug" = String, Path, description = "Hackathon slug"), ProjectFilter),
    responses((status = 200, body = [Project], description = "Projects ordered by team name"),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn list_projects(
    auth: Auth<ProjectManager>,
    State(state): State<AppState>,
    filter: Result<Query<ProjectFilter>, QueryRejection>,
) -> Result<Json<Vec<Project>>, ApiError> {
    let Query(filter) =
        filter.map_err(|_| ApiError::BadRequest("Invalid project filter.".into()))?;
    let db = &state.db;
    let hackathon = &auth.data;
    let mut query = project::Entity::find().filter(project::Column::HackathonId.eq(hackathon.id));
    if let Some(track_id) = filter.track_id {
        prize_track::Entity::find_by_id(track_id)
            .filter(prize_track::Column::HackathonId.eq(hackathon.id))
            .filter(prize_track::Column::IsActive.eq(true))
            .one(db)
            .await?
            .ok_or(ApiError::NotFound)?;
        let entered = project_track_sub::Entity::find()
            .filter(project_track_sub::Column::TrackId.eq(track_id))
            .filter(project_track_sub::Column::IsActive.eq(true))
            .all(db)
            .await?
            .into_iter()
            .map(|entry| entry.project_id);
        query = query.filter(project::Column::Id.is_in(entered));
    }
    Ok(Json(describe(db, query.all(db).await?).await?))
}

/// Builds responses for projects in one hackathon, ordered by team name.
async fn describe(
    db: &DatabaseConnection,
    projects: Vec<project::Model>,
) -> Result<Vec<Project>, ApiError> {
    let teams: HashMap<Uuid, team::Model> = team::Entity::find()
        .filter(team::Column::Id.is_in(projects.iter().map(|project| project.team_id)))
        .all(db)
        .await?
        .into_iter()
        .map(|team| (team.id, team))
        .collect();
    let entries = project_track_sub::Entity::find()
        .filter(
            project_track_sub::Column::ProjectId.is_in(projects.iter().map(|project| project.id)),
        )
        .filter(project_track_sub::Column::IsActive.eq(true))
        .all(db)
        .await?;
    let tracks = prize_track::Entity::find()
        .filter(prize_track::Column::Id.is_in(entries.iter().map(|entry| entry.track_id)))
        .filter(prize_track::Column::IsActive.eq(true))
        .order_by_asc(prize_track::Column::Name)
        .all(db)
        .await?;

    let mut described = projects
        .iter()
        .map(|project| {
            let team = teams
                .get(&project.team_id)
                .ok_or_else(|| ApiError::Internal(anyhow!("project {} has no team", project.id)))?;
            let entered: Vec<_> = tracks
                .iter()
                .filter(|track| {
                    entries
                        .iter()
                        .any(|entry| entry.project_id == project.id && entry.track_id == track.id)
                })
                .cloned()
                .collect();
            Ok(project_response(project, team, &entered))
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    described.sort_by(|a, b| a.team.name.cmp(&b.team.name).then(a.id.cmp(&b.id)));
    Ok(described)
}
