use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        refuse_populated_tables(manager).await?;

        create_types(manager).await?;
        replace_enum(
            manager,
            "team_member_status",
            &[
                "active",
                "invited",
                "removed",
                "pending",
                "withdrawn",
                "rejected",
            ],
            TeamMember::Table,
            TeamMember::Status,
        )
        .await?;
        replace_enum(
            manager,
            "reimbursement_status",
            &[
                "pending",
                "approved",
                "rejected",
                "paid",
                "denied",
                "withdrawn",
            ],
            ReimbursementApplication::Table,
            ReimbursementApplication::Status,
        )
        .await?;

        add_columns(manager).await?;
        reshape_columns(manager).await?;

        execute(manager, SPONSOR_PRIMARY_KEY).await?;
        create_unique_indexes(manager).await?;
        execute(manager, PARTIAL_UNIQUE_INDEXES).await?;
        execute(manager, CHECKS).await?;
        create_scope_foreign_keys(manager).await?;

        create_audit_event(manager).await?;
        execute(manager, AUDIT_FOREIGN_KEYS).await?;

        Ok(())
    }

    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom("RFC 0008 reconciliation is forward-only; restoring the old schema would discard workflow and audit history".into()))
    }
}

async fn execute(manager: &SchemaManager<'_>, statements: &[&str]) -> Result<(), DbErr> {
    let db = manager.get_connection();
    for statement in statements {
        db.execute_unprepared(statement).await?;
    }
    Ok(())
}

async fn refuse_populated_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let tables: [DynIden; 16] = [
        Hackathon::Table.into_iden(),
        Invitation::Table.into_iden(),
        UserHackathonRole::Table.into_iden(),
        Application::Table.into_iden(),
        ReimbursementApplication::Table.into_iden(),
        Team::Table.into_iden(),
        TeamMember::Table.into_iden(),
        Project::Table.into_iden(),
        PrizeTrack::Table.into_iden(),
        ProjectTrackSub::Table.into_iden(),
        JudgeTrackAssign::Table.into_iden(),
        Event::Table.into_iden(),
        EventCheckIn::Table.into_iden(),
        Evaluation::Table.into_iden(),
        Sponsor::Table.into_iden(),
        Organizer::Table.into_iden(),
    ];

    let db = manager.get_connection();
    for table in tables {
        let query = Query::select()
            .expr_as(
                Expr::exists(
                    Query::select()
                        .expr(Expr::val(1))
                        .from(table.clone())
                        .to_owned(),
                ),
                Alias::new("populated"),
            )
            .to_owned();
        let populated: bool = db
            .query_one(&query)
            .await?
            .ok_or_else(|| DbErr::Custom("EXISTS returned no row".into()))?
            .try_get("", "populated")?;

        if populated {
            return Err(DbErr::Migration(format!(
                "RFC 0008 migration needs an explicit legacy mapping for table {table}. No rows were changed. Supply deadlines, actors, membership times, submission snapshots and scoped grants before retrying."
            )));
        }
    }
    Ok(())
}

async fn create_types(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let types: [(&str, &[&str]); 6] = [
        (
            "capability",
            &[
                "application_management",
                "judging_management",
                "sponsorship_management",
                "hackathon_admin",
            ],
        ),
        (
            "attendance_status",
            &["pending", "confirmed", "declined", "expired", "revoked"],
        ),
        ("judge_availability", &["available", "on_break"]),
        ("project_status", &["draft", "submitted"]),
        (
            "audit_action",
            &[
                "created",
                "updated",
                "deactivated",
                "restored",
                "automatic",
                "accessed",
            ],
        ),
        (
            "audit_resource",
            &[
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
            ],
        ),
    ];

    for (name, values) in types {
        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new(name))
                    .values(values.iter().map(|value| Alias::new(*value)))
                    .to_owned(),
            )
            .await?;
    }
    Ok(())
}

/// Replaces a Postgres enum with a new value list, converting the column through text
async fn replace_enum(
    manager: &SchemaManager<'_>,
    name: &str,
    values: &[&str],
    table: impl IntoIden + 'static,
    column: impl IntoIden,
) -> Result<(), DbErr> {
    let legacy = format!("{name}_legacy");
    let column = column.into_iden();

    manager
        .alter_type(
            Type::alter()
                .name(Alias::new(name))
                .rename_to(Alias::new(&legacy))
                .to_owned(),
        )
        .await?;
    manager
        .create_type(
            Type::create()
                .as_enum(Alias::new(name))
                .values(values.iter().map(|value| Alias::new(*value)))
                .to_owned(),
        )
        .await?;
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .modify_column(
                    ColumnDef::new(column.clone())
                        .custom(Alias::new(name))
                        .using(
                            Expr::col(column)
                                .cast_as(Alias::new("text"))
                                .cast_as(Alias::new(name)),
                        ),
                )
                .to_owned(),
        )
        .await?;
    manager
        .drop_type(Type::drop().name(Alias::new(&legacy)).to_owned())
        .await
}

