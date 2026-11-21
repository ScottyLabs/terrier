use std::{collections::HashSet, time::SystemTime};

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    routing::{get, post},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set,
    entity::prelude::{DateTimeUtc, DateTimeWithTimeZone},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::{
    audit::{Audit, Scope},
    audited,
    auth::{
        current_user::CurrentUser,
        policy::{Auth, PublishTrackResults, TrackAction, active_hackathon, check_track_action},
    },
    entities::{
        evaluation, hackathon, prize_track, project, project_track_sub,
        sea_orm_active_enums::{EvaluationStatus, ProjectStatus},
    },
    error::ApiError,
    state::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(read_results, publish_results, read_project_result))]
pub struct TrackResultsApiDoc;

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/hackathons/{slug}/tracks/{track_id}/results",
        post(publish_results),
    )
}

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/hackathons/{slug}/tracks/{track_id}/results",
            get(read_results),
        )
        .route(
            "/hackathons/{slug}/tracks/{track_id}/results/{project_id}",
            get(read_project_result),
        )
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct Winner {
    project_id: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct PublishRequest {
    version: i32,
    winners: Vec<Winner>,
    #[schema(value_type = Option<String>, format = DateTime)]
    release_at: Option<DateTimeWithTimeZone>,
}

#[derive(Serialize, ToSchema)]
struct PublishedResults {
    track_id: String,
    winners: Vec<Winner>,
    #[schema(value_type = String, format = DateTime)]
    release_at: DateTimeWithTimeZone,
}

#[derive(Serialize, ToSchema)]
struct InternalEvaluation {
    id: String,
    judge_id: String,
    status: String,
    result: Option<Value>,
    rubric: Value,
}

#[derive(Serialize, ToSchema)]
struct ProjectResult {
    project_id: String,
    evaluations: Vec<InternalEvaluation>,
}

#[derive(Serialize, ToSchema)]
struct InternalResults {
    track_id: String,
    version: i32,
    projects: Vec<ProjectResult>,
}

#[derive(Serialize, ToSchema)]
#[serde(untagged)]
enum ResultsResponse {
    Internal(InternalResults),
    Published(PublishedResults),
}

async fn active_track(
    state: &AppState,
    slug: &str,
    track_id: Uuid,
) -> Result<(hackathon::Model, prize_track::Model), ApiError> {
    let hackathon = active_hackathon(&state.db, slug).await?;
    let track = prize_track::Entity::find_by_id(track_id)
        .filter(prize_track::Column::HackathonId.eq(hackathon.id))
        .filter(prize_track::Column::IsActive.eq(true))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok((hackathon, track))
}

fn public_snapshot(track: &prize_track::Model) -> Result<Option<PublishedResults>, ApiError> {
    let Some(release_at) = track.release_at else {
        return Ok(None);
    };
    let now = DateTimeWithTimeZone::from(DateTimeUtc::from(SystemTime::now()));
    if track.publication_revoked_at.is_some() || now < release_at {
        return Ok(None);
    }
    let Some(snapshot) = &track.published_winners else {
        return Ok(None);
    };
    let winners = serde_json::from_value(snapshot.clone())
        .map_err(|error| ApiError::Internal(error.into()))?;
    Ok(Some(PublishedResults {
        track_id: track.id.to_string(),
        winners,
        release_at,
    }))
}

async fn project_results(
    state: &AppState,
    hackathon_id: Uuid,
    track_id: Uuid,
) -> Result<Vec<ProjectResult>, ApiError> {
    let links = project_track_sub::Entity::find()
        .filter(project_track_sub::Column::HackathonId.eq(hackathon_id))
        .filter(project_track_sub::Column::TrackId.eq(track_id))
        .filter(project_track_sub::Column::IsActive.eq(true))
        .all(&state.db)
        .await?;
    let mut output = Vec::new();
    for link in links {
        let Some(project) = project::Entity::find_by_id(link.project_id)
            .filter(project::Column::HackathonId.eq(hackathon_id))
            .filter(project::Column::Status.eq(ProjectStatus::Submitted))
            .one(&state.db)
            .await?
        else {
            continue;
        };
        let evaluations = evaluation::Entity::find()
            .filter(evaluation::Column::HackathonId.eq(hackathon_id))
            .filter(evaluation::Column::TrackId.eq(track_id))
            .filter(evaluation::Column::ProjectId.eq(project.id))
            .all(&state.db)
            .await?
            .into_iter()
            .map(|item| InternalEvaluation {
                id: item.id.to_string(),
                judge_id: item.judge_id.to_string(),
                status: match item.status {
                    EvaluationStatus::Pending => "pending",
                    EvaluationStatus::Complete => "complete",
                    EvaluationStatus::Skipped => "skipped",
                }
                .to_owned(),
                result: item.result,
                rubric: item.rubric,
            })
            .collect();
        output.push(ProjectResult {
            project_id: project.id.to_string(),
            evaluations,
        });
    }
    output.sort_by(|a, b| a.project_id.cmp(&b.project_id));
    Ok(output)
}

#[utoipa::path(get, path = "/api/hackathons/{slug}/tracks/{track_id}/results", tag = "track results",
    params(("slug" = String, Path), ("track_id" = String, Path)),
    responses((status = 200, body = ResultsResponse, description = "Internal evaluations for authorized callers; released winners for public callers"),
        (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse), (status = 500, body = crate::error::ErrorResponse)))]
