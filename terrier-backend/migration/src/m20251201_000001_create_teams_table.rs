use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create teams table
        manager
            .create_table(
                Table::create()
                    .table(Teams::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Teams::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Teams::HackathonId).integer().not_null())
                    .col(ColumnDef::new(Teams::Name).string_len(100).not_null())
                    .col(ColumnDef::new(Teams::Description).text())
                    .col(ColumnDef::new(Teams::MaxMembers).integer().not_null().default(4))
                    .col(ColumnDef::new(Teams::CreatedById).integer().not_null())
                    .col(
                        ColumnDef::new(Teams::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Teams::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Teams::Table, Teams::HackathonId)
                            .to(Hackathons::Table, Hackathons::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Teams::Table, Teams::CreatedById)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create team_members table
        manager
            .create_table(
                Table::create()
                    .table(TeamMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TeamMembers::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TeamMembers::TeamId).integer().not_null())
                    .col(ColumnDef::new(TeamMembers::UserId).integer().not_null())
                    .col(
                        ColumnDef::new(TeamMembers::IsLeader)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(TeamMembers::JoinedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TeamMembers::Table, TeamMembers::TeamId)
                            .to(Teams::Table, Teams::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TeamMembers::Table, TeamMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index for team membership (user can only be in one team per hackathon)
        manager
            .create_index(
                Index::create()
                    .name("idx_team_members_unique")
                    .table(TeamMembers::Table)
                    .col(TeamMembers::TeamId)
                    .col(TeamMembers::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create team_join_requests table
        manager
            .create_table(
                Table::create()
                    .table(TeamJoinRequests::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TeamJoinRequests::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TeamJoinRequests::TeamId).integer().not_null())
                    .col(ColumnDef::new(TeamJoinRequests::UserId).integer().not_null())
                    .col(ColumnDef::new(TeamJoinRequests::Message).text())
                    .col(
                        ColumnDef::new(TeamJoinRequests::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(TeamJoinRequests::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(TeamJoinRequests::RespondedAt).timestamp())
                    .foreign_key(
                        ForeignKey::create()
                            .from(TeamJoinRequests::Table, TeamJoinRequests::TeamId)
                            .to(Teams::Table, Teams::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TeamJoinRequests::Table, TeamJoinRequests::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create team_invites table (for inviting members)
        manager
            .create_table(
                Table::create()
                    .table(TeamInvites::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TeamInvites::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TeamInvites::TeamId).integer().not_null())
                    .col(ColumnDef::new(TeamInvites::InvitedUserId).integer().not_null())
                    .col(ColumnDef::new(TeamInvites::InvitedById).integer().not_null())
                    .col(
                        ColumnDef::new(TeamInvites::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(TeamInvites::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(TeamInvites::RespondedAt).timestamp())
                    .foreign_key(
                        ForeignKey::create()
                            .from(TeamInvites::Table, TeamInvites::TeamId)
                            .to(Teams::Table, Teams::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TeamInvites::Table, TeamInvites::InvitedUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TeamInvites::Table, TeamInvites::InvitedById)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TeamInvites::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TeamJoinRequests::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TeamMembers::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Teams::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Teams {
    Table,
    Id,
    HackathonId,
    Name,
    Description,
    MaxMembers,
    CreatedById,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum TeamMembers {
    Table,
    Id,
    TeamId,
    UserId,
    IsLeader,
    JoinedAt,
}

#[derive(Iden)]
enum TeamJoinRequests {
    Table,
    Id,
    TeamId,
    UserId,
    Message,
    Status,
    CreatedAt,
    RespondedAt,
}

#[derive(Iden)]
enum TeamInvites {
    Table,
    Id,
    TeamId,
    InvitedUserId,
    InvitedById,
    Status,
    CreatedAt,
    RespondedAt,
}

#[derive(Iden)]
enum Hackathons {
    Table,
    Id,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