fn version(column: impl IntoIden) -> ColumnDef {
    let column = column.into_iden();
    ColumnDef::new(column.clone())
        .integer()
        .not_null()
        .default(1)
        .check(Expr::col(column).gt(0))
        .to_owned()
}

fn flag(column: impl IntoIden, default: bool) -> ColumnDef {
    ColumnDef::new(column)
        .boolean()
        .not_null()
        .default(default)
        .to_owned()
}

fn json(column: impl IntoIden, default: &str) -> ColumnDef {
    ColumnDef::new(column)
        .json_binary()
        .not_null()
        .default(default)
        .to_owned()
}

fn instant(column: impl IntoIden) -> ColumnDef {
    ColumnDef::new(column).timestamp_with_time_zone().to_owned()
}

fn required_instant(column: impl IntoIden) -> ColumnDef {
    ColumnDef::new(column)
        .timestamp_with_time_zone()
        .not_null()
        .to_owned()
}

fn uuid(column: impl IntoIden) -> ColumnDef {
    ColumnDef::new(column).uuid().to_owned()
}

fn required_uuid(column: impl IntoIden) -> ColumnDef {
    ColumnDef::new(column).uuid().not_null().to_owned()
}

fn capabilities(column: impl IntoIden) -> ColumnDef {
    ColumnDef::new(column)
        .array(ColumnType::Custom(Alias::new("capability").into_iden()))
        .not_null()
        .default("{}")
        .to_owned()
}

/// The foreign key Postgres creates for an inline `REFERENCES`, under the name it would give it
fn references(
    table: &str,
    column: impl IntoIden,
    target: impl IntoIden + 'static,
    target_column: impl IntoIden,
) -> TableForeignKey {
    let column = column.into_iden();
    TableForeignKey::new()
        .name(format!("{table}_{column}_fkey"))
        .from_col(column)
        .to_tbl(target)
        .to_col(target_column)
        .to_owned()
}

