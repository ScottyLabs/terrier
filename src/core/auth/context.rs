#[cfg(feature = "server")]
use crate::AppState;
use crate::auth::middleware::SyncedUser;
use crate::core::{errors::*, types::*};
use dioxus::prelude::ServerFnError;

#[cfg(feature = "server")]
use dioxus::fullstack::{FullstackContext, extract::State};
#[cfg(feature = "server")]
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

/// Request context that bundles commonly needed data for server functions
#[cfg(feature = "server")]
#[derive(Clone)]
pub struct RequestContext {
    pub state: AppState,
    pub user: SyncedUser,
    pub hackathon: Option<HackathonEntity>,
    pub user_role: Option<UserHackathonRoleEntity>,
}

#[cfg(feature = "server")]
impl RequestContext {
    /// Extract the base context (state and user)
    pub async fn extract() -> Result<Self, ServerFnError> {
        let State(state) = FullstackContext::extract::<State<AppState>, _>()
            .await
            .to_server_error("Failed to extract state")?;

        let user = FullstackContext::extract::<SyncedUser, _>()
            .await
            .to_server_error("Failed to extract user")?;

        Ok(Self {
            state,
            user,
            hackathon: None,
            user_role: None,
        })
    }

    /// Fetch and attach hackathon by slug
    pub async fn with_hackathon(mut self, slug: String) -> Result<Self, ServerFnError> {
        let hackathon = crate::entities::prelude::Hackathons::find()
            .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
            .one(&self.state.db)
            .await
            .to_server_error("Failed to fetch hackathon")?
            .ok_or_server_error("Hackathon not found")?;

        self.hackathon = Some(hackathon);
        Ok(self)
    }

    /// Fetch and attach user's role for the current hackathon
    /// Requires hackathon to be set first
    pub async fn with_user_role(mut self) -> Result<Self, ServerFnError> {
        let hackathon = self
            .hackathon
            .as_ref()
            .ok_or_server_error("Hackathon not set in context")?;

        // Check if user is global admin first
        if self.is_global_admin() {
            // Create a synthetic admin role for global admins
            self.user_role = Some(UserHackathonRoleEntity {
                id: 0, // Special ID for synthetic roles
                user_id: self.user.0.id,
                hackathon_id: hackathon.id,
                role: "admin".to_string(),
                team_id: None,
            });
            return Ok(self);
        }

        // Look up role in database
        let role = crate::entities::prelude::UserHackathonRoles::find()
            .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(self.user.0.id))
            .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
            .one(&self.state.db)
            .await
            .to_server_error("Failed to fetch user role")?;

        self.user_role = role;
        Ok(self)
    }

    /// Check if the current user is a global admin
    pub fn is_global_admin(&self) -> bool {
        self.state
            .config
            .admin_emails
            .contains(&self.user.0.email.to_lowercase())
    }

    /// Get the hackathon, or return an error if not set
    pub fn require_hackathon(&self) -> Result<&HackathonEntity, ServerFnError> {
        self.hackathon
            .as_ref()
            .ok_or_server_error("Hackathon not set in context")
    }

    /// Get the user role, or return an error if not set
    pub fn require_user_role(&self) -> Result<&UserHackathonRoleEntity, ServerFnError> {
        self.user_role
            .as_ref()
            .ok_or_server_error("User role not set in context")
    }
}
