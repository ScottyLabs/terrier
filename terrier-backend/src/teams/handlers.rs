use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, JoinType, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    AppState,
    auth::extractors::ApplicantRole,
    entities::{prelude::*, team_invites, team_join_requests, team_members, teams, users},
};

// ============ Path Parameters ============

#[derive(Deserialize, IntoParams)]
pub struct TeamPath {
    pub slug: String,
    pub team_id: i32,
}

#[derive(Deserialize, IntoParams)]
pub struct TeamRequestPath {
    pub slug: String,
    pub team_id: i32,
    pub request_id: i32,
}

#[derive(Deserialize, IntoParams)]
pub struct InvitePath {
    pub slug: String,
    pub invite_id: i32,
}

// ============ Response Types ============

#[derive(Serialize, ToSchema)]
pub struct TeamMemberInfo {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub email: String,
    pub is_leader: bool,
}

#[derive(Serialize, ToSchema)]
pub struct TeamResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub max_members: i32,
    pub member_count: i32,
    pub members: Vec<TeamMemberInfo>,
    pub is_member: bool,
    pub is_leader: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize, ToSchema)]
pub struct TeamListItem {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub member_count: i32,
    pub max_members: i32,
}

#[derive(Serialize, ToSchema)]
pub struct JoinRequestResponse {
    pub id: i32,
    pub user_id: i32,
    pub user_name: String,
    pub user_email: String,
    pub message: Option<String>,
    pub status: String,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize, ToSchema)]
pub struct TeamInviteResponse {
    pub id: i32,
    pub team_id: i32,
    pub team_name: String,
    pub invited_by_name: String,
    pub status: String,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize, ToSchema)]
pub struct MyTeamResponse {
    pub team: Option<TeamResponse>,
    pub pending_invites: Vec<TeamInviteResponse>,
}

// ============ Request Types ============

#[derive(Deserialize, ToSchema)]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct JoinRequestRequest {
    pub message: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct InviteMemberRequest {
    pub user_id: i32,
}

#[derive(Deserialize, ToSchema)]
pub struct RespondToRequestRequest {
    pub accept: bool,
}

// ============ Handlers ============

