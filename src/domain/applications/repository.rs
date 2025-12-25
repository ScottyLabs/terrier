#[cfg(feature = "server")]
use crate::entities::{applications, prelude::*};
#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

#[cfg(feature = "server")]
use crate::domain::applications::types::ApplicationWithUser;

#[cfg(feature = "server")]
pub struct ApplicationRepository<'a> {
    db: &'a DatabaseConnection,
}

#[cfg(feature = "server")]
impl<'a> ApplicationRepository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// Find an application by ID
    pub async fn find_by_id(&self, id: i32) -> Result<applications::Model, ServerFnError> {
        Applications::find_by_id(id)
            .one(self.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch application: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Application not found"))
    }

    /// Find an application by user and hackathon
    pub async fn find_by_user_and_hackathon(
        &self,
        user_id: i32,
        hackathon_id: i32,
    ) -> Result<Option<applications::Model>, ServerFnError> {
        Applications::find()
            .filter(applications::Column::UserId.eq(user_id))
            .filter(applications::Column::HackathonId.eq(hackathon_id))
            .one(self.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch application: {}", e)))
    }

    /// Find an application by user and hackathon, returning an error if not found
    pub async fn find_by_user_and_hackathon_or_error(
        &self,
        user_id: i32,
        hackathon_id: i32,
        error_msg: &str,
    ) -> Result<applications::Model, ServerFnError> {
        self.find_by_user_and_hackathon(user_id, hackathon_id)
            .await?
            .ok_or_else(|| ServerFnError::new(error_msg))
    }

    /// Get all applications for a hackathon
    pub async fn find_all_by_hackathon(
        &self,
        hackathon_id: i32,
    ) -> Result<Vec<applications::Model>, ServerFnError> {
        Applications::find()
            .filter(applications::Column::HackathonId.eq(hackathon_id))
            .all(self.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch applications: {}", e)))
    }

    /// Get all applications with user data for a hackathon
    pub async fn find_all_with_users_by_hackathon(
        &self,
        hackathon_id: i32,
    ) -> Result<Vec<ApplicationWithUser>, ServerFnError> {
        let applications = Applications::find()
            .filter(applications::Column::HackathonId.eq(hackathon_id))
            .find_also_related(Users)
            .all(self.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch applications: {}", e)))?;

        let results = applications
            .into_iter()
            .filter_map(|(app, user_opt)| {
                user_opt.map(|user| ApplicationWithUser {
                    id: app.id,
                    user_id: app.user_id,
                    user_name: user.name,
                    user_email: user.email,
                    user_picture: user.picture,
                    form_data: app.form_data,
                    status: app.status,
                    created_at: app.created_at.to_string(),
                    updated_at: app.updated_at.to_string(),
                })
            })
            .collect();

        Ok(results)
    }

    /// Get all applications with a specific status for a hackathon
    pub async fn find_all_by_hackathon_and_status(
        &self,
        hackathon_id: i32,
        status: &str,
    ) -> Result<Vec<applications::Model>, ServerFnError> {
        Applications::find()
            .filter(applications::Column::HackathonId.eq(hackathon_id))
            .filter(applications::Column::Status.eq(status))
            .all(self.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch applications: {}", e)))
    }

    /// Get all applications with user data and a specific status for a hackathon
    pub async fn find_all_with_users_by_hackathon_and_status(
        &self,
        hackathon_id: i32,
        status: &str,
    ) -> Result<Vec<ApplicationWithUser>, ServerFnError> {
        let applications = Applications::find()
            .filter(applications::Column::HackathonId.eq(hackathon_id))
            .filter(applications::Column::Status.eq(status))
            .find_also_related(Users)
            .all(self.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch applications: {}", e)))?;

        let results = applications
            .into_iter()
            .filter_map(|(app, user_opt)| {
                user_opt.map(|user| ApplicationWithUser {
                    id: app.id,
                    user_id: app.user_id,
                    user_name: user.name,
                    user_email: user.email,
                    user_picture: user.picture,
                    form_data: app.form_data,
                    status: app.status.clone(),
                    created_at: app.created_at.to_string(),
                    updated_at: app.updated_at.to_string(),
                })
            })
            .collect();

        Ok(results)
    }

    /// Find applications by IDs for a specific hackathon
    pub async fn find_by_ids_and_hackathon(
        &self,
        application_ids: Vec<i32>,
        hackathon_id: i32,
    ) -> Result<Vec<applications::Model>, ServerFnError> {
        Applications::find()
            .filter(applications::Column::Id.is_in(application_ids))
            .filter(applications::Column::HackathonId.eq(hackathon_id))
            .all(self.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch applications: {}", e)))
    }
}
