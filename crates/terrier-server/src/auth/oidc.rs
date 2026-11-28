use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Query, Request, State},
    http::{StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use super::{
    config::AuthConfig,
    current_user::CurrentUser,
    middleware::{check_csrf, resolve_user},
};
use crate::{error::ApiError, state::AppState};

const RETURN_TO: &str = "return_to";
pub(super) const CSRF: &str = "csrf";

#[derive(utoipa::OpenApi)]
#[openapi(paths(finish_login, callback, logout, status))]
pub struct AuthApiDoc;

mod client;
pub use client::OidcClient;

pub async fn client(config: &AuthConfig) -> anyhow::Result<OidcClient> {
    OidcClient::discover(config).await
}

pub(super) async fn authenticate(
    State(client): State<OidcClient>,
    session: Session,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if let Some(claims) = client.claims(&session).await? {
        request.extensions_mut().insert(claims);
    }
    Ok(next.run(request).await)
}

pub async fn refresh_metadata(client: OidcClient) {
    let period = std::time::Duration::from_secs(15 * 60);
    let mut interval = tokio::time::interval_at(tokio::time::Instant::now() + period, period);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        if let Err(error) = client.rediscover().await {
            tracing::error!(%error, "failed to refresh OIDC provider metadata");
        }
    }
}

pub fn routes(config: Arc<AuthConfig>, client: OidcClient, state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", get(finish_login))
        .route("/api/auth/callback", get(callback))
        .route(
            "/api/auth/status",
            get(status)
                .layer(middleware::from_fn_with_state(state, resolve_user))
                .layer(middleware::from_fn_with_state(client.clone(), authenticate)),
        )
        .route("/api/auth/logout", post(logout))
        .layer(Extension(config))
        .layer(Extension(client))
}

#[derive(Deserialize)]
struct LoginQuery {
    return_to: Option<String>,
}

pub fn local_return_path(path: &str) -> Result<&str, ApiError> {
    let uri: Uri = path
        .parse()
        .map_err(|_error| ApiError::BadRequest("Invalid return path.".into()))?;
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.contains(['\\', '#'])
        || path.chars().any(char::is_control)
        || uri.scheme().is_some()
        || uri.authority().is_some()
        || uri.path().starts_with("/api/auth")
    {
        return Err(ApiError::BadRequest(
            "Return path must be a local application path.".into(),
        ));
    }
    Ok(path)
}

#[utoipa::path(get, path = "/api/auth/login", tag = "auth",
    params(("return_to" = Option<String>, Query, description = "Local application path after sign-in")),
    responses((status = 303, description = "Redirect to Keycloak or the requested page"),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn finish_login(
    State(state): State<AppState>,
    Extension(client): Extension<OidcClient>,
    session: Session,
    query: Result<Query<LoginQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Redirect, ApiError> {
    let Query(query) =
        query.map_err(|_error| ApiError::BadRequest("Invalid login query.".into()))?;
    if let Some(path) = query.return_to {
        local_return_path(&path)?;
        session
            .insert(RETURN_TO, path)
            .await
            .map_err(session_error)?;
        client::clear_credentials(&session).await?;
    }
    if let Some(claims) = client.claims(&session).await? {
        CurrentUser::resolve(&state.db, &claims).await?;
        let destination = session
            .remove::<String>(RETURN_TO)
            .await
            .map_err(session_error)?
            .unwrap_or_else(|| "/".into());
        local_return_path(&destination)?;
        return Ok(Redirect::to(&destination));
    }
    Ok(Redirect::to(client.begin(&session).await?.as_str()))
}

#[derive(Deserialize)]
struct CallbackQuery {
    state: String,
    code: Option<String>,
    error: Option<String>,
    iss: Option<String>,
}

#[utoipa::path(get, path = "/api/auth/callback", tag = "auth",
    params(("code" = Option<String>, Query), ("state" = String, Query),
        ("error" = Option<String>, Query), ("iss" = Option<String>, Query)),
    responses((status = 303, description = "Complete sign-in"),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn callback(
    session: Session,
    Extension(client): Extension<OidcClient>,
    uri: Uri,
) -> Result<Response, ApiError> {
    let Query(query) = Query::<CallbackQuery>::try_from_uri(&uri)
        .map_err(|_error| ApiError::BadRequest("Invalid OIDC callback.".into()))?;
    client
        .complete(&session, query.state, query.code, query.error, query.iss)
        .await?;
    Ok(Redirect::to("/api/auth/login").into_response())
}

#[utoipa::path(post, path = "/api/auth/logout", tag = "auth",
    params(("x-csrf-token" = String, Header, description = "Session-bound CSRF token")),
    responses((status = 204, description = "Local session deleted"),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn logout(
    Extension(config): Extension<Arc<AuthConfig>>,
    session: Session,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, ApiError> {
    check_csrf(&session, &headers, &config).await?;
    session.flush().await.map_err(session_error)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, utoipa::ToSchema)]
struct AuthStatus {
    user: Option<UserProfile>,
    csrf_token: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
struct UserProfile {
    id: String,
    email: String,
    display_name: String,
}

#[utoipa::path(get, path = "/api/auth/status", tag = "auth",
    responses((status = 200, body = AuthStatus), (status = 500, body = crate::error::ErrorResponse)))]
async fn status(user: Option<CurrentUser>, session: Session) -> Result<Json<AuthStatus>, ApiError> {
    let Some(user) = user else {
        return Ok(Json(AuthStatus {
            user: None,
            csrf_token: None,
        }));
    };
    let csrf_token: String = session
        .get(CSRF)
        .await
        .map_err(session_error)?
        .ok_or_else(|| internal("authenticated session is missing its CSRF token"))?;
    Ok(Json(AuthStatus {
        user: Some(UserProfile {
            id: user.user().id.to_string(),
            email: user.user().email.clone(),
            display_name: user.user().display_name.clone(),
        }),
        csrf_token: Some(csrf_token),
    }))
}

fn internal(message: &'static str) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(message))
}

pub(super) fn session_error(error: tower_sessions::session::Error) -> ApiError {
    use tower_sessions::{session::Error, session_store};
    match error {
        Error::Store(session_store::Error::Backend(message)) => {
            ApiError::Internal(anyhow::anyhow!(message).context("session backend failed"))
        }
        // Serialization errors can include the stored tokens or profile values.
        Error::Store(_) | Error::SerdeJson(_) => internal("invalid session data"),
    }
}

#[cfg(test)]
mod tests;
