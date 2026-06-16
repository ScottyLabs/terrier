use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ====================================================================
        // 1. Create PostgreSQL enum types
        // ====================================================================

        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new("hackathon_role"))
                    .values([
                        Alias::new("hacker"),
                        Alias::new("organizer"),
                        Alias::new("judge"),
                        Alias::new("sponsor"),
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new("application_status"))
                    .values([
                        Alias::new("pending"),
                        Alias::new("accepted"),
                        Alias::new("rejected"),
                        Alias::new("waitlisted"),
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new("reimbursement_status"))
                    .values([
                        Alias::new("pending"),
                        Alias::new("approved"),
                        Alias::new("rejected"),
                        Alias::new("paid"),
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new("team_member_status"))
                    .values([
                        Alias::new("active"),
                        Alias::new("invited"),
                        Alias::new("removed"),
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new("judging_method"))
                    .values([Alias::new("scoring"), Alias::new("expo")])
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new("evaluation_status"))
                    .values([
                        Alias::new("pending"),
                        Alias::new("complete"),
                        Alias::new("skipped"),
                    ])
                    .to_owned(),
            )
            .await?;

        // ====================================================================
        // 2. Create independent tables (no foreign keys)
        // ====================================================================

        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(User::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(User::Email).string().not_null())
                    .col(ColumnDef::new(User::DisplayName).string().not_null())
                    .col(ColumnDef::new(User::IsGlobalAdmin).boolean().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SponsorOrg::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SponsorOrg::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SponsorOrg::Name).string().not_null())
                    .col(ColumnDef::new(SponsorOrg::Description).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Hackathon::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Hackathon::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Hackathon::Slug).string().not_null())
                    .col(ColumnDef::new(Hackathon::Name).string().not_null())
                    .col(ColumnDef::new(Hackathon::Description).string().not_null())
                    .col(ColumnDef::new(Hackathon::IsActive).boolean().not_null())
                    .col(ColumnDef::new(Hackathon::MaxTeamSize).integer().not_null())
                    .col(
                        ColumnDef::new(Hackathon::MaxParticipants)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Hackathon::Location).string().not_null())
                    .col(ColumnDef::new(Hackathon::ContactEmail).string().not_null())
                    .col(
                        ColumnDef::new(Hackathon::TravelReimbursementEnabled)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Hackathon::ApplicationSchema)
                            .json_binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Hackathon::Data).json_binary().not_null())
                    .to_owned(),
            )
            .await?;

        // ====================================================================
        // 3. Create tables with hackathon/user/sponsor_org foreign keys
        // ====================================================================

        manager
            .create_table(
                Table::create()
                    .table(Invitation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Invitation::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Invitation::HackathonId).uuid().not_null())
                    .col(ColumnDef::new(Invitation::Code).string().not_null())
                    .col(
                        ColumnDef::new(Invitation::Role)
                            .custom(Alias::new("hackathon_role"))
                            .not_null(),
                    )
                    .col(ColumnDef::new(Invitation::TargetEmail).string())
                    .col(ColumnDef::new(Invitation::IsUsed).boolean().not_null())
                    .col(ColumnDef::new(Invitation::CreatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-invitation-hackathon")
                            .from(Invitation::Table, Invitation::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UserHackathonRole::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserHackathonRole::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserHackathonRole::HackathonId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserHackathonRole::Role)
                            .custom(Alias::new("hackathon_role"))
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(UserHackathonRole::UserId)
                            .col(UserHackathonRole::HackathonId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-uhr-user")
                            .from(UserHackathonRole::Table, UserHackathonRole::UserId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-uhr-hackathon")
                            .from(UserHackathonRole::Table, UserHackathonRole::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Application::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Application::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Application::UserId).uuid().not_null())
                    .col(ColumnDef::new(Application::HackathonId).uuid().not_null())
                    .col(
                        ColumnDef::new(Application::Status)
                            .custom(Alias::new("application_status"))
                            .not_null(),
                    )
                    .col(ColumnDef::new(Application::Data).json_binary().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-application-user")
                            .from(Application::Table, Application::UserId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-application-hackathon")
                            .from(Application::Table, Application::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Organizer::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Organizer::UserId).uuid().not_null())
                    .col(ColumnDef::new(Organizer::HackathonId).uuid().not_null())
                    .primary_key(
                        Index::create()
                            .col(Organizer::UserId)
                            .col(Organizer::HackathonId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-organizer-user")
                            .from(Organizer::Table, Organizer::UserId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-organizer-hackathon")
                            .from(Organizer::Table, Organizer::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Sponsor::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Sponsor::UserId).uuid().not_null())
                    .col(ColumnDef::new(Sponsor::SponsorOrgId).uuid().not_null())
                    .primary_key(
                        Index::create()
                            .col(Sponsor::UserId)
                            .col(Sponsor::SponsorOrgId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-sponsor-user")
                            .from(Sponsor::Table, Sponsor::UserId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-sponsor-org")
                            .from(Sponsor::Table, Sponsor::SponsorOrgId)
                            .to(SponsorOrg::Table, SponsorOrg::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Team::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Team::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Team::HackathonId).uuid().not_null())
                    .col(ColumnDef::new(Team::Name).string().not_null())
                    .col(ColumnDef::new(Team::TableNumber).string())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-team-hackathon")
                            .from(Team::Table, Team::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Event::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Event::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Event::HackathonId).uuid().not_null())
                    .col(ColumnDef::new(Event::Name).string().not_null())
                    .col(ColumnDef::new(Event::Description).string().not_null())
                    .col(ColumnDef::new(Event::StartTime).date_time().not_null())
                    .col(ColumnDef::new(Event::EndTime).date_time().not_null())
                    .col(ColumnDef::new(Event::Location).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-event-hackathon")
                            .from(Event::Table, Event::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PrizeTrack::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PrizeTrack::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PrizeTrack::HackathonId).uuid().not_null())
                    .col(ColumnDef::new(PrizeTrack::SponsorOrgId).uuid().not_null())
                    .col(ColumnDef::new(PrizeTrack::Name).string().not_null())
                    .col(ColumnDef::new(PrizeTrack::Description).string().not_null())
                    .col(
                        ColumnDef::new(PrizeTrack::IsJudgingStarted)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PrizeTrack::JudgingMethod)
                            .custom(Alias::new("judging_method"))
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-prize-track-hackathon")
                            .from(PrizeTrack::Table, PrizeTrack::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-prize-track-sponsor-org")
                            .from(PrizeTrack::Table, PrizeTrack::SponsorOrgId)
                            .to(SponsorOrg::Table, SponsorOrg::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // ====================================================================
        // 4. Create tables with level-2 foreign keys
        // ====================================================================

        manager
            .create_table(
                Table::create()
                    .table(ReimbursementApplication::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ReimbursementApplication::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ReimbursementApplication::ApplicationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ReimbursementApplication::Status)
                            .custom(Alias::new("reimbursement_status"))
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ReimbursementApplication::Data)
                            .json_binary()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reimbursement-application")
                            .from(
                                ReimbursementApplication::Table,
                                ReimbursementApplication::ApplicationId,
                            )
                            .to(Application::Table, Application::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TeamMember::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TeamMember::TeamId).uuid().not_null())
                    .col(ColumnDef::new(TeamMember::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(TeamMember::Status)
                            .custom(Alias::new("team_member_status"))
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(TeamMember::TeamId)
                            .col(TeamMember::UserId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-team-member-team")
                            .from(TeamMember::Table, TeamMember::TeamId)
                            .to(Team::Table, Team::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-team-member-user")
                            .from(TeamMember::Table, TeamMember::UserId)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Project::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Project::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Project::HackathonId).uuid().not_null())
                    .col(ColumnDef::new(Project::EventId).uuid().not_null())
                    .col(ColumnDef::new(Project::TeamId).uuid().not_null())
                    .col(ColumnDef::new(Project::Data).json_binary().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-project-hackathon")
                            .from(Project::Table, Project::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-project-event")
                            .from(Project::Table, Project::EventId)
                            .to(Event::Table, Event::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-project-team")
                            .from(Project::Table, Project::TeamId)
                            .to(Team::Table, Team::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(JudgeTrackAssign::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(JudgeTrackAssign::UserId).uuid().not_null())
                    .col(ColumnDef::new(JudgeTrackAssign::TrackId).uuid().not_null())
                    .primary_key(
                        Index::create()
                            .col(JudgeTrackAssign::UserId)
                            .col(JudgeTrackAssign::TrackId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-judge-track-assign-user")
                            .from(JudgeTrackAssign::Table, JudgeTrackAssign::UserId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-judge-track-assign-track")
                            .from(JudgeTrackAssign::Table, JudgeTrackAssign::TrackId)
                            .to(PrizeTrack::Table, PrizeTrack::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(EventCheckIn::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EventCheckIn::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EventCheckIn::EventId).uuid().not_null())
                    .col(ColumnDef::new(EventCheckIn::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(EventCheckIn::CheckInTime)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(EventCheckIn::CheckedInBy).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-event-check-in-event")
                            .from(EventCheckIn::Table, EventCheckIn::EventId)
                            .to(Event::Table, Event::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-event-check-in-user")
                            .from(EventCheckIn::Table, EventCheckIn::UserId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-event-check-in-checked-in-by")
                            .from(EventCheckIn::Table, EventCheckIn::CheckedInBy)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // ====================================================================
        // 5. Create tables with level-3 foreign keys
        // ====================================================================

        manager
            .create_table(
                Table::create()
                    .table(ProjectTrackSub::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ProjectTrackSub::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(ProjectTrackSub::TrackId).uuid().not_null())
                    .primary_key(
                        Index::create()
                            .col(ProjectTrackSub::ProjectId)
                            .col(ProjectTrackSub::TrackId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-project-track-sub-project")
                            .from(ProjectTrackSub::Table, ProjectTrackSub::ProjectId)
                            .to(Project::Table, Project::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-project-track-sub-track")
                            .from(ProjectTrackSub::Table, ProjectTrackSub::TrackId)
                            .to(PrizeTrack::Table, PrizeTrack::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Evaluation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Evaluation::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Evaluation::TrackId).uuid().not_null())
                    .col(ColumnDef::new(Evaluation::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(Evaluation::JudgeId).uuid().not_null())
                    .col(
                        ColumnDef::new(Evaluation::EvaluationData)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Evaluation::JudgingMethod)
                            .custom(Alias::new("judging_method"))
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Evaluation::Status)
                            .custom(Alias::new("evaluation_status"))
                            .not_null(),
                    )
                    .col(ColumnDef::new(Evaluation::CreatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-evaluation-track")
                            .from(Evaluation::Table, Evaluation::TrackId)
                            .to(PrizeTrack::Table, PrizeTrack::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-evaluation-project")
                            .from(Evaluation::Table, Evaluation::ProjectId)
                            .to(Project::Table, Project::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-evaluation-judge")
                            .from(Evaluation::Table, Evaluation::JudgeId)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop new tables in reverse creation order.
        manager
            .drop_table(Table::drop().table(Evaluation::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ProjectTrackSub::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(EventCheckIn::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(JudgeTrackAssign::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Project::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TeamMember::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(ReimbursementApplication::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(PrizeTrack::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Event::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Team::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Sponsor::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Organizer::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Application::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(UserHackathonRole::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Invitation::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Hackathon::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(SponsorOrg::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;

        // Drop enum types after all tables are removed.
        manager
            .drop_type(
                Type::drop()
                    .name(Alias::new("evaluation_status"))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_type(
                Type::drop()
                    .name(Alias::new("judging_method"))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_type(
                Type::drop()
                    .name(Alias::new("team_member_status"))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_type(
                Type::drop()
                    .name(Alias::new("reimbursement_status"))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_type(
                Type::drop()
                    .name(Alias::new("application_status"))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_type(
                Type::drop()
                    .name(Alias::new("hackathon_role"))
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Email,
    DisplayName,
    IsGlobalAdmin,
}

#[derive(DeriveIden)]
enum SponsorOrg {
    Table,
    Id,
    Name,
    Description,
}

#[derive(DeriveIden)]
enum Hackathon {
    Table,
    Id,
    Slug,
    Name,
    Description,
    IsActive,
    MaxTeamSize,
    MaxParticipants,
    Location,
    ContactEmail,
    TravelReimbursementEnabled,
    ApplicationSchema,
    Data,
}

#[derive(DeriveIden)]
enum Invitation {
    Table,
    Id,
    HackathonId,
    Code,
    Role,
    TargetEmail,
    IsUsed,
    CreatedAt,
}

#[derive(DeriveIden)]
enum UserHackathonRole {
    Table,
    UserId,
    HackathonId,
    Role,
}

#[derive(DeriveIden)]
enum Application {
    Table,
    Id,
    UserId,
    HackathonId,
    Status,
    Data,
}

#[derive(DeriveIden)]
enum ReimbursementApplication {
    Table,
    Id,
    ApplicationId,
    Status,
    Data,
}

#[derive(DeriveIden)]
enum Team {
    Table,
    Id,
    HackathonId,
    Name,
    TableNumber,
}

#[derive(DeriveIden)]
enum TeamMember {
    Table,
    TeamId,
    UserId,
    Status,
}

#[derive(DeriveIden)]
enum Project {
    Table,
    Id,
    HackathonId,
    EventId,
    TeamId,
    Data,
}

#[derive(DeriveIden)]
enum PrizeTrack {
    Table,
    Id,
    HackathonId,
    SponsorOrgId,
    Name,
    Description,
    IsJudgingStarted,
    JudgingMethod,
}

#[derive(DeriveIden)]
enum ProjectTrackSub {
    Table,
    ProjectId,
    TrackId,
}

#[derive(DeriveIden)]
enum JudgeTrackAssign {
    Table,
    UserId,
    TrackId,
}

#[derive(DeriveIden)]
enum Event {
    Table,
    Id,
    HackathonId,
    Name,
    Description,
    StartTime,
    EndTime,
    Location,
}

#[derive(DeriveIden)]
enum EventCheckIn {
    Table,
    Id,
    EventId,
    UserId,
    CheckInTime,
    CheckedInBy,
}

#[derive(DeriveIden)]
enum Evaluation {
    Table,
    Id,
    TrackId,
    ProjectId,
    JudgeId,
    #[sea_orm(iden = "evaluation")]
    EvaluationData,
    JudgingMethod,
    Status,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Sponsor {
    Table,
    UserId,
    SponsorOrgId,
}

#[derive(DeriveIden)]
enum Organizer {
    Table,
    UserId,
    HackathonId,
}
