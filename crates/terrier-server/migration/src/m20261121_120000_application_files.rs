use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Hackathon::Table)
                    .add_column(
                        ColumnDef::new(Hackathon::FileFields)
                            .json_binary()
                            .not_null()
                            .default(
                                r#"{"resume":{"types":["application/pdf"],"max_bytes":5242880}}"#,
                            ),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Application::Table)
                    .add_column(
                        ColumnDef::new(Application::Attachments)
                            .json_binary()
                            .not_null()
                            .default("[]"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Application::Table)
                    .drop_column(Application::Attachments)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Hackathon::Table)
                    .drop_column(Hackathon::FileFields)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Hackathon {
    Table,
    FileFields,
}

#[derive(DeriveIden)]
enum Application {
    Table,
    Attachments,
}
