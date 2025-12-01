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
                    .add_column(ColumnDef::new(Hackathons::BannerUrl).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Hackathons::Table)
                    .drop_column(Hackathons::BannerUrl)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Hackathons {
    Table,
    BannerUrl,
}
