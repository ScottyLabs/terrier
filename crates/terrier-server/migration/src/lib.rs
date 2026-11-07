pub use sea_orm_migration::prelude::*;

mod m20260616_200457_redesign_schema;

mod m20261105_161638_oidc_identity;
mod m20261107_205124_reconcile_database_rfc;
mod m20261107_210438_audit_transactions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260616_200457_redesign_schema::Migration),
            Box::new(m20261105_161638_oidc_identity::Migration),
            Box::new(m20261107_205124_reconcile_database_rfc::Migration),
            Box::new(m20261107_210438_audit_transactions::Migration),
        ]
    }
}
