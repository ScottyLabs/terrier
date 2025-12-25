use dioxus::prelude::ServerFnError;
use sea_orm::EntityTrait;

use crate::{
    auth::HackathonRoleType,
    domain::people::repository::UserRoleRepository,
    domain::teams::repository::TeamRepository,
    entities::{prelude::*, teams},
};

use super::context::RequestContext;

/// Permission checking utility that provides an interface for all authorization checks.
///
/// All methods take a `RequestContext` which provides access to the current user, state, and
/// optionally hackathon and role information.
pub struct Permissions;

impl Permissions {
    // Global Admin Checks

    /// Check if the current user is a global admin, configured via `ADMIN_EMAILS`
    pub fn is_global_admin(ctx: &RequestContext) -> bool {
        ctx.state
            .config
            .admin_emails
            .contains(&ctx.user.email.to_lowercase())
    }

    /// Returns an error if the user is not a global admin.
    pub fn require_global_admin(ctx: &RequestContext) -> Result<(), ServerFnError> {
        if Self::is_global_admin(ctx) {
            Ok(())
        } else {
            Err(ServerFnError::new("Admin access required"))
        }
    }

    // Hackathon-Level Checks

    /// Check if the user is an admin for the current hackathon
    ///
    /// Requires that hackathon has been loaded into context via `with_hackathon()`.
    pub async fn is_hackathon_admin(ctx: &RequestContext) -> Result<bool, ServerFnError> {
        let hackathon = ctx.hackathon()?;
        let role_repo = UserRoleRepository::new(&ctx.state.db);
        role_repo.is_admin(ctx.user.id, hackathon.id).await
    }

    /// Check if the user is an admin or organizer for the current hackathon
    ///
    /// Requires that hackathon has been loaded into context via `with_hackathon()`.
    pub async fn is_admin_or_organizer(ctx: &RequestContext) -> Result<bool, ServerFnError> {
        let hackathon = ctx.hackathon()?;
        let role_repo = UserRoleRepository::new(&ctx.state.db);
        role_repo
            .is_admin_or_organizer(ctx.user.id, hackathon.id)
            .await
    }

    /// Require that the user is an admin or organizer for the current hackathon
    ///
    /// Requires that hackathon has been loaded into context via `with_hackathon()`.
    pub async fn require_admin_or_organizer(ctx: &RequestContext) -> Result<(), ServerFnError> {
        if Self::is_global_admin(ctx) {
            return Ok(());
        }

        if Self::is_admin_or_organizer(ctx).await? {
            Ok(())
        } else {
            Err(ServerFnError::new("Admin or organizer access required"))
        }
    }

    /// Require that the user is either a global admin or a hackathon admin
    ///
    /// Requires that hackathon has been loaded into context via `with_hackathon()`.
    pub async fn require_admin(ctx: &RequestContext) -> Result<(), ServerFnError> {
        if Self::is_global_admin(ctx) {
            return Ok(());
        }

        if Self::is_hackathon_admin(ctx).await? {
            Ok(())
        } else {
            Err(ServerFnError::new("Admin access required"))
        }
    }

    // Team-Level Checks

    /// Check if the user owns the specified team
    pub async fn is_team_owner(ctx: &RequestContext, team_id: i32) -> Result<bool, ServerFnError> {
        let team_repo = TeamRepository::new(&ctx.state.db);
        let team = team_repo.find_by_id(team_id).await?;
        Ok(team.owner_id == ctx.user.id)
    }

    /// Require that the user owns the specified team
    ///
    /// Returns the team if the user is the owner, otherwise returns an error.
    pub async fn require_team_ownership(
        ctx: &RequestContext,
        team_id: i32,
    ) -> Result<teams::Model, ServerFnError> {
        let team_repo = TeamRepository::new(&ctx.state.db);
        let team = team_repo.find_by_id(team_id).await?;

        if team.owner_id == ctx.user.id {
            Ok(team)
        } else {
            Err(ServerFnError::new(
                "Only team owner can perform this action",
            ))
        }
    }

    // Role Checks

    /// Check if the user has any of the specified roles
    ///
    /// Requires that user_role has been loaded into context via `with_user_role()`.
    pub fn has_role(
        ctx: &RequestContext,
        allowed: &[HackathonRoleType],
    ) -> Result<bool, ServerFnError> {
        let role = ctx.user_role()?;
        let role_str = role.role.as_str();

        Ok(HackathonRoleType::from_str(role_str)
            .map(|r| allowed.contains(&r))
            .unwrap_or(false))
    }

    /// Require that the user has one of the specified roles
    ///
    /// Requires that user_role has been loaded into context via `with_user_role()`.
    pub fn require_role(
        ctx: &RequestContext,
        allowed: &[HackathonRoleType],
    ) -> Result<(), ServerFnError> {
        if Self::has_role(ctx, allowed)? {
            Ok(())
        } else {
            Err(ServerFnError::new("Insufficient permissions"))
        }
    }

    // Resource Ownership Checks

    /// Check if the user is the recipient of a team invitation
    pub async fn is_invitation_recipient(
        ctx: &RequestContext,
        invitation_id: i32,
    ) -> Result<bool, ServerFnError> {
        let invitation = TeamInvitations::find_by_id(invitation_id)
            .one(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch invitation: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Invitation not found"))?;

        Ok(invitation.user_id == ctx.user.id)
    }

    /// Require that the user is the recipient of a team invitation
    pub async fn require_invitation_ownership(
        ctx: &RequestContext,
        invitation_id: i32,
    ) -> Result<(), ServerFnError> {
        if Self::is_invitation_recipient(ctx, invitation_id).await? {
            Ok(())
        } else {
            Err(ServerFnError::new("This invitation is not for you"))
        }
    }

    /// Check if the user owns a join request
    pub async fn is_request_owner(
        ctx: &RequestContext,
        request_id: i32,
    ) -> Result<bool, ServerFnError> {
        let request = TeamJoinRequests::find_by_id(request_id)
            .one(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch join request: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Join request not found"))?;

        Ok(request.user_id == ctx.user.id)
    }

    /// Require that the user owns the join request
    pub async fn require_request_ownership(
        ctx: &RequestContext,
        request_id: i32,
    ) -> Result<(), ServerFnError> {
        if Self::is_request_owner(ctx, request_id).await? {
            Ok(())
        } else {
            Err(ServerFnError::new("You can only cancel your own requests"))
        }
    }

    // Team Request Validation (owner receiving requests)

    /// Verify that a join request is for a team owned by the current user
    ///
    /// Returns the join request if valid, otherwise returns an error.
    pub async fn require_team_request_ownership(
        ctx: &RequestContext,
        request_id: i32,
        team_id: i32,
    ) -> Result<(), ServerFnError> {
        // First verify the user owns the team
        Self::require_team_ownership(ctx, team_id).await?;

        // Then verify the request is for this team
        let request = TeamJoinRequests::find_by_id(request_id)
            .one(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch join request: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Join request not found"))?;

        if request.team_id == team_id {
            Ok(())
        } else {
            Err(ServerFnError::new("This request is not for your team"))
        }
    }
}