async fn add_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let alters = [
        Table::alter()
            .table(User::Table)
            .add_column(json(User::ProfileData, "{}"))
            .add_column(version(User::Version))
            .to_owned(),
        Table::alter()
            .table(Hackathon::Table)
            .add_column(instant(Hackathon::CompletedAt))
            .add_column(
                ColumnDef::new(Hackathon::MaxTeamsPerTable)
                    .integer()
                    .not_null()
                    .default(1)
                    .check(Expr::col(Hackathon::MaxTeamsPerTable).gt(0)),
            )
            .add_column(required_instant(Hackathon::ApplicationDeadline))
            .add_column(required_instant(Hackathon::TeamFormationDeadline))
            .add_column(required_instant(Hackathon::SubmissionDeadline))
            .add_column(json(Hackathon::ProjectSchema, "{}"))
            .add_column(json(Hackathon::DirectorySchema, "{}"))
            .add_column(json(Hackathon::DirectoryFields, "[]"))
            .add_column(json(Hackathon::ReimbursementCategories, "{}"))
            .add_column(json(Hackathon::Sponsors, "{}"))
            .add_column(version(Hackathon::Version))
            .to_owned(),
        Table::alter()
            .table(Invitation::Table)
            .add_column(required_uuid(Invitation::IssuerUserId))
            .add_foreign_key(&references(
                "invitation",
                Invitation::IssuerUserId,
                User::Table,
                User::Id,
            ))
            .add_column(required_uuid(Invitation::TargetUserId))
            .add_foreign_key(&references(
                "invitation",
                Invitation::TargetUserId,
                User::Table,
                User::Id,
            ))
            .add_column(
                ColumnDef::new(Invitation::TokenHash)
                    .text()
                    .not_null()
                    .unique_key(),
            )
            .add_column(capabilities(Invitation::Capabilities))
            .add_column(
                ColumnDef::new(Invitation::TrackIds)
                    .array(ColumnType::Uuid)
                    .not_null()
                    .default("{}"),
            )
            .add_column(uuid(Invitation::SponsorOrgId))
            .add_foreign_key(&references(
                "invitation",
                Invitation::SponsorOrgId,
                SponsorOrg::Table,
                SponsorOrg::Id,
            ))
            .add_column(required_instant(Invitation::ExpiresAt))
            .add_column(instant(Invitation::RevokedAt))
            .add_column(instant(Invitation::RedeemedAt))
            .add_column(version(Invitation::Version))
            .to_owned(),
        Table::alter()
            .table(UserHackathonRole::Table)
            .add_column(json(UserHackathonRole::ProfileData, "{}"))
            .add_column(flag(UserHackathonRole::DirectoryOptIn, false))
            .add_column(flag(UserHackathonRole::IsActive, true))
            .add_column(
                ColumnDef::new(UserHackathonRole::JudgeAvailability)
                    .custom(Alias::new("judge_availability"))
                    .not_null()
                    .default("available"),
            )
            .add_column(version(UserHackathonRole::Version))
            .to_owned(),
        Table::alter()
            .table(Application::Table)
            .add_column(json(Application::DraftData, "{}"))
            .add_column(uuid(Application::SubmittedAuditId))
            .add_column(uuid(Application::ReviewedAuditId))
            .add_column(uuid(Application::DecidedBy))
            .add_foreign_key(&references(
                "application",
                Application::DecidedBy,
                User::Table,
                User::Id,
            ))
            .add_column(instant(Application::SubmittedAt))
            .add_column(instant(Application::DecidedAt))
            .add_column(ColumnDef::new(Application::DecisionReason).text())
            .add_column(
                ColumnDef::new(Application::AttendanceStatus)
                    .custom(Alias::new("attendance_status")),
            )
            .add_column(instant(Application::ConfirmationDeadline))
            .add_column(flag(Application::IsActive, true))
            .add_column(version(Application::Version))
            .to_owned(),
        Table::alter()
            .table(ReimbursementApplication::Table)
            .add_column(
                ColumnDef::new(ReimbursementApplication::CategoryKey)
                    .text()
                    .not_null(),
            )
            .add_column(required_uuid(ReimbursementApplication::HackathonId))
            .add_foreign_key(&references(
                "reimbursement_application",
                ReimbursementApplication::HackathonId,
                Hackathon::Table,
                Hackathon::Id,
            ))
            .add_column(json(ReimbursementApplication::Evidence, "{}"))
            .add_column(ColumnDef::new(ReimbursementApplication::ApprovedAmount).decimal())
            .add_column(ColumnDef::new(ReimbursementApplication::Currency).text())
            .add_column(uuid(ReimbursementApplication::DecidedBy))
            .add_foreign_key(&references(
                "reimbursement_application",
                ReimbursementApplication::DecidedBy,
                User::Table,
                User::Id,
            ))
            .add_column(instant(ReimbursementApplication::DecidedAt))
            .add_column(ColumnDef::new(ReimbursementApplication::DecisionReason).text())
            .add_column(version(ReimbursementApplication::Version))
            .to_owned(),
        Table::alter()
            .table(Team::Table)
            .add_column(
                ColumnDef::new(Team::Description)
                    .text()
                    .not_null()
                    .default(""),
            )
            .add_column(flag(Team::IsActive, true))
            .add_column(version(Team::Version))
            .to_owned(),
        Table::alter()
            .table(TeamMember::Table)
            .add_column(required_uuid(TeamMember::HackathonId))
            .add_foreign_key(&references(
                "team_member",
                TeamMember::HackathonId,
                Hackathon::Table,
                Hackathon::Id,
            ))
            .add_column(flag(TeamMember::IsLeader, false))
            .add_column(required_instant(TeamMember::RequestedAt))
            .add_column(instant(TeamMember::JoinedAt))
            .add_column(instant(TeamMember::EndedAt))
            .add_column(version(TeamMember::Version))
            .to_owned(),
        Table::alter()
            .table(Project::Table)
            .add_column(
                ColumnDef::new(Project::Status)
                    .custom(Alias::new("project_status"))
                    .not_null()
                    .default("draft"),
            )
            .add_column(instant(Project::SubmittedAt))
            .add_column(version(Project::Version))
            .to_owned(),
        Table::alter()
            .table(PrizeTrack::Table)
            .add_column(flag(PrizeTrack::EditingLocked, false))
            .add_column(json(PrizeTrack::Criteria, "{}"))
            .add_column(json(PrizeTrack::Eligibility, "{}"))
            .add_column(json(PrizeTrack::Winners, "[]"))
            .add_column(ColumnDef::new(PrizeTrack::PublishedWinners).json_binary())
            .add_column(instant(PrizeTrack::ReleaseAt))
            .add_column(uuid(PrizeTrack::PublishedBy))
            .add_foreign_key(&references(
                "prize_track",
                PrizeTrack::PublishedBy,
                User::Table,
                User::Id,
            ))
            .add_column(instant(PrizeTrack::PublicationRevokedAt))
            .add_column(flag(PrizeTrack::IsActive, true))
            .add_column(version(PrizeTrack::Version))
            .to_owned(),
        Table::alter()
            .table(ProjectTrackSub::Table)
            .add_column(required_uuid(ProjectTrackSub::HackathonId))
            .add_foreign_key(&references(
                "project_track_sub",
                ProjectTrackSub::HackathonId,
                Hackathon::Table,
                Hackathon::Id,
            ))
            .add_column(flag(ProjectTrackSub::IsActive, true))
            .add_column(version(ProjectTrackSub::Version))
            .to_owned(),
        Table::alter()
            .table(JudgeTrackAssign::Table)
            .add_column(required_uuid(JudgeTrackAssign::HackathonId))
            .add_foreign_key(&references(
                "judge_track_assign",
                JudgeTrackAssign::HackathonId,
                Hackathon::Table,
                Hackathon::Id,
            ))
            .add_column(flag(JudgeTrackAssign::ManuallyGranted, false))
            .add_column(flag(JudgeTrackAssign::SponsorGranted, false))
            .add_column(flag(JudgeTrackAssign::IsSelected, false))
            .add_column(version(JudgeTrackAssign::Version))
            .to_owned(),
        Table::alter()
            .table(Event::Table)
            .add_column(flag(Event::IsActive, true))
            .add_column(version(Event::Version))
            .to_owned(),
        Table::alter()
            .table(EventCheckIn::Table)
            .add_column(flag(EventCheckIn::IsValid, true))
            .add_column(version(EventCheckIn::Version))
            .to_owned(),
        Table::alter()
            .table(Evaluation::Table)
            .add_column(required_uuid(Evaluation::HackathonId))
            .add_foreign_key(&references(
                "evaluation",
                Evaluation::HackathonId,
                Hackathon::Table,
                Hackathon::Id,
            ))
            .add_column(uuid(Evaluation::ProjectAuditId))
            .add_column(json(Evaluation::Rubric, "{}"))
            .add_column(flag(Evaluation::ManualOverride, false))
            .add_column(instant(Evaluation::EndedAt))
            .add_column(version(Evaluation::Version))
            .to_owned(),
        Table::alter()
            .table(Sponsor::Table)
            .add_column(required_uuid(Sponsor::HackathonId))
            .add_foreign_key(&references(
                "sponsor",
                Sponsor::HackathonId,
                Hackathon::Table,
                Hackathon::Id,
            ))
            .add_column(flag(Sponsor::IsActive, true))
            .add_column(version(Sponsor::Version))
            .to_owned(),
        Table::alter()
            .table(Organizer::Table)
            .add_column(capabilities(Organizer::Capabilities))
            .add_column(flag(Organizer::IsActive, true))
            .add_column(version(Organizer::Version))
            .to_owned(),
        Table::alter()
            .table(SponsorOrg::Table)
            .add_column(version(SponsorOrg::Version))
            .to_owned(),
    ];

    for alter in alters {
        manager.alter_table(alter).await?;
    }
    Ok(())
}

