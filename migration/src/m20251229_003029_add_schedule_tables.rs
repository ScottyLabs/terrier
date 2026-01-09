use sea_orm_migration::prelude::*;

use crate::m20251130_100554_create_user_hackathon_tables::Hackathons;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Events::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Events::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Events::HackathonId).integer().not_null())
                    .col(ColumnDef::new(Events::Name).string().not_null())
                    .col(ColumnDef::new(Events::Slug).string().not_null())
                    .col(ColumnDef::new(Events::Description).string())
                    .col(ColumnDef::new(Events::StartTime).timestamp().not_null())
                    .col(ColumnDef::new(Events::EndTime).timestamp().not_null())
                    // NULL = visible to everyone, otherwise must match user's role exactly
                    .col(ColumnDef::new(Events::VisibleToRole).string())
                    // Event type for color coding: default, hacking, speaker, sponsor, food
                    .col(
                        ColumnDef::new(Events::EventType)
                            .string()
                            .not_null()
                            .default("default"),
                    )
                    .col(
                        ColumnDef::new(Events::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Events::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .on_delete(ForeignKeyAction::NoAction)
                            .on_update(ForeignKeyAction::NoAction)
                            .from(Events::Table, Events::HackathonId)
                            .to(Hackathons::Table, Hackathons::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Events::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Events {
    Table,
    Id,
    HackathonId, // FK
    Name,
    Slug,
    Description,
    StartTime,
    EndTime,
    VisibleToRole, // NULL = visible to everyone, otherwise exact role match
    EventType,     // default, hacking, speaker, sponsor, food
    CreatedAt,
    UpdatedAt,
}
