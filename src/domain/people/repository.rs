#[cfg(feature = "server")]
use crate::entities::{prelude::*, user_hackathon_roles, users};
#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

#[cfg(feature = "server")]
pub struct UserRoleRepository<'a> {
    db: &'a DatabaseConnection,
}

#[cfg(feature = "server")]
impl<'a> UserRoleRepository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// Find a user's role for a specific hackathon
    pub async fn find_user_role(
        &self,
        user_id: i32,
        hackathon_id: i32,
    ) -> Result<Option<user_hackathon_roles::Model>, ServerFnError> {
        UserHackathonRoles::find()
            .filter(user_hackathon_roles::Column::UserId.eq(user_id))
            .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon_id))
            .one(self.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))
    }

    /// Find a user's role for a specific hackathon, or return an error if not found
    pub async fn find_user_role_or_error(
        &self,
        user_id: i32,
        hackathon_id: i32,
        error_msg: &str,
    ) -> Result<user_hackathon_roles::Model, ServerFnError> {
        self.find_user_role(user_id, hackathon_id)
            .await?
            .ok_or_else(|| ServerFnError::new(error_msg))
    }

    /// Check if a user has any of the specified roles for a hackathon
    pub async fn has_role(
        &self,
        user_id: i32,
        hackathon_id: i32,
        allowed_roles: &[&str],
    ) -> Result<bool, ServerFnError> {
        let role = self.find_user_role(user_id, hackathon_id).await?;

        Ok(role
            .as_ref()
            .map(|r| allowed_roles.contains(&r.role.as_str()))
            .unwrap_or(false))
    }

    /// Get all users with roles for a hackathon, excluding specific roles
    pub async fn find_all_roles_for_hackathon(
        &self,
        hackathon_id: i32,
    ) -> Result<Vec<(user_hackathon_roles::Model, Option<users::Model>)>, ServerFnError> {
        UserHackathonRoles::find()
            .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon_id))
            .find_also_related(Users)
            .all(self.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch roles: {}", e)))
    }

    /// Get all users with roles for a hackathon, excluding specific roles
    pub async fn find_all_roles_for_hackathon_excluding_role(
        &self,
        hackathon_id: i32,
        excluded_role: &str,
    ) -> Result<Vec<(user_hackathon_roles::Model, Option<users::Model>)>, ServerFnError> {
        UserHackathonRoles::find()
            .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon_id))
            .filter(user_hackathon_roles::Column::Role.ne(excluded_role))
            .find_also_related(Users)
            .all(self.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch roles: {}", e)))
    }

    /// Check if user is admin or organizer for a hackathon
    pub async fn is_admin_or_organizer(
        &self,
        user_id: i32,
        hackathon_id: i32,
    ) -> Result<bool, ServerFnError> {
        self.has_role(user_id, hackathon_id, &["admin", "organizer"])
            .await
    }

    /// Check if user is admin for a hackathon
    pub async fn is_admin(&self, user_id: i32, hackathon_id: i32) -> Result<bool, ServerFnError> {
        self.has_role(user_id, hackathon_id, &["admin"]).await
    }
}
