use crate::domain::teams::types::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::core::auth::{
    context::RequestContext, middleware::SyncedUser, permissions::Permissions,
};
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

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
    use crate::domain::teams::repository::TeamRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Check if user is already in a team
    let team_repo = TeamRepository::new(&ctx.state.db);
    let user_role = team_repo.find_user_role(ctx.user.id, hackathon.id).await?;

    let Some(role) = user_role else {
        return Err(ServerFnError::new("User not registered for this hackathon"));
    };

    if role.team_id.is_some() {
        return Err(ServerFnError::new("You are already in a team"));
    }

    // Verify team exists
    let team = team_repo.find_by_id(req.team_id).await?;

    if team.hackathon_id != hackathon.id {
        return Err(ServerFnError::new("Team does not belong to this hackathon"));
    }

    // Check if team is full
    let member_count = team_repo
        .count_team_members(req.team_id, hackathon.id)
        .await?;

    if member_count >= hackathon.max_team_size as usize {
        return Err(ServerFnError::new("Team is full"));
    }

    // Check if user already has a pending request for this team
    let existing_request = crate::entities::prelude::TeamJoinRequests::find()
        .filter(crate::entities::team_join_requests::Column::TeamId.eq(req.team_id))
        .filter(crate::entities::team_join_requests::Column::UserId.eq(ctx.user.id))
        .one(&ctx.state.db)
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
        user_id: Set(ctx.user.id),
        message: Set(req.message),
        ..Default::default()
    };

    new_request
        .insert(&ctx.state.db)
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
    use crate::domain::teams::repository::TeamRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get user's team_id
    let team_repo = TeamRepository::new(&ctx.state.db);
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
    Permissions::require_team_ownership(&ctx, team_id).await?;

    // Fetch join requests for this team
    let requests = crate::entities::prelude::TeamJoinRequests::find()
        .filter(crate::entities::team_join_requests::Column::TeamId.eq(team_id))
        .order_by_desc(crate::entities::team_join_requests::Column::CreatedAt)
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch join requests: {}", e)))?;

    let mut result = Vec::new();
    for request in requests {
        let request_user = crate::entities::prelude::Users::find_by_id(request.user_id)
            .one(&ctx.state.db)
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
    use crate::domain::teams::repository::TeamRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Fetch join requests made by this user for teams in this hackathon
    let requests = crate::entities::prelude::TeamJoinRequests::find()
        .filter(crate::entities::team_join_requests::Column::UserId.eq(ctx.user.id))
        .order_by_desc(crate::entities::team_join_requests::Column::CreatedAt)
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch join requests: {}", e)))?;

    let team_repo = TeamRepository::new(&ctx.state.db);
    let mut result = Vec::new();
    for request in requests {
        // Fetch the team to ensure it belongs to this hackathon
        if let Ok(team) = team_repo.find_by_id(request.team_id).await {
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
    let ctx = RequestContext::extract(&user).await?;

    // Fetch the request
    let request = crate::entities::prelude::TeamJoinRequests::find_by_id(request_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch request: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Join request not found"))?;

    // Verify it's the user's request
    if request.user_id != ctx.user.id {
        return Err(ServerFnError::new("You can only cancel your own requests"));
    }

    // Delete the request
    crate::entities::prelude::TeamJoinRequests::delete_by_id(request_id)
        .exec(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to cancel request: {}", e)))?;

    // Suppress unused variable warning for slug (required by route path)
    let _ = slug;

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
    use crate::domain::teams::repository::TeamRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Fetch join request
    let join_request = crate::entities::prelude::TeamJoinRequests::find_by_id(request_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch join request: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Join request not found"))?;

    // Get user's team_id
    let team_repo = TeamRepository::new(&ctx.state.db);
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

    // Verify request is for user's team and user is the owner
    Permissions::require_team_request_ownership(&ctx, request_id, team_id).await?;

    // Check if team is full
    let member_count = team_repo.count_team_members(team_id, hackathon.id).await?;

    if member_count >= hackathon.max_team_size as usize {
        return Err(ServerFnError::new("Team is full"));
    }

    // Get the requesting user's role
    let requesting_user_role = team_repo
        .find_user_role_or_error(
            join_request.user_id,
            hackathon.id,
            "Requesting user not registered for this hackathon",
        )
        .await?;

    // Check if user already in a team
    if requesting_user_role.team_id.is_some() {
        return Err(ServerFnError::new("User is already in a team"));
    }

    // Add user to team
    let mut role: crate::entities::user_hackathon_roles::ActiveModel = requesting_user_role.into();
    role.team_id = Set(Some(team_id));

    role.update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to add user to team: {}", e)))?;

    // Delete the join request
    let request_to_delete: crate::entities::team_join_requests::ActiveModel = join_request.into();
    request_to_delete
        .delete(&ctx.state.db)
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
    use crate::domain::teams::repository::TeamRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Fetch join request
    let join_request = crate::entities::prelude::TeamJoinRequests::find_by_id(request_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch join request: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Join request not found"))?;

    // Get user's team_id
    let team_repo = TeamRepository::new(&ctx.state.db);
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

    // Verify request is for user's team and user is the owner
    Permissions::require_team_request_ownership(&ctx, request_id, team_id).await?;

    // Delete the join request
    let request_to_delete: crate::entities::team_join_requests::ActiveModel = join_request.into();
    request_to_delete
        .delete(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete join request: {}", e)))?;

    Ok(())
}
