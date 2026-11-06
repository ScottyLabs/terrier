use std::sync::Arc;

use axum::{
    Extension, Router,
    error_handling::HandleErrorLayer,
    extract::{Query, Request, State},
    http::{StatusCode, Uri, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_oidc::{
    OidcAuthLayer, OidcClaims, OidcClient, OidcLoginLayer,
    error::{HandlerError, MiddlewareError},
    openidconnect::{ClientId, ClientSecret, CsrfToken, IssuerUrl, Scope},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_sessions::Session;

use super::{
    config::AuthConfig,
    session::{Claims, OIDC_KEY, OidcSessionStore},
    user::{OidcIdentity, sync_user},
};
use crate::{error::ApiError, state::AppState};

const RETURN_TO: &str = "return_to";
const CSRF: &str = "csrf";

#[derive(utoipa::OpenApi)]
#[openapi(paths(finish_login, callback, logout))]
pub struct AuthApiDoc;

#[derive(Serialize)]
struct RelayState<'a> {
    return_to: &'a str,
    csrf: uuid::Uuid,
}

pub async fn client(config: &AuthConfig) -> anyhow::Result<OidcClient<Claims>> {
    let callback = config.app_url.join("api/auth/callback")?.to_string();
    // Discovery and token requests must not follow redirects to other hosts.
    let http = axum_oidc::openidconnect::reqwest::Client::builder()
        .redirect(axum_oidc::openidconnect::reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    Ok(OidcClient::builder()
        .with_http_client(http)
        .with_redirect_url(config.relay_url.as_str().parse()?)
        .with_client_id(ClientId::new(config.client_id.clone()))
        .with_client_secret(ClientSecret::new(config.client_secret.clone()))
        .with_scopes(vec![
            Scope::new("email".into()),
            Scope::new("profile".into()),
        ])
        .with_state_generator(move || {
            let state = RelayState {
                return_to: &callback,
                csrf: uuid::Uuid::new_v4(),
            };
            CsrfToken::new(
                URL_SAFE_NO_PAD
                    .encode(serde_json::to_vec(&state).expect("relay state is serializable")),
            )
        })
        .discover(IssuerUrl::new(config.issuer.to_string())?)
        .await
        .map_err(|error| {
            anyhow::anyhow!("failed to discover Keycloak OIDC configuration: {error}")
        })?
        .build())
}

pub fn routes(config: Arc<AuthConfig>, client: OidcClient<Claims>) -> Router<AppState> {
    let auth = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(oidc_error))
        .layer(OidcAuthLayer::<Claims, OidcSessionStore>::new(client));
    let login = ServiceBuilder::new()
        .layer(middleware::from_fn(prepare_login))
        .layer(auth.clone())
        .layer(HandleErrorLayer::new(oidc_error))
        .layer(OidcLoginLayer::<Claims, OidcSessionStore>::new());
    Router::new()
        .route("/api/auth/login", get(finish_login).layer(login))
        .route("/api/auth/callback", get(callback).layer(auth))
        .route("/api/auth/logout", post(logout))
        .layer(Extension(config))
}

#[derive(Deserialize)]
struct LoginQuery {
    return_to: Option<String>,
}

pub fn local_return_path(path: &str) -> Result<&str, ApiError> {
    let uri: Uri = path
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid return path.".into()))?;
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

async fn prepare_login(request: Request, next: Next) -> Result<Response, ApiError> {
    let Query(query) = Query::<LoginQuery>::try_from_uri(request.uri())
        .map_err(|_| ApiError::BadRequest("Invalid login query.".into()))?;
    if let Some(path) = query.return_to {
        local_return_path(&path)?;
        let session = request
            .extensions()
            .get::<Session>()
            .ok_or_else(|| internal("session middleware is missing"))?;
        session
            .insert(RETURN_TO, path)
            .await
            .map_err(session_error)?;
        // An explicit destination starts a fresh login, including verification rechecks.
        session
            .remove_value(OIDC_KEY)
            .await
            .map_err(session_error)?;
    }
    Ok(next.run(request).await)
}

#[utoipa::path(get, path = "/api/auth/login", tag = "auth",
    params(("return_to" = Option<String>, Query, description = "Local application path after sign-in")),
    responses((status = 303, description = "Redirect to Keycloak or the requested page"),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn finish_login(
    State(state): State<AppState>,
    claims: OidcClaims<Claims>,
    session: Session,
) -> Result<Redirect, ApiError> {
    let claims = &claims.0;
    let email = claims
        .email()
        .ok_or_else(|| internal("OIDC email is missing"))?;
    let name = claims
        .name()
        .and_then(|name| name.get(None))
        .ok_or_else(|| internal("OIDC display name is missing"))?;
    sync_user(
        &state.db,
        &OidcIdentity {
            issuer: claims.issuer().as_str(),
            subject: claims.subject().as_str(),
            email: email.as_str(),
            display_name: name.as_str(),
        },
    )
    .await
    .map_err(ApiError::Internal)?;
    let destination = session
        .remove::<String>(RETURN_TO)
        .await
        .map_err(session_error)?
        .unwrap_or_else(|| "/".into());
    local_return_path(&destination)?;
    Ok(Redirect::to(&destination))
}

#[utoipa::path(get, path = "/api/auth/callback", tag = "auth",
    params(("code" = String, Query), ("state" = String, Query)),
    responses((status = 303, description = "Complete sign-in"),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn callback(
    session: Session,
    Extension(client): Extension<OidcClient<Claims>>,
    uri: Uri,
) -> Result<Response, ApiError> {
    let query = Query::try_from_uri(&uri)
        .map_err(|_| ApiError::BadRequest("Invalid OIDC callback.".into()))?;
    session.cycle_id().await.map_err(session_error)?;
    axum_oidc::handle_oidc_redirect(OidcSessionStore(session.clone()), Extension(client), query)
        .await
        .map_err(callback_error)?;
    session
        .insert(CSRF, uuid::Uuid::new_v4().to_string())
        .await
        .map_err(session_error)?;
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
    if let Some(origin) = headers.get(header::ORIGIN)
        && origin.to_str().ok() != Some(config.app_url.origin().ascii_serialization().as_str())
    {
        return Err(ApiError::Forbidden);
    }
    let expected: Option<String> = session.get(CSRF).await.map_err(session_error)?;
    let supplied = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok());
    if expected.as_deref().is_none() || expected.as_deref() != supplied {
        return Err(ApiError::Forbidden);
    }
    session.flush().await.map_err(session_error)?;
    Ok(StatusCode::NO_CONTENT)
}

fn callback_error(error: HandlerError) -> ApiError {
    match error {
        HandlerError::RedirectWithInvalidSessionState
        | HandlerError::CsrfTokenInvalid
        | HandlerError::ClaimsVerification(_)
        | HandlerError::AccessTokenHashInvalid => {
            ApiError::BadRequest("Invalid or expired login response. Start sign-in again.".into())
        }
        HandlerError::Session(_) => internal("failed to persist OIDC callback session"),
        _ => internal("OIDC callback failed"),
    }
}

async fn oidc_error(error: MiddlewareError) -> ApiError {
    // Provider errors can contain token responses; log only the failure category.
    match error {
        MiddlewareError::Session(_) => internal("OIDC session storage failed"),
        MiddlewareError::RequestToken(_) => internal("OIDC token refresh failed"),
        MiddlewareError::UserInfoRetrieval(_) => internal("OIDC user info request failed"),
        _ => internal("OIDC middleware failed"),
    }
}

fn internal(message: &'static str) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(message))
}

fn session_error(_: tower_sessions::session::Error) -> ApiError {
    internal("session storage failed")
}

#[cfg(test)]
mod tests;
