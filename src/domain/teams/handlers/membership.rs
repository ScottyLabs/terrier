use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::auth::middleware::SyncedUser;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

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

    // Get owner's team
    let owner_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch owner role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Owner role not found"))?;

    let Some(team_id) = owner_role.team_id else {
        return Err(ServerFnError::new("You are not in a team"));
    };

    // Verify owner
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    if team.owner_id != user.0.id {
        return Err(ServerFnError::new("Only the team owner can kick members"));
    }

    // Cannot kick yourself
    if user_id == user.0.id {
        return Err(ServerFnError::new(
            "Cannot kick yourself. Use leave team instead.",
        ));
    }

    // Get member to kick
    let member_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch member role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Member not found in your team"))?;

    // Remove member from team
    let mut role: crate::entities::user_hackathon_roles::ActiveModel = member_role.into();
    role.team_id = Set(None);

    role.update(&state.db)
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

    // Fetch team to check ownership
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    let is_owner = team.owner_id == user.0.id;

    // Get team member count
    let member_count = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .count(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to count members: {}", e)))?
        as usize;

    // Owner can only leave if they're the only member
    if is_owner && member_count > 1 {
        return Err(ServerFnError::new(
            "As the team owner, you can only leave if you're the only member.",
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

    // Fetch team to check ownership
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    let is_owner = team.owner_id == user.0.id;

    // Get all team members
    let team_members = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch members: {}", e)))?;

    let member_count = team_members.len();

    if is_owner && force && member_count > 1 {
        // Remove all members from team
        for member in team_members {
            let mut role: crate::entities::user_hackathon_roles::ActiveModel = member.into();
            role.team_id = Set(None);
            role.update(&state.db)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to remove member: {}", e)))?;
        }

        // Delete the team
        crate::entities::prelude::Teams::delete_by_id(team_id)
            .exec(&state.db)
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

    // Get current owner's team
    let owner_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch owner role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Owner role not found"))?;

    let Some(team_id) = owner_role.team_id else {
        return Err(ServerFnError::new("You are not in a team"));
    };

    // Verify owner
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    if team.owner_id != user.0.id {
        return Err(ServerFnError::new(
            "Only the team owner can transfer ownership",
        ));
    }

    // Cannot transfer to yourself
    if new_owner_id == user.0.id {
        return Err(ServerFnError::new("You are already the owner"));
    }

    // Verify new owner is a team member
    let _new_owner_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(new_owner_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch new owner role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("New owner must be a member of the team"))?;

    // Transfer ownership
    let mut team_model: crate::entities::teams::ActiveModel = team.into();
    team_model.owner_id = Set(new_owner_id);

    team_model
        .update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to transfer ownership: {}", e)))?;

    Ok(())
}
