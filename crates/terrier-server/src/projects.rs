use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::anyhow;
use axum::{
    Json, Router,
    extract::{
        Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    http::StatusCode,
    routing::get,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set, SqlErr, prelude::ChronoUtc,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use crate::{
    audit::{Audit, Scope},
    audited,
    auth::policy::{
        Auth, ProjectEditor, ProjectManager, ProjectReader, ProjectSubmitter, Submitter,
        TeamProjectReader, submitting_team,
    },
    entities::{
        application, event, hackathon, prize_track, project, project_track_sub,
        sea_orm_active_enums::ProjectStatus, team,
    },
    error::ApiError,
    state::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(
    read_project,
    list_team_projects,
    list_projects,
    create_project,
    update_project
))]
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

#[derive(Clone, Copy, PartialEq, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
enum SubmissionStatus {
    Draft,
    Submitted,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct NewProject {
    /// UUID of an active event in the hackathon where the project is presented
    #[schema(value_type = String)]
    event_id: Uuid,
    /// The team's answers to the submission form, as a JSON object
    data: serde_json::Value,
    /// UUIDs of the active prize tracks to enter, each at most once
    #[schema(value_type = Vec<String>)]
    track_ids: Vec<Uuid>,
    /// `submitted` submits the project now; omit it or send `draft` to save a draft
    status: Option<SubmissionStatus>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct ProjectChanges {
    /// The `version` of the project the client last read
    version: i32,
    /// Replacement answers to the submission form, as a JSON object
    data: Option<serde_json::Value>,
    /// Replacement prize track selection
    #[schema(value_type = Option<Vec<String>>)]
    track_ids: Option<Vec<Uuid>>,
    /// `submitted` submits a draft; a submitted project cannot return to `draft`
    status: Option<SubmissionStatus>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/hackathons/{slug}/projects",
            get(list_projects).post(create_project),
        )
        .route(
            "/hackathons/{slug}/projects/{project_id}",
            get(read_project).patch(update_project),
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

fn parse<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    input
        .map(|Json(input)| input)
        .map_err(|_| ApiError::BadRequest("Invalid JSON request.".into()))
}

/// Checks the fields that need no database access.
fn validate_input(
    data: Option<&serde_json::Value>,
    track_ids: Option<&[Uuid]>,
) -> Result<(), ApiError> {
    let mut errors = BTreeMap::new();
    if data.is_some_and(|data| !data.is_object()) {
        errors.insert(
            "data".into(),
            "Send the submission answers as an object.".into(),
        );
    }
    if let Some(track_ids) = track_ids
        && track_ids.iter().collect::<HashSet<_>>().len() != track_ids.len()
    {
        errors.insert(
            "track_ids".into(),
            "Choose each prize track only once.".into(),
        );
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ApiError::Validation(errors))
    }
}

/// Locks the caller's application and team in RFC 0008 order, then rechecks eligibility and the submission deadline against the locked state.
async fn lock_submission(db: &DatabaseTransaction, submitter: &Submitter) -> Result<(), ApiError> {
    let Submitter {
        hackathon,
        user_id,
        team_id,
    } = submitter;
    application::Entity::find()
        .filter(application::Column::UserId.eq(*user_id))
        .filter(application::Column::HackathonId.eq(hackathon.id))
        .lock_shared()
        .one(db)
        .await?;
    team::Entity::find_by_id(*team_id)
        .lock_exclusive()
        .one(db)
        .await?;
    if submitting_team(db, *user_id, hackathon.id).await? != *team_id {
        return Err(ApiError::Conflict(
            "Your team changed while saving. Reload the page and try again.".into(),
        ));
    }
    let deadline = hackathon::Entity::find_by_id(hackathon.id)
        .one(db)
        .await?
        .ok_or(ApiError::NotFound)?
        .submission_deadline;
    if ChronoUtc::now() >= deadline {
        return Err(ApiError::Conflict(
            "The submission deadline has passed.".into(),
        ));
    }
    Ok(())
}

/// Checks that the event and tracks are active and belong to the hackathon.
async fn validate_references(
    db: &DatabaseTransaction,
    hackathon_id: Uuid,
    event_id: Option<Uuid>,
    track_ids: Option<&[Uuid]>,
) -> Result<(), ApiError> {
    let mut errors = BTreeMap::new();
    if let Some(event_id) = event_id {
        let found = event::Entity::find_by_id(event_id)
            .filter(event::Column::HackathonId.eq(hackathon_id))
            .filter(event::Column::IsActive.eq(true))
            .one(db)
            .await?;
        if found.is_none() {
            errors.insert(
                "event_id".into(),
                "Choose an active event in this hackathon.".into(),
            );
        }
    }
    if let Some(track_ids) = track_ids {
        let found = prize_track::Entity::find()
            .filter(prize_track::Column::Id.is_in(track_ids.iter().copied()))
            .filter(prize_track::Column::HackathonId.eq(hackathon_id))
            .filter(prize_track::Column::IsActive.eq(true))
            .all(db)
            .await?;
        if found.len() != track_ids.len() {
            errors.insert(
                "track_ids".into(),
                "Choose only active prize tracks in this hackathon.".into(),
            );
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ApiError::Validation(errors))
    }
}

async fn reload(db: &DatabaseConnection, project_id: Uuid) -> Result<Project, ApiError> {
    let project = project::Entity::find_by_id(project_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::Internal(anyhow!("saved project {project_id} is missing")))?;
    describe(db, vec![project])
        .await?
        .pop()
        .ok_or_else(|| ApiError::Internal(anyhow!("saved project {project_id} has no response")))
}

#[utoipa::path(post, path = "/api/hackathons/{slug}/projects", tag = "projects",
    params(("slug" = String, Path, description = "Hackathon slug")),
    request_body = NewProject,
    responses((status = 201, body = Project),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 409, body = crate::error::ErrorResponse,
            description = "The team already has a project, the deadline has passed, or the team changed"),
        (status = 422, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn create_project(
    auth: Auth<ProjectSubmitter>,
    audit: Audit,
    State(state): State<AppState>,
    input: Result<Json<NewProject>, JsonRejection>,
) -> Result<(StatusCode, Json<Project>), ApiError> {
    let input = parse(input)?;
    validate_input(Some(&input.data), Some(&input.track_ids))?;
    let submitter = auth.data;
    let hackathon_id = submitter.hackathon.id;
    let project_id = audited!(audit, Scope::Hackathon(hackathon_id), |db| {
        lock_submission(db, &submitter).await?;
        validate_references(
            db,
            hackathon_id,
            Some(input.event_id),
            Some(&input.track_ids),
        )
        .await?;
        let submitted = input.status == Some(SubmissionStatus::Submitted);
        let project = project::ActiveModel {
            id: Set(Uuid::new_v4()),
            hackathon_id: Set(hackathon_id),
            event_id: Set(input.event_id),
            team_id: Set(submitter.team_id),
            data: Set(input.data),
            status: Set(if submitted {
                ProjectStatus::Submitted
            } else {
                ProjectStatus::Draft
            }),
            submitted_at: Set(submitted.then(|| ChronoUtc::now().fixed_offset())),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|error| match error.sql_err() {
            Some(SqlErr::UniqueConstraintViolation(_)) => {
                ApiError::Conflict("Your team already has a project.".into())
            }
            _ => error.into(),
        })?;
        for track_id in input.track_ids {
            project_track_sub::ActiveModel {
                project_id: Set(project.id),
                track_id: Set(track_id),
                hackathon_id: Set(hackathon_id),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        Ok(project.id)
    })
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(reload(&state.db, project_id).await?),
    ))
}

#[utoipa::path(patch, path = "/api/hackathons/{slug}/projects/{project_id}", tag = "projects",
    params(("slug" = String, Path, description = "Hackathon slug"),
        ("project_id" = String, Path, description = "Project UUID")),
    request_body = ProjectChanges,
    responses((status = 200, body = Project),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 409, body = crate::error::ErrorResponse,
            description = "`stale_revision` if the project changed since `version`; otherwise the deadline has passed, the project is already submitted, or the team changed"),
        (status = 422, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn update_project(
    auth: Auth<ProjectEditor>,
    audit: Audit,
    State(state): State<AppState>,
    input: Result<Json<ProjectChanges>, JsonRejection>,
) -> Result<Json<Project>, ApiError> {
    let input = parse(input)?;
    if input.data.is_none() && input.track_ids.is_none() && input.status.is_none() {
        return Err(ApiError::BadRequest(
            "Include data, track_ids, or status to change.".into(),
        ));
    }
    validate_input(input.data.as_ref(), input.track_ids.as_deref())?;
    let (submitter, project_id) = auth.data;
    let scope = Scope::Hackathon(submitter.hackathon.id);
    if input.track_ids.is_some() {
        audited!(
            audit,
            scope,
            reason = "Team changed its prize track selection",
            |db| { apply_changes(db, &submitter, project_id, input).await }
        )
        .await?;
    } else {
        audited!(audit, scope, |db| {
            apply_changes(db, &submitter, project_id, input).await
        })
        .await?;
    }
    Ok(Json(reload(&state.db, project_id).await?))
}

async fn apply_changes(
    db: &DatabaseTransaction,
    submitter: &Submitter,
    project_id: Uuid,
    input: ProjectChanges,
) -> Result<(), ApiError> {
    let hackathon_id = submitter.hackathon.id;
    lock_submission(db, submitter).await?;
    let project = project::Entity::find_by_id(project_id)
        .one(db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if project.version != input.version {
        return Err(ApiError::StaleRevision);
    }
    if input.status == Some(SubmissionStatus::Draft) && project.status == ProjectStatus::Submitted {
        return Err(ApiError::Conflict(
            "A submitted project cannot return to draft.".into(),
        ));
    }
    validate_references(db, hackathon_id, None, input.track_ids.as_deref()).await?;

    let submits =
        input.status == Some(SubmissionStatus::Submitted) && project.status == ProjectStatus::Draft;
    let changes_data = input
        .data
        .as_ref()
        .is_some_and(|data| *data != project.data);
    if submits || changes_data {
        let mut changes = project.into_active_model();
        if let Some(data) = input.data {
            changes.data = Set(data);
        }
        if submits {
            changes.status = Set(ProjectStatus::Submitted);
            changes.submitted_at = Set(Some(ChronoUtc::now().fixed_offset()));
        }
        changes.update(db).await?;
    }
    if let Some(track_ids) = input.track_ids {
        select_tracks(db, project_id, hackathon_id, &track_ids).await?;
    }
    Ok(())
}

/// Makes the project's active track entries match `track_ids`, keeping retained entries' identities across deactivation and re-entry.
async fn select_tracks(
    db: &DatabaseTransaction,
    project_id: Uuid,
    hackathon_id: Uuid,
    track_ids: &[Uuid],
) -> Result<(), ApiError> {
    let selected: HashSet<Uuid> = track_ids.iter().copied().collect();
    let entries = project_track_sub::Entity::find()
        .filter(project_track_sub::Column::ProjectId.eq(project_id))
        .all(db)
        .await?;
    for entry in &entries {
        let keep = selected.contains(&entry.track_id);
        if entry.is_active != keep {
            let mut changes = entry.clone().into_active_model();
            changes.is_active = Set(keep);
            changes.update(db).await?;
        }
    }
    for track_id in selected {
        if !entries.iter().any(|entry| entry.track_id == track_id) {
            project_track_sub::ActiveModel {
                project_id: Set(project_id),
                track_id: Set(track_id),
                hackathon_id: Set(hackathon_id),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}
