use crate::domain::teams::types::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::auth::middleware::SyncedUser;
#[cfg(feature = "server")]
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

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

    // Fetch all team members
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
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

    // Sort members so owner is first
    members.sort_by_key(|m| if m.user_id == team.owner_id { 0 } else { 1 });

    // Check if current user is a member or owner
    let is_member = members.iter().any(|m| m.user_id == user.0.id);
    let is_owner = team.owner_id == user.0.id;

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
