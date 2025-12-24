use crate::domain::teams::types::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::auth::middleware::SyncedUser;
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

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
    let team = crate::entities::prelude::Teams::find_by_id(team_id)
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    if team.owner_id != user.0.id {
        return Err(ServerFnError::new("Only team owner can send invitations"));
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
