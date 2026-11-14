use std::sync::Arc;

use axum::{
    Extension,
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use leash::Policy;
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::{config::AuthConfig, current_user::CurrentUser};
use crate::{
    entities::{
        application, hackathon, organizer, project,
        sea_orm_active_enums::{
            ApplicationStatus, AttendanceStatus, Capability, HackathonRole, TeamMemberStatus,
        },
        team_member, user_hackathon_role,
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
    if user_hackathon_role::Entity::find_by_id((user_id, hackathon_id))
        .filter(user_hackathon_role::Column::IsActive.eq(true))
        .one(db)
        .await?
        .is_none()
    {
        return Ok(None);
    }
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

pub(crate) async fn active_hackathon(
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

        let hackathon = active_hackathon(&state.db, &slug).await?;
        ensure_manager(&state.db, user.user().id, hackathon.id).await?;
        Ok(hackathon)
    }
}

pub struct InstanceAdmin;

impl Policy<AppState> for InstanceAdmin {
    type Output = Uuid;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Uuid, ApiError> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Extension(config) = Extension::<Arc<AuthConfig>>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::Internal(anyhow::anyhow!("Missing auth configuration")))?;
        if !config
            .instance_admin_subjects
            .contains(&user.user().oidc_subject)
        {
            return Err(ApiError::Forbidden);
        }
        Ok(user.user().id)
    }
}

pub(crate) async fn ensure_manager(
    db: &impl ConnectionTrait,
    user: Uuid,
    event: Uuid,
) -> Result<(), ApiError> {
    let membership = user_hackathon_role::Entity::find_by_id((user, event))
        .filter(user_hackathon_role::Column::IsActive.eq(true))
        .one(db)
        .await?;
    let grant = organizer::Entity::find_by_id((user, event))
        .filter(organizer::Column::IsActive.eq(true))
        .one(db)
        .await?;
    if membership.is_none()
        || !grant.is_some_and(|grant| grant.capabilities.contains(&Capability::HackathonAdmin))
    {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

/// Judge and sponsor access to submitted projects is not granted here.
pub struct ProjectReader;

impl Policy<AppState> for ProjectReader {
    type Output = project::Model;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path((slug, project_id)) = Path::<(String, Uuid)>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid project ID.".into()))?;
        let hackathon = active_hackathon(&state.db, &slug).await?;
        let project = project::Entity::find_by_id(project_id)
            .filter(project::Column::HackathonId.eq(hackathon.id))
            .one(&state.db)
            .await?
            .ok_or(ApiError::NotFound)?;

        let user_id = user.user().id;
        let is_member = team_member::Entity::find_by_id((project.team_id, user_id))
            .one(&state.db)
            .await?
            .is_some_and(|member| member.status == TeamMemberStatus::Active);
        if is_member {
            return Ok(project);
        }
        let is_organizer = user_hackathon_role::Entity::find_by_id((user_id, hackathon.id))
            .one(&state.db)
            .await?
            .is_some_and(|role| role.is_active && role.role == HackathonRole::Organizer);
        if is_organizer {
            return Ok(project);
        }
        Err(ApiError::Forbidden)
    }
}
