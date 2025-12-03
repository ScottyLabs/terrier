use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Hackathons::Table)
                    .add_column(ColumnDef::new(Hackathons::BackgroundUrl).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Hackathons::Table)
                    .drop_column(Hackathons::BackgroundUrl)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Hackathons {
    Table,
    BackgroundUrl,
}