/// List all teams in a hackathon
#[utoipa::path(
    get,
    path = "/hackathons/{slug}/teams",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "List of teams", body = Vec<TeamListItem>),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "No access to this hackathon")
    ),
    tag = "Teams"
)]
pub async fn list_teams(
    State(state): State<AppState>,
    role: ApplicantRole,
) -> Result<Json<Vec<TeamListItem>>, StatusCode> {
    let teams_list = Teams::find()
        .filter(teams::Column::HackathonId.eq(role.hackathon_id))
        .order_by_asc(teams::Column::Name)
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut result = Vec::new();
    for team in teams_list {
        let member_count = TeamMembers::find()
            .filter(team_members::Column::TeamId.eq(team.id))
            .count(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as i32;

        result.push(TeamListItem {
            id: team.id,
            name: team.name,
            description: team.description,
            member_count,
            max_members: team.max_members,
        });
    }

    Ok(Json(result))
}

/// Get current user's team and pending invites
#[utoipa::path(
    get,
    path = "/hackathons/{slug}/teams/my-team",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "User's team info", body = MyTeamResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "No access to this hackathon")
    ),
    tag = "Teams"
)]
pub async fn get_my_team(
    State(state): State<AppState>,
    role: ApplicantRole,
) -> Result<Json<MyTeamResponse>, StatusCode> {
    tracing::info!(
        "get_my_team called for user_id={}, hackathon_id={}",
        role.user_id,
        role.hackathon_id
    );

    // Find user's team membership
    // Use find_also_related which handles the join automatically
    let membership = TeamMembers::find()
        .filter(team_members::Column::UserId.eq(role.user_id))
        .find_also_related(Teams)
        .all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find team membership: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Filter for the correct hackathon
    let membership = membership.into_iter().find(|(_, team)| {
        team.as_ref()
            .map(|t| t.hackathon_id == role.hackathon_id)
            .unwrap_or(false)
    });

    tracing::info!("membership query result: {:?}", membership.is_some());

    let team = if let Some((member, Some(team))) = membership {
        let members = get_team_members(&state, team.id).await?;
        Some(TeamResponse {
            id: team.id,
            name: team.name,
            description: team.description,
            max_members: team.max_members,
            member_count: members.len() as i32,
            is_member: true,
            is_leader: member.is_leader,
            members,
            created_at: team.created_at,
        })
    } else {
        None
    };

    // Get pending invites for the user
    // Use find_also_related which handles the join automatically
    let invites = TeamInvites::find()
        .filter(team_invites::Column::InvitedUserId.eq(role.user_id))
        .filter(team_invites::Column::Status.eq("pending"))
        .find_also_related(Teams)
        .all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find invites: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Filter for the correct hackathon
    let invites: Vec<_> = invites
        .into_iter()
        .filter(|(_, team)| {
            team.as_ref()
                .map(|t| t.hackathon_id == role.hackathon_id)
                .unwrap_or(false)
        })
        .collect();

    let mut pending_invites = Vec::new();
    for (invite, team) in invites {
        if let Some(team) = team {
            let inviter = Users::find_by_id(invite.invited_by_id)
                .one(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            pending_invites.push(TeamInviteResponse {
                id: invite.id,
                team_id: team.id,
                team_name: team.name,
                invited_by_name: inviter.and_then(|u| u.name).unwrap_or_default(),
                status: invite.status,
                created_at: invite.created_at,
            });
        }
    }

    Ok(Json(MyTeamResponse {
        team,
        pending_invites,
    }))
}

/// Get a specific team's details
#[utoipa::path(
    get,
    path = "/hackathons/{slug}/teams/{team_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("team_id" = i32, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Team details", body = TeamResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "No access to this hackathon"),
        (status = 404, description = "Team not found")
    ),
    tag = "Teams"
)]
pub async fn get_team(
    State(state): State<AppState>,
    role: ApplicantRole,
    Path(path): Path<TeamPath>,
) -> Result<Json<TeamResponse>, StatusCode> {
    let team = Teams::find_by_id(path.team_id)
        .filter(teams::Column::HackathonId.eq(role.hackathon_id))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let members = get_team_members(&state, team.id).await?;

    // Check if current user is a member
    let is_member = members.iter().any(|m| m.user_id == role.user_id);
    let is_leader = members
        .iter()
        .any(|m| m.user_id == role.user_id && m.is_leader);

    Ok(Json(TeamResponse {
        id: team.id,
        name: team.name,
        description: team.description,
        max_members: team.max_members,
        member_count: members.len() as i32,
        members,
        is_member,
        is_leader,
        created_at: team.created_at,
    }))
}

/// Create a new team
#[utoipa::path(
    post,
    path = "/hackathons/{slug}/teams",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = CreateTeamRequest,
    responses(
        (status = 201, description = "Team created", body = TeamResponse),
        (status = 400, description = "User already in a team"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "No access to this hackathon")
    ),
    tag = "Teams"
)]
pub async fn create_team(
    State(state): State<AppState>,
    role: ApplicantRole,
    Json(payload): Json<CreateTeamRequest>,
) -> Result<(StatusCode, Json<TeamResponse>), (StatusCode, String)> {
    // Check if user is already in a team
    let existing = TeamMembers::find()
        .join(JoinType::InnerJoin, team_members::Relation::Teams.def())
        .filter(team_members::Column::UserId.eq(role.user_id))
        .filter(teams::Column::HackathonId.eq(role.hackathon_id))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    if existing.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            "You are already in a team".to_string(),
        ));
    }

    let now = chrono::Utc::now().naive_utc();

    // Create the team
    let team = teams::ActiveModel {
        hackathon_id: Set(role.hackathon_id),
        name: Set(payload.name),
        description: Set(payload.description),
        max_members: Set(4),
        created_by_id: Set(role.user_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let team = team.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create team".to_string(),
        )
    })?;

    // Add creator as team leader
    let member = team_members::ActiveModel {
        team_id: Set(team.id),
        user_id: Set(role.user_id),
        is_leader: Set(true),
        joined_at: Set(now),
        ..Default::default()
    };

    member.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to add member".to_string(),
        )
    })?;

    let members = get_team_members(&state, team.id).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to get members".to_string(),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(TeamResponse {
            id: team.id,
            name: team.name,
            description: team.description,
            max_members: team.max_members,
            member_count: members.len() as i32,
            members,
            is_member: true,
            is_leader: true,
            created_at: team.created_at,
        }),
    ))
}

