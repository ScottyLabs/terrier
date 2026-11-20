use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use leash::Policy;
use uuid::Uuid;

use super::current_user::CurrentUser;
use crate::{error::ApiError, state::AppState};

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::entities::{
    hackathon, organizer,
    sea_orm_active_enums::{Capability, HackathonRole},
    user_hackathon_role,
};

pub type Auth<P> = leash::Authorized<P, AppState>;

pub struct OwnProfile;

impl Policy<AppState> for OwnProfile {
    type Output = CurrentUser;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path(id) = Path::<Uuid>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid user ID.".into()))?;
        if user.user().id != id {
            return Err(ApiError::Forbidden);
        }
        Ok(user)
    }
}

#[derive(serde::Deserialize)]
struct HackathonPath {
    slug: String,
}

pub struct ManageOrganizers;

impl Policy<AppState> for ManageOrganizers {
    type Output = hackathon::Model;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;

        let Path(HackathonPath { slug }) = Path::<HackathonPath>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid hackathon slug.".into()))?;

        let hackathon = hackathon::Entity::find()
            .filter(hackathon::Column::Slug.eq(slug))
            .filter(hackathon::Column::IsActive.eq(true))
            .one(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::NotFound)?;

        let role = user_hackathon_role::Entity::find_by_id((user.user().id, hackathon.id))
            .filter(user_hackathon_role::Column::IsActive.eq(true))
            .filter(user_hackathon_role::Column::Role.eq(HackathonRole::Organizer))
            .one(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?;

        if role.is_none() {
            return Err(ApiError::Forbidden);
        }

        let organizer = organizer::Entity::find_by_id((user.user().id, hackathon.id))
            .filter(organizer::Column::IsActive.eq(true))
            .one(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::Forbidden)?;

        if !organizer.capabilities.contains(&Capability::HackathonAdmin) {
            return Err(ApiError::Forbidden);
        }

        Ok(hackathon)
    }
}
