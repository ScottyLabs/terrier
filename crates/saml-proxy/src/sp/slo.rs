use crate::error::Error;
use crate::state::AppState;
use axum::extract::State;
use axum::response::{Html, IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use samael::schema::{Issuer, LogoutRequest, LogoutResponse, Status, StatusCode};
use samael::traits::ToXml;
use serde::Deserialize;
use std::sync::Arc;

const SAML_SUCCESS: &str = "urn:oasis:names:tc:SAML:2.0:status:Success";

#[derive(Deserialize)]
pub struct SloForm {
    #[serde(rename = "SAMLRequest")]
    pub saml_request: String,
    #[serde(rename = "RelayState")]
    pub relay_state: Option<String>,
}

/// Receives a SAML LogoutRequest from a university IdP (HTTP-POST binding).
/// Since the proxy has no persistent user sessions, this acknowledges the
/// logout with a success LogoutResponse posted back to the IdP.
pub async fn single_logout_service(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<SloForm>,
) -> Result<Response, Error> {
    let xml_bytes = STANDARD
        .decode(&form.saml_request)
        .map_err(|e| Error::InvalidSamlRequest(format!("base64 decode failed: {e}")))?;

    let xml = String::from_utf8(xml_bytes)
        .map_err(|e| Error::InvalidSamlRequest(format!("invalid UTF-8: {e}")))?;

    let logout_request: LogoutRequest = xml
        .parse()
        .map_err(|e| Error::InvalidSamlRequest(format!("failed to parse LogoutRequest: {e}")))?;

    let request_id = logout_request.id.as_deref().unwrap_or("unknown");
    let issuer_value = logout_request
        .issuer
        .as_ref()
        .and_then(|i| i.value.as_deref())
        .unwrap_or("unknown");

    tracing::info!(
        request_id,
        issuer = issuer_value,
        "received logout request from university IdP"
    );

    // The proxy is stateless -- there are no persistent user sessions to
    // invalidate, so we respond with success unconditionally.
    let response_id = format!("_slo_{}", uuid::Uuid::new_v4());

    let logout_response = LogoutResponse {
        id: Some(response_id),
        in_response_to: logout_request.id.clone(),
        version: Some("2.0".into()),
        issue_instant: Some(chrono::Utc::now()),
        destination: logout_request.destination.clone(),
        consent: None,
        issuer: Some(Issuer {
            value: Some(state.config.entity_id.clone()),
            ..Issuer::default()
        }),
        signature: None,
        status: Some(Status {
            status_code: StatusCode {
                value: Some(SAML_SUCCESS.into()),
            },
            status_message: None,
            status_detail: None,
        }),
    };

    let response_xml = logout_response
        .to_string()
        .map_err(|e| Error::Internal(anyhow::anyhow!("{e}")))?;

    let b64 = STANDARD.encode(response_xml.as_bytes());

    // If the request came with a destination, use it as the response URL.
    // Otherwise fall back to the issuer entity ID.
    let response_url = logout_request
        .destination
        .as_deref()
        .or_else(|| {
            logout_request
                .issuer
                .as_ref()
                .and_then(|i| i.value.as_deref())
        })
        .unwrap_or_default();

    let relay_state_input = form
        .relay_state
        .as_ref()
        .map(|rs| format!(r#"<input type="hidden" name="RelayState" value="{rs}" />"#))
        .unwrap_or_default();

    let html = format!(
        r#"<!DOCTYPE html>
<html><body onload="document.forms[0].submit()">
<form method="POST" action="{response_url}">
<input type="hidden" name="SAMLResponse" value="{b64}" />
{relay_state_input}
</form></body></html>"#,
    );

    Ok(Html(html).into_response())
}
