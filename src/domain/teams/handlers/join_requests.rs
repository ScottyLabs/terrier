use crate::domain::teams::types::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::auth::middleware::SyncedUser;
#[cfg(feature = "server")]
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

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
        return Err(ServerFnError::new(
            "You already have a pending request for this team",
        ));
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

    // Fetch team and verify user is the owner
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    if team.owner_id != user.0.id {
        return Err(ServerFnError::new("Only team owner can view join requests"));
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

/// Get outgoing join requests
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/outgoing-join-requests",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Outgoing join requests retrieved successfully", body = Vec<OutgoingJoinRequestResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[get("/api/hackathons/:slug/outgoing-join-requests", user: SyncedUser)]
pub async fn get_outgoing_join_requests(
    slug: String,
) -> Result<Vec<OutgoingJoinRequestResponse>, ServerFnError> {
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

    // Fetch join requests made by this user for teams in this hackathon
    let requests = crate::entities::prelude::TeamJoinRequests::find()
        .filter(crate::entities::team_join_requests::Column::UserId.eq(user.0.id))
        .order_by_desc(crate::entities::team_join_requests::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch join requests: {}", e)))?;

    let mut result = Vec::new();
    for request in requests {
        // Fetch the team to ensure it belongs to this hackathon
        let team = crate::entities::prelude::Teams::find_by_id(request.team_id)
            .one(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?;

        if let Some(team) = team {
            if team.hackathon_id == hackathon.id {
                result.push(OutgoingJoinRequestResponse {
                    id: request.id,
                    team_id: request.team_id,
                    team_name: team.name,
                    message: request.message,
                    created_at: request.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                });
            }
        }
    }

    Ok(result)
}

/// Cancel outgoing join request
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/outgoing-join-requests/{request_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("request_id" = i32, Path, description = "Join request ID")
    ),
    responses(
        (status = 200, description = "Join request cancelled successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not your request"),
        (status = 404, description = "Request not found"),
        (status = 500, description = "Server error")
    ),
    tag = "teams"
))]
#[delete("/api/hackathons/:slug/outgoing-join-requests/:request_id", user: SyncedUser)]
pub async fn cancel_outgoing_join_request(
    slug: String,
    request_id: i32,
) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch the request
    let request = crate::entities::prelude::TeamJoinRequests::find_by_id(request_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch request: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Join request not found"))?;

    // Verify it's the user's request
    if request.user_id != user.0.id {
        return Err(ServerFnError::new("You can only cancel your own requests"));
    }

    // Delete the request
    crate::entities::prelude::TeamJoinRequests::delete_by_id(request_id)
        .exec(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to cancel request: {}", e)))?;

    Ok(())
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

    // Fetch team and verify user is the owner
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    if team.owner_id != user.0.id {
        return Err(ServerFnError::new(
            "Only team owner can accept join requests",
        ));
    }

    // Fetch team members to check if team is full
    let members_roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::TeamId.eq(team_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch members: {}", e)))?;

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

    // Fetch team and verify user is the owner
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    if team.owner_id != user.0.id {
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
