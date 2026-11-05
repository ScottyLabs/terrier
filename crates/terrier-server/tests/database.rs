use std::collections::BTreeSet;

use anyhow::{Context, ensure};
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend,
    EntityTrait, PaginatorTrait, Set, Statement,
};
use terrier_server::{
    auth::user::{OidcIdentity, sync_user},
    entities::user,
    state::AppState,
};
use uuid::Uuid;

struct TestDatabase {
    admin: DatabaseConnection,
    state: AppState,
    schema: String,
}

impl TestDatabase {
    async fn new() -> anyhow::Result<Self> {
        let url = std::env::var("TEST_DATABASE_URL").context("TEST_DATABASE_URL must be set")?;
        let admin = Database::connect(url.clone()).await?;
        let schema = format!("terrier_test_{}", Uuid::new_v4().simple());
        admin
            .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
            .await?;
        let mut options = ConnectOptions::new(url);
        options.set_schema_search_path(&schema);
        let state = AppState {
            db: Database::connect(options).await?,
        };
        Ok(Self {
            admin,
            state,
            schema,
        })
    }

    async fn close(self) -> anyhow::Result<()> {
        self.state.db.close().await?;
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .await?;
        self.admin.close().await?;
        Ok(())
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn concurrent_startups_apply_migrations_once() -> anyhow::Result<()> {
    let database = TestDatabase::new().await?;
    let url = std::env::var("TEST_DATABASE_URL")?;
    let mut options = ConnectOptions::new(url);
    options.set_schema_search_path(&database.schema);
    let (first, second) = tokio::join!(
        AppState::connect(options.clone()),
        AppState::connect(options.clone())
    );
    first?.db.close().await?;
    second?.db.close().await?;
    ensure!(user::Entity::find().count(&database.state.db).await? == 0);
    ensure!(
        Migrator::get_applied_migrations(&database.state.db)
            .await?
            .len()
            == 2
    );
    AppState::connect(options).await?.db.close().await?;
    database.close().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn failed_startup_rolls_back_pending_migrations() -> anyhow::Result<()> {
    let database = TestDatabase::new().await?;
    database
        .state
        .db
        .execute_unprepared("CREATE TYPE application_status AS ENUM ('pending')")
        .await?;
    let mut options = ConnectOptions::new(std::env::var("TEST_DATABASE_URL")?);
    options.set_schema_search_path(&database.schema);
    let error = AppState::connect(options)
        .await
        .err()
        .context("startup accepted a conflicting schema")?;
    ensure!(
        error
            .to_string()
            .contains("failed to apply database migrations")
    );
    let row = database
        .state
        .db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT to_regtype('hackathon_role') IS NULL AS rolled_back",
        ))
        .await?
        .context("failed to inspect migration rollback")?;
    ensure!(row.try_get::<bool>("", "rolled_back")?);
    database.close().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn concurrent_logins_preserve_identity_and_local_profile() -> anyhow::Result<()> {
    let database = TestDatabase::new().await?;
    let db = &database.state.db;
    Migrator::up(db, None).await?;

    let mut identity = OidcIdentity {
        issuer: "https://keycloak.example/realms/terrier",
        subject: "not-a-uuid",
        email: "user@example.com",
        display_name: "Initial name",
    };
    let (first, second) = tokio::join!(sync_user(db, &identity), sync_user(db, &identity));
    let first = first?;
    let second = second?;
    ensure!(
        first.id == second.id,
        "concurrent logins created different users"
    );

    user::ActiveModel {
        id: Set(first.id),
        display_name: Set("Local name".into()),
        ..Default::default()
    }
    .update(db)
    .await?;
    identity.email = "changed@example.com";
    identity.display_name = "Changed provider name";
    let updated = sync_user(db, &identity).await?;
    ensure!(updated.id == first.id, "login replaced the local UUID");
    ensure!(
        updated.email == identity.email,
        "email was not synchronized"
    );
    ensure!(
        updated.display_name == "Local name",
        "login replaced the local name"
    );

    identity.subject = "another-subject";
    let same_email = sync_user(db, &identity).await?;
    identity.subject = "not-a-uuid";
    identity.issuer = "https://another-issuer.example";
    let same_subject = sync_user(db, &identity).await?;
    ensure!(
        BTreeSet::from([first.id, same_email.id, same_subject.id]).len() == 3,
        "different OIDC identities were merged"
    );
    ensure!(
        user::Entity::find().count(db).await? == 3,
        "unexpected user count"
    );

    database.close().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn migration_rejects_unmapped_users_without_changing_them() -> anyhow::Result<()> {
    let database = TestDatabase::new().await?;
    let db = &database.state.db;
    Migrator::up(db, Some(1)).await?;
    let id = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"INSERT INTO "user" (id, email, display_name, is_global_admin)
            VALUES ($1, 'existing@example.com', 'Existing user', true)"#,
        [id.into()],
    ))
    .await?;

    let mut options = ConnectOptions::new(std::env::var("TEST_DATABASE_URL")?);
    options.set_schema_search_path(&database.schema);
    let error = AppState::connect(options)
        .await
        .err()
        .context("startup accepted an unmapped user")?;
    ensure!(format!("{error:#}").contains("explicit issuer/subject mapping"));
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT display_name, is_global_admin FROM "user" WHERE id = $1"#,
            [id.into()],
        ))
        .await?
        .context("existing user was removed")?;
    ensure!(row.try_get::<String>("", "display_name")? == "Existing user");
    ensure!(row.try_get::<bool>("", "is_global_admin")?);

    database.close().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn missing_claims_do_not_create_users() -> anyhow::Result<()> {
    let database = TestDatabase::new().await?;
    let db = &database.state.db;
    Migrator::up(db, None).await?;
    for (issuer, subject, email, display_name) in [
        ("", "subject", "user@example.com", "Name"),
        ("issuer", " ", "user@example.com", "Name"),
        ("issuer", "subject", "", "Name"),
        ("issuer", "subject", "user@example.com", ""),
    ] {
        let identity = OidcIdentity {
            issuer,
            subject,
            email,
            display_name,
        };
        ensure!(sync_user(db, &identity).await.is_err());
    }
    ensure!(user::Entity::find().count(db).await? == 0);
    database.close().await
}
