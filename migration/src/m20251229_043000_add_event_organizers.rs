use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create event_organizers junction table
        manager
            .create_table(
                Table::create()
                    .table(EventOrganizers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EventOrganizers::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(EventOrganizers::EventId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(EventOrganizers::UserId).integer().not_null())
                    .col(
                        ColumnDef::new(EventOrganizers::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction)
                            .from(EventOrganizers::Table, EventOrganizers::EventId)
                            .to(Events::Table, Events::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction)
                            .from(EventOrganizers::Table, EventOrganizers::UserId)
                            .to(Users::Table, Users::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // Add unique constraint to prevent duplicate assignments
        manager
            .create_index(
                Index::create()
                    .name("idx_event_organizers_unique")
                    .table(EventOrganizers::Table)
                    .col(EventOrganizers::EventId)
                    .col(EventOrganizers::UserId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EventOrganizers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum EventOrganizers {
    Table,
    Id,
    EventId,
    UserId,
    CreatedAt,
}

// Local enum definitions for foreign key references
#[derive(DeriveIden)]
enum Events {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