/// Update team details (leader only)
#[utoipa::path(
    put,
    path = "/hackathons/{slug}/teams/{team_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("team_id" = i32, Path, description = "Team ID")
    ),
    request_body = UpdateTeamRequest,
    responses(
        (status = 200, description = "Team updated", body = TeamResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not team leader"),
        (status = 404, description = "Team not found")
    ),
    tag = "Teams"
)]
pub async fn update_team(
    State(state): State<AppState>,
    role: ApplicantRole,
    Path(path): Path<TeamPath>,
    Json(payload): Json<UpdateTeamRequest>,
) -> Result<Json<TeamResponse>, StatusCode> {
    // Verify user is team leader
    let membership = TeamMembers::find()
        .filter(team_members::Column::TeamId.eq(path.team_id))
        .filter(team_members::Column::UserId.eq(role.user_id))
        .filter(team_members::Column::IsLeader.eq(true))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::FORBIDDEN)?;

    let team = Teams::find_by_id(path.team_id)
        .filter(teams::Column::HackathonId.eq(role.hackathon_id))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: teams::ActiveModel = team.into();
    if let Some(name) = payload.name {
        active.name = Set(name);
    }
    if let Some(description) = payload.description {
        active.description = Set(Some(description));
    }
    active.updated_at = Set(chrono::Utc::now().naive_utc());

    let team = active
        .update(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let members = get_team_members(&state, team.id).await?;

    Ok(Json(TeamResponse {
        id: team.id,
        name: team.name,
        description: team.description,
        max_members: team.max_members,
        member_count: members.len() as i32,
        members,
        is_member: true,
        is_leader: membership.is_leader,
        created_at: team.created_at,
    }))
}

/// Leave a team
#[utoipa::path(
    post,
    path = "/hackathons/{slug}/teams/{team_id}/leave",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("team_id" = i32, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Left team"),
        (status = 400, description = "Cannot leave as only leader"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not a member"),
        (status = 404, description = "Team not found")
    ),
    tag = "Teams"
)]
pub async fn leave_team(
    State(state): State<AppState>,
    role: ApplicantRole,
    Path(path): Path<TeamPath>,
) -> Result<StatusCode, (StatusCode, String)> {
    let membership = TeamMembers::find()
        .filter(team_members::Column::TeamId.eq(path.team_id))
        .filter(team_members::Column::UserId.eq(role.user_id))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or((StatusCode::FORBIDDEN, "Not a member".to_string()))?;

    // If leader, check if there are other members
    if membership.is_leader {
        let member_count = TeamMembers::find()
            .filter(team_members::Column::TeamId.eq(path.team_id))
            .count(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })?;

        if member_count > 1 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Transfer leadership before leaving".to_string(),
            ));
        }

        // Delete the team if only member
        Teams::delete_by_id(path.team_id)
            .exec(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to delete team".to_string(),
                )
            })?;
    } else {
        // Just remove membership
        TeamMembers::delete_by_id(membership.id)
            .exec(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to leave".to_string(),
                )
            })?;
    }

    Ok(StatusCode::OK)
}

