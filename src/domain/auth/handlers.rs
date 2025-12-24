#[cfg(feature = "server")]
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
#[cfg(feature = "server")]
use axum_oidc::{EmptyAdditionalClaims, OidcClaims, OidcRpInitiatedLogout};
use dioxus::prelude::*;
#[cfg(feature = "server")]
use http::Uri;

#[cfg(feature = "server")]
use crate::AppState;
#[cfg(feature = "server")]
use crate::auth::LoginQuery;
use crate::auth::UserInfo;
#[cfg(feature = "server")]
use crate::auth::middleware::SyncedUser;

/// Initiate login flow
#[cfg(feature = "server")]
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
#[cfg(feature = "server")]
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

/// Get current user information (server function)
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/auth/status",
    responses(
        (status = 200, description = "User information retrieved", body = Option<UserInfo>),
        (status = 500, description = "Server error")
    ),
    tag = "auth"
))]
#[get("/auth/status")]
pub async fn get_current_user() -> Result<Option<UserInfo>, ServerFnError> {
    use dioxus::fullstack::{FullstackContext, extract::State as DxState};

    // Try to extract user from context
    let user_data = match FullstackContext::extract::<SyncedUser, _>().await {
        Ok(u) => u,
        Err(_) => return Ok(None),
    };

    // Extract state from context
    let DxState(state) = FullstackContext::extract::<DxState<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    let email = user_data.0.email.clone();
    let is_admin = state.config.admin_emails.contains(&email.to_lowercase());

    Ok(Some(UserInfo {
        id: user_data.0.id.to_string(),
        email: user_data.0.email.clone(),
        name: user_data.0.name.clone(),
        picture: user_data.0.picture.clone(),
        is_admin,
    }))
}