async fn reshape_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let alters = [
        Table::alter()
            .table(Hackathon::Table)
            .drop_column(Hackathon::TravelReimbursementEnabled)
            .to_owned(),
        Table::alter()
            .table(Invitation::Table)
            .rename_column(Invitation::Role, Invitation::InitialRole)
            .to_owned(),
        Table::alter()
            .table(Invitation::Table)
            .modify_column(ColumnDef::new(Invitation::InitialRole).null())
            .drop_column(Invitation::Code)
            .drop_column(Invitation::TargetEmail)
            .drop_column(Invitation::IsUsed)
            .drop_column(Invitation::CreatedAt)
            .to_owned(),
        Table::alter()
            .table(Application::Table)
            .modify_column(ColumnDef::new(Application::Status).null())
            .to_owned(),
        Table::alter()
            .table(PrizeTrack::Table)
            .modify_column(ColumnDef::new(PrizeTrack::SponsorOrgId).null())
            .to_owned(),
        Table::alter()
            .table(Evaluation::Table)
            .modify_column(ColumnDef::new(Evaluation::Result).null())
            .to_owned(),
    ];

    for alter in alters {
        manager.alter_table(alter).await?;
    }
    Ok(())
}

// TODO: sea-query 0.32 can't change a primary key
const SPONSOR_PRIMARY_KEY: &[&str] = &[
    "ALTER TABLE sponsor DROP CONSTRAINT sponsor_pkey",
    "ALTER TABLE sponsor ADD PRIMARY KEY (user_id, hackathon_id, org_id)",
];

