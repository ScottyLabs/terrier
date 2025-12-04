use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Teams::Table)
                    .add_column(
                        ColumnDef::new(Teams::OwnerId)
                            .integer()
                            .not_null()
                            .default(1)
                    )
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint
        let foreign_key = TableForeignKey::new()
            .name("fk_teams_owner_id")
            .from_tbl(Teams::Table)
            .from_col(Teams::OwnerId)
            .to_tbl(Users::Table)
            .to_col(Users::Id)
            .on_delete(ForeignKeyAction::Restrict)
            .on_update(ForeignKeyAction::NoAction)
            .to_owned();

        manager
            .alter_table(
                Table::alter()
                    .table(Teams::Table)
                    .add_foreign_key(&foreign_key)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop foreign key
        manager
            .alter_table(
                Table::alter()
                    .table(Teams::Table)
                    .drop_foreign_key(Alias::new("fk_teams_owner_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Teams::Table)
                    .drop_column(Teams::OwnerId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Teams {
    Table,
    Id,
    OwnerId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
