pub use sea_orm_migration::prelude::*;

mod m20251130_100554_create_user_hackathon_tables;
mod m20251201_023320_create_teams_tables;
mod m20251201_044615_add_hackathon_banner_url;
mod m20251201_165412_add_hackathon_form_config;
mod m20251201_165433_create_applications_table;
mod m20251203_041138_create_team_join_requests;
mod m20251203_145251_create_team_invitations;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20251130_100554_create_user_hackathon_tables::Migration),
            Box::new(m20251201_023320_create_teams_tables::Migration),
            Box::new(m20251201_044615_add_hackathon_banner_url::Migration),
            Box::new(m20251201_165412_add_hackathon_form_config::Migration),
            Box::new(m20251201_165433_create_applications_table::Migration),
            Box::new(m20251203_041138_create_team_join_requests::Migration),
            Box::new(m20251203_145251_create_team_invitations::Migration),
        ]
    }
}