/// Request to join a team
#[utoipa::path(
    post,
    path = "/hackathons/{slug}/teams/{team_id}/request",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("team_id" = i32, Path, description = "Team ID")
    ),
    request_body = JoinRequestRequest,
    responses(
        (status = 201, description = "Request sent"),
        (status = 400, description = "Already in a team or team full"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Team not found")
    ),
    tag = "Teams"
)]
pub async fn request_to_join(
    State(state): State<AppState>,
    role: ApplicantRole,
    Path(path): Path<TeamPath>,
    Json(payload): Json<JoinRequestRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Check if user is already in a team
    let existing_membership = TeamMembers::find()
        .join(JoinType::InnerJoin, team_members::Relation::Teams.def())
        .filter(team_members::Column::UserId.eq(role.user_id))
        .filter(teams::Column::HackathonId.eq(role.hackathon_id))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    if existing_membership.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            "You are already in a team".to_string(),
        ));
    }

    // Check if team exists and has space
    let team = Teams::find_by_id(path.team_id)
        .filter(teams::Column::HackathonId.eq(role.hackathon_id))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Team not found".to_string()))?;

    let member_count = TeamMembers::find()
        .filter(team_members::Column::TeamId.eq(path.team_id))
        .count(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })? as i32;

    if member_count >= team.max_members {
        return Err((StatusCode::BAD_REQUEST, "Team is full".to_string()));
    }

    // Check if already requested
    let existing_request = TeamJoinRequests::find()
        .filter(team_join_requests::Column::TeamId.eq(path.team_id))
        .filter(team_join_requests::Column::UserId.eq(role.user_id))
        .filter(team_join_requests::Column::Status.eq("pending"))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    if existing_request.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Request already pending".to_string(),
        ));
    }

    // Create request
    let request = team_join_requests::ActiveModel {
        team_id: Set(path.team_id),
        user_id: Set(role.user_id),
        message: Set(payload.message),
        status: Set("pending".to_string()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    };

    request.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create request".to_string(),
        )
    })?;

    Ok(StatusCode::CREATED)
}

/// Get join requests for a team (leader only)
#[utoipa::path(
    get,
    path = "/hackathons/{slug}/teams/{team_id}/requests",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("team_id" = i32, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "List of join requests", body = Vec<JoinRequestResponse>),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not team leader")
    ),
    tag = "Teams"
)]
pub async fn get_join_requests(
    State(state): State<AppState>,
    role: ApplicantRole,
    Path(path): Path<TeamPath>,
) -> Result<Json<Vec<JoinRequestResponse>>, StatusCode> {
    // Verify user is team member (any member can view requests)
    let _membership = TeamMembers::find()
        .filter(team_members::Column::TeamId.eq(path.team_id))
        .filter(team_members::Column::UserId.eq(role.user_id))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::FORBIDDEN)?;

    let requests = TeamJoinRequests::find()
        .filter(team_join_requests::Column::TeamId.eq(path.team_id))
        .filter(team_join_requests::Column::Status.eq("pending"))
        .order_by_desc(team_join_requests::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut result = Vec::new();
    for req in requests {
        let user = Users::find_by_id(req.user_id)
            .one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(user) = user {
            result.push(JoinRequestResponse {
                id: req.id,
                user_id: user.id,
                user_name: user.name.unwrap_or_default(),
                user_email: user.email,
                message: req.message,
                status: req.status,
                created_at: req.created_at,
            });
        }
    }

    Ok(Json(result))
}

/// Respond to a join request (leader only)
#[utoipa::path(
    post,
    path = "/hackathons/{slug}/teams/{team_id}/requests/{request_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("team_id" = i32, Path, description = "Team ID"),
        ("request_id" = i32, Path, description = "Request ID")
    ),
    request_body = RespondToRequestRequest,
    responses(
        (status = 200, description = "Response recorded"),
        (status = 400, description = "Team is full"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not team leader"),
        (status = 404, description = "Request not found")
    ),
    tag = "Teams"
)]
pub async fn respond_to_request(
    State(state): State<AppState>,
    role: ApplicantRole,
    Path(path): Path<TeamRequestPath>,
    Json(payload): Json<RespondToRequestRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Verify user is team leader
    let _membership = TeamMembers::find()
        .filter(team_members::Column::TeamId.eq(path.team_id))
        .filter(team_members::Column::UserId.eq(role.user_id))
        .filter(team_members::Column::IsLeader.eq(true))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or((StatusCode::FORBIDDEN, "Not team leader".to_string()))?;

    let request = TeamJoinRequests::find_by_id(path.request_id)
        .filter(team_join_requests::Column::TeamId.eq(path.team_id))
        .filter(team_join_requests::Column::Status.eq("pending"))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Request not found".to_string()))?;

    let now = chrono::Utc::now().naive_utc();

    if payload.accept {
        // Check team capacity
        let team = Teams::find_by_id(path.team_id)
            .one(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })?
            .ok_or((StatusCode::NOT_FOUND, "Team not found".to_string()))?;

        let member_count = TeamMembers::find()
            .filter(team_members::Column::TeamId.eq(path.team_id))
            .count(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })? as i32;

        if member_count >= team.max_members {
            return Err((StatusCode::BAD_REQUEST, "Team is full".to_string()));
        }

        // Add member
        let member = team_members::ActiveModel {
            team_id: Set(path.team_id),
            user_id: Set(request.user_id),
            is_leader: Set(false),
            joined_at: Set(now),
            ..Default::default()
        };

        member.insert(&state.db).await.map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to add member".to_string(),
            )
        })?;
    }

    // Update request status
    let mut active: team_join_requests::ActiveModel = request.into();
    active.status = Set(if payload.accept {
        "accepted".to_string()
    } else {
        "rejected".to_string()
    });
    active.responded_at = Set(Some(now));
    active.update(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to update request".to_string(),
        )
    })?;

    Ok(StatusCode::OK)
}

