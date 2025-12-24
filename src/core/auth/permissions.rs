#[cfg(feature = "server")]
use crate::auth::{HackathonRoleType, has_role};
#[cfg(feature = "server")]
use crate::core::{auth::context::RequestContext, errors::*, types::*};
use dioxus::prelude::ServerFnError;

#[cfg(feature = "server")]
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

/// Permission checking utilities
pub struct Permissions;

#[cfg(feature = "server")]
impl Permissions {
    /// Require that the user is either a global admin or an organizer for the current hackathon
    pub async fn require_admin_or_organizer(ctx: &RequestContext) -> Result<(), ServerFnError> {
        if ctx.is_global_admin() {
            return Ok(());
        }

        let role = ctx.require_user_role()?;
        if has_role(
            &role.role,
            &[HackathonRoleType::Admin, HackathonRoleType::Organizer],
        ) {
            return Ok(());
        }

        Err(ServerFnError::new("Insufficient permissions"))
    }

    /// Require that the user is a global admin or hackathon admin
    pub async fn require_admin(ctx: &RequestContext) -> Result<(), ServerFnError> {
        if ctx.is_global_admin() {
            return Ok(());
        }

        let role = ctx.require_user_role()?;
        if has_role(&role.role, &[HackathonRoleType::Admin]) {
            return Ok(());
        }

        Err(ServerFnError::new("Admin permissions required"))
    }

    /// Check if the user has any of the specified roles
    pub fn has_any_role(ctx: &RequestContext, allowed_roles: &[HackathonRoleType]) -> bool {
        if ctx.is_global_admin() {
            return true;
        }

        if let Some(role) = &ctx.user_role {
            return has_role(&role.role, allowed_roles);
        }

        false
    }

    /// Require that the user owns the specified team
    /// Returns the team if the user is the owner
    pub async fn require_team_ownership(
        ctx: &RequestContext,
        team_id: i32,
    ) -> Result<TeamEntity, ServerFnError> {
        let team = crate::entities::prelude::Teams::find_by_id(team_id)
            .one(&ctx.state.db)
            .await
            .to_server_error("Failed to fetch team")?
            .ok_or_server_error("Team not found")?;

        if team.owner_id != ctx.user.0.id {
            return Err(ServerFnError::new(
                "You must be the team owner to perform this action",
            ));
        }

        Ok(team)
    }

    /// Require that the user is a member of the specified team
    /// Returns true if the user is a member
    pub async fn is_team_member(ctx: &RequestContext, team_id: i32) -> Result<bool, ServerFnError> {
        use crate::entities::team_members;

        let member = team_members::Entity::find()
            .filter(team_members::Column::TeamId.eq(team_id))
            .filter(team_members::Column::UserId.eq(ctx.user.0.id))
            .filter(team_members::Column::Status.eq("approved"))
            .one(&ctx.state.db)
            .await
            .to_server_error("Failed to check team membership")?;

        Ok(member.is_some())
    }

    /// Require that the user is a member of the specified team
    pub async fn require_team_membership(
        ctx: &RequestContext,
        team_id: i32,
    ) -> Result<(), ServerFnError> {
        if !Self::is_team_member(ctx, team_id).await? {
            return Err(ServerFnError::new(
                "You must be a team member to perform this action",
            ));
        }

        Ok(())
    }
}
