pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20260314_210918_create_users_table;
mod m20260314_213956_create_initial_tables;
mod m20260314_215609_create_initial_tables;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260314_213956_create_initial_tables::Migration)]
    }
}
