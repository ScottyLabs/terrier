use anyhow::ensure;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, sea_query::OnConflict,
};
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

    if let Some(existing) = user::Entity::find()
        .filter(user::Column::OidcIssuer.eq(identity.issuer))
        .filter(user::Column::OidcSubject.eq(identity.subject))
        .one(db)
        .await
        .map_err(|error| anyhow::anyhow!(error).context("read synchronized OIDC identity"))?
        && existing.email == identity.email
    {
        return Ok(existing);
    }
    let issuer = identity.issuer.to_owned();
    let subject = identity.subject.to_owned();
    let email = identity.email.to_owned();
    let display_name = identity.display_name.to_owned();
    crate::audited_automatic!(
        db,
        crate::audit::Scope::Global,
        "Synchronize validated OIDC identity",
        |db| {
            let saved = user::Entity::insert(user::ActiveModel {
                id: Set(Uuid::new_v4()),
                oidc_issuer: Set(issuer),
                oidc_subject: Set(subject),
                email: Set(email),
                display_name: Set(display_name),
                ..Default::default()
            })
            .on_conflict(
                OnConflict::columns([user::Column::OidcIssuer, user::Column::OidcSubject])
                    .update_column(user::Column::Email)
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await?;
            Ok(saved)
        }
    )
    .await
    .map_err(|error| match error {
        crate::error::ApiError::Internal(error) => error.context("failed to synchronize OIDC user"),
        error => anyhow::anyhow!("failed to synchronize OIDC user: {error:?}"),
    })
}
