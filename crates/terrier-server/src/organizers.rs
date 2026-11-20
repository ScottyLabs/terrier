use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::entities::sea_orm_active_enums::Capability;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::{
    audit::{Audit, Scope},
    audited,
    auth::{
        current_user::CurrentUser,
        policy::{Auth, ManageOrganizers},
    },
    entities::{
        hackathon, organizer,
        sea_orm_active_enums::{HackathonRole, JudgeAvailability},
        user, user_hackathon_role,
    },
    error::ApiError,
};
use axum::{
    Json, Router,
    extract::{Path, rejection::JsonRejection},
    http::StatusCode,
    routing::{patch, post},
};
use utoipa::OpenApi;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum OrganizerCapability {
    ApplicationManagement,
    JudgingManagement,
    SponsorshipManagement,
    HackathonAdmin,
}

impl From<OrganizerCapability> for Capability {
    fn from(value: OrganizerCapability) -> Self {
        match value {
            OrganizerCapability::ApplicationManagement => Self::ApplicationManagement,
            OrganizerCapability::JudgingManagement => Self::JudgingManagement,
            OrganizerCapability::SponsorshipManagement => Self::SponsorshipManagement,
            OrganizerCapability::HackathonAdmin => Self::HackathonAdmin,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct CreateOrganizer {
    user_id: String,
    capabilities: Vec<OrganizerCapability>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct UpdateOrganizer {
    is_active: bool,
}

#[derive(OpenApi)]
#[openapi(paths(create_organizer, update_organizer))]
pub struct OrganizerApiDoc;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/hackathons/{slug}/organizers", post(create_organizer))
        .route(
            "/hackathons/{slug}/organizers/{user_id}",
            patch(update_organizer),
        )
}

#[utoipa::path(
    post,
    path = "/api/hackathons/{slug}/organizers",
    tag = "organizers",
    params(("slug" = String, Path, description = "Hackathon slug")),
    request_body = CreateOrganizer,
    responses(
        (status = 201, description = "Organizer created"),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)
    )
)]
async fn create_organizer(
    auth: Auth<ManageOrganizers>,
    current_user: CurrentUser,
    audit: Audit,
    input: Result<Json<CreateOrganizer>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest("Invalid JSON request.".into()))?;

    let user_id = Uuid::parse_str(&input.user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID.".into()))?;

    let capabilities: Vec<Capability> = input.capabilities.into_iter().map(Into::into).collect();

    let hackathon_id = auth.data.id;
    let actor_id = current_user.user().id;

    audited!(audit, Scope::HackathonExclusive(hackathon_id), |db| {
        hackathon::Entity::find_by_id(hackathon_id)
            .filter(hackathon::Column::IsActive.eq(true))
            .one(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::NotFound)?;

        let role = user_hackathon_role::Entity::find_by_id((actor_id, hackathon_id))
            .filter(user_hackathon_role::Column::IsActive.eq(true))
            .one(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?;

        if role.is_none() {
            return Err(ApiError::Forbidden);
        }

        let actor = organizer::Entity::find_by_id((actor_id, hackathon_id))
            .filter(organizer::Column::IsActive.eq(true))
            .one(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::Forbidden)?;

        if !actor.capabilities.contains(&Capability::HackathonAdmin) {
            return Err(ApiError::Forbidden);
        }

        user::Entity::find_by_id(user_id)
            .one(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::NotFound)?;

        let existing_role = user_hackathon_role::Entity::find_by_id((user_id, hackathon_id))
            .one(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?;

        if existing_role.as_ref().is_some_and(|role| !role.is_active) {
            return Err(ApiError::Forbidden);
        }
        if organizer::Entity::find_by_id((user_id, hackathon_id))
            .one(db)
            .await?
            .is_some()
        {
            return Err(ApiError::BadRequest(
                "User already has an organizer record.".into(),
            ));
        }
        if existing_role.is_none() {
            user_hackathon_role::ActiveModel {
                user_id: Set(user_id),
                hackathon_id: Set(hackathon_id),
                role: Set(HackathonRole::Organizer),
                profile_data: Set(serde_json::json!({})),
                directory_opt_in: Set(false),
                is_active: Set(true),
                judge_availability: Set(JudgeAvailability::Available),
                version: Set(1),
            }
            .insert(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?;
        }

        organizer::ActiveModel {
            user_id: Set(user_id),
            hackathon_id: Set(hackathon_id),
            capabilities: Set(capabilities),
            is_active: Set(true),
            version: Set(1),
        }
        .insert(db)
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;

        Ok(())
    })
    .await?;

    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    patch,
    path = "/api/hackathons/{slug}/organizers/{user_id}",
    tag = "organizers",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("user_id" = String, Path, description = "Organizer user UUID")
    ),
    request_body = UpdateOrganizer,
    responses(
        (status = 204, description = "Organizer deactivated"),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)
    )
)]
async fn update_organizer(
    auth: Auth<ManageOrganizers>,
    current_user: CurrentUser,
    audit: Audit,
    Path((_slug, user_id)): Path<(String, String)>,
    input: Result<Json<UpdateOrganizer>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest("Invalid JSON request.".into()))?;

    if input.is_active {
        return Err(ApiError::BadRequest(
            "Organizer reactivation is not supported.".into(),
        ));
    }

    let user_id =
        Uuid::parse_str(&user_id).map_err(|_| ApiError::BadRequest("Invalid user ID.".into()))?;

    let hackathon_id = auth.data.id;
    let actor_id = current_user.user().id;

    if actor_id == user_id {
        return Err(ApiError::Forbidden);
    }

    audited!(audit, Scope::HackathonExclusive(hackathon_id), |db| {
        hackathon::Entity::find_by_id(hackathon_id)
            .filter(hackathon::Column::IsActive.eq(true))
            .one(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::NotFound)?;

        let actor_role = user_hackathon_role::Entity::find_by_id((actor_id, hackathon_id))
            .filter(user_hackathon_role::Column::IsActive.eq(true))
            .one(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?;

        if actor_role.is_none() {
            return Err(ApiError::Forbidden);
        }

        let actor = organizer::Entity::find_by_id((actor_id, hackathon_id))
            .filter(organizer::Column::IsActive.eq(true))
            .one(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::Forbidden)?;

        if !actor.capabilities.contains(&Capability::HackathonAdmin) {
            return Err(ApiError::Forbidden);
        }

        let target = organizer::Entity::find_by_id((user_id, hackathon_id))
            .filter(organizer::Column::IsActive.eq(true))
            .one(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::NotFound)?;

        let target_role = user_hackathon_role::Entity::find_by_id((user_id, hackathon_id))
            .filter(user_hackathon_role::Column::IsActive.eq(true))
            .one(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::NotFound)?;

        organizer::ActiveModel {
            user_id: Set(user_id),
            hackathon_id: Set(hackathon_id),
            is_active: Set(false),
            version: Set(target.version + 1),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;

        if target_role.role == HackathonRole::Organizer {
            user_hackathon_role::ActiveModel {
                user_id: Set(user_id),
                hackathon_id: Set(hackathon_id),
                is_active: Set(false),
                version: Set(target_role.version + 1),
                ..Default::default()
            }
            .update(db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?;
        }

        Ok(())
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