async fn create_unique_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let indexes = [
        Index::create()
            .name("hackathon_slug_key")
            .table(Hackathon::Table)
            .col(Hackathon::Slug)
            .unique()
            .to_owned(),
        Index::create()
            .name("application_user_id_hackathon_id_key")
            .table(Application::Table)
            .col(Application::UserId)
            .col(Application::HackathonId)
            .unique()
            .to_owned(),
        Index::create()
            .name("application_id_hackathon_id_key")
            .table(Application::Table)
            .col(Application::Id)
            .col(Application::HackathonId)
            .unique()
            .to_owned(),
        Index::create()
            .name("reimbursement_application_application_id_category_key_key")
            .table(ReimbursementApplication::Table)
            .col(ReimbursementApplication::ApplicationId)
            .col(ReimbursementApplication::CategoryKey)
            .unique()
            .to_owned(),
        Index::create()
            .name("team_id_hackathon_id_key")
            .table(Team::Table)
            .col(Team::Id)
            .col(Team::HackathonId)
            .unique()
            .to_owned(),
        Index::create()
            .name("event_id_hackathon_id_key")
            .table(Event::Table)
            .col(Event::Id)
            .col(Event::HackathonId)
            .unique()
            .to_owned(),
        Index::create()
            .name("project_team_id_hackathon_id_key")
            .table(Project::Table)
            .col(Project::TeamId)
            .col(Project::HackathonId)
            .unique()
            .to_owned(),
        Index::create()
            .name("project_id_hackathon_id_key")
            .table(Project::Table)
            .col(Project::Id)
            .col(Project::HackathonId)
            .unique()
            .to_owned(),
        Index::create()
            .name("prize_track_id_hackathon_id_key")
            .table(PrizeTrack::Table)
            .col(PrizeTrack::Id)
            .col(PrizeTrack::HackathonId)
            .unique()
            .to_owned(),
        Index::create()
            .name("project_track_sub_project_id_track_id_hackathon_id_key")
            .table(ProjectTrackSub::Table)
            .col(ProjectTrackSub::ProjectId)
            .col(ProjectTrackSub::TrackId)
            .col(ProjectTrackSub::HackathonId)
            .unique()
            .to_owned(),
        Index::create()
            .name("judge_track_assign_user_id_track_id_hackathon_id_key")
            .table(JudgeTrackAssign::Table)
            .col(JudgeTrackAssign::UserId)
            .col(JudgeTrackAssign::TrackId)
            .col(JudgeTrackAssign::HackathonId)
            .unique()
            .to_owned(),
        Index::create()
            .name("event_check_in_event_id_user_id_key")
            .table(EventCheckIn::Table)
            .col(EventCheckIn::EventId)
            .col(EventCheckIn::UserId)
            .unique()
            .to_owned(),
    ];

    for index in indexes {
        manager.create_index(index).await?;
    }
    Ok(())
}

// TODO: sea-query 0.32 has no partial indexes
const PARTIAL_UNIQUE_INDEXES: &[&str] = &[
    "CREATE UNIQUE INDEX team_member_current_user ON team_member(user_id,hackathon_id) WHERE status IN ('active','pending')",
    "CREATE UNIQUE INDEX team_member_one_leader ON team_member(team_id) WHERE status='active' AND is_leader",
    "CREATE UNIQUE INDEX evaluation_current_assignment ON evaluation(judge_id,project_id,track_id) WHERE ended_at IS NULL",
];

// TODO: sea-query 0.32 can't add a check constraint to an existing table
const CHECKS: &[&str] = &[
    "ALTER TABLE hackathon ADD CHECK (max_team_size > 0 AND max_participants > 0)",
    "ALTER TABLE team_member ADD CHECK (NOT is_leader OR status='active')",
    "ALTER TABLE team_member ADD CHECK ((status='active' AND joined_at IS NOT NULL AND ended_at IS NULL)
        OR (status='pending' AND joined_at IS NULL AND ended_at IS NULL)
        OR (status IN ('removed','withdrawn','rejected') AND ended_at IS NOT NULL AND NOT is_leader)
        OR status='invited')",
    "ALTER TABLE project ADD CHECK ((status='draft' AND submitted_at IS NULL) OR (status='submitted' AND submitted_at IS NOT NULL))",
    "ALTER TABLE application ADD CHECK ((status IS NULL AND submitted_at IS NULL AND submitted_audit_id IS NULL)
        OR (status IS NOT NULL AND submitted_at IS NOT NULL AND submitted_audit_id IS NOT NULL))",
    "ALTER TABLE application ADD CHECK (attendance_status IS NULL OR status='accepted')",
    "ALTER TABLE reimbursement_application ADD CHECK (approved_amount IS NULL OR approved_amount >= 0)",
    "ALTER TABLE reimbursement_application ADD CHECK ((approved_amount IS NULL) = (currency IS NULL))",
    "ALTER TABLE reimbursement_application ADD CHECK (status <> 'approved' OR (approved_amount IS NOT NULL AND currency IS NOT NULL))",
    "ALTER TABLE reimbursement_application ADD CHECK (currency IS NULL OR currency ~ '^[A-Z]{3}$')",
    "ALTER TABLE invitation ADD CHECK (expires_at > '-infinity'::timestamptz AND expires_at < 'infinity'::timestamptz)",
    "ALTER TABLE invitation ADD CHECK (array_position(capabilities,NULL) IS NULL AND array_position(track_ids,NULL) IS NULL)",
    "ALTER TABLE organizer ADD CHECK (array_position(capabilities,NULL) IS NULL)",
    "ALTER TABLE evaluation ADD CHECK (status <> 'complete' OR (result IS NOT NULL AND project_audit_id IS NOT NULL))",
];

