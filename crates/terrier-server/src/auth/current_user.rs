use std::convert::Infallible;

use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::request::Parts,
};
use axum_oidc::OidcClaims;
use sea_orm::DatabaseConnection;

use super::{
    session::Claims,
    user::{OidcIdentity, sync_user},
};
use crate::{entities::user, error::ApiError};

/// A local user resolved from validated OIDC claims. Contains no permissions.
#[derive(Clone)]
pub struct CurrentUser {
    user: user::Model,
}

impl CurrentUser {
    pub fn user(&self) -> &user::Model {
        &self.user
    }

    pub(super) async fn resolve(
        db: &DatabaseConnection,
        claims: &OidcClaims<Claims>,
    ) -> Result<Self, ApiError> {
        let email = claims
            .email()
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("OIDC email is missing")))?;
        let name = claims
            .name()
            .and_then(|name| name.get(None))
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("OIDC display name is missing")))?;
        let user = sync_user(
            db,
            &OidcIdentity {
                issuer: claims.issuer().as_str(),
                subject: claims.subject().as_str(),
                email: email.as_str(),
                display_name: name.as_str(),
            },
        )
        .await
        .map_err(ApiError::Internal)?;
        Ok(Self { user })
    }
}

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or(ApiError::Unauthenticated)
    }
}

impl<S: Send + Sync> OptionalFromRequestParts<S> for CurrentUser {
    type Rejection = Infallible;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Option<Self>, Self::Rejection> {
        Ok(parts.extensions.get::<Self>().cloned())
    }
}
