use crate::state::AppState;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use samael::crypto::CertificateDer;
use samael::key_info::{KeyInfo, X509Data};
use samael::metadata::{
    Endpoint, EntityDescriptor, HTTP_POST_BINDING, IndexedEndpoint, KeyDescriptor, SpSsoDescriptor,
};
use samael::traits::ToXml;
use std::sync::Arc;

/// Generates the proxy's SP metadata XML (EntityDescriptor with
/// SPSSODescriptor) for upstream Identity Providers to consume.
pub async fn metadata(State(state): State<Arc<AppState>>) -> Response {
    match build_metadata(&state) {
        Ok(xml) => (
            [(header::CONTENT_TYPE, "application/samlmetadata+xml")],
            xml,
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to generate SP metadata");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn build_metadata(state: &AppState) -> anyhow::Result<String> {
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

    let sp_descriptor = SpSsoDescriptor {
        authn_requests_signed: Some(true),
        want_assertions_signed: Some(true),
        protocol_support_enumeration: Some("urn:oasis:names:tc:SAML:2.0:protocol".to_string()),
        key_descriptors: Some(vec![key_descriptor]),
        name_id_formats: Some(vec![
            "urn:oasis:names:tc:SAML:2.0:nameid-format:transient".to_string(),
        ]),
        assertion_consumer_services: vec![IndexedEndpoint {
            binding: HTTP_POST_BINDING.to_string(),
            location: format!("{}/sp/acs", state.config.base_url),
            response_location: None,
            index: 0,
            is_default: Some(true),
        }],
        single_logout_services: Some(vec![Endpoint {
            binding: HTTP_POST_BINDING.to_string(),
            location: format!("{}/sp/slo", state.config.base_url),
            response_location: None,
        }]),
        ..Default::default()
    };

    let entity_descriptor = EntityDescriptor {
        entity_id: Some(state.config.entity_id.clone()),
        sp_sso_descriptors: Some(vec![sp_descriptor]),
        ..Default::default()
    };

    let xml = entity_descriptor
        .to_string()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(xml)
}