async fn create_scope_foreign_keys(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let keys = [
        ForeignKey::create()
            .name("scope_application_0")
            .from(
                Application::Table,
                (Application::UserId, Application::HackathonId),
            )
            .to(
                UserHackathonRole::Table,
                (UserHackathonRole::UserId, UserHackathonRole::HackathonId),
            )
            .to_owned(),
        ForeignKey::create()
            .name("scope_reimbursement_application_1")
            .from(
                ReimbursementApplication::Table,
                (
                    ReimbursementApplication::ApplicationId,
                    ReimbursementApplication::HackathonId,
                ),
            )
            .to(
                Application::Table,
                (Application::Id, Application::HackathonId),
            )
            .to_owned(),
        ForeignKey::create()
            .name("scope_team_member_2")
            .from(
                TeamMember::Table,
                (TeamMember::TeamId, TeamMember::HackathonId),
            )
            .to(Team::Table, (Team::Id, Team::HackathonId))
            .to_owned(),
        ForeignKey::create()
            .name("scope_team_member_3")
            .from(
                TeamMember::Table,
                (TeamMember::UserId, TeamMember::HackathonId),
            )
            .to(
                UserHackathonRole::Table,
                (UserHackathonRole::UserId, UserHackathonRole::HackathonId),
            )
            .to_owned(),
        ForeignKey::create()
            .name("scope_project_4")
            .from(Project::Table, (Project::TeamId, Project::HackathonId))
            .to(Team::Table, (Team::Id, Team::HackathonId))
            .to_owned(),
        ForeignKey::create()
            .name("scope_project_5")
            .from(Project::Table, (Project::EventId, Project::HackathonId))
            .to(Event::Table, (Event::Id, Event::HackathonId))
            .to_owned(),
        ForeignKey::create()
            .name("scope_project_track_sub_6")
            .from(
                ProjectTrackSub::Table,
                (ProjectTrackSub::ProjectId, ProjectTrackSub::HackathonId),
            )
            .to(Project::Table, (Project::Id, Project::HackathonId))
            .to_owned(),
        ForeignKey::create()
            .name("scope_project_track_sub_7")
            .from(
                ProjectTrackSub::Table,
                (ProjectTrackSub::TrackId, ProjectTrackSub::HackathonId),
            )
            .to(PrizeTrack::Table, (PrizeTrack::Id, PrizeTrack::HackathonId))
            .to_owned(),
        ForeignKey::create()
            .name("scope_judge_track_assign_8")
            .from(
                JudgeTrackAssign::Table,
                (JudgeTrackAssign::UserId, JudgeTrackAssign::HackathonId),
            )
            .to(
                UserHackathonRole::Table,
                (UserHackathonRole::UserId, UserHackathonRole::HackathonId),
            )
            .to_owned(),
        ForeignKey::create()
            .name("scope_judge_track_assign_9")
            .from(
                JudgeTrackAssign::Table,
                (JudgeTrackAssign::TrackId, JudgeTrackAssign::HackathonId),
            )
            .to(PrizeTrack::Table, (PrizeTrack::Id, PrizeTrack::HackathonId))
            .to_owned(),
        ForeignKey::create()
            .name("scope_evaluation_10")
            .from(
                Evaluation::Table,
                (
                    Evaluation::ProjectId,
                    Evaluation::TrackId,
                    Evaluation::HackathonId,
                ),
            )
            .to(
                ProjectTrackSub::Table,
                (
                    ProjectTrackSub::ProjectId,
                    ProjectTrackSub::TrackId,
                    ProjectTrackSub::HackathonId,
                ),
            )
            .to_owned(),
        ForeignKey::create()
            .name("scope_evaluation_11")
            .from(
                Evaluation::Table,
                (
                    Evaluation::JudgeId,
                    Evaluation::TrackId,
                    Evaluation::HackathonId,
                ),
            )
            .to(
                JudgeTrackAssign::Table,
                (
                    JudgeTrackAssign::UserId,
                    JudgeTrackAssign::TrackId,
                    JudgeTrackAssign::HackathonId,
                ),
            )
            .to_owned(),
        ForeignKey::create()
            .name("scope_sponsor_12")
            .from(Sponsor::Table, (Sponsor::UserId, Sponsor::HackathonId))
            .to(
                UserHackathonRole::Table,
                (UserHackathonRole::UserId, UserHackathonRole::HackathonId),
            )
            .to_owned(),
        ForeignKey::create()
            .name("scope_organizer_13")
            .from(
                Organizer::Table,
                (Organizer::UserId, Organizer::HackathonId),
            )
            .to(
                UserHackathonRole::Table,
                (UserHackathonRole::UserId, UserHackathonRole::HackathonId),
            )
            .to_owned(),
    ];

    for key in keys {
        manager.create_foreign_key(key).await?;
    }
    Ok(())
}

