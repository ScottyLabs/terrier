use anyhow::{Context, ensure};
use sea_orm::{DatabaseConnection, EntityTrait, Set, sea_query::OnConflict};
use uuid::Uuid;

use crate::entities::user;

/// Profile attributes from validated Keycloak claims.
pub struct OidcIdentity<'a> {
    pub issuer: &'a str,
    pub subject: &'a str,
    pub email: &'a str,
    pub display_name: &'a str,
}

/// Creates a user or updates their email without replacing their UUID or display name.
pub async fn sync_user(
    db: &DatabaseConnection,
    identity: &OidcIdentity<'_>,
) -> anyhow::Result<user::Model> {
    for (name, value) in [
        ("issuer", identity.issuer),
        ("subject", identity.subject),
        ("email", identity.email),
        ("display name", identity.display_name),
    ] {
        ensure!(!value.trim().is_empty(), "OIDC {name} is missing");
    }

    user::Entity::insert(user::ActiveModel {
        id: Set(Uuid::new_v4()),
        oidc_issuer: Set(identity.issuer.into()),
        oidc_subject: Set(identity.subject.into()),
        email: Set(identity.email.into()),
        display_name: Set(identity.display_name.into()),
    })
    .on_conflict(
        OnConflict::columns([user::Column::OidcIssuer, user::Column::OidcSubject])
            .update_column(user::Column::Email)
            .to_owned(),
    )
    .exec_with_returning(db)
    .await
    .context("failed to synchronize OIDC user")
}
