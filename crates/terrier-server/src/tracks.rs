use std::collections::BTreeMap;

use axum::{
    Json, Router,
    extract::rejection::JsonRejection,
    http::StatusCode,
    routing::{patch, post},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::{
    audit::{Audit, Scope},
    audited,
    auth::policy::{
        Auth, CreateTrack, EditTrack, TrackAction, check_track_action, check_track_creation_access,
    },
    entities::{prize_track, sea_orm_active_enums::JudgingMethod},
    error::ApiError,
    state::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(create_track, update_track))]
pub struct TracksApiDoc;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/hackathons/{slug}/tracks", post(create_track))
        .route("/hackathons/{slug}/tracks/{track_id}", patch(update_track))
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum TrackJudgingMethod {
    Scoring,
    Expo,
}

#[derive(serde::Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct CreateTrackRequest {
    name: String,
    description: String,
    judging_method: TrackJudgingMethod,
    sponsor_org_id: Option<String>,
}

#[derive(serde::Serialize, ToSchema)]
struct TrackResponse {
    id: String,
    name: String,
    description: String,
    judging_method: TrackJudgingMethod,
    sponsor_org_id: Option<String>,
    version: i32,
    is_active: bool,
}

#[derive(serde::Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct UpdateTrackRequest {
    version: i32,
    name: Option<String>,
    description: Option<String>,
    is_active: Option<bool>,
}

fn response(track: prize_track::Model) -> TrackResponse {
    TrackResponse {
        id: track.id.to_string(),
        name: track.name,
        description: track.description,
        judging_method: match track.judging_method {
            JudgingMethod::Scoring => TrackJudgingMethod::Scoring,
            JudgingMethod::Expo => TrackJudgingMethod::Expo,
        },
        sponsor_org_id: track.sponsor_org_id.map(|id| id.to_string()),
        version: track.version,
        is_active: track.is_active,
    }
}

#[utoipa::path(
    post,
    path = "/api/hackathons/{slug}/tracks",
    tag = "tracks",
    params(("slug" = String, Path, description = "Hackathon slug")),
    request_body = CreateTrackRequest,
    responses(
        (status = 201, body = TrackResponse),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 422, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)
    )
)]
async fn create_track(
    auth: Auth<CreateTrack>,
    audit: Audit,
    input: Result<Json<CreateTrackRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<TrackResponse>), ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest("Invalid JSON request.".into()))?;

    let name = input.name.trim().to_owned();
    if name.is_empty() {
        return Err(ApiError::Validation(BTreeMap::from([(
            "name".into(),
            "Enter a track name.".into(),
        )])));
    }

    let sponsor_org_id = input
        .sponsor_org_id
        .map(|id| {
            Uuid::parse_str(&id)
                .map_err(|_| ApiError::BadRequest("Invalid sponsor organization ID.".into()))
        })
        .transpose()?;

    let judging_method = match input.judging_method {
        TrackJudgingMethod::Scoring => JudgingMethod::Scoring,
        TrackJudgingMethod::Expo => JudgingMethod::Expo,
    };

    let hackathon_id = auth.data.hackathon.id;
    let caller_id = auth.data.user.user().id;

    let saved = audited!(audit, Scope::HackathonExclusive(hackathon_id), |db| {
        check_track_creation_access(db, caller_id, hackathon_id, sponsor_org_id).await?;

        let track = prize_track::ActiveModel {
            id: Set(Uuid::new_v4()),
            hackathon_id: Set(hackathon_id),
            sponsor_org_id: Set(sponsor_org_id),
            name: Set(name),
            description: Set(input.description),
            is_judging_started: Set(false),
            judging_method: Set(judging_method),
            ..Default::default()
        };

        Ok(track.insert(db).await?)
    })
    .await?;

    Ok((StatusCode::CREATED, Json(response(saved))))
}

#[utoipa::path(
    patch, path = "/api/hackathons/{slug}/tracks/{track_id}", tag = "tracks",
    params(("slug" = String, Path), ("track_id" = String, Path)),
    request_body = UpdateTrackRequest,
    responses((status = 200, body = TrackResponse),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 409, body = crate::error::ErrorResponse),
        (status = 422, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse))
)]
async fn update_track(
    auth: Auth<EditTrack>,
    audit: Audit,
    input: Result<Json<UpdateTrackRequest>, JsonRejection>,
) -> Result<Json<TrackResponse>, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest("Invalid JSON request.".into()))?;
    if input.is_active == Some(true)
        || (input.name.is_none() && input.description.is_none() && input.is_active.is_none())
    {
        return Err(ApiError::BadRequest(
            "Send an editable field or deactivate the track.".into(),
        ));
    }
    if input
        .name
        .as_ref()
        .is_some_and(|name| name.trim().is_empty())
    {
        return Err(ApiError::Validation(BTreeMap::from([(
            "name".into(),
            "Enter a track name.".into(),
        )])));
    }
    let hackathon_id = auth.data.hackathon.id;
    let user_id = auth.data.user.user().id;
    let track_id = auth.data.track.as_ref().expect("EditTrack has a track").id;
    let saved = audited!(audit, Scope::HackathonExclusive(hackathon_id), |db| {
        let hackathon = crate::entities::hackathon::Entity::find_by_id(hackathon_id)
            .filter(crate::entities::hackathon::Column::IsActive.eq(true))
            .one(db)
            .await?
            .ok_or(ApiError::NotFound)?;
        let action = if input.is_active == Some(false) {
            TrackAction::Deactivate
        } else {
            TrackAction::Edit
        };
        let current = check_track_action(db, user_id, &hackathon, Some(track_id), action)
            .await?
            .ok_or(ApiError::NotFound)?;
        if current.version != input.version {
            return Err(ApiError::Conflict(
                "The track changed. Reload before saving.".into(),
            ));
        }
        if current.editing_locked || current.is_judging_started {
            return Err(ApiError::Conflict(
                "Track details are locked after judging starts.".into(),
            ));
        }
        let mut model: prize_track::ActiveModel = current.into();
        if let Some(name) = input.name {
            model.name = Set(name.trim().to_owned());
        }
        if let Some(description) = input.description {
            model.description = Set(description);
        }
        if input.is_active == Some(false) {
            model.is_active = Set(false);
        }
        Ok(model.update(db).await?)
    })
    .await?;
    Ok(Json(response(saved)))
}
