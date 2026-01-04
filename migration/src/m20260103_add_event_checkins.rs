use sea_orm_migration::prelude::*;

use crate::m20251229_003029_add_schedule_tables::Events;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Event check-ins table and checkin_type column for events
#[derive(Iden)]
pub enum EventCheckins {
    Table,
    Id,
    EventId,
    UserId,
    CheckedInAt,
    CheckedInBy, // NULL for self-checkin, organizer user_id for QR scan
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add checkin_type column to events table (default to 'qr_scan')
        manager
            .alter_table(
                Table::alter()
                    .table(Events::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("checkin_type"))
                            .string()
                            .not_null()
                            .default("qr_scan"),
                    )
                    .to_owned(),
            )
            .await?;

        // Create event_checkins table
        manager
            .create_table(
                Table::create()
                    .table(EventCheckins::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EventCheckins::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EventCheckins::EventId).integer().not_null())
                    .col(ColumnDef::new(EventCheckins::UserId).integer().not_null())
                    .col(
                        ColumnDef::new(EventCheckins::CheckedInAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(ColumnDef::new(EventCheckins::CheckedInBy).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_event_checkins_event")
                            .from(EventCheckins::Table, EventCheckins::EventId)
                            .to(Events::Table, Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Add unique constraint: one check-in per user per event
        manager
            .create_index(
                Index::create()
                    .name("idx_event_checkins_unique")
                    .table(EventCheckins::Table)
                    .col(EventCheckins::EventId)
                    .col(EventCheckins::UserId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EventCheckins::Table).to_owned())
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Events::Table)
                    .drop_column(Alias::new("checkin_type"))
                    .to_owned(),
            )
            .await
    }
}
