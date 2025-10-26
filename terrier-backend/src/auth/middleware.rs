use axum::{body, extract::State, http, middleware::Next, response::Response};
use axum_oidc::{EmptyAdditionalClaims, OidcClaims};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    AppState,
    entities::{prelude::*, users},
};

pub async fn sync_user_middleware(
    State(state): State<AppState>,
    claims: OidcClaims<EmptyAdditionalClaims>,
    request: http::Request<body::Body>,
    next: Next,
) -> Response {
    let oidc_sub = claims.subject().to_string();
    let email = claims.email().map(|e| e.to_string()).unwrap_or_default();

    let user = Users::find()
        .filter(users::Column::OidcSub.eq(&oidc_sub))
        .one(&state.db)
        .await
        .ok()
        .flatten();

    if user.is_none() {
        // Create new user
        let new_user = users::ActiveModel {
            oidc_sub: Set(oidc_sub),
            email: Set(email),
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

        if let Err(e) = new_user.insert(&state.db).await {
            tracing::error!("Failed to create user: {:?}", e);
        }
    }

    next.run(request).await
}
