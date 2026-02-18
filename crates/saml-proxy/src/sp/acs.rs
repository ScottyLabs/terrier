use crate::error::Error;
use crate::state::AppState;
use axum::extract::State;
use axum::response::{Html, IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use samael::crypto::CertificateDer;
use samael::idp::IdentityProvider;
use samael::idp::response_builder::ResponseAttribute;
use samael::idp::sp_extractor::RequiredAttribute;
use samael::service_provider::ServiceProvider;
use samael::traits::ToXml;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct AcsForm {
    #[serde(rename = "SAMLResponse")]
    pub saml_response: String,
    #[serde(rename = "RelayState")]
    pub relay_state: Option<String>,
}

/// Receives the SAML Response from a university IdP, validates the signature
/// and assertions, extracts eduPerson attributes, builds a new SAML Response
/// addressed to the original Service Provider, and returns an auto-submitting
/// HTML form that POSTs the response to the SP's ACS URL.
pub async fn assertion_consumer_service(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<AcsForm>,
) -> Result<Response, Error> {
    // The RelayState carries the proxy session ID, set during sp/initiate.
    let session_id = form
        .relay_state
        .as_deref()
        .ok_or_else(|| Error::InvalidSamlResponse("missing RelayState".into()))?;

    let session = state
        .sessions
        .remove(session_id)
        .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))?;

    let entity_id = session
        .selected_university
        .as_ref()
        .ok_or(Error::MissingUniversitySelection)?;

    // Fetch the university's metadata again for signature validation.
    // The MDQ client caches responses, so this is typically a cache hit.
    let idp_metadata = state
        .mdq_client
        .fetch_entity(entity_id)
        .await
        .map_err(|e| Error::MdqFetchFailed(e.to_string()))?;

    let proxy_req_id = session
        .proxy_request_id
        .as_deref()
        .ok_or_else(|| Error::InvalidSamlResponse("no proxy request ID in session".into()))?;

    let sp = ServiceProvider {
        entity_id: Some(state.config.entity_id.clone()),
        acs_url: Some(format!("{}/sp/acs", state.config.base_url)),
        idp_metadata,
        allow_idp_initiated: false,
        max_issue_delay: chrono::Duration::minutes(5),
        ..ServiceProvider::default()
    };

    let assertion = sp
        .parse_base64_response(&form.saml_response, Some(&[proxy_req_id]))
        .map_err(|e| Error::InvalidSamlResponse(e.to_string()))?;

    let name_id = assertion
        .subject
        .as_ref()
        .and_then(|s| s.name_id.as_ref())
        .map(|n| n.value.as_str())
        .unwrap_or("unknown");

    let attrs = crate::attributes::extract_attributes(&assertion);

    // Build ResponseAttribute list from extracted eduPerson attributes.
    let response_attrs: Vec<ResponseAttribute> = attrs
        .iter()
        .map(|(name, value)| ResponseAttribute {
            required_attribute: RequiredAttribute {
                name: name.clone(),
                format: Some("urn:oasis:names:tc:SAML:2.0:attrname-format:uri".into()),
            },
            value: value.as_str(),
        })
        .collect();

    let idp = IdentityProvider::from_rsa_private_key_der(&state.idp_key_der)
        .map_err(|e| Error::Internal(anyhow::anyhow!("{e}")))?;

    let cert_der = CertificateDer::from(state.idp_cert_der.clone());

    let response = idp
        .sign_authn_response(
            &cert_der,
            name_id,
            &session.sp_entity_id,
            &session.sp_acs_url,
            &state.config.entity_id,
            &session.original_request_id,
            &response_attrs,
        )
        .map_err(|e| Error::Internal(anyhow::anyhow!("{e}")))?;

    let response_xml = response
        .to_string()
        .map_err(|e| Error::Internal(anyhow::anyhow!("{e}")))?;

    let b64 = STANDARD.encode(response_xml.as_bytes());

    let relay_state_input = session
        .relay_state
        .as_ref()
        .map(|rs| format!(r#"<input type="hidden" name="RelayState" value="{rs}" />"#))
        .unwrap_or_default();

    // Auto-submitting form that POSTs the signed SAML Response to the
    // original Service Provider's Assertion Consumer Service URL.
    let html = format!(
        r#"<!DOCTYPE html>
<html><body onload="document.forms[0].submit()">
<form method="POST" action="{acs_url}">
<input type="hidden" name="SAMLResponse" value="{b64}" />
{relay_state_input}
</form></body></html>"#,
        acs_url = session.sp_acs_url,
    );

    tracing::info!(
        sp_entity_id = session.sp_entity_id,
        name_id,
        attribute_count = attrs.len(),
        "forwarding assertion to service provider"
    );

    Ok(Html(html).into_response())
}
