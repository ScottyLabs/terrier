use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::core::auth::{
    context::RequestContext, middleware::SyncedUser, permissions::Permissions,
};
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, Set};

/// Kick a member from team (owner only)
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/teams/members/{user_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("user_id" = i32, Path, description = "User ID to kick")
    ),
    responses(
        (status = 200, description = "Member kicked successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not team owner"),
        (status = 404, description = "Team or member not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[delete("/api/hackathons/:slug/teams/members/:user_id", user: SyncedUser)]
pub async fn kick_member(slug: String, user_id: i32) -> Result<(), ServerFnError> {
    use crate::domain::teams::repository::TeamRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get owner's team
    let team_repo = TeamRepository::new(&ctx.state.db);
    let owner_role = team_repo
        .find_user_role_or_error(ctx.user.id, hackathon.id, "Owner role not found")
        .await?;

    let Some(team_id) = owner_role.team_id else {
        return Err(ServerFnError::new("You are not in a team"));
    };

    // Verify owner
    Permissions::require_team_ownership(&ctx, team_id).await?;

    // Cannot kick yourself
    if user_id == ctx.user.id {
        return Err(ServerFnError::new(
            "Cannot kick yourself. Use leave team instead.",
        ));
    }

    // Get member to kick
    let member_role = team_repo
        .find_team_member_role_or_error(
            user_id,
            hackathon.id,
            team_id,
            "Member not found in your team",
        )
        .await?;

    // Remove member from team
    let mut role: crate::entities::user_hackathon_roles::ActiveModel = member_role.into();
    role.team_id = Set(None);

    role.update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to kick member: {}", e)))?;

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
    use crate::domain::teams::repository::TeamRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get user's role
    let team_repo = TeamRepository::new(&ctx.state.db);
    let user_role = team_repo
        .find_user_role_or_error(ctx.user.id, hackathon.id, "User role not found")
        .await?;

    let Some(team_id) = user_role.team_id else {
        return Err(ServerFnError::new("You are not in a team"));
    };

    // Fetch team to check ownership
    let team = team_repo.find_by_id(team_id).await?;

    let is_owner = team.owner_id == ctx.user.id;

    // Get team member count
    let member_count = team_repo.count_team_members(team_id, hackathon.id).await?;

    // Owner can only leave if they're the only member
    if is_owner && member_count > 1 {
        return Err(ServerFnError::new(
            "As the team owner, you can only leave if you're the only member.",
        ));
    }

    // If user is the only member, delete the team
    if member_count == 1 {
        // Delete the team
        let team_to_delete: crate::entities::teams::ActiveModel = team.into();
        team_to_delete
            .delete(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete team: {}", e)))?;
    }

    // Remove user from team
    let mut role: crate::entities::user_hackathon_roles::ActiveModel = user_role.into();
    role.team_id = Set(None);

    role.update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to leave team: {}", e)))?;

    Ok(())
}

/// Leave team with force option for owners (will delete team)
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/teams/leave/{force}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("force" = bool, Path, description = "Force leave and delete team (owner only)")
    ),
    responses(
        (status = 200, description = "Left team successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Cannot leave team with members"),
        (status = 404, description = "Team not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[delete("/api/hackathons/:slug/teams/leave/:force", user: SyncedUser)]
pub async fn leave_team_force(slug: String, force: bool) -> Result<(), ServerFnError> {
    use crate::domain::teams::repository::TeamRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get user's role
    let team_repo = TeamRepository::new(&ctx.state.db);
    let user_role = team_repo
        .find_user_role_or_error(ctx.user.id, hackathon.id, "User role not found")
        .await?;

    let Some(team_id) = user_role.team_id else {
        return Err(ServerFnError::new("You are not in a team"));
    };

    // Fetch team to check ownership
    let team = team_repo.find_by_id(team_id).await?;

    let is_owner = team.owner_id == ctx.user.id;

    // Get all team members
    let team_members = team_repo
        .get_team_member_roles(team_id, hackathon.id)
        .await?;

    let member_count = team_members.len();

    if is_owner && force && member_count > 1 {
        // Remove all members from team
        for member in team_members {
            let mut role: crate::entities::user_hackathon_roles::ActiveModel = member.into();
            role.team_id = Set(None);
            role.update(&ctx.state.db)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to remove member: {}", e)))?;
        }

        // Delete the team
        let team_to_delete: crate::entities::teams::ActiveModel = team.into();
        team_to_delete
            .delete(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete team: {}", e)))?;

        return Ok(());
    }

    // Owner can only leave if they're the only member (without force)
    if is_owner && member_count > 1 {
        return Err(ServerFnError::new(
            "Cannot leave team: as the team owner, you can only leave if you're the only member. Transfer ownership or have other members leave first.",
        ));
    }

    // If user is the only member, delete the team
    if member_count == 1 {
        let team_to_delete: crate::entities::teams::ActiveModel = team.into();
        team_to_delete
            .delete(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete team: {}", e)))?;
    }

    // Remove user from team
    let mut role: crate::entities::user_hackathon_roles::ActiveModel = user_role.into();
    role.team_id = Set(None);

    role.update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to leave team: {}", e)))?;

    Ok(())
}

/// Transfer team ownership to another member
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/teams/transfer/{new_owner_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("new_owner_id" = i32, Path, description = "New owner's user ID")
    ),
    responses(
        (status = 200, description = "Ownership transferred successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not team owner"),
        (status = 404, description = "Team or new owner not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[post("/api/hackathons/:slug/teams/transfer/:new_owner_id", user: SyncedUser)]
pub async fn transfer_ownership(slug: String, new_owner_id: i32) -> Result<(), ServerFnError> {
    use crate::domain::teams::repository::TeamRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get current owner's team
    let team_repo = TeamRepository::new(&ctx.state.db);
    let owner_role = team_repo
        .find_user_role_or_error(ctx.user.id, hackathon.id, "Owner role not found")
        .await?;

    let Some(team_id) = owner_role.team_id else {
        return Err(ServerFnError::new("You are not in a team"));
    };

    // Verify owner
    let team = Permissions::require_team_ownership(&ctx, team_id).await?;

    // Cannot transfer to yourself
    if new_owner_id == ctx.user.id {
        return Err(ServerFnError::new("You are already the owner"));
    }

    // Verify new owner is a team member
    let _new_owner_role = team_repo
        .find_team_member_role_or_error(
            new_owner_id,
            hackathon.id,
            team_id,
            "New owner must be a member of the team",
        )
        .await?;

    // Transfer ownership
    let mut team_model: crate::entities::teams::ActiveModel = team.into();
    team_model.owner_id = Set(new_owner_id);

    team_model
        .update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to transfer ownership: {}", e)))?;

    Ok(())
}
