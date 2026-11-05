use anyhow::Context;
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait, QuerySelect,
    TransactionTrait,
};

use crate::entities::user;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}

impl AppState {
    pub async fn connect(options: impl Into<ConnectOptions>) -> anyhow::Result<Self> {
        let db = Database::connect(options)
            .await
            .context("failed to connect to PostgreSQL")?;
        let transaction = db.begin().await.context("failed to begin migrations")?;
        transaction
            .execute_unprepared("SELECT pg_advisory_xact_lock(hashtext('terrier:migrations'))")
            .await
            .context("failed to acquire migration lock")?;
        Migrator::up(&transaction, None)
            .await
            .context("failed to apply database migrations")?;
        transaction
            .commit()
            .await
            .context("failed to commit database migrations")?;
        user::Entity::find()
            .limit(0)
            .all(&db)
            .await
            .context("database schema is incompatible after migrations")?;
        Ok(Self { db })
    }
}
