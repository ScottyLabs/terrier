use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
                    .col(
                        ColumnDef::new(TeamJoinRequests::TeamId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TeamJoinRequests::UserId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TeamJoinRequests::Message).text())
                    .col(
                        ColumnDef::new(TeamJoinRequests::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
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
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TeamJoinRequests::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TeamJoinRequests {
    Table,
    Id,
    TeamId,
    UserId,
    Message,
    CreatedAt,
}

#[derive(DeriveIden)]
pub enum Teams {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
