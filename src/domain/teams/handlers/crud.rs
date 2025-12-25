use crate::domain::teams::types::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::core::auth::{
    context::RequestContext, middleware::SyncedUser, permissions::Permissions,
};
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

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
    use crate::domain::teams::repository::TeamRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get user's team
    let team_repo = TeamRepository::new(&ctx.state.db);
    let team_id = match team_repo.find_user_team(ctx.user.id, hackathon.id).await? {
        Some(id) => id,
        None => return Ok(None),
    };

    // Get team with members
    let team_data = team_repo
        .get_team_with_members(team_id, hackathon.id, ctx.user.id, hackathon.max_team_size)
        .await?;

    Ok(Some(team_data))
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
    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

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
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch teams: {}", e)))?;

    let team_repo = crate::domain::teams::repository::TeamRepository::new(&ctx.state.db);
    let mut result = Vec::new();
    for team in teams {
        // Count members
        let member_count = team_repo.count_team_members(team.id, hackathon.id).await?;

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
    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get user's team_id
    let team_repo = crate::domain::teams::repository::TeamRepository::new(&ctx.state.db);
    let user_role = team_repo
        .find_user_role_or_error(
            ctx.user.id,
            hackathon.id,
            "User not registered for this hackathon",
        )
        .await?;

    let team_id = user_role
        .team_id
        .ok_or_else(|| ServerFnError::new("User is not in a team"))?;

    // Verify user is the team owner
    let team = Permissions::require_team_ownership(&ctx, team_id).await?;

    let mut team: crate::entities::teams::ActiveModel = team.into();
    team.name = Set(req.name);
    team.description = Set(req.description);
    team.updated_at = Set(Utc::now().naive_utc());

    team.update(&ctx.state.db)
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
    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Check if user is already in a team
    let team_repo = crate::domain::teams::repository::TeamRepository::new(&ctx.state.db);
    let user_role = team_repo.find_user_role(ctx.user.id, hackathon.id).await?;

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
        owner_id: Set(ctx.user.id),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };

    let created_team = new_team
        .insert(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create team: {}", e)))?;

    // Add user to team
    let mut role: crate::entities::user_hackathon_roles::ActiveModel = role.into();
    role.team_id = Set(Some(created_team.id));

    role.update(&ctx.state.db)
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
    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get user's team_id
    let team_repo = crate::domain::teams::repository::TeamRepository::new(&ctx.state.db);
    let user_role = team_repo
        .find_user_role_or_error(
            ctx.user.id,
            hackathon.id,
            "User not registered for this hackathon",
        )
        .await?;

    let team_id = user_role
        .team_id
        .ok_or_else(|| ServerFnError::new("User is not in a team"))?;

    // Verify user is the team owner
    let team = Permissions::require_team_ownership(&ctx, team_id).await?;

    // Delete the team
    let team_to_delete: crate::entities::teams::ActiveModel = team.into();
    team_to_delete
        .delete(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete team: {}", e)))?;

    Ok(())
}
