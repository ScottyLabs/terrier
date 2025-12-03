use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct TeamData {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub member_count: usize,
    pub max_size: i32,
    pub is_owner: bool,
    pub is_member: bool,
    pub members: Vec<TeamMemberData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct TeamMemberData {
    pub user_id: i32,
    pub name: Option<String>,
    pub email: String,
    pub picture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct TeamListItem {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub member_count: usize,
    pub max_size: i32,
    pub is_full: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct UpdateTeamRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct JoinTeamRequest {
    pub team_id: i32,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct JoinRequestResponse {
    pub id: i32,
    pub team_id: i32,
    pub user_id: i32,
    pub user_name: Option<String>,
    pub user_email: String,
    pub user_picture: Option<String>,
    pub message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct UserWithoutTeam {
    pub id: i32,
    pub name: Option<String>,
    pub email: String,
    pub picture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct SendInvitationRequest {
    pub user_id: i32,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct InvitationResponse {
    pub id: i32,
    pub team_id: i32,
    pub team_name: String,
    pub user_id: i32,
    pub user_name: Option<String>,
    pub user_email: String,
    pub user_picture: Option<String>,
    pub message: Option<String>,
    pub created_at: String,
}

#[cfg(feature = "server")]
use crate::auth::middleware::SyncedUser;
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

/// Get the user's current team
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/team",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Team data retrieved successfully", body = Option<TeamData>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[get("/api/hackathons/:slug/team", user: SyncedUser)]
pub async fn get_my_team(slug: String) -> Result<Option<TeamData>, ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Get user's role to find their team_id
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?;

    let Some(role) = user_role else {
        return Ok(None);
    };

    let Some(team_id) = role.team_id else {
        return Ok(None);
    };

    // Fetch team
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    // Fetch all team members (ordered by ID to ensure first member is owner)
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .order_by_asc(crate::entities::user_hackathon_roles::Column::Id)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch members: {}", e)))?;

    let mut members = Vec::new();
    for member_role in members_roles.iter() {
        let user_entity = crate::entities::prelude::Users::find_by_id(member_role.user_id)
            .one(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch member: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Member user not found"))?;

        members.push(TeamMemberData {
            user_id: user_entity.id,
            name: user_entity.name,
            email: user_entity.email,
            picture: user_entity.picture,
        });
    }

    // Determine if current user is the owner (first member/creator)
    let is_owner = members
        .first()
        .map(|m| m.user_id == user.0.id)
        .unwrap_or(false);

    Ok(Some(TeamData {
        id: team.id,
        name: team.name,
        description: team.description,
        member_count: members.len(),
        max_size: hackathon.max_team_size,
        is_owner,
        is_member: true,
        members,
    }))
}

/// Get all teams for a hackathon with optional search
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/teams",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = Option<String>,
    responses(
        (status = 200, description = "Teams retrieved successfully", body = Vec<TeamListItem>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/teams", user: SyncedUser)]
pub async fn get_all_teams(
    slug: String,
    search: Option<String>,
) -> Result<Vec<TeamListItem>, ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Fetch all teams
    let mut query = crate::entities::prelude::Teams::find()
        .filter(crate::entities::teams::Column::HackathonId.eq(hackathon.id));

    // Apply search filter if provided
    if let Some(search_term) = search {
        if !search_term.is_empty() {
            query = query.filter(
                crate::entities::teams::Column::Name
                    .contains(&search_term)
                    .or(crate::entities::teams::Column::Description.contains(&search_term)),
            );
        }
    }

    let teams = query
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch teams: {}", e)))?;

    let mut result = Vec::new();
    for team in teams {
        // Count members
        let member_count = crate::entities::prelude::UserHackathonRoles::find()
            .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team.id))
            .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
            .count(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to count members: {}", e)))?
            as usize;

        result.push(TeamListItem {
            id: team.id,
            name: team.name,
            description: team.description,
            member_count,
            max_size: hackathon.max_team_size,
            is_full: member_count >= hackathon.max_team_size as usize,
        });
    }

    Ok(result)
}

/// Update team details (owner only)
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}/team",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = UpdateTeamRequest,
    responses(
        (status = 200, description = "Team updated successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Only team owner can update"),
        (status = 404, description = "Hackathon or team not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[put("/api/hackathons/:slug/team", user: SyncedUser)]
pub async fn update_team(slug: String, req: UpdateTeamRequest) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Get user's team_id
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User not registered for this hackathon"))?;

    let team_id = user_role
        .team_id
        .ok_or_else(|| ServerFnError::new("User is not in a team"))?;

    // Verify user is the team owner
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .order_by_asc(crate::entities::user_hackathon_roles::Column::Id)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch members: {}", e)))?;

    if members_roles.is_empty() {
        return Err(ServerFnError::new("Team not found"));
    }

    // First member is the owner
    if members_roles[0].user_id != user.0.id {
        return Err(ServerFnError::new(
            "Only team owner can update team details",
        ));
    }

    // Update team
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    let mut team: crate::entities::teams::ActiveModel = team.into();
    team.name = Set(req.name);
    team.description = Set(req.description);
    team.updated_at = Set(Utc::now().naive_utc());

    team.update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update team: {}", e)))?;

    Ok(())
}

/// Create a new team
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/team/create",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = CreateTeamRequest,
    responses(
        (status = 200, description = "Team created successfully"),
        (status = 400, description = "User already in a team"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/team/create", user: SyncedUser)]
pub async fn create_team(slug: String, req: CreateTeamRequest) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Check if user is already in a team
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?;

    let Some(role) = user_role else {
        return Err(ServerFnError::new("User not registered for this hackathon"));
    };

    if role.team_id.is_some() {
        return Err(ServerFnError::new("You are already in a team"));
    }

    // Create team
    let new_team = crate::entities::teams::ActiveModel {
        hackathon_id: Set(hackathon.id),
        name: Set(req.name),
        description: Set(req.description),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };

    let created_team = new_team
        .insert(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create team: {}", e)))?;

    // Add user to team
    let mut role: crate::entities::user_hackathon_roles::ActiveModel = role.into();
    role.team_id = Set(Some(created_team.id));

    role.update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to join team: {}", e)))?;

    Ok(())
}

/// Request to join a team
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/team/request-join",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = JoinTeamRequest,
    responses(
        (status = 200, description = "Join request created successfully"),
        (status = 400, description = "Team is full or user already in team or request already exists"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon or team not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/team/request-join", user: SyncedUser)]
pub async fn request_join_team(slug: String, req: JoinTeamRequest) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Check if user is already in a team
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?;

    let Some(role) = user_role else {
        return Err(ServerFnError::new("User not registered for this hackathon"));
    };

    if role.team_id.is_some() {
        return Err(ServerFnError::new("You are already in a team"));
    }

    // Verify team exists
    let team = crate::entities::prelude::Teams::find_by_id(req.team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    if team.hackathon_id != hackathon.id {
        return Err(ServerFnError::new("Team does not belong to this hackathon"));
    }

    // Check if team is full
    let member_count = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(req.team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .count(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to count members: {}", e)))?;

    if member_count >= hackathon.max_team_size as u64 {
        return Err(ServerFnError::new("Team is full"));
    }

    // Check if user already has a pending request for this team
    let existing_request = crate::entities::prelude::TeamJoinRequests::find()
        .filter(crate::entities::team_join_requests::Column::TeamId.eq(req.team_id))
        .filter(crate::entities::team_join_requests::Column::UserId.eq(user.0.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to check existing request: {}", e)))?;

    if existing_request.is_some() {
        return Err(ServerFnError::new("You already have a pending request for this team"));
    }

    // Create join request
    let new_request = crate::entities::team_join_requests::ActiveModel {
        team_id: Set(req.team_id),
        user_id: Set(user.0.id),
        message: Set(req.message),
        ..Default::default()
    };

    new_request
        .insert(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create join request: {}", e)))?;

    Ok(())
}

/// Get pending join requests for user's team (owner only)
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/team/requests",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Join requests retrieved successfully", body = Vec<JoinRequestResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Only team owner can view requests"),
        (status = 404, description = "Hackathon not found or user not in team"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[get("/api/hackathons/:slug/team/requests", user: SyncedUser)]
pub async fn get_join_requests(slug: String) -> Result<Vec<JoinRequestResponse>, ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Get user's team_id
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User not registered for this hackathon"))?;

    let team_id = user_role
        .team_id
        .ok_or_else(|| ServerFnError::new("User is not in a team"))?;

    // Verify user is the team owner
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .order_by_asc(crate::entities::user_hackathon_roles::Column::Id)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch members: {}", e)))?;

    if members_roles.is_empty() {
        return Err(ServerFnError::new("Team not found"));
    }

    // First member is the owner
    if members_roles[0].user_id != user.0.id {
        return Err(ServerFnError::new(
            "Only team owner can view join requests",
        ));
    }

    // Fetch join requests for this team
    let requests = crate::entities::prelude::TeamJoinRequests::find()
        .filter(crate::entities::team_join_requests::Column::TeamId.eq(team_id))
        .order_by_desc(crate::entities::team_join_requests::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch join requests: {}", e)))?;

    let mut result = Vec::new();
    for request in requests {
        let request_user = crate::entities::prelude::Users::find_by_id(request.user_id)
            .one(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch user: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Request user not found"))?;

        result.push(JoinRequestResponse {
            id: request.id,
            team_id: request.team_id,
            user_id: request.user_id,
            user_name: request_user.name,
            user_email: request_user.email,
            user_picture: request_user.picture,
            message: request.message,
            created_at: request.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        });
    }

    Ok(result)
}

/// Accept a join request (owner only)
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/team/requests/{request_id}/accept",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("request_id" = i32, Path, description = "Join request ID")
    ),
    responses(
        (status = 200, description = "Join request accepted successfully"),
        (status = 400, description = "Team is full or user already in team"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Only team owner can accept requests"),
        (status = 404, description = "Request not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/team/requests/:request_id/accept", user: SyncedUser)]
pub async fn accept_join_request(slug: String, request_id: i32) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Fetch join request
    let join_request = crate::entities::prelude::TeamJoinRequests::find_by_id(request_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch join request: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Join request not found"))?;

    // Get user's team_id and verify they're the owner
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User not registered for this hackathon"))?;

    let team_id = user_role
        .team_id
        .ok_or_else(|| ServerFnError::new("User is not in a team"))?;

    if team_id != join_request.team_id {
        return Err(ServerFnError::new("This request is not for your team"));
    }

    // Verify user is the team owner
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .order_by_asc(crate::entities::user_hackathon_roles::Column::Id)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch members: {}", e)))?;

    if members_roles.is_empty() {
        return Err(ServerFnError::new("Team not found"));
    }

    // First member is the owner
    if members_roles[0].user_id != user.0.id {
        return Err(ServerFnError::new(
            "Only team owner can accept join requests",
        ));
    }

    // Check if team is full
    if members_roles.len() >= hackathon.max_team_size as usize {
        return Err(ServerFnError::new("Team is full"));
    }

    // Get the requesting user's role
    let requesting_user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(join_request.user_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch requesting user role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Requesting user not registered for this hackathon"))?;

    // Check if user already in a team
    if requesting_user_role.team_id.is_some() {
        return Err(ServerFnError::new("User is already in a team"));
    }

    // Add user to team
    let mut role: crate::entities::user_hackathon_roles::ActiveModel = requesting_user_role.into();
    role.team_id = Set(Some(team_id));

    role.update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to add user to team: {}", e)))?;

    // Delete the join request
    let request_to_delete: crate::entities::team_join_requests::ActiveModel = join_request.into();
    request_to_delete
        .delete(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete join request: {}", e)))?;

    Ok(())
}

/// Reject a join request (owner only)
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/team/requests/{request_id}/reject",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("request_id" = i32, Path, description = "Join request ID")
    ),
    responses(
        (status = 200, description = "Join request rejected successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Only team owner can reject requests"),
        (status = 404, description = "Request not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/team/requests/:request_id/reject", user: SyncedUser)]
pub async fn reject_join_request(slug: String, request_id: i32) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Fetch join request
    let join_request = crate::entities::prelude::TeamJoinRequests::find_by_id(request_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch join request: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Join request not found"))?;

    // Get user's team_id and verify they're the owner
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User not registered for this hackathon"))?;

    let team_id = user_role
        .team_id
        .ok_or_else(|| ServerFnError::new("User is not in a team"))?;

    if team_id != join_request.team_id {
        return Err(ServerFnError::new("This request is not for your team"));
    }

    // Verify user is the team owner
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .order_by_asc(crate::entities::user_hackathon_roles::Column::Id)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch members: {}", e)))?;

    if members_roles.is_empty() {
        return Err(ServerFnError::new("Team not found"));
    }

    // First member is the owner
    if members_roles[0].user_id != user.0.id {
        return Err(ServerFnError::new(
            "Only team owner can reject join requests",
        ));
    }

    // Delete the join request
    let request_to_delete: crate::entities::team_join_requests::ActiveModel = join_request.into();
    request_to_delete
        .delete(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete join request: {}", e)))?;

    Ok(())
}

/// Get users without a team for invitations
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/users-without-team",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = Option<String>,
    responses(
        (status = 200, description = "Users retrieved successfully", body = Vec<UserWithoutTeam>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/users-without-team", user: SyncedUser)]
pub async fn get_users_without_team(
    slug: String,
    search: Option<String>,
) -> Result<Vec<UserWithoutTeam>, ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Get all user roles for this hackathon where team_id is null
    let roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.is_null())
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch roles: {}", e)))?;

    let mut result = Vec::new();
    for role in roles {
        let user_entity = crate::entities::prelude::Users::find_by_id(role.user_id)
            .one(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch user: {}", e)))?
            .ok_or_else(|| ServerFnError::new("User not found"))?;

        // Apply search filter if provided
        if let Some(ref search_term) = search {
            if !search_term.is_empty() {
                let name_match = user_entity
                    .name
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&search_term.to_lowercase()))
                    .unwrap_or(false);
                let email_match = user_entity
                    .email
                    .to_lowercase()
                    .contains(&search_term.to_lowercase());

                if !name_match && !email_match {
                    continue;
                }
            }
        }

        result.push(UserWithoutTeam {
            id: user_entity.id,
            name: user_entity.name,
            email: user_entity.email,
            picture: user_entity.picture,
        });
    }

    Ok(result)
}

/// Leave current team
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/team/leave",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Successfully left team"),
        (status = 400, description = "Cannot leave - you are the only member"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon or team not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/team/leave", user: SyncedUser)]
pub async fn leave_team(slug: String) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Get user's role
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User role not found"))?;

    let Some(team_id) = user_role.team_id else {
        return Err(ServerFnError::new("You are not in a team"));
    };

    // Get all team members to check if user is owner
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .order_by_asc(crate::entities::user_hackathon_roles::Column::Id)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch members: {}", e)))?;

    let member_count = members_roles.len();
    let is_owner = !members_roles.is_empty() && members_roles[0].user_id == user.0.id;

    // Owner can only leave if they're the only member
    // Non-owners can leave anytime
    if is_owner && member_count > 1 {
        return Err(ServerFnError::new(
            "Cannot leave team: as the team owner, you can only leave if you're the only member. Transfer ownership or have other members leave first.",
        ));
    }

    // If user is the only member, delete the team
    if member_count == 1 {
        // Delete the team
        crate::entities::prelude::Teams::delete_by_id(team_id)
            .exec(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete team: {}", e)))?;
    }

    // Remove user from team
    let mut role: crate::entities::user_hackathon_roles::ActiveModel = user_role.into();
    role.team_id = Set(None);

    role.update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to leave team: {}", e)))?;

    Ok(())
}

/// Get team details
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/teams/{team_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("team_id" = i32, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Team details retrieved successfully", body = TeamData),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon or team not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[get("/api/hackathons/:slug/teams/:team_id", user: SyncedUser)]
pub async fn get_team_details(slug: String, team_id: i32) -> Result<TeamData, ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Fetch team
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    if team.hackathon_id != hackathon.id {
        return Err(ServerFnError::new("Team does not belong to this hackathon"));
    }

    // Fetch all team members, ordered by ID
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .order_by_asc(crate::entities::user_hackathon_roles::Column::Id)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch members: {}", e)))?;

    let mut members = Vec::new();
    for member_role in members_roles.iter() {
        let member_user = crate::entities::prelude::Users::find_by_id(member_role.user_id)
            .one(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch member: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Member user not found"))?;

        members.push(TeamMemberData {
            user_id: member_user.id,
            name: member_user.name,
            email: member_user.email,
            picture: member_user.picture,
        });
    }

    // Check if current user is a member or owner
    let is_member = members.iter().any(|m| m.user_id == user.0.id);
    let is_owner = members
        .first()
        .map(|m| m.user_id == user.0.id)
        .unwrap_or(false);

    Ok(TeamData {
        id: team.id,
        name: team.name,
        description: team.description,
        member_count: members.len(),
        max_size: hackathon.max_team_size,
        is_owner,
        is_member,
        members,
    })
}

/// Send a team invitation to a user
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/team/invite",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = SendInvitationRequest,
    responses(
        (status = 200, description = "Invitation sent successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon or user not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/team/invite", user: SyncedUser)]
pub async fn send_invitation(
    slug: String,
    req: SendInvitationRequest,
) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Get sender's team_id and verify they're the owner
    let sender_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User not registered for this hackathon"))?;

    let team_id = sender_role
        .team_id
        .ok_or_else(|| ServerFnError::new("You must be in a team to send invitations"))?;

    // Verify sender is the team owner
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .order_by_asc(crate::entities::user_hackathon_roles::Column::Id)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch members: {}", e)))?;

    if members_roles.is_empty() {
        return Err(ServerFnError::new("Team not found"));
    }

    // First member is the owner
    if members_roles[0].user_id != user.0.id {
        return Err(ServerFnError::new(
            "Only team owner can send invitations",
        ));
    }

    // Verify target user is registered for this hackathon and has no team
    let target_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(req.user_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch target user role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User not registered for this hackathon"))?;

    if target_role.team_id.is_some() {
        return Err(ServerFnError::new("User is already in a team"));
    }

    // Check if invitation already exists
    let existing_invitation = crate::entities::prelude::TeamInvitations::find()
        .filter(crate::entities::team_invitations::Column::TeamId.eq(team_id))
        .filter(crate::entities::team_invitations::Column::UserId.eq(req.user_id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to check existing invitation: {}", e)))?;

    if existing_invitation.is_some() {
        return Err(ServerFnError::new("Invitation already sent to this user"));
    }

    // Create invitation
    let invitation = crate::entities::team_invitations::ActiveModel {
        team_id: sea_orm::Set(team_id),
        user_id: sea_orm::Set(req.user_id),
        message: sea_orm::Set(req.message),
        created_at: sea_orm::Set(Utc::now().naive_utc()),
        ..Default::default()
    };

    invitation
        .insert(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create invitation: {}", e)))?;

    Ok(())
}

/// Get invitations for the current user
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/team/invitations",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Invitations retrieved successfully", body = Vec<InvitationResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[get("/api/hackathons/:slug/team/invitations", user: SyncedUser)]
pub async fn get_my_invitations(slug: String) -> Result<Vec<InvitationResponse>, ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Fetch invitations for current user
    let invitations = crate::entities::prelude::TeamInvitations::find()
        .filter(crate::entities::team_invitations::Column::UserId.eq(user.0.id))
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch invitations: {}", e)))?;

    let mut result = Vec::new();
    for invitation in invitations {
        // Fetch team details
        let team = crate::entities::prelude::Teams::find_by_id(invitation.team_id)
            .one(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Team not found"))?;

        // Check if team belongs to this hackathon
        if team.hackathon_id != hackathon.id {
            continue;
        }

        result.push(InvitationResponse {
            id: invitation.id,
            team_id: invitation.team_id,
            team_name: team.name,
            user_id: invitation.user_id,
            user_name: user.0.name.clone(),
            user_email: user.0.email.clone(),
            user_picture: user.0.picture.clone(),
            message: invitation.message,
            created_at: invitation.created_at.to_string(),
        });
    }

    Ok(result)
}

/// Accept a team invitation
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/team/invitations/{invitation_id}/accept",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("invitation_id" = i32, Path, description = "Invitation ID")
    ),
    responses(
        (status = 200, description = "Invitation accepted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Invitation not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/team/invitations/:invitation_id/accept", user: SyncedUser)]
pub async fn accept_invitation(slug: String, invitation_id: i32) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Fetch invitation
    let invitation = crate::entities::prelude::TeamInvitations::find_by_id(invitation_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch invitation: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Invitation not found"))?;

    // Verify the invitation is for the current user
    if invitation.user_id != user.0.id {
        return Err(ServerFnError::new("This invitation is not for you"));
    }

    // Verify user doesn't already have a team
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User not registered for this hackathon"))?;

    if user_role.team_id.is_some() {
        return Err(ServerFnError::new("You are already in a team"));
    }

    // Check if team is full
    let team_members_count = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(invitation.team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .count(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to count team members: {}", e)))?;

    if team_members_count >= hackathon.max_team_size as u64 {
        return Err(ServerFnError::new("Team is full"));
    }

    // Update user's team_id
    let mut user_role_active: crate::entities::user_hackathon_roles::ActiveModel = user_role.into();
    user_role_active.team_id = sea_orm::Set(Some(invitation.team_id));
    user_role_active
        .update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update user role: {}", e)))?;

    // Delete the invitation
    let invitation_to_delete: crate::entities::team_invitations::ActiveModel = invitation.into();
    invitation_to_delete
        .delete(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete invitation: {}", e)))?;

    Ok(())
}

/// Decline a team invitation
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/team/invitations/{invitation_id}/decline",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("invitation_id" = i32, Path, description = "Invitation ID")
    ),
    responses(
        (status = 200, description = "Invitation declined successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Invitation not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/team/invitations/:invitation_id/decline", user: SyncedUser)]
pub async fn decline_invitation(slug: String, invitation_id: i32) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch invitation
    let invitation = crate::entities::prelude::TeamInvitations::find_by_id(invitation_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch invitation: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Invitation not found"))?;

    // Verify the invitation is for the current user
    if invitation.user_id != user.0.id {
        return Err(ServerFnError::new("This invitation is not for you"));
    }

    // Delete the invitation
    let invitation_to_delete: crate::entities::team_invitations::ActiveModel = invitation.into();
    invitation_to_delete
        .delete(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete invitation: {}", e)))?;

    Ok(())
}
