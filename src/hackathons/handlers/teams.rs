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
pub struct JoinTeamRequest {
    pub team_id: i32,
}

#[cfg(feature = "server")]
use crate::auth::middleware::SyncedUser;
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
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

    // Verify user is the team owner (first member - ordered by ID)
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

/// Join a team
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/team/join",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = JoinTeamRequest,
    responses(
        (status = 200, description = "Successfully joined team"),
        (status = 400, description = "Team is full or user already in team"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon or team not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/team/join", user: SyncedUser)]
pub async fn join_team(slug: String, req: JoinTeamRequest) -> Result<(), ServerFnError> {
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

    // Add user to team
    let mut role: crate::entities::user_hackathon_roles::ActiveModel = role.into();
    role.team_id = Set(Some(req.team_id));

    role.update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to join team: {}", e)))?;

    Ok(())
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

    // Check if user is the only member
    let member_count = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .count(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to count members: {}", e)))?;

    if member_count <= 1 {
        return Err(ServerFnError::new(
            "Cannot leave team: you are the only member",
        ));
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
