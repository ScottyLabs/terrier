use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use leash::Policy;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::current_user::CurrentUser;
use crate::{
    entities::{
        application, hackathon, organizer,
        sea_orm_active_enums::{ApplicationStatus, AttendanceStatus, Capability, HackathonRole},
        user_hackathon_role,
    },
    error::ApiError,
    state::AppState,
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

/// How the caller may see an application.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ApplicationAccess {
    Owner,
    Manager,
    Organizer,
}

async fn organizer_access(
    db: &DatabaseConnection,
    user_id: Uuid,
    hackathon_id: Uuid,
) -> Result<Option<ApplicationAccess>, ApiError> {
    if let Some(row) = organizer::Entity::find_by_id((user_id, hackathon_id))
        .filter(organizer::Column::IsActive.eq(true))
        .one(db)
        .await?
    {
        let manages = row.capabilities.iter().any(|capability| {
            matches!(
                capability,
                Capability::ApplicationManagement | Capability::HackathonAdmin
            )
        });
        return Ok(Some(if manages {
            ApplicationAccess::Manager
        } else {
            ApplicationAccess::Organizer
        }));
    }
    let role = user_hackathon_role::Entity::find_by_id((user_id, hackathon_id))
        .filter(user_hackathon_role::Column::IsActive.eq(true))
        .filter(user_hackathon_role::Column::Role.eq(HackathonRole::Organizer))
        .one(db)
        .await?;
    Ok(role.map(|_| ApplicationAccess::Organizer))
}

async fn active_hackathon(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<hackathon::Model, ApiError> {
    hackathon::Entity::find()
        .filter(hackathon::Column::Slug.eq(slug))
        .filter(hackathon::Column::IsActive.eq(true))
        .one(db)
        .await?
        .ok_or(ApiError::NotFound)
}

pub struct Applicant {
    pub user_id: Uuid,
    pub hackathon_id: Uuid,
    pub has_role: bool,
}

pub struct OwnApplication;

impl Policy<AppState> for OwnApplication {
    type Output = Applicant;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path((slug, user_id)) = Path::<(String, Uuid)>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid user ID.".into()))?;
        if user.user().id != user_id {
            return Err(ApiError::Forbidden);
        }
        let hackathon = active_hackathon(&state.db, &slug).await?;
        let role = user_hackathon_role::Entity::find_by_id((user_id, hackathon.id))
            .one(&state.db)
            .await?;
        Ok(Applicant {
            user_id,
            hackathon_id: hackathon.id,
            has_role: role.is_some(),
        })
    }
}

pub struct ApplicationTarget {
    pub hackathon_id: Uuid,
    pub applicant_id: Uuid,
    pub access: ApplicationAccess,
}

pub struct ReadApplication;

impl Policy<AppState> for ReadApplication {
    type Output = ApplicationTarget;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path((slug, applicant_id)) = Path::<(String, Uuid)>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid user ID.".into()))?;
        let hackathon = active_hackathon(&state.db, &slug).await?;
        let access = if user.user().id == applicant_id {
            ApplicationAccess::Owner
        } else {
            organizer_access(&state.db, user.user().id, hackathon.id)
                .await?
                .ok_or(ApiError::Forbidden)?
        };
        Ok(ApplicationTarget {
            hackathon_id: hackathon.id,
            applicant_id,
            access,
        })
    }
}

pub struct ListApplications;

impl Policy<AppState> for ListApplications {
    type Output = (Uuid, ApplicationAccess);
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path(slug) = Path::<String>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid hackathon.".into()))?;
        let hackathon = active_hackathon(&state.db, &slug).await?;
        let access = organizer_access(&state.db, user.user().id, hackathon.id)
            .await?
            .ok_or(ApiError::Forbidden)?;
        Ok((hackathon.id, access))
    }
}

/// Organizers and confirmed attendees of the hackathon may browse its participant directory.
pub struct BrowseDirectory;

impl Policy<AppState> for BrowseDirectory {
    type Output = hackathon::Model;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path(slug) = Path::<String>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid hackathon.".into()))?;
        let hackathon = active_hackathon(&state.db, &slug).await?;
        if organizer_access(&state.db, user.user().id, hackathon.id)
            .await?
            .is_some()
        {
            return Ok(hackathon);
        }
        application::Entity::find()
            .filter(application::Column::UserId.eq(user.user().id))
            .filter(application::Column::HackathonId.eq(hackathon.id))
            .filter(application::Column::IsActive.eq(true))
            .filter(application::Column::Status.eq(ApplicationStatus::Accepted))
            .filter(application::Column::AttendanceStatus.eq(AttendanceStatus::Confirmed))
            .one(&state.db)
            .await?
            .ok_or(ApiError::Forbidden)?;
        Ok(hackathon)
    }
}
