use anyhow::{Context, ensure};
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait,
    QuerySelect, Statement, TransactionTrait,
};

use crate::entities::user;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}

impl AppState {
    pub async fn connect(
        runtime_options: impl Into<ConnectOptions>,
        migration_options: impl Into<ConnectOptions>,
    ) -> anyhow::Result<Self> {
        let db = Database::connect(runtime_options)
            .await
            .context("connect runtime PostgreSQL role")?;
        let migrations = Database::connect(migration_options)
            .await
            .context("connect migration PostgreSQL role")?;
        let probe = db
            .begin()
            .await
            .context("inspect runtime database target")?;
        let nonce = uuid::Uuid::new_v4().to_string();
        let runtime_target = probe
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT pg_backend_pid() AS pid, set_config('application_name',$1,true)",
                [nonce.clone().into()],
            ))
            .await?
            .context("identify runtime database connection")?;
        let pid: i32 = runtime_target.try_get("", "pid")?;
        let same_database = migrations.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND application_name=$2 AND datname=current_database()) AS matches",
            [pid.into(),nonce.into()])).await?.context("compare database targets")?;
        ensure!(
            same_database.try_get::<bool>("", "matches")?,
            "DATABASE_URL and MIGRATION_DATABASE_URL must target the same PostgreSQL database"
        );
        probe.rollback().await?;
        let runtime = db.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT current_user AS name, rolsuper OR rolcreaterole AS privileged FROM pg_roles WHERE rolname=current_user"))
            .await?.context("inspect runtime database role")?;
        let runtime_name: String = runtime.try_get("", "name")?;
        ensure!(
            !runtime.try_get::<bool>("", "privileged")?,
            "DATABASE_URL must use a non-superuser role without CREATEROLE"
        );
        let owner = migrations
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT current_user AS name",
            ))
            .await?
            .context("inspect migration database role")?;
        let owner_name: String = owner.try_get("", "name")?;
        let membership = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT pg_has_role(current_user, $1, 'MEMBER') AS inherited",
                [owner_name.into()],
            ))
            .await?
            .context("inspect runtime role membership")?;
        ensure!(
            !membership.try_get::<bool>("", "inherited")?,
            "Runtime role must not own or inherit the migration role"
        );
        let transaction = migrations.begin().await.context("begin migrations")?;
        transaction
            .execute_unprepared("SELECT pg_advisory_xact_lock(hashtext('terrier:migrations'))")
            .await
            .context("acquire migration lock")?;
        Migrator::up(&transaction, None)
            .await
            .context("apply database migrations")?;
        let row = transaction
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT current_schema() AS name",
            ))
            .await?
            .context("inspect migrated schema")?;
        let schema: String = row.try_get("", "name")?;
        let quoted_schema = quote_identifier(&schema);
        let role = quote_identifier(&runtime_name);
        transaction
            .execute_unprepared(&format!("GRANT USAGE ON SCHEMA {quoted_schema} TO {role}"))
            .await?;
        for table in [
            "user",
            "hackathon",
            "invitation",
            "user_hackathon_role",
            "application",
            "reimbursement_application",
            "team",
            "team_member",
            "project",
            "sponsor_org",
            "prize_track",
            "project_track_sub",
            "judge_track_assign",
            "event",
            "event_check_in",
            "evaluation",
            "sponsor",
            "organizer",
        ] {
            transaction
                .execute_unprepared(&format!(
                    "GRANT SELECT, INSERT, UPDATE ON {quoted_schema}.{} TO {role}",
                    quote_identifier(table)
                ))
                .await?;
        }
        transaction
            .execute_unprepared(&format!(
                "GRANT SELECT ON {quoted_schema}.audit_event TO {role}"
            ))
            .await?;
        transaction
            .commit()
            .await
            .context("commit database migrations")?;
        migrations
            .close()
            .await
            .context("close migration connection pool")?;
        let runtime_schema = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT current_schema() AS name",
            ))
            .await?
            .context("inspect runtime schema")?;
        ensure!(
            runtime_schema.try_get::<String>("", "name")? == schema,
            "Runtime and migration logins must use the same schema search path"
        );
        let privileges = db.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT has_table_privilege(current_user, 'audit_event', 'INSERT,UPDATE,DELETE,TRUNCATE')
                OR has_schema_privilege(current_user,current_schema(),'CREATE')
                OR pg_has_role(current_user,(SELECT relowner FROM pg_class WHERE oid='audit_event'::regclass),'MEMBER') AS unsafe"))
            .await?.context("inspect audit storage privileges")?;
        ensure!(
            !privileges.try_get::<bool>("", "unsafe")?,
            "Runtime role can modify audit storage or its schema; remove ownership, role membership and write/CREATE grants"
        );
        user::Entity::find()
            .limit(0)
            .all(&db)
            .await
            .context("database schema is incompatible after migrations")?;
        Ok(Self { db })
    }
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
