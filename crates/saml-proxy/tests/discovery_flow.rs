use axum::http::{self, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use flate2::Compression;
use flate2::write::DeflateEncoder;
use saml_proxy::config::Config;
use saml_proxy::discovery::federation_index::EntityEntry;
use saml_proxy::state::AppState;
use std::io::Write;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceExt;

const AUTHN_REQUEST_XML: &str = r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" ID="_test" Version="2.0" IssueInstant="2026-01-01T00:00:00Z" AssertionConsumerServiceURL="http://localhost:3000/acs"><saml:Issuer xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">http://localhost:3000</saml:Issuer></samlp:AuthnRequest>"#;

fn encode_redirect_binding(xml: &str) -> String {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xml.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();
    STANDARD.encode(&compressed)
}

fn test_config() -> Config {
    Config {
        base_url: "http://localhost:8443".into(),
        entity_id: "http://localhost:8443/saml/idp".into(),
        idp_cert_path: "certs/idp-cert.pem".into(),
        idp_key_path: "certs/idp-key.pem".into(),
        host: "127.0.0.1".into(),
        port: 8443,
    }
}

fn static_dir() -> String {
    format!("{}/static", env!("CARGO_MANIFEST_DIR"))
}

async fn test_app() -> axum::Router {
    let config = test_config();
    let state = Arc::new(AppState::new(config).expect("failed to create AppState"));

    // Pre-populate the federation index so search works without fetching the
    // real InCommon aggregate.
    {
        let mut entries = state.federation_index.entries().write().await;
        *entries = vec![
            EntityEntry {
                entity_id: "https://idp.example.edu".into(),
                display_name: "Example University".into(),
            },
            EntityEntry {
                entity_id: "https://login.cmu.edu/idp/shibboleth".into(),
                display_name: "Carnegie Mellon University".into(),
            },
        ];
    }

    saml_proxy::app(state, static_dir())
}

fn extract_location(response: &http::Response<axum::body::Body>) -> &str {
    response
        .headers()
        .get(http::header::LOCATION)
        .expect("missing Location header")
        .to_str()
        .expect("invalid Location header")
}

#[tokio::test]
async fn sso_creates_session_and_redirects_to_discovery() {
    let app = test_app().await;

    let encoded = encode_redirect_binding(AUTHN_REQUEST_XML);
    let uri = format!(
        "/saml/sso?SAMLRequest={}&RelayState=test",
        urlencoding::encode(&encoded)
    );

    let response = app
        .oneshot(
            http::Request::builder()
                .uri(&uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = extract_location(&response);
    assert!(
        location.starts_with("/discovery?session="),
        "expected redirect to discovery, got: {location}"
    );
}

#[tokio::test]
async fn full_discovery_flow() {
    let app = test_app().await;

    // SSO endpoint creates session and redirects to discovery
    let encoded = encode_redirect_binding(AUTHN_REQUEST_XML);
    let uri = format!(
        "/saml/sso?SAMLRequest={}&RelayState=test",
        urlencoding::encode(&encoded)
    );

    let response = app
        .clone()
        .oneshot(
            http::Request::builder()
                .uri(&uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let discovery_path = extract_location(&response).to_string();
    let session_id = discovery_path
        .strip_prefix("/discovery?session=")
        .expect("unexpected redirect path");

    // Discovery page renders
    let response = app
        .clone()
        .oneshot(
            http::Request::builder()
                .uri(&discovery_path)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("Select Your University"));
    assert!(body.contains(session_id));

    // Search API returns pre-populated entries
    let response = app
        .clone()
        .oneshot(
            http::Request::builder()
                .uri("/api/entities/search?q=Carnegie")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("Carnegie Mellon University"));
    assert!(body.contains("login.cmu.edu"));

    // Submit university selection redirects to SP initiation
    let form_body = format!(
        "session_id={}&entity_id={}",
        urlencoding::encode(session_id),
        urlencoding::encode("https://login.cmu.edu/idp/shibboleth")
    );

    let response = app
        .oneshot(
            http::Request::builder()
                .method(http::Method::POST)
                .uri("/discovery")
                .header(
                    http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(axum::body::Body::from(form_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let initiate_path = extract_location(&response);
    assert!(
        initiate_path.starts_with("/sp/initiate?session="),
        "expected redirect to sp/initiate, got: {initiate_path}"
    );
    assert!(initiate_path.contains(session_id));
}

#[tokio::test]
async fn search_returns_empty_for_no_match() {
    let app = test_app().await;

    let response = app
        .oneshot(
            http::Request::builder()
                .uri("/api/entities/search?q=nonexistent")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert_eq!(body, "[]");
}

#[tokio::test]
async fn discovery_page_rejects_invalid_session() {
    let app = test_app().await;

    let response = app
        .oneshot(
            http::Request::builder()
                .uri("/discovery?session=nonexistent")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn metadata_endpoint_returns_xml() {
    let app = test_app().await;

    let response = app
        .oneshot(
            http::Request::builder()
                .uri("/saml/metadata")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("samlmetadata+xml"));

    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("EntityDescriptor"));
    assert!(body.contains("SingleLogoutService"));
    assert!(body.contains("http://localhost:8443/saml/idp"));
}

/// Starts a real HTTP server backed by the InCommon federation index and prints
/// a discovery URL you can open in a browser. Run with:
///
///     cargo test -p saml-proxy -- --ignored manual_discovery_ui --nocapture
///
/// Press Enter in the terminal to shut down the server when done.
#[tokio::test]
#[ignore]
async fn manual_discovery_ui() {
    let port = 18443;
    let base_url = format!("http://localhost:{port}");
    let config = Config {
        base_url: base_url.clone(),
        entity_id: format!("{base_url}/saml/idp"),
        idp_cert_path: "certs/idp-cert.pem".into(),
        idp_key_path: "certs/idp-key.pem".into(),
        host: "127.0.0.1".into(),
        port,
    };

    let state = Arc::new(AppState::new(config).expect("failed to create AppState"));

    tokio::spawn(saml_proxy::session::session_cleanup_task(
        state.sessions.clone(),
    ));

    eprintln!("  Fetching InCommon federation index...");
    state
        .federation_index
        .refresh()
        .await
        .expect("failed to fetch federation index");
    let count = state.federation_index.entries().read().await.len();
    eprintln!("  Loaded {count} IdP entities");

    // Create a session by processing a fake AuthnRequest
    let encoded = encode_redirect_binding(AUTHN_REQUEST_XML);
    let uri = format!(
        "/saml/sso?SAMLRequest={}&RelayState=test",
        urlencoding::encode(&encoded)
    );

    let app = saml_proxy::app(state, static_dir());

    let response = app
        .clone()
        .oneshot(
            http::Request::builder()
                .uri(&uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let discovery_path = extract_location(&response).to_string();

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .expect("failed to bind port");

    eprintln!();
    eprintln!("  Discovery UI: {base_url}{discovery_path}");
    eprintln!("  Metadata:     {base_url}/saml/metadata");
    eprintln!("  Search API:   {base_url}/api/entities/search?q=carnegie");
    eprintln!();
    eprintln!("  Press Enter to stop the server...");
    eprintln!();

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Wait for Enter on stdin
    tokio::task::spawn_blocking(|| {
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok();
    })
    .await
    .unwrap();

    server.abort();
}
