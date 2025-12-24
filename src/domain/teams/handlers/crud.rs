use crate::domain::teams::types::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::auth::middleware::SyncedUser;
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
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

    // Fetch all team members
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
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

    // Sort members so owner is first
    members.sort_by_key(|m| if m.user_id == team.owner_id { 0 } else { 1 });

    // Determine if current user is the owner
    let is_owner = team.owner_id == user.0.id;

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
            use sea_orm::sea_query::{Expr, Func};
            let search_pattern = format!("%{}%", search_term.to_lowercase());
            query = query.filter(
                Expr::expr(Func::lower(Expr::col(crate::entities::teams::Column::Name)))
                    .like(&search_pattern)
                    .or(Expr::expr(Func::lower(Expr::col(
                        crate::entities::teams::Column::Description,
                    )))
                    .like(&search_pattern)),
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

    // Fetch team and verify user is the owner
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    if team.owner_id != user.0.id {
        return Err(ServerFnError::new(
            "Only team owner can update team details",
        ));
    }

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
        owner_id: Set(user.0.id),
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

/// Delete team (owner only)
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/team",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Team deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Only team owner can delete"),
        (status = 404, description = "Hackathon or team not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[delete("/api/hackathons/:slug/team", user: SyncedUser)]
pub async fn delete_team(slug: String) -> Result<(), ServerFnError> {
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

    // Fetch team and verify user is the owner
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    if team.owner_id != user.0.id {
        return Err(ServerFnError::new("Only team owner can delete team"));
    }

    // Delete the team
    crate::entities::prelude::Teams::delete_by_id(team_id)
        .exec(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete team: {}", e)))?;

    Ok(())
}
