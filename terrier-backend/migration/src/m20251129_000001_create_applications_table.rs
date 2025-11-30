use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Applications::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Applications::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Applications::UserId).integer().not_null())
                    .col(
                        ColumnDef::new(Applications::HackathonId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Applications::FormData)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Applications::Status)
                            .string()
                            .not_null()
                            .default("draft"),
                    )
                    .col(ColumnDef::new(Applications::SubmittedAt).timestamp())
                    .col(
                        ColumnDef::new(Applications::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Applications::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_applications_user_id")
                            .from(Applications::Table, Applications::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_applications_hackathon_id")
                            .from(Applications::Table, Applications::HackathonId)
                            .to(Hackathons::Table, Hackathons::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index for one application per user per hackathon
        manager
            .create_index(
                Index::create()
                    .name("idx_applications_user_hackathon")
                    .table(Applications::Table)
                    .col(Applications::UserId)
                    .col(Applications::HackathonId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Applications::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Applications {
    Table,
    Id,
    UserId,
    HackathonId,
    FormData,
    Status,
    SubmittedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Hackathons {
    Table,
    Id,
}
