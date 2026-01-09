use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add location column to events table
        manager
            .alter_table(
                Table::alter()
                    .table(Events::Table)
                    .add_column(ColumnDef::new(Events::Location).string())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Events::Table)
                    .drop_column(Events::Location)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Events {
    Table,
    Location,
}
