use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DbBackend, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let row = db
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                "SELECT EXISTS (SELECT 1 FROM \"user\") AS has_users",
            ))
            .await?
            .ok_or_else(|| DbErr::Custom("failed to inspect existing users".into()))?;
        if row.try_get::<bool>("", "has_users")? {
            return Err(DbErr::Custom(
                "existing users need an explicit issuer/subject mapping before the OIDC migration"
                    .into(),
            ));
        }

        db.execute_unprepared(
            r#"ALTER TABLE "user"
                ADD COLUMN oidc_issuer text NOT NULL,
                ADD COLUMN oidc_subject text NOT NULL,
                ADD CONSTRAINT user_oidc_identity_key UNIQUE (oidc_issuer, oidc_subject),
                DROP COLUMN is_global_admin"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "OIDC identity migration cannot be reversed without losing identity mappings".into(),
        ))
    }
}
