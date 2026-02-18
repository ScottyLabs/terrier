use crate::state::AppState;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use samael::crypto::CertificateDer;
use samael::key_info::{KeyInfo, X509Data};
use samael::metadata::{
    Endpoint, EntityDescriptor, HTTP_POST_BINDING, HTTP_REDIRECT_BINDING, IdpSsoDescriptor,
    KeyDescriptor,
};
use samael::traits::ToXml;
use std::sync::Arc;

/// Generates the proxy's IdP metadata XML (EntityDescriptor) for Service
/// Providers to consume when configuring this proxy as their Identity Provider.
pub async fn metadata(State(state): State<Arc<AppState>>) -> Response {
    match build_metadata(&state) {
        Ok(xml) => (
            [(header::CONTENT_TYPE, "application/samlmetadata+xml")],
            xml,
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to generate IdP metadata");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn build_metadata(state: &AppState) -> anyhow::Result<String> {
    let sso_url = format!("{}/saml/sso", state.config.base_url);

    let cert_der = CertificateDer::from(state.idp_cert_der.clone());
    let cert_b64 = samael::crypto::mime_encode_x509_cert(&cert_der);

    let key_descriptor = KeyDescriptor {
        key_use: Some("signing".to_string()),
        key_info: KeyInfo {
            id: None,
            x509_data: Some(X509Data {
                certificates: vec![cert_b64],
            }),
        },
        encryption_methods: None,
    };

    let idp_descriptor = IdpSsoDescriptor {
        want_authn_requests_signed: Some(false),
        protocol_support_enumeration: Some("urn:oasis:names:tc:SAML:2.0:protocol".to_string()),
        key_descriptors: vec![key_descriptor],
        name_id_formats: vec![
            "urn:oasis:names:tc:SAML:2.0:nameid-format:transient".to_string(),
            "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent".to_string(),
        ],
        single_sign_on_services: vec![
            Endpoint {
                binding: HTTP_REDIRECT_BINDING.to_string(),
                location: sso_url.clone(),
                response_location: None,
            },
            Endpoint {
                binding: HTTP_POST_BINDING.to_string(),
                location: sso_url,
                response_location: None,
            },
        ],
        id: None,
        valid_until: None,
        cache_duration: None,
        error_url: None,
        signature: None,
        organization: None,
        contact_people: vec![],
        artifact_resolution_service: vec![],
        single_logout_services: vec![],
        manage_name_id_services: vec![],
        name_id_mapping_services: vec![],
        assertion_id_request_services: vec![],
        attribute_profiles: vec![],
        attributes: vec![],
    };

    let entity_descriptor = EntityDescriptor {
        entity_id: Some(state.config.entity_id.clone()),
        idp_sso_descriptors: Some(vec![idp_descriptor]),
        ..Default::default()
    };

    let xml = entity_descriptor
        .to_string()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(xml)
}
