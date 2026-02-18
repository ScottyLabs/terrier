use crate::error::Error;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use flate2::read::DeflateDecoder;
use samael::schema::AuthnRequest;
use serde::Deserialize;
use std::io::Read;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct SsoRedirectParams {
    #[serde(rename = "SAMLRequest")]
    pub saml_request: String,
    #[serde(rename = "RelayState")]
    pub relay_state: Option<String>,
}

#[derive(Deserialize)]
pub struct SsoPostParams {
    #[serde(rename = "SAMLRequest")]
    pub saml_request: String,
    #[serde(rename = "RelayState")]
    pub relay_state: Option<String>,
}

/// HTTP-Redirect binding: receives a deflated, base64-encoded AuthnRequest as
/// a query parameter from a Service Provider, creates a session, and redirects
/// the user to the discovery UI to select their university.
pub async fn sso_redirect(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SsoRedirectParams>,
) -> Result<impl IntoResponse, Error> {
    let xml = decode_redirect_binding(&params.saml_request)?;
    let session_id = process_authn_request(&state, &xml, params.relay_state)?;
    Ok(Redirect::to(&format!("/discovery?session={session_id}")))
}

/// HTTP-POST binding: receives a base64-encoded AuthnRequest as a form field
/// from a Service Provider, creates a session, and redirects the user to the
/// discovery UI to select their university.
pub async fn sso_post(
    State(state): State<Arc<AppState>>,
    axum::Form(params): axum::Form<SsoPostParams>,
) -> Result<impl IntoResponse, Error> {
    let xml = decode_post_binding(&params.saml_request)?;
    let session_id = process_authn_request(&state, &xml, params.relay_state)?;
    Ok(Redirect::to(&format!("/discovery?session={session_id}")))
}

/// Decodes HTTP-Redirect binding: base64 -> DEFLATE decompress -> XML string.
fn decode_redirect_binding(encoded: &str) -> Result<String, Error> {
    let compressed = STANDARD
        .decode(encoded)
        .map_err(|e| Error::InvalidSamlRequest(format!("base64 decode failed: {e}")))?;

    let mut decoder = DeflateDecoder::new(&compressed[..]);
    let mut xml = String::new();
    decoder
        .read_to_string(&mut xml)
        .map_err(|e| Error::InvalidSamlRequest(format!("deflate decompress failed: {e}")))?;

    Ok(xml)
}

/// Decodes HTTP-POST binding: base64 -> XML string (no compression).
fn decode_post_binding(encoded: &str) -> Result<String, Error> {
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|e| Error::InvalidSamlRequest(format!("base64 decode failed: {e}")))?;

    String::from_utf8(bytes).map_err(|e| Error::InvalidSamlRequest(format!("invalid UTF-8: {e}")))
}

/// Parses the AuthnRequest XML, extracts the SP's ACS URL and entity ID,
/// and creates a session to track the authentication flow.
fn process_authn_request(
    state: &AppState,
    xml: &str,
    relay_state: Option<String>,
) -> Result<String, Error> {
    let authn_request: AuthnRequest = xml
        .parse()
        .map_err(|e| Error::InvalidSamlRequest(format!("failed to parse AuthnRequest: {e}")))?;

    let sp_acs_url = authn_request
        .assertion_consumer_service_url
        .clone()
        .ok_or_else(|| {
            Error::InvalidSamlRequest("AuthnRequest missing AssertionConsumerServiceURL".into())
        })?;

    let sp_entity_id = authn_request.issuer_value().unwrap_or_default();

    let session_id = state
        .sessions
        .create(authn_request.id, sp_acs_url, sp_entity_id, relay_state);

    tracing::info!(session_id, "created session for incoming AuthnRequest");
    Ok(session_id)
}
