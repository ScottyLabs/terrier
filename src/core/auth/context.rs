use dioxus::fullstack::{FullstackContext, extract::State};
use dioxus::prelude::ServerFnError;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;

use crate::{
    AppState,
    entities::{hackathons, prelude::*, user_hackathon_roles, users},
};

use super::middleware::SyncedUser;

/// Request context that provides access to commonly needed data in handlers
#[derive(Clone)]
pub struct RequestContext {
    pub state: AppState,
    pub user: Arc<users::Model>,
    pub hackathon: Option<hackathons::Model>,
    pub user_role: Option<user_hackathon_roles::Model>,
}

impl RequestContext {
    /// Extract the state and user
    pub async fn extract(user: &SyncedUser) -> Result<Self, ServerFnError> {
        // Extract state from Dioxus fullstack context
        let State(state) = FullstackContext::extract::<State<AppState>, _>()
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

        Ok(Self {
            state,
            user: user.0.clone(),
            hackathon: None,
            user_role: None,
        })
    }

    /// Add hackathon to context by slug
    pub async fn with_hackathon(mut self, slug: &str) -> Result<Self, ServerFnError> {
        let hackathon = Hackathons::find()
            .filter(hackathons::Column::Slug.eq(slug))
            .one(&self.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

        self.hackathon = Some(hackathon);
        Ok(self)
    }

    /// Add user's role in the current hackathon to context
    ///
    /// Requires that hackathon has already been added via `with_hackathon()`.
    pub async fn with_user_role(mut self) -> Result<Self, ServerFnError> {
        let hackathon = self
            .hackathon
            .as_ref()
            .ok_or_else(|| ServerFnError::new("Hackathon must be loaded before user role"))?;

        let user_role = UserHackathonRoles::find()
            .filter(user_hackathon_roles::Column::UserId.eq(self.user.id))
            .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
            .one(&self.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?;

        self.user_role = user_role;
        Ok(self)
    }

    /// Get the user's team ID from their role, if they're in a team
    ///
    /// Returns `None` if the user role hasn't been loaded or if the user isn't in a team.
    pub fn team_id(&self) -> Option<i32> {
        self.user_role.as_ref().and_then(|role| role.team_id)
    }

    /// Get the hackathon, returning an error if not loaded
    pub fn hackathon(&self) -> Result<&hackathons::Model, ServerFnError> {
        self.hackathon
            .as_ref()
            .ok_or_else(|| ServerFnError::new("Hackathon not loaded in context"))
    }

    /// Get the user role, returning an error if not loaded
    pub fn user_role(&self) -> Result<&user_hackathon_roles::Model, ServerFnError> {
        self.user_role
            .as_ref()
            .ok_or_else(|| ServerFnError::new("User role not loaded in context"))
    }

    /// Require that the user has a role (is registered for the hackathon)
    ///
    /// Returns an error if the user role hasn't been loaded or doesn't exist.
    pub fn require_registered(&self) -> Result<&user_hackathon_roles::Model, ServerFnError> {
        self.user_role()
    }
}
