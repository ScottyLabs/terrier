use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TeamInvitations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TeamInvitations::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TeamInvitations::TeamId).integer().not_null())
                    .col(ColumnDef::new(TeamInvitations::UserId).integer().not_null())
                    .col(ColumnDef::new(TeamInvitations::Message).text())
                    .col(
                        ColumnDef::new(TeamInvitations::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TeamInvitations::Table, TeamInvitations::TeamId)
                            .to(Teams::Table, Teams::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TeamInvitations::Table, TeamInvitations::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TeamInvitations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TeamInvitations {
    Table,
    Id,
    TeamId,
    UserId,
    Message,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Teams {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
