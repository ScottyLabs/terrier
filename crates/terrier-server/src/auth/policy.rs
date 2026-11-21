use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use leash::Policy;
use uuid::Uuid;

use super::current_user::CurrentUser;
use crate::{
    entities::{
        application, hackathon, judge_track_assign, prize_track,
        sea_orm_active_enums::{AttendanceStatus, HackathonRole, TeamMemberStatus},
        team, team_member, user_hackathon_role,
    },
    error::ApiError,
    state::AppState,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

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

pub struct ActiveHackathon;

#[derive(serde::Deserialize)]
struct HackathonPath {
    slug: String,
}

// Checks whether the requested hackathon is active.
impl Policy<AppState> for ActiveHackathon {
    type Output = hackathon::Model;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
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

        Ok(hackathon)
    }
}

pub struct EligibleHacker;

//Checks if user has a hacker role and with their application status set to confirmed
impl Policy<AppState> for EligibleHacker {
    type Output = HackathonMembership;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let membership = HackathonMember::check(parts, state).await?;

        if membership.role.role != HackathonRole::Hacker {
            return Err(ApiError::Forbidden);
        }

        Ok(membership)
    }
}

// Checked information available to the handler after authorization.
pub struct HackathonMembership {
    pub user: CurrentUser,
    pub hackathon: hackathon::Model,
    pub role: user_hackathon_role::Model,
}

pub struct HackathonMember;
// User is either a judge, organizer, or eligible hacker for the hackathon. Uses EligibleHacker.
impl Policy<AppState> for HackathonMember {
    type Output = HackathonMembership;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        // 1. Identify the authenticated caller.
        let user = CurrentUser::from_request_parts(parts, state).await?;

        // 2. Resolve the slug and require an active hackathon.
        let hackathon = ActiveHackathon::check(parts, state).await?;

        // 3. Require an active role for this user in this hackathon.
        let role = user_hackathon_role::Entity::find()
            .filter(user_hackathon_role::Column::UserId.eq(user.user().id))
            .filter(user_hackathon_role::Column::HackathonId.eq(hackathon.id))
            .filter(user_hackathon_role::Column::IsActive.eq(true))
            .one(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::Forbidden)?;

        // 4. Apply the eligibility rule.
        match &role.role {
            HackathonRole::Judge | HackathonRole::Organizer => {
                // These roles qualify directly.
            }

            HackathonRole::Hacker => {
                // Hackers must have confirmed attendance on an active application for the same hackathon.
                application::Entity::find()
                    .filter(application::Column::UserId.eq(user.user().id))
                    .filter(application::Column::HackathonId.eq(hackathon.id))
                    .filter(application::Column::IsActive.eq(true))
                    .filter(application::Column::AttendanceStatus.eq(AttendanceStatus::Confirmed))
                    .one(&state.db)
                    .await
                    .map_err(|error| ApiError::Internal(error.into()))?
                    .ok_or(ApiError::Forbidden)?;
            }

            // Sponsors do not qualify under your latest rule.
            HackathonRole::Sponsor => {
                return Err(ApiError::Forbidden);
            }
        }

        // 5. Return the checked information.
        Ok(HackathonMembership {
            user,
            hackathon,
            role,
        })
    }
}

#[derive(serde::Deserialize)]
struct TeamDetailsPath {
    team_id: Uuid,
}

pub struct TeamDetailsAccess {
    pub membership: HackathonMembership,
    pub team: team::Model,
    pub can_view_restricted_details: bool,
}

pub struct ViewTeamDetails;

//Check whether a person can view full details of a team including members and submission, or can just view basic info when seeking for a team.
impl Policy<AppState> for ViewTeamDetails {
    type Output = TeamDetailsAccess;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let membership = HackathonMember::check(parts, state).await?;

        // Read the team UUID from the same request.
        let Path(TeamDetailsPath { team_id }) =
            Path::<TeamDetailsPath>::from_request_parts(parts, state)
                .await
                .map_err(|_| ApiError::BadRequest("Invalid team ID.".into()))?;

        // Require an active team in the checked hackathon.
        let team = team::Entity::find_by_id(team_id)
            .filter(team::Column::HackathonId.eq(membership.hackathon.id))
            .filter(team::Column::IsActive.eq(true))
            .one(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::NotFound)?;

        let can_view_restricted_details = match &membership.role.role {
            // Judges and organizers can view this team's details.
            HackathonRole::Judge | HackathonRole::Organizer => true,

            // Hackers can view restricted details only for a team they actively belong to.
            HackathonRole::Hacker => team_member::Entity::find()
                .filter(team_member::Column::UserId.eq(membership.user.user().id))
                .filter(team_member::Column::HackathonId.eq(membership.hackathon.id))
                .filter(team_member::Column::TeamId.eq(team.id))
                .filter(team_member::Column::Status.eq(TeamMemberStatus::Active))
                .one(&state.db)
                .await
                .map_err(|error| ApiError::Internal(error.into()))?
                .is_some(),

            HackathonRole::Sponsor => {
                return Err(ApiError::Forbidden);
            }
        };

        Ok(TeamDetailsAccess {
            membership,
            team,
            can_view_restricted_details,
        })
    }
}

#[derive(serde::Deserialize)]
struct RemoveTeamMemberPath {
    team_id: Uuid,
    user_id: Uuid,
}

