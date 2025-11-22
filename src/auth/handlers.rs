use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_oidc::{EmptyAdditionalClaims, OidcClaims, OidcRpInitiatedLogout};
use http::Uri;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    AppState,
    entities::{prelude::*, users},
};
use super::{LoginQuery, UserInfo};

/// Get current user information
pub async fn status(
    State(state): State<AppState>,
    claims: Option<OidcClaims<EmptyAdditionalClaims>>,
) -> Result<Json<UserInfo>, StatusCode> {
    match claims {
        Some(claims) => {
            let email = claims
                .email()
                .map(|s| s.to_string())
                .ok_or(StatusCode::UNAUTHORIZED)?;

            let is_admin = state.config.admin_emails.contains(&email.to_lowercase());

            let oidc_sub = claims.subject().to_string();
            let user = Users::find()
                .filter(users::Column::OidcSub.eq(&oidc_sub))
                .one(&state.db)
                .await
                .ok()
                .flatten();

            user.map(|user| {
                Json(UserInfo {
                    id: user.id.to_string(),
                    email: user.email,
                    name: user.name,
                    picture: user.picture,
                    is_admin,
                })
            })
            .ok_or(StatusCode::UNAUTHORIZED)
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Initiate login flow
pub async fn login(
    _claims: OidcClaims<EmptyAdditionalClaims>,
    State(state): State<AppState>,
    Query(params): Query<LoginQuery>,
) -> impl IntoResponse {
    // OidcLoginLayer will have handled login, so redirect the user back at this point
    let redirect_to = params
        .redirect_uri
        .filter(|uri| uri.starts_with(&state.config.app_url))
        .unwrap_or_else(|| state.config.app_url.clone());

    Redirect::to(&redirect_to)
}

/// Log the current user out
pub async fn logout(
    logout: OidcRpInitiatedLogout,
    State(state): State<AppState>,
) -> impl IntoResponse {
    logout
        .with_post_logout_redirect(
            Uri::from_maybe_shared(state.config.app_url.clone()).expect("valid APP_URL"),
        )
        .into_response()
}
