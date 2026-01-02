use crate::core::errors::*;
use dioxus::prelude::ServerFnError;
use sea_orm::{DatabaseConnection, EntityTrait};

/// Generic repository helper for database operations
pub struct Repository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> Repository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// Get the database connection
    pub fn db(&self) -> &DatabaseConnection {
        self.db
    }

    /// Find entity by ID with proper error handling
    pub async fn find_by_id<E>(&self, id: i32) -> Result<E::Model, ServerFnError>
    where
        E: EntityTrait,
        <E::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<i32>,
    {
        E::find_by_id(id)
            .one(self.db)
            .await
            .to_server_error("Failed to fetch entity")?
            .ok_or_server_error("Entity not found")
    }

    /// Find one entity by filter
    pub async fn find_one<E, F>(&self, filter: F) -> Result<Option<E::Model>, ServerFnError>
    where
        E: EntityTrait,
        F: FnOnce(sea_orm::Select<E>) -> sea_orm::Select<E>,
    {
        filter(E::find())
            .one(self.db)
            .await
            .to_server_error("Failed to fetch entity")
    }

    /// Find one entity by filter, return error if not found
    pub async fn find_one_or_error<E, F>(
        &self,
        filter: F,
        error_msg: &str,
    ) -> Result<E::Model, ServerFnError>
    where
        E: EntityTrait,
        F: FnOnce(sea_orm::Select<E>) -> sea_orm::Select<E>,
    {
        self.find_one::<E, F>(filter)
            .await?
            .ok_or_server_error(error_msg)
    }

    /// Find all entities by filter
    pub async fn find_all<E, F>(&self, filter: F) -> Result<Vec<E::Model>, ServerFnError>
    where
        E: EntityTrait,
        F: FnOnce(sea_orm::Select<E>) -> sea_orm::Select<E>,
    {
        filter(E::find())
            .all(self.db)
            .await
            .to_server_error("Failed to fetch entities")
    }
}