// Checked information the removal handler receives.
pub struct TeamMemberRemovalAccess {
    pub user: CurrentUser,
    pub hackathon: hackathon::Model,
    pub team: team::Model,
    pub caller_membership: team_member::Model,
    pub target_membership: team_member::Model,
}

pub struct CanRemoveTeamMember;

impl Policy<AppState> for CanRemoveTeamMember {
    type Output = TeamMemberRemovalAccess;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        // 1. Identify the authenticated caller.
        let user = CurrentUser::from_request_parts(parts, state).await?;

        // 2. Resolve the slug and require an active hackathon.
        let hackathon = ActiveHackathon::check(parts, state).await?;

        // 3. Extract the requested team and target user.
        let Path(RemoveTeamMemberPath { team_id, user_id }) =
            Path::<RemoveTeamMemberPath>::from_request_parts(parts, state)
                .await
                .map_err(|_| ApiError::BadRequest("Invalid team ID or user ID.".into()))?;

        // 4. Require an active team in this hackathon.
        let team = team::Entity::find_by_id(team_id)
            .filter(team::Column::HackathonId.eq(hackathon.id))
            .filter(team::Column::IsActive.eq(true))
            .one(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::NotFound)?;

        // 5. The caller must actively belong to this exact team.
        let caller_membership = team_member::Entity::find()
            .filter(team_member::Column::TeamId.eq(team.id))
            .filter(team_member::Column::HackathonId.eq(hackathon.id))
            .filter(team_member::Column::UserId.eq(user.user().id))
            .filter(team_member::Column::Status.eq(TeamMemberStatus::Active))
            .one(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::Forbidden)?;

        // 6. Ordinary members may target only themselves.
        let removing_self = caller_membership.user_id == user_id;

        if !removing_self && !caller_membership.is_leader {
            return Err(ApiError::Forbidden);
        }

        // 7. The target must also actively belong to this team.
        // Reuse the caller's record for self-removal.
        let target_membership = if removing_self {
            caller_membership.clone()
        } else {
            team_member::Entity::find()
                .filter(team_member::Column::TeamId.eq(team.id))
                .filter(team_member::Column::HackathonId.eq(hackathon.id))
                .filter(team_member::Column::UserId.eq(user_id))
                .filter(team_member::Column::Status.eq(TeamMemberStatus::Active))
                .one(&state.db)
                .await
                .map_err(|error| ApiError::Internal(error.into()))?
                .ok_or(ApiError::NotFound)?
        };

        Ok(TeamMemberRemovalAccess {
            user,
            hackathon,
            team,
            caller_membership,
            target_membership,
        })
    }
}

#[derive(serde::Deserialize)]
struct JudgePath {
    judge_id: Uuid,
}

pub struct CurrentJudge;
//Checks if the current user requesting an API is a judge and the judge id passed in is themselves.
impl Policy<AppState> for CurrentJudge {
    type Output = HackathonMembership;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        // Requires an authenticated user, an active hackathon,
        // and an active role in that hackathon.
        let membership = HackathonMember::check(parts, state).await?;

        let Path(JudgePath { judge_id }) = Path::<JudgePath>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid judge ID.".into()))?;

        // Only the judge named in the URL may access it.
        if membership.user.user().id != judge_id {
            return Err(ApiError::Forbidden);
        }

        // Require the Judge role.
        if membership.role.role != HackathonRole::Judge {
            return Err(ApiError::Forbidden);
        }

        Ok(membership)
    }
}

#[derive(serde::Deserialize)]
struct JudgeAssignmentsPath {
    track_id: Uuid,
    judge_id: Uuid,
}

pub struct JudgeAssignmentsAccess {
    pub hackathon_id: Uuid,
    pub track_id: Uuid,
    pub judge_id: Uuid,
}

pub struct ViewOwnJudgeAssignments;
//Checks if the current user requesting an API is a judge and the judge id passed in is themselves, and that the track id passed in is an active track in the same hackathon as the judge.
impl Policy<AppState> for ViewOwnJudgeAssignments {
    type Output = JudgeAssignmentsAccess;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        // Checks authentication, ownership, active hackathon,
        // and active Judge role.
        let membership = CurrentJudge::check(parts, state).await?;

        let Path(JudgeAssignmentsPath { track_id, judge_id }) =
            Path::<JudgeAssignmentsPath>::from_request_parts(parts, state)
                .await
                .map_err(|_| ApiError::BadRequest("Invalid track ID or judge ID.".into()))?;

        let hackathon_id = membership.hackathon.id;

        // Require an active track in the same hackathon.
        prize_track::Entity::find_by_id(track_id)
            .filter(prize_track::Column::HackathonId.eq(hackathon_id))
            .filter(prize_track::Column::IsActive.eq(true))
            .one(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::NotFound)?;

        // A retained row alone does not prove current access.
        // At least one independent grant must still exist.
        let assignment = judge_track_assign::Entity::find()
            .filter(judge_track_assign::Column::UserId.eq(judge_id))
            .filter(judge_track_assign::Column::TrackId.eq(track_id))
            .filter(judge_track_assign::Column::HackathonId.eq(hackathon_id))
            .one(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or(ApiError::Forbidden)?;

        if !assignment.manually_granted && !assignment.sponsor_granted {
            return Err(ApiError::Forbidden);
        }

        Ok(JudgeAssignmentsAccess {
            hackathon_id,
            track_id,
            judge_id,
        })
    }
}
