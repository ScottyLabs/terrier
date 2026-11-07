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
    runtime_options: ConnectOptions,
    runtime_role: String,
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
        let runtime_role = format!("runtime_{}", Uuid::new_v4().simple());
        let password = Uuid::new_v4().simple().to_string();
        admin
            .execute_unprepared(&format!(
                "CREATE ROLE {runtime_role} LOGIN PASSWORD '{password}'"
            ))
            .await?;
        let mut runtime_url = url::Url::parse(&std::env::var("TEST_DATABASE_URL")?)?;
        runtime_url
            .query_pairs_mut()
            .append_pair("user", &runtime_role)
            .append_pair("password", &password);
        let mut runtime_options = ConnectOptions::new(runtime_url.to_string());
        runtime_options.set_schema_search_path(&schema);
        runtime_options.max_connections(1);
        Ok(Self {
            runtime_options,
            runtime_role,
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
        self.admin
            .execute_unprepared(&format!("DROP ROLE {}", self.runtime_role))
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
        AppState::connect(database.runtime_options.clone(), options.clone()),
        AppState::connect(database.runtime_options.clone(), options.clone())
    );
    first?.db.close().await?;
    second?.db.close().await?;
    ensure!(user::Entity::find().count(&database.state.db).await? == 0);
    ensure!(
        Migrator::get_applied_migrations(&database.state.db)
            .await?
            .len()
            == 4
    );
    AppState::connect(database.runtime_options.clone(), options)
        .await?
        .db
        .close()
        .await?;
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
    let error = AppState::connect(database.runtime_options.clone(), options)
        .await
        .err()
        .context("startup accepted a conflicting schema")?;
    ensure!(error.to_string().contains("apply database migrations"));
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

    let first_id = first.id;
    terrier_server::audit::automatic(
        db,
        terrier_server::audit::Scope::Global,
        "Edit test display name",
        |tx| {
            Box::pin(async move {
                user::ActiveModel {
                    id: Set(first_id),
                    display_name: Set("Local name".into()),
                    ..Default::default()
                }
                .update(tx)
                .await
                .map_err(|error| terrier_server::error::ApiError::Internal(error.into()))
            })
        },
    )
    .await
    .map_err(|error| anyhow::anyhow!("{error:?}"))?;
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
    let error = AppState::connect(database.runtime_options.clone(), options)
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

async fn mutate(
    db: &DatabaseConnection,
    scope: terrier_server::audit::Scope,
    sql: String,
) -> Result<(), terrier_server::error::ApiError> {
    terrier_server::audit::automatic(db, scope, "Integration test fixture", |tx| {
        Box::pin(async move {
            tx.execute_unprepared(&sql)
                .await
                .map_err(|error| terrier_server::error::ApiError::Internal(error.into()))?;
            Ok(())
        })
    })
    .await
}

async fn scalar(db: &DatabaseConnection, sql: &str) -> anyhow::Result<i64> {
    Ok(db
        .query_one(Statement::from_string(DbBackend::Postgres, sql))
        .await?
        .context("missing scalar")?
        .try_get("", "value")?)
}

struct DomainFixture {
    hackathon: Uuid,
    team: Uuid,
    event: Uuid,
    users: [Uuid; 3],
}

impl DomainFixture {
    async fn new(db: &DatabaseConnection) -> anyhow::Result<Self> {
        use terrier_server::audit::Scope;
        let f = Self {
            hackathon: Uuid::new_v4(),
            team: Uuid::new_v4(),
            event: Uuid::new_v4(),
            users: [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()],
        };
        let mut sql = String::new();
        for id in f.users {
            sql.push_str(&format!("INSERT INTO \"user\"(id,email,display_name,oidc_issuer,oidc_subject) VALUES('{id}','fixture@example.com','Fixture','https://example.com','{id}');"));
        }
        sql.push_str(&format!("INSERT INTO hackathon(id,slug,name,description,is_active,max_team_size,max_participants,location,contact_email,application_schema,data,application_deadline,team_formation_deadline,submission_deadline) VALUES('{0}','{0}','Fixture','Fixture',true,2,100,'Campus','fixture@example.com','{{}}','{{}}',clock_timestamp()+interval '1 day',clock_timestamp()+interval '2 days',clock_timestamp()+interval '3 days');",f.hackathon));
        mutate(db, Scope::Global, sql)
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let mut sql = String::new();
        for id in f.users {
            sql.push_str(&format!("INSERT INTO user_hackathon_role(user_id,hackathon_id,role) VALUES('{id}','{}','hacker');",f.hackathon));
        }
        sql.push_str(&format!("INSERT INTO event(id,hackathon_id,name,description,start_time,end_time,location) VALUES('{0}','{1}','Expo','Expo',now(),now()+interval '1 hour','Campus'); INSERT INTO team(id,hackathon_id,name) VALUES('{2}','{1}','Team'); INSERT INTO team_member(team_id,user_id,hackathon_id,status,is_leader,requested_at,joined_at) VALUES('{2}','{3}','{1}','active',true,now(),now());",f.event,f.hackathon,f.team,f.users[0]));
        mutate(db, Scope::HackathonExclusive(f.hackathon), sql)
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(f)
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn rfc_schema_enforces_uniqueness_scope_and_atomic_audit() -> anyhow::Result<()> {
    use terrier_server::audit::Scope;
    let database = TestDatabase::new().await?;
    let db = &database.state.db;
    Migrator::up(db, None).await?;
    let f = DomainFixture::new(db).await?;
    let before = scalar(db, "SELECT count(*) AS value FROM audit_event").await?;
    let result=mutate(db,Scope::Hackathon(f.hackathon),format!("INSERT INTO application(id,user_id,hackathon_id,data) VALUES('{}','{}','{}','{{}}'),('{}','{}','{}','{{}}');",Uuid::new_v4(),f.users[0],f.hackathon,Uuid::new_v4(),f.users[0],f.hackathon)).await;
    ensure!(result.is_err(), "duplicate application accepted");
    ensure!(scalar(db, "SELECT count(*) AS value FROM application").await? == 0);
    ensure!(
        scalar(db, "SELECT count(*) AS value FROM audit_event").await? == before,
        "failed mutation left audit entries"
    );
    let other = DomainFixture::new(db).await?;
    ensure!(mutate(db,Scope::Hackathon(f.hackathon),format!("INSERT INTO project(id,hackathon_id,event_id,team_id,data) VALUES('{}','{}','{}','{}','{{}}');",Uuid::new_v4(),f.hackathon,other.event,f.team)).await.is_err(),"cross-hackathon event accepted");
    ensure!(
        mutate(
            db,
            Scope::Hackathon(other.hackathon),
            format!("UPDATE team SET name='Wrong scope' WHERE id='{}';", f.team)
        )
        .await
        .is_err(),
        "audit scope mismatch accepted"
    );
    ensure!(
        db.execute_unprepared(&format!(
            "UPDATE team SET name='Unaudited' WHERE id='{}'",
            f.team
        ))
        .await
        .is_err(),
        "write without audit context accepted"
    );
    for sql in [
        "UPDATE audit_event SET reason='changed'",
        "DELETE FROM audit_event",
        "TRUNCATE audit_event",
        "TRUNCATE \"user\" CASCADE",
    ] {
        ensure!(
            db.execute_unprepared(sql).await.is_err(),
            "mutable audit history"
        );
    }
    ensure!(
        mutate(
            db,
            Scope::Hackathon(f.hackathon),
            format!("DELETE FROM team WHERE id='{}'", f.team)
        )
        .await
        .is_err(),
        "physical domain deletion accepted"
    );
    database.close().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn concurrent_membership_changes_preserve_capacity_and_leadership() -> anyhow::Result<()> {
    use terrier_server::audit::Scope;
    let database = TestDatabase::new().await?;
    let db = &database.state.db;
    Migrator::up(db, None).await?;
    let f = DomainFixture::new(db).await?;
    let join = |id: Uuid| {
        mutate(
            db,
            Scope::Hackathon(f.hackathon),
            format!(
                "SELECT id FROM team WHERE id='{0}' FOR UPDATE; INSERT INTO team_member(team_id,user_id,hackathon_id,status,is_leader,requested_at,joined_at) VALUES('{0}','{1}','{2}','active',false,now(),now());",
                f.team, id, f.hackathon
            ),
        )
    };
    let (a, b) = tokio::join!(join(f.users[1]), join(f.users[2]));
    ensure!(a.is_ok() ^ b.is_ok(), "concurrent joins overfilled a team");
    ensure!(
        scalar(
            db,
            "SELECT count(*) AS value FROM team_member WHERE status='active'"
        )
        .await?
            == 2
    );
    ensure!(
        mutate(
            db,
            Scope::Hackathon(f.hackathon),
            format!(
                "UPDATE team_member SET is_leader=false WHERE team_id='{}'",
                f.team
            )
        )
        .await
        .is_err(),
        "nonempty team lost its leader"
    );
    ensure!(
        mutate(
            db,
            Scope::HackathonExclusive(f.hackathon),
            format!(
                "UPDATE hackathon SET max_team_size=1 WHERE id='{}'",
                f.hackathon
            )
        )
        .await
        .is_err(),
        "configuration accepted an oversized roster"
    );
    database.close().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn runtime_role_cannot_modify_audit_storage_or_become_its_owner() -> anyhow::Result<()> {
    let database = TestDatabase::new().await?;
    let mut owner = ConnectOptions::new(std::env::var("TEST_DATABASE_URL")?);
    owner.set_schema_search_path(&database.schema);
    let state = AppState::connect(database.runtime_options.clone(), owner.clone()).await?;
    for sql in [
        "UPDATE audit_event SET reason='changed'",
        "DELETE FROM audit_event",
        "TRUNCATE audit_event",
        "TRUNCATE \"user\" CASCADE",
        "ALTER TABLE audit_event DROP COLUMN reason",
        "CREATE TABLE bypass(id int)",
    ] {
        ensure!(
            state.db.execute_unprepared(sql).await.is_err(),
            "runtime role accepted {sql}"
        );
    }
    let user = sync_user(
        &state.db,
        &OidcIdentity {
            issuer: "https://example.com",
            subject: "runtime",
            email: "runtime@example.com",
            display_name: "Runtime",
        },
    )
    .await?;
    ensure!(user.version == 1);
    state
        .db
        .execute_unprepared(&format!(
            "CREATE TEMP TABLE audit_event AS SELECT * FROM {}.audit_event WITH NO DATA",
            database.schema
        ))
        .await?;

    sync_user(
        &state.db,
        &OidcIdentity {
            issuer: "https://example.com",
            subject: "runtime",
            email: "updated@example.com",
            display_name: "Runtime",
        },
    )
    .await?;
    ensure!(
        scalar(
            &state.db,
            "SELECT count(*) AS value FROM pg_temp.audit_event"
        )
        .await?
            == 0,
        "temporary table captured audit history"
    );
    state
        .db
        .execute_unprepared("DROP TABLE pg_temp.audit_event")
        .await?;
    ensure!(
        scalar(
            &state.db,
            "SELECT count(*) AS value FROM audit_event WHERE resource_type='user'"
        )
        .await?
            == 2,
        "upsert created phantom audit snapshots"
    );
    ensure!(scalar(&state.db,"SELECT count(*) AS value FROM audit_event WHERE resource_type='user' AND actor_user_id IS NULL AND action='automatic'").await?==2);
    ensure!(
        state
            .db
            .execute_unprepared("UPDATE \"user\" SET display_name='Unaudited'")
            .await
            .is_err(),
        "audit context leaked across transactions"
    );
    state.db.close().await?;
    ensure!(
        AppState::connect(owner.clone(), owner).await.is_err(),
        "server accepted schema-owner runtime credentials"
    );
    database.close().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn migration_rejects_unmapped_legacy_domain_data() -> anyhow::Result<()> {
    let database = TestDatabase::new().await?;
    let db = &database.state.db;
    Migrator::up(db, Some(2)).await?;
    db.execute_unprepared("INSERT INTO hackathon(id,slug,name,description,is_active,max_team_size,max_participants,location,contact_email,travel_reimbursement_enabled,application_schema,data) VALUES(gen_random_uuid(),'legacy','Legacy','Legacy',true,4,100,'Campus','legacy@example.com',false,'{}','{}')").await?;
    let tx = sea_orm::TransactionTrait::begin(db).await?;
    let failure = Migrator::up(&tx, None)
        .await
        .err()
        .context("migration guessed legacy workflow data")?;
    ensure!(
        failure
            .to_string()
            .contains("explicit legacy mapping for table hackathon")
    );
    tx.rollback().await?;
    ensure!(scalar(db, "SELECT count(*) AS value FROM hackathon").await? == 1);
    ensure!(scalar(db,"SELECT count(*) AS value FROM information_schema.columns WHERE table_schema=current_schema() AND table_name='hackathon' AND column_name='submission_deadline'").await?==0);
    database.close().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn submission_snapshots_validate_identity_content_and_selected_tracks() -> anyhow::Result<()>
{
    use terrier_server::audit::Scope;
    let database = TestDatabase::new().await?;
    let db = &database.state.db;
    Migrator::up(db, None).await?;
    let f = DomainFixture::new(db).await?;
    let track = Uuid::new_v4();
    let project = Uuid::new_v4();
    let application = Uuid::new_v4();
    mutate(db,Scope::HackathonExclusive(f.hackathon),format!("INSERT INTO prize_track(id,hackathon_id,name,description,is_judging_started,judging_method) VALUES('{track}','{0}','Track','Track',false,'scoring'); INSERT INTO judge_track_assign(user_id,track_id,hackathon_id,manually_granted) VALUES('{1}','{track}','{0}',true);",f.hackathon,f.users[1])).await.map_err(|e|anyhow::anyhow!("{e:?}"))?;
    mutate(db,Scope::Hackathon(f.hackathon),format!("INSERT INTO project(id,hackathon_id,event_id,team_id,data) VALUES('{project}','{0}','{1}','{2}','{{}}'); INSERT INTO project_track_sub(project_id,track_id,hackathon_id) VALUES('{project}','{track}','{0}'); UPDATE project SET status='submitted', submitted_at=clock_timestamp() WHERE id='{project}'; INSERT INTO application(id,user_id,hackathon_id,status,data,submitted_at) VALUES('{application}','{3}','{0}','pending','{{\"answer\":false}}',clock_timestamp());",f.hackathon,f.event,f.team,f.users[0])).await.map_err(|e|anyhow::anyhow!("{e:?}"))?;
    let row=db.query_one(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT id,after_data FROM audit_event WHERE resource_type='project' AND resource_key=jsonb_build_object('id',$1::uuid) ORDER BY version DESC LIMIT 1",[project.into()])).await?.context("project snapshot missing")?;
    let snapshot: Uuid = row.try_get("", "id")?;
    let content: serde_json::Value = row.try_get("", "after_data")?;
    ensure!(content["track_ids"] == serde_json::json!([track]));
    ensure!(content.get("form_schema").is_some());
    let valid = format!(
        "INSERT INTO evaluation(id,track_id,project_id,judge_id,hackathon_id,project_audit_id,judging_method,status,evaluation,created_at) VALUES('{}','{track}','{project}','{}','{}','{snapshot}','scoring','complete','{{\"score\":1}}',now())",
        Uuid::new_v4(),
        f.users[1],
        f.hackathon
    );
    mutate(db, Scope::Hackathon(f.hackathon), valid)
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let application_snapshot: Uuid = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT submitted_audit_id FROM application WHERE id=$1",
            [application.into()],
        ))
        .await?
        .unwrap()
        .try_get("", "submitted_audit_id")?;
    ensure!(mutate(db,Scope::Hackathon(f.hackathon),format!("UPDATE evaluation SET project_audit_id='{application_snapshot}' WHERE project_id='{project}'")).await.is_err(),"evaluation accepted another resource's snapshot");
    ensure!(
        mutate(
            db,
            Scope::Hackathon(f.hackathon),
            format!(
                "UPDATE application SET reviewed_audit_id='{snapshot}' WHERE id='{application}'"
            )
        )
        .await
        .is_err(),
        "application accepted a project snapshot"
    );
    mutate(db,Scope::Hackathon(f.hackathon),format!("UPDATE application SET reviewed_audit_id='{application_snapshot}',status='accepted',decided_by='{}',decided_at=now() WHERE id='{application}'; UPDATE application SET draft_data='{{\"answer\":0}}' WHERE id='{application}'",f.users[0])).await.map_err(|e|anyhow::anyhow!("{e:?}"))?;
    let data=db.query_one(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT submitted_audit_id,reviewed_audit_id,data,draft_data FROM application WHERE id=$1",[application.into()])).await?.unwrap();
    ensure!(data.try_get::<Uuid>("", "submitted_audit_id")? == application_snapshot);
    ensure!(data.try_get::<Uuid>("", "reviewed_audit_id")? == application_snapshot);
    ensure!(data.try_get::<serde_json::Value>("", "data")? == serde_json::json!({"answer":false}));
    ensure!(
        data.try_get::<serde_json::Value>("", "draft_data")? == serde_json::json!({"answer":0})
    );
    database.close().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]
async fn failed_audit_insert_rolls_back_the_domain_change() -> anyhow::Result<()> {
    use terrier_server::audit::Scope;
    let database = TestDatabase::new().await?;
    let db = &database.state.db;
    Migrator::up(db, None).await?;
    let f = DomainFixture::new(db).await?;
    let count = scalar(db, "SELECT count(*) AS value FROM audit_event").await?;
    db.execute_unprepared(
        "ALTER TABLE audit_event ADD CONSTRAINT injected_audit_failure CHECK(false) NOT VALID",
    )
    .await?;
    ensure!(
        mutate(
            db,
            Scope::Hackathon(f.hackathon),
            format!(
                "UPDATE team SET name='Must roll back' WHERE id='{}'",
                f.team
            )
        )
        .await
        .is_err()
    );
    db.execute_unprepared("ALTER TABLE audit_event DROP CONSTRAINT injected_audit_failure")
        .await?;
    ensure!(
        scalar(
            db,
            "SELECT count(*) AS value FROM team WHERE name='Must roll back'"
        )
        .await?
            == 0
    );
    ensure!(scalar(db, "SELECT count(*) AS value FROM audit_event").await? == count);
    database.close().await
}