async fn create_audit_event(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(AuditEvent::Table)
                .col(
                    ColumnDef::new(AuditEvent::Id)
                        .uuid()
                        .primary_key()
                        .default(Func::cust(Alias::new("gen_random_uuid"))),
                )
                .col(uuid(AuditEvent::HackathonId))
                .col(uuid(AuditEvent::ActorUserId))
                .col(
                    ColumnDef::new(AuditEvent::Action)
                        .custom(Alias::new("audit_action"))
                        .not_null(),
                )
                .col(
                    ColumnDef::new(AuditEvent::ResourceType)
                        .custom(Alias::new("audit_resource"))
                        .not_null(),
                )
                .col(
                    ColumnDef::new(AuditEvent::ResourceKey)
                        .json_binary()
                        .not_null()
                        .check(
                            Expr::expr(
                                Func::cust(Alias::new("jsonb_typeof"))
                                    .arg(Expr::col(AuditEvent::ResourceKey)),
                            )
                            .eq("object")
                            .and(Expr::col(AuditEvent::ResourceKey).ne("{}")),
                        ),
                )
                .col(
                    ColumnDef::new(AuditEvent::OccurredAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Func::cust(Alias::new("clock_timestamp"))),
                )
                .col(ColumnDef::new(AuditEvent::Reason).text())
                .col(
                    ColumnDef::new(AuditEvent::Version)
                        .integer()
                        .check(Expr::col(AuditEvent::Version).gt(0)),
                )
                .col(ColumnDef::new(AuditEvent::BeforeData).json_binary())
                .col(ColumnDef::new(AuditEvent::AfterData).json_binary())
                .check(
                    Expr::col(AuditEvent::ActorUserId)
                        .is_not_null()
                        .or(Expr::col(AuditEvent::Action)
                            .eq("automatic")
                            .and(Expr::col(AuditEvent::Reason).is_not_null())
                            .and(
                                Expr::expr(
                                    Func::cust(Alias::new("length")).arg(
                                        Func::cust(Alias::new("btrim"))
                                            .arg(Expr::col(AuditEvent::Reason)),
                                    ),
                                )
                                .gt(0),
                            )),
                )
                .check(
                    Expr::col(AuditEvent::Action)
                        .eq("accessed")
                        .or(Expr::col(AuditEvent::Version)
                            .is_not_null()
                            .and(Expr::col(AuditEvent::AfterData).is_not_null())),
                )
                .index(
                    Index::create()
                        .name("audit_event_resource_type_resource_key_version_key")
                        .col(AuditEvent::ResourceType)
                        .col(AuditEvent::ResourceKey)
                        .col(AuditEvent::Version)
                        .unique(),
                )
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("audit_event_scope_time")
                .table(AuditEvent::Table)
                .col(AuditEvent::HackathonId)
                .col(AuditEvent::OccurredAt)
                .to_owned(),
        )
        .await
}

// TODO: sea-query 0.32 has no deferrable foreign keys
const AUDIT_FOREIGN_KEYS: &[&str] = &[
    "ALTER TABLE audit_event ADD CONSTRAINT audit_event_hackathon_id_fkey FOREIGN KEY (hackathon_id) REFERENCES hackathon(id) DEFERRABLE INITIALLY DEFERRED",
    "ALTER TABLE audit_event ADD CONSTRAINT audit_event_actor_user_id_fkey FOREIGN KEY (actor_user_id) REFERENCES \"user\"(id) DEFERRABLE INITIALLY DEFERRED",
    "ALTER TABLE application ADD CONSTRAINT application_submitted_audit_id_fkey FOREIGN KEY (submitted_audit_id) REFERENCES audit_event(id) DEFERRABLE INITIALLY DEFERRED",
    "ALTER TABLE application ADD CONSTRAINT application_reviewed_audit_id_fkey FOREIGN KEY (reviewed_audit_id) REFERENCES audit_event(id) DEFERRABLE INITIALLY DEFERRED",
    "ALTER TABLE evaluation ADD CONSTRAINT evaluation_project_audit_id_fkey FOREIGN KEY (project_audit_id) REFERENCES audit_event(id) DEFERRABLE INITIALLY DEFERRED",
];

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    ProfileData,
    Version,
}

