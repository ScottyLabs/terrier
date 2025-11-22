use axum::{
    body,
    extract::State,
    middleware::Next,
    response::Response,
};
use axum_oidc::{EmptyAdditionalClaims, OidcClaims};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    AppState,
    entities::{prelude::*, users},
};

/// Middleware to sync OIDC user information to the database
pub async fn sync_user_middleware(
    State(state): State<AppState>,
    claims: OidcClaims<EmptyAdditionalClaims>,
    request: http::Request<body::Body>,
    next: Next,
) -> Response {
    let oidc_sub = claims.subject().to_string();
    let email = claims.email().map(|e| e.to_string()).unwrap_or_default();

    // Check if user exists
    let user = Users::find()
        .filter(users::Column::OidcSub.eq(&oidc_sub))
        .one(&state.db)
        .await
        .ok()
        .flatten();

    if user.is_none() {
        // Create new user with claims data
        let new_user = users::ActiveModel {
            oidc_sub: Set(oidc_sub.clone()),
            email: Set(email.clone()),
            name: Set(claims.name().and_then(|n| n.get(None)).map(|s| s.to_string())),
            given_name: Set(claims.given_name().and_then(|n| n.get(None)).map(|s| s.to_string())),
            family_name: Set(claims.family_name().and_then(|n| n.get(None)).map(|s| s.to_string())),
            picture: Set(claims.picture().and_then(|p| p.get(None)).map(|s| s.to_string())),
            oidc_issuer: Set(claims.issuer().to_string()),
            ..Default::default()
        };

        if let Err(e) = new_user.insert(&state.db).await {
            tracing::error!("Failed to create user: {:?}", e);
        } else {
            tracing::info!("Created new user: {} ({})", email, oidc_sub);
        }
    } else {
        tracing::debug!("Authenticated user: {} ({})", email, oidc_sub);
    }

    next.run(request).await
}