/// Invite a user to the team
#[utoipa::path(
    post,
    path = "/hackathons/{slug}/teams/{team_id}/invite",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("team_id" = i32, Path, description = "Team ID")
    ),
    request_body = InviteMemberRequest,
    responses(
        (status = 201, description = "Invite sent"),
        (status = 400, description = "User already in a team or team full"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not team member"),
        (status = 404, description = "User not found")
    ),
    tag = "Teams"
)]
pub async fn invite_member(
    State(state): State<AppState>,
    role: ApplicantRole,
    Path(path): Path<TeamPath>,
    Json(payload): Json<InviteMemberRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Verify user is team member
    let _membership = TeamMembers::find()
        .filter(team_members::Column::TeamId.eq(path.team_id))
        .filter(team_members::Column::UserId.eq(role.user_id))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or((StatusCode::FORBIDDEN, "Not team member".to_string()))?;

    // Check team capacity
    let team = Teams::find_by_id(path.team_id)
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Team not found".to_string()))?;

    let member_count = TeamMembers::find()
        .filter(team_members::Column::TeamId.eq(path.team_id))
        .count(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })? as i32;

    if member_count >= team.max_members {
        return Err((StatusCode::BAD_REQUEST, "Team is full".to_string()));
    }

    // Check if user is already in a team
    let existing = TeamMembers::find()
        .join(JoinType::InnerJoin, team_members::Relation::Teams.def())
        .filter(team_members::Column::UserId.eq(payload.user_id))
        .filter(teams::Column::HackathonId.eq(role.hackathon_id))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    if existing.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            "User is already in a team".to_string(),
        ));
    }

    // Check if already invited
    let existing_invite = TeamInvites::find()
        .filter(team_invites::Column::TeamId.eq(path.team_id))
        .filter(team_invites::Column::InvitedUserId.eq(payload.user_id))
        .filter(team_invites::Column::Status.eq("pending"))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    if existing_invite.is_some() {
        return Err((StatusCode::BAD_REQUEST, "User already invited".to_string()));
    }

    // Create invite
    let invite = team_invites::ActiveModel {
        team_id: Set(path.team_id),
        invited_user_id: Set(payload.user_id),
        invited_by_id: Set(role.user_id),
        status: Set("pending".to_string()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    };

    invite.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create invite".to_string(),
        )
    })?;

    Ok(StatusCode::CREATED)
}

