use crate::error::Error;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use samael::metadata::HTTP_REDIRECT_BINDING;
use samael::service_provider::ServiceProvider;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct InitiateParams {
    pub session: String,
}

/// Fetches the selected university's metadata via MDQ, constructs a SAML
/// AuthnRequest using samael's ServiceProvider, and redirects the user to the
/// university's SSO endpoint.
pub async fn initiate(
    State(state): State<Arc<AppState>>,
    Query(params): Query<InitiateParams>,
) -> Result<Response, Error> {
    let session = state
        .sessions
        .get(&params.session)
        .ok_or_else(|| Error::SessionNotFound(params.session.clone()))?;

    let entity_id = session
        .selected_university
        .as_ref()
        .ok_or(Error::MissingUniversitySelection)?
        .clone();

    drop(session);

    let idp_metadata = state
        .mdq_client
        .fetch_entity(&entity_id)
        .await
        .map_err(|e| Error::MdqFetchFailed(e.to_string()))?;

    let sp = ServiceProvider {
        entity_id: Some(state.config.entity_id.clone()),
        acs_url: Some(format!("{}/sp/acs", state.config.base_url)),
        slo_url: Some(format!("{}/sp/slo", state.config.base_url)),
        idp_metadata,
        ..ServiceProvider::default()
    };

    let sso_url = sp
        .sso_binding_location(HTTP_REDIRECT_BINDING)
        .ok_or_else(|| {
            Error::MdqFetchFailed("no HTTP-Redirect SSO endpoint in university metadata".into())
        })?;

    let authn_request = sp
        .make_authentication_request(&sso_url)
        .map_err(|e| Error::Internal(anyhow::anyhow!("{e}")))?;

    state
        .sessions
        .update_proxy_request_id(&params.session, authn_request.id.clone());

    // Use session ID as RelayState so we can look up the session when the
    // university IdP posts the response back to our ACS endpoint.
    let redirect_url = authn_request
        .redirect(&params.session)
        .map_err(|e| Error::Internal(anyhow::anyhow!("{e}")))?
        .ok_or_else(|| Error::Internal(anyhow::anyhow!("AuthnRequest has no destination")))?;

    tracing::info!(
        session_id = params.session,
        university = entity_id,
        "redirecting to university IdP"
    );

    Ok(Redirect::to(redirect_url.as_str()).into_response())
}
