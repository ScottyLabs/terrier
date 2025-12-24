/// Base repository pattern for common database operations
/// Domain-specific repositories can use these helper functions
use crate::core::errors::*;
use dioxus::prelude::ServerFnError;
use sea_orm::{DatabaseConnection, EntityTrait};

/// Find an entity by ID with proper error handling
pub async fn find_by_id_or_error<E>(
    db: &DatabaseConnection,
    id: i32,
) -> Result<E::Model, ServerFnError>
where
    E: EntityTrait,
    <E::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<i32>,
{
    E::find_by_id(id)
        .one(db)
        .await
        .to_server_error("Failed to fetch entity")?
        .ok_or_server_error("Entity not found")
}
