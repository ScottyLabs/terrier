use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add max_team_size to hackathons table
        manager
            .alter_table(
                Table::alter()
                    .table(Hackathons::Table)
                    .add_column(
                        ColumnDef::new(Hackathons::MaxTeamSize)
                            .integer()
                            .not_null()
                            .default(4),
                    )
                    .to_owned(),
            )
            .await?;

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
                    .col(ColumnDef::new(Teams::Name).string().not_null())
                    .col(ColumnDef::new(Teams::Description).text())
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
                    .to_owned(),
            )
            .await?;

        // Add team_id to user_hackathon_roles table
        // Every applicant/participant belongs to exactly one team
        manager
            .alter_table(
                Table::alter()
                    .table(UserHackathonRoles::Table)
                    .add_column(ColumnDef::new(UserHackathonRoles::TeamId).integer())
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_user_hackathon_roles_team")
                            .from_tbl(UserHackathonRoles::Table)
                            .from_col(UserHackathonRoles::TeamId)
                            .to_tbl(Teams::Table)
                            .to_col(Teams::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserHackathonRoles::Table)
                    .drop_column(UserHackathonRoles::TeamId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Teams::Table).to_owned())
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Hackathons::Table)
                    .drop_column(Hackathons::MaxTeamSize)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Hackathons {
    Table,
    Id,
    MaxTeamSize,
}

#[derive(DeriveIden)]
pub enum Teams {
    Table,
    Id,
    HackathonId,
    Name,
    Description,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserHackathonRoles {
    Table,
    TeamId,
}