/// Respond to a team invite
#[utoipa::path(
    post,
    path = "/hackathons/{slug}/teams/invites/{invite_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("invite_id" = i32, Path, description = "Invite ID")
    ),
    request_body = RespondToRequestRequest,
    responses(
        (status = 200, description = "Response recorded"),
        (status = 400, description = "Already in a team or team full"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Invite not found")
    ),
    tag = "Teams"
)]
pub async fn respond_to_invite(
    State(state): State<AppState>,
    role: ApplicantRole,
    Path(path): Path<InvitePath>,
    Json(payload): Json<RespondToRequestRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let invite = TeamInvites::find_by_id(path.invite_id)
        .filter(team_invites::Column::InvitedUserId.eq(role.user_id))
        .filter(team_invites::Column::Status.eq("pending"))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Invite not found".to_string()))?;

    let now = chrono::Utc::now().naive_utc();

    if payload.accept {
        // Check if user is already in a team
        let existing = TeamMembers::find()
            .join(JoinType::InnerJoin, team_members::Relation::Teams.def())
            .filter(team_members::Column::UserId.eq(role.user_id))
            .filter(teams::Column::HackathonId.eq(role.hackathon_id))
            .one(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })?;

        if existing.is_some() {
            return Err((
                StatusCode::BAD_REQUEST,
                "You are already in a team".to_string(),
            ));
        }

        // Check team capacity
        let team = Teams::find_by_id(invite.team_id)
            .one(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })?
            .ok_or((StatusCode::NOT_FOUND, "Team not found".to_string()))?;

        let member_count = TeamMembers::find()
            .filter(team_members::Column::TeamId.eq(invite.team_id))
            .count(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })? as i32;

        if member_count >= team.max_members {
            return Err((StatusCode::BAD_REQUEST, "Team is full".to_string()));
        }

        // Add member
        let member = team_members::ActiveModel {
            team_id: Set(invite.team_id),
            user_id: Set(role.user_id),
            is_leader: Set(false),
            joined_at: Set(now),
            ..Default::default()
        };

        member.insert(&state.db).await.map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to add member".to_string(),
            )
        })?;
    }

    // Update invite status
    let mut active: team_invites::ActiveModel = invite.into();
    active.status = Set(if payload.accept {
        "accepted".to_string()
    } else {
        "rejected".to_string()
    });
    active.responded_at = Set(Some(now));
    active.update(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to update invite".to_string(),
        )
    })?;

    Ok(StatusCode::OK)
}

/// Search participants to invite (those not in a team)
#[utoipa::path(
    get,
    path = "/hackathons/{slug}/teams/search-participants",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("q" = String, Query, description = "Search query")
    ),
    responses(
        (status = 200, description = "List of participants", body = Vec<TeamMemberInfo>),
        (status = 401, description = "Not authenticated")
    ),
    tag = "Teams"
)]
pub async fn search_participants(
    State(state): State<AppState>,
    role: ApplicantRole,
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
) -> Result<Json<Vec<TeamMemberInfo>>, StatusCode> {
    use sea_orm::Condition;

    // Get users who are participants but not in a team
    let search = params.q.unwrap_or_default().to_lowercase();

    // First, get all user IDs who are already in teams for this hackathon
    let team_member_ids: Vec<i32> = TeamMembers::find()
        .join(JoinType::InnerJoin, team_members::Relation::Teams.def())
        .filter(teams::Column::HackathonId.eq(role.hackathon_id))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|m| m.user_id)
        .collect();

    // Get participants not in teams
    let mut condition = Condition::all();
    if !search.is_empty() {
        condition = condition.add(
            Condition::any()
                .add(users::Column::Name.contains(&search))
                .add(users::Column::Email.contains(&search)),
        );
    }

    let users_list = Users::find()
        .filter(condition)
        .limit(20)
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: Vec<TeamMemberInfo> = users_list
        .into_iter()
        .filter(|u| !team_member_ids.contains(&u.id) && u.id != role.user_id)
        .map(|u| TeamMemberInfo {
            id: 0,
            user_id: u.id,
            name: u.name.unwrap_or_default(),
            email: u.email,
            is_leader: false,
        })
        .collect();

    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
}

// Helper function
async fn get_team_members(
    state: &AppState,
    team_id: i32,
) -> Result<Vec<TeamMemberInfo>, StatusCode> {
    let members = TeamMembers::find()
        .filter(team_members::Column::TeamId.eq(team_id))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut result = Vec::new();
    for member in members {
        let user = Users::find_by_id(member.user_id)
            .one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(user) = user {
            result.push(TeamMemberInfo {
                id: member.id,
                user_id: user.id,
                name: user.name.unwrap_or_default(),
                email: user.email,
                is_leader: member.is_leader,
            });
        }
    }

    Ok(result)
}
