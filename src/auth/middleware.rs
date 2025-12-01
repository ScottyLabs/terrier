use axum::{
    body,
    extract::{FromRequestParts, State},
    middleware::Next,
    response::Response,
};
use axum_oidc::{EmptyAdditionalClaims, OidcClaims};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;

use crate::{
    AppState,
    entities::{prelude::*, users},
};

#[derive(Clone)]
pub struct SyncedUser(pub Arc<crate::entities::users::Model>);

impl<S> FromRequestParts<S> for SyncedUser
where
    S: Send + Sync,
{
    type Rejection = axum::http::StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<SyncedUser>()
            .cloned()
            .ok_or(axum::http::StatusCode::UNAUTHORIZED)
    }
}

/// Middleware to sync OIDC user information to the database
pub async fn sync_user_middleware(
    State(state): State<AppState>,
    claims: OidcClaims<EmptyAdditionalClaims>,
    mut request: http::Request<body::Body>,
    next: Next,
) -> Response {
    // Skip if already processed this request
    if request.extensions().get::<SyncedUser>().is_some() {
        return next.run(request).await;
    }

    let oidc_sub = claims.subject().to_string();
    let email = claims.email().map(|e| e.to_string()).unwrap_or_default();

    // Check if user exists
    let user = Users::find()
        .filter(users::Column::OidcSub.eq(&oidc_sub))
        .one(&state.db)
        .await
        .ok()
        .flatten();

    match user {
        Some(user) => {
            tracing::debug!("Authenticated user: {} ({})", email, oidc_sub);
            request.extensions_mut().insert(SyncedUser(Arc::new(user)));
        }
        None => {
            // Create new user with claims data
            let new_user = users::ActiveModel {
                oidc_sub: Set(oidc_sub.clone()),
                email: Set(email.clone()),
                name: Set(claims
                    .name()
                    .and_then(|n| n.get(None))
                    .map(|s| s.to_string())),
                given_name: Set(claims
                    .given_name()
                    .and_then(|n| n.get(None))
                    .map(|s| s.to_string())),
                family_name: Set(claims
                    .family_name()
                    .and_then(|n| n.get(None))
                    .map(|s| s.to_string())),
                picture: Set(claims
                    .picture()
                    .and_then(|p| p.get(None))
                    .map(|s| s.to_string())),
                oidc_issuer: Set(claims.issuer().to_string()),
                ..Default::default()
            };

            match new_user.insert(&state.db).await {
                Ok(created) => {
                    tracing::info!("Created new user: {} ({})", email, oidc_sub);
                    request
                        .extensions_mut()
                        .insert(SyncedUser(Arc::new(created)));
                }
                Err(e) => {
                    tracing::error!("Failed to create user: {:?}", e);
                }
            }
        }
    }

    next.run(request).await
}
