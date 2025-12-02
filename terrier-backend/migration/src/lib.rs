pub use sea_orm_migration::prelude::*;

mod m20250922_204521_create_users_table;
mod m20250929_221212_add_user_role;
mod m20250930_182935_create_hackathons_table;
mod m20250930_183801_create_user_hackathon_roles_table;
mod m20250930_225342_remove_individual_user_role;
mod m20251129_000001_create_applications_table;
mod m20251201_000001_create_teams_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250922_204521_create_users_table::Migration),
            Box::new(m20250929_221212_add_user_role::Migration),
            Box::new(m20250930_182935_create_hackathons_table::Migration),
            Box::new(m20250930_183801_create_user_hackathon_roles_table::Migration),
            Box::new(m20250930_225342_remove_individual_user_role::Migration),
            Box::new(m20251129_000001_create_applications_table::Migration),
            Box::new(m20251201_000001_create_teams_table::Migration),
        ]
    }
}
