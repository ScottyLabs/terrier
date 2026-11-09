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
            .context("connect PostgreSQL")?;
        let transaction = db.begin().await.context("begin migrations")?;
        transaction
            .execute_unprepared("SELECT pg_advisory_xact_lock(hashtext('terrier:migrations'))")
            .await
            .context("acquire migration lock")?;
        Migrator::up(&transaction, None)
            .await
            .context("apply database migrations")?;
        transaction
            .commit()
            .await
            .context("commit database migrations")?;
        user::Entity::find()
            .limit(0)
            .all(&db)
            .await
            .context("database schema is incompatible after migrations")?;
        Ok(Self { db })
    }
}
