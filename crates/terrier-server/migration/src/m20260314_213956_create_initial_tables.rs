use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ====================================================================
        // 1. Create Independent Tables First (No Foreign Keys)
        // ====================================================================

        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(User::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(User::Username).string().not_null())
                    .col(ColumnDef::new(User::Password).string().not_null())
                    .col(ColumnDef::new(User::Role).string().not_null())
                    .to_owned(),
            )
            .await?;

        // Applicant represents a person who has applied but may not yet have
        // a User account. The UserId FK (nullable) is added here so that once
        // an applicant is accepted and an account is created the two records
        // can be linked without a separate junction table.
        manager
            .create_table(
                Table::create()
                    .table(Applicant::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Applicant::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Applicant::FirstName).string().not_null())
                    .col(ColumnDef::new(Applicant::LastName).string().not_null())
                    .col(ColumnDef::new(Applicant::Email).string().not_null())
                    .col(ColumnDef::new(Applicant::ApplicationId).string().not_null())
                    .col(ColumnDef::new(Applicant::UserId).string())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-applicant-user")
                            .from(Applicant::Table, Applicant::UserId)
                            .to(User::Table, User::Id),
                    )
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
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Hackathon::Name).string().not_null())
                    .col(ColumnDef::new(Hackathon::StartDate).date_time().not_null())
                    .col(ColumnDef::new(Hackathon::EndDate).date_time().not_null())
                    .col(ColumnDef::new(Hackathon::Location).string().not_null())
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
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SponsorOrg::Name).string().not_null())
                    .col(ColumnDef::new(SponsorOrg::Address).string().not_null())
                    .to_owned(),
            )
            .await?;

        // ====================================================================
        // 2. Create Level 1 Dependencies
        // ====================================================================

        manager
            .create_table(
                Table::create()
                    .table(Track::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Track::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Track::Name).string().not_null())
                    .col(ColumnDef::new(Track::HackathonId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-track-hackathon")
                            .from(Track::Table, Track::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Events::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Events::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Events::Name).string().not_null())
                    .col(ColumnDef::new(Events::StartTime).date_time().not_null())
                    .col(ColumnDef::new(Events::EndTime).date_time().not_null())
                    .col(ColumnDef::new(Events::HackathonId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-events-hackathon")
                            .from(Events::Table, Events::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Team::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Team::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Team::Name).string().not_null())
                    .col(ColumnDef::new(Team::HackathonId).string().not_null())
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
                    .table(Judge::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Judge::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Judge::UserId).string().not_null())
                    .col(ColumnDef::new(Judge::Expertise).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-judge-user")
                            .from(Judge::Table, Judge::UserId)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // Sponsor.UserId and Sponsor.SponsorOrgId are both nullable:
        //   - a sponsor contact may or may not have a system User account.
        //   - a sponsor may be an individual (no SponsorOrg) or part of an org.
        manager
            .create_table(
                Table::create()
                    .table(Sponsor::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Sponsor::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Sponsor::Name).string().not_null())
                    .col(ColumnDef::new(Sponsor::Description).string().not_null())
                    .col(ColumnDef::new(Sponsor::UserId).string())
                    .col(ColumnDef::new(Sponsor::SponsorOrgId).string())
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

        // ====================================================================
        // 3. Create Level 2 Dependencies
        // ====================================================================

        manager
            .create_table(
                Table::create()
                    .table(Hacker::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Hacker::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Hacker::FirstName).string().not_null())
                    .col(ColumnDef::new(Hacker::LastName).string().not_null())
                    .col(ColumnDef::new(Hacker::Email).string().not_null())
                    // Nullable: a hacker may not yet be on a team.
                    .col(ColumnDef::new(Hacker::TeamId).string())
                    // Nullable: a hacker may not yet have a User account.
                    // TODO: Maybe make this not nullable
                    .col(ColumnDef::new(Hacker::UserId).string())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-hacker-team")
                            .from(Hacker::Table, Hacker::TeamId)
                            .to(Team::Table, Team::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-hacker-user")
                            .from(Hacker::Table, Hacker::UserId)
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
                    .col(
                        ColumnDef::new(Project::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Project::Name).string().not_null())
                    .col(ColumnDef::new(Project::Description).string().not_null())
                    .col(ColumnDef::new(Project::TeamId).string().not_null())
                    .col(ColumnDef::new(Project::HackathonId).string().not_null())
                    .col(
                        ColumnDef::new(Project::SubmissionDate)
                            .date_time()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-project-team")
                            .from(Project::Table, Project::TeamId)
                            .to(Team::Table, Team::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-project-hackathon")
                            .from(Project::Table, Project::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(JudgeAssignment::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JudgeAssignment::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(JudgeAssignment::JudgeId).string().not_null())
                    .col(
                        ColumnDef::new(JudgeAssignment::HackathonId)
                            .string()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-ja-judge")
                            .from(JudgeAssignment::Table, JudgeAssignment::JudgeId)
                            .to(Judge::Table, Judge::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-ja-hackathon")
                            .from(JudgeAssignment::Table, JudgeAssignment::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // ====================================================================
        // 4. Create Level 3 Dependencies
        // ====================================================================

        manager
            .create_table(
                Table::create()
                    .table(Checkins::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Checkins::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Checkins::HackerId).string().not_null())
                    .col(ColumnDef::new(Checkins::CheckInTime).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-checkins-hacker")
                            .from(Checkins::Table, Checkins::HackerId)
                            .to(Hacker::Table, Hacker::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Submission::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Submission::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Submission::ProjectId).string().not_null())
                    .col(ColumnDef::new(Submission::TrackId).string().not_null())
                    .col(
                        ColumnDef::new(Submission::EvaluationCount)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-submission-project")
                            .from(Submission::Table, Submission::ProjectId)
                            .to(Project::Table, Project::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-submission-track")
                            .from(Submission::Table, Submission::TrackId)
                            .to(Track::Table, Track::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // ====================================================================
        // 5. Create Level 4 Dependencies (Prizes & Evaluations)
        // ====================================================================

        manager
            .create_table(
                Table::create()
                    .table(Prize::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Prize::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Prize::Name).string().not_null())
                    .col(ColumnDef::new(Prize::HackathonId).string().not_null())
                    .col(ColumnDef::new(Prize::SubmissionId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-prize-hackathon")
                            .from(Prize::Table, Prize::HackathonId)
                            .to(Hackathon::Table, Hackathon::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-prize-submission")
                            .from(Prize::Table, Prize::SubmissionId)
                            .to(Submission::Table, Submission::Id),
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
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Evaluation::SubmissionId).string().not_null())
                    .col(ColumnDef::new(Evaluation::JudgeId).string().not_null())
                    .col(ColumnDef::new(Evaluation::Score).float().not_null())
                    .col(ColumnDef::new(Evaluation::Feedback).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-eval-submission")
                            .from(Evaluation::Table, Evaluation::SubmissionId)
                            .to(Submission::Table, Submission::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-eval-judge")
                            .from(Evaluation::Table, Evaluation::JudgeId)
                            .to(Judge::Table, Judge::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ExpoEvaluation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ExpoEvaluation::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ExpoEvaluation::SubmissionId1)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExpoEvaluation::SubmissionId2)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ExpoEvaluation::JudgeId).string().not_null())
                    .col(ColumnDef::new(ExpoEvaluation::WinnerId).string().not_null())
                    .col(ColumnDef::new(ExpoEvaluation::Feedback).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-expo-sub1")
                            .from(ExpoEvaluation::Table, ExpoEvaluation::SubmissionId1)
                            .to(Submission::Table, Submission::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-expo-sub2")
                            .from(ExpoEvaluation::Table, ExpoEvaluation::SubmissionId2)
                            .to(Submission::Table, Submission::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-expo-judge")
                            .from(ExpoEvaluation::Table, ExpoEvaluation::JudgeId)
                            .to(Judge::Table, Judge::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-expo-winner")
                            .from(ExpoEvaluation::Table, ExpoEvaluation::WinnerId)
                            .to(Submission::Table, Submission::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in exact reverse order of creation to satisfy FK constraints.
        manager
            .drop_table(Table::drop().table(ExpoEvaluation::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Evaluation::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Prize::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Submission::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Checkins::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(JudgeAssignment::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Project::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Hacker::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Sponsor::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Judge::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Team::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Events::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Track::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(SponsorOrg::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Hackathon::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Applicant::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;

        Ok(())
    }
}

// -------------------------------------------------------------
// Iden Enums
// -------------------------------------------------------------

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Username,
    Password,
    Role,
}

#[derive(DeriveIden)]
enum Applicant {
    Table,
    Id,
    FirstName,
    LastName,
    Email,
    ApplicationId,
    UserId,
}

#[derive(DeriveIden)]
enum Hackathon {
    Table,
    Id,
    Name,
    StartDate,
    EndDate,
    Location,
}

#[derive(DeriveIden)]
enum SponsorOrg {
    Table,
    Id,
    Name,
    Address,
}

#[derive(DeriveIden)]
enum Sponsor {
    Table,
    Id,
    Name,
    Description,
    UserId,
    SponsorOrgId,
}

#[derive(DeriveIden)]
enum Judge {
    Table,
    Id,
    UserId,
    Expertise,
}

#[derive(DeriveIden)]
enum JudgeAssignment {
    Table,
    Id,
    JudgeId,
    HackathonId,
}

#[derive(DeriveIden)]
enum Track {
    Table,
    Id,
    Name,
    HackathonId,
}

#[derive(DeriveIden)]
enum Events {
    Table,
    Id,
    Name,
    StartTime,
    EndTime,
    HackathonId,
}

#[derive(DeriveIden)]
enum Team {
    Table,
    Id,
    Name,
    HackathonId,
}

#[derive(DeriveIden)]
enum Hacker {
    Table,
    Id,
    FirstName,
    LastName,
    Email,
    TeamId,
    UserId,
}

#[derive(DeriveIden)]
enum Project {
    Table,
    Id,
    Name,
    Description,
    TeamId,
    HackathonId,
    SubmissionDate,
}

#[derive(DeriveIden)]
enum Checkins {
    Table,
    Id,
    HackerId,
    CheckInTime,
}

#[derive(DeriveIden)]
enum Submission {
    Table,
    Id,
    ProjectId,
    TrackId,
    EvaluationCount,
}

#[derive(DeriveIden)]
enum Prize {
    Table,
    Id,
    Name,
    HackathonId,
    SubmissionId,
}

#[derive(DeriveIden)]
enum Evaluation {
    Table,
    Id,
    SubmissionId,
    JudgeId,
    Score,
    Feedback,
}

#[derive(DeriveIden)]
enum ExpoEvaluation {
    Table,
    Id,
    SubmissionId1,
    SubmissionId2,
    JudgeId,
    WinnerId,
    Feedback,
}