async fn read_results(
    State(state): State<AppState>,
    user: Option<CurrentUser>,
    Path((slug, track_id)): Path<(String, Uuid)>,
) -> Result<Json<ResultsResponse>, ApiError> {
    let (hackathon, track) = active_track(&state, &slug, track_id).await?;
    let signed_in = user.is_some();
    if let Some(user) = user {
        match check_track_action(
            &state.db,
            user.user().id,
            &hackathon,
            Some(track_id),
            TrackAction::ReadResults,
        )
        .await
        {
            Ok(_) => {
                let result = InternalResults {
                    track_id: track_id.to_string(),
                    version: track.version,
                    projects: project_results(&state, hackathon.id, track_id).await?,
                };
                return Ok(Json(ResultsResponse::Internal(result)));
            }
            Err(ApiError::Forbidden) => {}
            Err(error) => return Err(error),
        }
    }
    if let Some(snapshot) = public_snapshot(&track)? {
        return Ok(Json(ResultsResponse::Published(snapshot)));
    }
    Err(if signed_in {
        ApiError::Forbidden
    } else {
        ApiError::Unauthenticated
    })
}

#[utoipa::path(get, path = "/api/hackathons/{slug}/tracks/{track_id}/results/{project_id}", tag = "track results",
    params(("slug" = String, Path), ("track_id" = String, Path), ("project_id" = String, Path)),
    responses((status = 200, body = ProjectResult), (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn read_project_result(
    State(state): State<AppState>,
    user: Option<CurrentUser>,
    Path((slug, track_id, project_id)): Path<(String, Uuid, Uuid)>,
) -> Result<Json<ProjectResult>, ApiError> {
    let user = user.ok_or(ApiError::Unauthenticated)?;
    let (hackathon, _) = active_track(&state, &slug, track_id).await?;
    check_track_action(
        &state.db,
        user.user().id,
        &hackathon,
        Some(track_id),
        TrackAction::ReadResults,
    )
    .await?;
    let results = project_results(&state, hackathon.id, track_id).await?;
    Ok(Json(
        results
            .into_iter()
            .find(|item| item.project_id == project_id.to_string())
            .ok_or(ApiError::NotFound)?,
    ))
}

#[utoipa::path(post, path = "/api/hackathons/{slug}/tracks/{track_id}/results", tag = "track results",
    params(("slug" = String, Path), ("track_id" = String, Path)), request_body = PublishRequest,
    responses((status = 200, body = PublishedResults), (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse),
        (status = 422, body = crate::error::ErrorResponse), (status = 500, body = crate::error::ErrorResponse)))]
async fn publish_results(
    auth: Auth<PublishTrackResults>,
    audit: Audit,
    input: Result<Json<PublishRequest>, JsonRejection>,
) -> Result<Json<PublishedResults>, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest("Invalid JSON request.".into()))?;
    let mut ids = HashSet::new();
    for winner in &input.winners {
        let id = Uuid::parse_str(&winner.project_id)
            .map_err(|_| ApiError::BadRequest("Invalid winner project ID.".into()))?;
        if !ids.insert(id) {
            return Err(ApiError::BadRequest("Duplicate winner project ID.".into()));
        }
    }
    let hackathon_id = auth.data.hackathon.id;
    let user_id = auth.data.user.user().id;
    let track_id = auth
        .data
        .track
        .as_ref()
        .expect("PublishTrackResults has a track")
        .id;
    let release_at = input
        .release_at
        .unwrap_or_else(|| DateTimeWithTimeZone::from(DateTimeUtc::from(SystemTime::now())));
    let winners = input.winners.clone();
    let saved = audited!(audit, Scope::HackathonExclusive(hackathon_id), |db| {
        let hackathon = hackathon::Entity::find_by_id(hackathon_id)
            .filter(hackathon::Column::IsActive.eq(true))
            .one(db)
            .await?
            .ok_or(ApiError::NotFound)?;
        let track = check_track_action(
            db,
            user_id,
            &hackathon,
            Some(track_id),
            TrackAction::Publish,
        )
        .await?
        .ok_or(ApiError::NotFound)?;
        if track.version != input.version {
            return Err(ApiError::Conflict(
                "The track changed. Reload before publishing.".into(),
            ));
        }
        if track.published_winners.is_some() {
            return Err(ApiError::Conflict(
                "Results are already scheduled or published.".into(),
            ));
        }
        for project_id in &ids {
            let link = project_track_sub::Entity::find_by_id((*project_id, track_id))
                .filter(project_track_sub::Column::HackathonId.eq(hackathon_id))
                .filter(project_track_sub::Column::IsActive.eq(true))
                .one(db)
                .await?;
            let project = project::Entity::find_by_id(*project_id)
                .filter(project::Column::HackathonId.eq(hackathon_id))
                .filter(project::Column::Status.eq(ProjectStatus::Submitted))
                .one(db)
                .await?;
            if link.is_none() || project.is_none() {
                return Err(ApiError::BadRequest(
                    "Winner is not an active submitted project in this track.".into(),
                ));
            }
        }
        let snapshot = serde_json::to_value(&input.winners)
            .map_err(|error| ApiError::Internal(error.into()))?;
        let mut model: prize_track::ActiveModel = track.into();
        model.published_winners = Set(Some(snapshot));
        model.release_at = Set(Some(release_at));
        model.published_by = Set(Some(user_id));
        Ok(model.update(db).await?)
    })
    .await?;
    Ok(Json(PublishedResults {
        track_id: saved.id.to_string(),
        winners,
        release_at,
    }))
}
