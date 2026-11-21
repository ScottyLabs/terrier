use std::sync::Arc;

use axum::{
    Extension,
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use leash::Policy;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use uuid::Uuid;

use super::{config::AuthConfig, current_user::CurrentUser};
use crate::{
    entities::{
        application, evaluation, hackathon, judge_track_assign, organizer, prize_track, project,
        project_track_sub,
        sea_orm_active_enums::{
            ApplicationStatus, AttendanceStatus, Capability, EvaluationStatus, HackathonRole,
            ProjectStatus, TeamMemberStatus,
        },
        sponsor, team, team_member, user_hackathon_role,
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

/// A project the caller may read, with the active tracks it is entered in.
pub struct ReadableProject {
    pub project: project::Model,
    pub tracks: Vec<prize_track::Model>,
}

/// Team members and organizers read drafts. Sponsors and judges read submitted projects only:
/// a sponsor through a track their organization owns, a judge through an assignment on a track they hold a grant for.
pub struct ProjectReader;

impl Policy<AppState> for ProjectReader {
    type Output = ReadableProject;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path((slug, project_id)) = Path::<(String, Uuid)>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid project ID.".into()))?;
        let db = &state.db;
        let hackathon = active_hackathon(db, &slug).await?;
        let project = project::Entity::find_by_id(project_id)
            .filter(project::Column::HackathonId.eq(hackathon.id))
            .one(db)
            .await?
            .ok_or(ApiError::NotFound)?;
        readable(db, user.user().id, project).await
    }
}

/// An active team and its project, which the caller may read when present.
pub struct TeamProject {
    pub team: team::Model,
    pub project: Option<ReadableProject>,
}

/// Applies [`ProjectReader`] to the team's project. A team without a project is visible only to its members and organizers.
pub struct TeamProjectReader;

impl Policy<AppState> for TeamProjectReader {
    type Output = TeamProject;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path((slug, team_id)) = Path::<(String, Uuid)>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid team ID.".into()))?;
        let db = &state.db;
        let hackathon = active_hackathon(db, &slug).await?;
        let team = team::Entity::find_by_id(team_id)
            .filter(team::Column::HackathonId.eq(hackathon.id))
            .filter(team::Column::IsActive.eq(true))
            .one(db)
            .await?
            .ok_or(ApiError::NotFound)?;
        let user_id = user.user().id;
        let project = match project::Entity::find()
            .filter(project::Column::TeamId.eq(team.id))
            .one(db)
            .await?
        {
            Some(project) => Some(readable(db, user_id, project).await?),
            None if is_member(db, team.id, user_id).await?
                || active_role(db, user_id, hackathon.id).await?
                    == Some(HackathonRole::Organizer) =>
            {
                None
            }
            None => return Err(ApiError::Forbidden),
        };
        Ok(TeamProject { team, project })
    }
}

/// Organizers with judging-management or hackathon-admin capability in an active hackathon.
pub struct ProjectManager;

impl Policy<AppState> for ProjectManager {
    type Output = hackathon::Model;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path(slug) = Path::<String>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid hackathon.".into()))?;
        let db = &state.db;
        let hackathon = active_hackathon(db, &slug).await?;
        let user_id = user.user().id;
        if active_role(db, user_id, hackathon.id).await? != Some(HackathonRole::Organizer) {
            return Err(ApiError::Forbidden);
        }
        let manages = organizer::Entity::find_by_id((user_id, hackathon.id))
            .filter(organizer::Column::IsActive.eq(true))
            .one(db)
            .await?
            .is_some_and(|organizer| {
                organizer.capabilities.iter().any(|capability| {
                    matches!(
                        capability,
                        Capability::JudgingManagement | Capability::HackathonAdmin
                    )
                })
            });
        if !manages {
            return Err(ApiError::Forbidden);
        }
        Ok(hackathon)
    }
}

async fn readable(
    db: &DatabaseConnection,
    user_id: Uuid,
    project: project::Model,
) -> Result<ReadableProject, ApiError> {
    let entered = project_track_sub::Entity::find()
        .filter(project_track_sub::Column::ProjectId.eq(project.id))
        .filter(project_track_sub::Column::IsActive.eq(true))
        .all(db)
        .await?
        .into_iter()
        .map(|entry| entry.track_id);
    let tracks = prize_track::Entity::find()
        .filter(prize_track::Column::Id.is_in(entered))
        .filter(prize_track::Column::IsActive.eq(true))
        .order_by_asc(prize_track::Column::Name)
        .all(db)
        .await?;

    let allowed = is_member(db, project.team_id, user_id).await?
        || match active_role(db, user_id, project.hackathon_id).await? {
            None => false,
            Some(HackathonRole::Organizer) => true,
            Some(_) => {
                project.status == ProjectStatus::Submitted
                    && (sponsors(db, &project, &tracks, user_id).await?
                        || judges(db, &project, &tracks, user_id).await?)
            }
        };
    if !allowed {
        return Err(ApiError::Forbidden);
    }
    Ok(ReadableProject { project, tracks })
}

async fn is_member(
    db: &DatabaseConnection,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<bool, ApiError> {
    Ok(team_member::Entity::find_by_id((team_id, user_id))
        .one(db)
        .await?
        .is_some_and(|member| member.status == TeamMemberStatus::Active))
}

async fn active_role(
    db: &DatabaseConnection,
    user_id: Uuid,
    hackathon_id: Uuid,
) -> Result<Option<HackathonRole>, ApiError> {
    Ok(
        user_hackathon_role::Entity::find_by_id((user_id, hackathon_id))
            .one(db)
            .await?
            .filter(|role| role.is_active)
            .map(|role| role.role),
    )
}

async fn sponsors(
    db: &DatabaseConnection,
    project: &project::Model,
    tracks: &[prize_track::Model],
    user_id: Uuid,
) -> Result<bool, ApiError> {
    let organizations = tracks.iter().filter_map(|track| track.sponsor_org_id);
    Ok(sponsor::Entity::find()
        .filter(sponsor::Column::UserId.eq(user_id))
        .filter(sponsor::Column::HackathonId.eq(project.hackathon_id))
        .filter(sponsor::Column::IsActive.eq(true))
        .filter(sponsor::Column::OrgId.is_in(organizations))
        .one(db)
        .await?
        .is_some())
}

async fn judges(
    db: &DatabaseConnection,
    project: &project::Model,
    tracks: &[prize_track::Model],
    user_id: Uuid,
) -> Result<bool, ApiError> {
    let granted = judge_track_assign::Entity::find()
        .filter(judge_track_assign::Column::UserId.eq(user_id))
        .filter(judge_track_assign::Column::TrackId.is_in(tracks.iter().map(|track| track.id)))
        .filter(
            Condition::any()
                .add(judge_track_assign::Column::ManuallyGranted.eq(true))
                .add(judge_track_assign::Column::SponsorGranted.eq(true)),
        )
        .all(db)
        .await?
        .into_iter()
        .map(|grant| grant.track_id);
    Ok(evaluation::Entity::find()
        .filter(evaluation::Column::ProjectId.eq(project.id))
        .filter(evaluation::Column::JudgeId.eq(user_id))
        .filter(evaluation::Column::TrackId.is_in(granted))
        .filter(evaluation::Column::Status.ne(EvaluationStatus::Skipped))
        .one(db)
        .await?
        .is_some())
}