#[derive(DeriveIden)]
enum Hackathon {
    Table,
    Id,
    Slug,
    CompletedAt,
    MaxTeamsPerTable,
    ApplicationDeadline,
    TeamFormationDeadline,
    SubmissionDeadline,
    ProjectSchema,
    DirectorySchema,
    DirectoryFields,
    ReimbursementCategories,
    Sponsors,
    Version,
    TravelReimbursementEnabled,
}

#[derive(DeriveIden)]
enum Invitation {
    Table,
    IssuerUserId,
    TargetUserId,
    TokenHash,
    Capabilities,
    TrackIds,
    SponsorOrgId,
    ExpiresAt,
    RevokedAt,
    RedeemedAt,
    Version,
    Role,
    InitialRole,
    Code,
    TargetEmail,
    IsUsed,
    CreatedAt,
}

#[derive(DeriveIden)]
enum UserHackathonRole {
    Table,
    UserId,
    HackathonId,
    ProfileData,
    DirectoryOptIn,
    IsActive,
    JudgeAvailability,
    Version,
}

#[derive(DeriveIden)]
enum Application {
    Table,
    Id,
    UserId,
    HackathonId,
    Status,
    DraftData,
    SubmittedAuditId,
    ReviewedAuditId,
    DecidedBy,
    SubmittedAt,
    DecidedAt,
    DecisionReason,
    AttendanceStatus,
    ConfirmationDeadline,
    IsActive,
    Version,
}

#[derive(DeriveIden)]
enum ReimbursementApplication {
    Table,
    ApplicationId,
    HackathonId,
    Status,
    CategoryKey,
    Evidence,
    ApprovedAmount,
    Currency,
    DecidedBy,
    DecidedAt,
    DecisionReason,
    Version,
}

#[derive(DeriveIden)]
enum Team {
    Table,
    Id,
    HackathonId,
    Description,
    IsActive,
    Version,
}

#[derive(DeriveIden)]
enum TeamMember {
    Table,
    TeamId,
    UserId,
    HackathonId,
    Status,
    IsLeader,
    RequestedAt,
    JoinedAt,
    EndedAt,
    Version,
}

#[derive(DeriveIden)]
enum Project {
    Table,
    Id,
    TeamId,
    EventId,
    HackathonId,
    Status,
    SubmittedAt,
    Version,
}

#[derive(DeriveIden)]
enum PrizeTrack {
    Table,
    Id,
    HackathonId,
    SponsorOrgId,
    EditingLocked,
    Criteria,
    Eligibility,
    Winners,
    PublishedWinners,
    ReleaseAt,
    PublishedBy,
    PublicationRevokedAt,
    IsActive,
    Version,
}

#[derive(DeriveIden)]
enum ProjectTrackSub {
    Table,
    ProjectId,
    TrackId,
    HackathonId,
    IsActive,
    Version,
}

#[derive(DeriveIden)]
enum JudgeTrackAssign {
    Table,
    UserId,
    TrackId,
    HackathonId,
    ManuallyGranted,
    SponsorGranted,
    IsSelected,
    Version,
}

#[derive(DeriveIden)]
enum Event {
    Table,
    Id,
    HackathonId,
    IsActive,
    Version,
}

#[derive(DeriveIden)]
enum EventCheckIn {
    Table,
    EventId,
    UserId,
    IsValid,
    Version,
}

#[derive(DeriveIden)]
enum Evaluation {
    Table,
    ProjectId,
    TrackId,
    JudgeId,
    HackathonId,
    ProjectAuditId,
    Rubric,
    ManualOverride,
    EndedAt,
    Version,
    Result,
}

#[derive(DeriveIden)]
enum Sponsor {
    Table,
    UserId,
    HackathonId,
    IsActive,
    Version,
}

#[derive(DeriveIden)]
enum Organizer {
    Table,
    UserId,
    HackathonId,
    Capabilities,
    IsActive,
    Version,
}

#[derive(DeriveIden)]
enum SponsorOrg {
    Table,
    Id,
    Version,
}

#[derive(DeriveIden)]
enum AuditEvent {
    Table,
    Id,
    HackathonId,
    ActorUserId,
    Action,
    ResourceType,
    ResourceKey,
    OccurredAt,
    Reason,
    Version,
    BeforeData,
    AfterData,
}
