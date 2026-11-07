use std::sync::Arc;

use axum::{
    Extension, Router,
    error_handling::HandleErrorLayer,
    extract::{Request, State},
    http::{HeaderMap, header},
    middleware::{self, Next},
    response::Response,
};
use axum_oidc::{OidcAuthLayer, OidcClaims, OidcClient};
use tower::ServiceBuilder;
use tower_sessions::Session;

use super::{
    config::AuthConfig,
    current_user::CurrentUser,
    oidc::{CSRF, oidc_error, session_error},
    session::{Claims, OidcSessionStore},
};
use crate::{error::ApiError, state::AppState};

pub(super) async fn resolve_user(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if let Some(claims) = request.extensions().get::<OidcClaims<Claims>>() {
        let user = CurrentUser::resolve(&state.db, claims).await?;
        request.extensions_mut().insert(user);
    }
    Ok(next.run(request).await)
}

/// Applies authentication and CSRF checks to application routes.
/// Register all routes first, then nest the result under `/api`.
/// Routes added afterwards bypass these layers.
pub fn application_routes(
    routes: Router<AppState>,
    state: AppState,
    config: Arc<AuthConfig>,
    client: OidcClient<Claims>,
) -> Router<AppState> {
    routes
        .layer(middleware::from_fn(require_user))
        .layer(middleware::from_fn_with_state(state, resolve_user))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(oidc_error))
                .layer(OidcAuthLayer::<Claims, OidcSessionStore>::new(client)),
        )
        .layer(Extension(config))
        .fallback(|| async { ApiError::NotFound })
}

async fn require_user(
    _user: CurrentUser,
    session: Session,
    Extension(config): Extension<Arc<AuthConfig>>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if !request.method().is_safe() {
        check_csrf(&session, request.headers(), &config).await?;
    }
    Ok(next.run(request).await)
}

pub(super) async fn check_csrf(
    session: &Session,
    headers: &HeaderMap,
    config: &AuthConfig,
) -> Result<(), ApiError> {
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
    Ok(())
}
