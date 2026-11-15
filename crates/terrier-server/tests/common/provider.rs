use super::Claims;
use axum::{
    Form, Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_oidc::openidconnect::{
    PkceCodeChallenge, PkceCodeVerifier,
    core::{CoreHmacKey, CoreJwsSigningAlgorithm},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tower_sessions::cookie::time::OffsetDateTime;

pub const CLIENT_SECRET: &str = "oidc-test-client-secret";

#[derive(Default)]
pub struct Exchange {
    pub nonce: String,
    pub challenge: String,
    pub overrides: serde_json::Map<String, Value>,
    pub refresh_status: Option<StatusCode>,
    pub refreshes: usize,
}

pub struct Provider {
    pub url: String,
    pub exchange: Mutex<Exchange>,
    pub discovered: tokio::sync::Notify,
}

impl Provider {
    pub async fn start() -> (Arc<Self>, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let provider = Arc::new(Provider {
            url: format!("http://{}", listener.local_addr().unwrap()),
            exchange: Mutex::new(Exchange::default()),
            discovered: tokio::sync::Notify::new(),
        });
        let metadata = provider.clone();
        let mock = Router::new()
            .route(
                "/.well-known/openid-configuration",
                get(move || async move {
                    metadata.discovered.notify_one();
                    Json(json!({
                        "issuer": format!("{}/", metadata.url),
                        "authorization_endpoint": format!("{}/authorize", metadata.url),
                        "token_endpoint": format!("{}/token", metadata.url),
                        "userinfo_endpoint": format!("{}/userinfo", metadata.url),
                        "jwks_uri": format!("{}/jwks", metadata.url),
                        "response_types_supported": ["code"],
                        "subject_types_supported": ["public"],
                        "id_token_signing_alg_values_supported": ["HS256"],
                    }))
                }),
            )
            .route("/jwks", get(|| async { Json(json!({"keys": []})) }))
            .route(
                "/userinfo",
                get(|| async { Json(json!({"sub": "member"})) }),
            )
            .route("/token", post(token))
            .with_state(provider.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, mock).await.unwrap();
        });
        (provider, task)
    }
}

async fn token(
    State(provider): State<Arc<Provider>>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let mut exchange = provider.exchange.lock().unwrap();
    if form["grant_type"] == "authorization_code" {
        let verifier = PkceCodeVerifier::new(form["code_verifier"].clone());
        assert_eq!(
            PkceCodeChallenge::from_code_verifier_sha256(&verifier).as_str(),
            exchange.challenge
        );
    } else {
        exchange.refreshes += 1;
        if let Some(status) = exchange.refresh_status {
            return (status, Json(json!({"error": if status == StatusCode::BAD_REQUEST {"invalid_grant"} else {"temporarily_unavailable"}}))).into_response();
        }
    }
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let mut claims = json!({
        "iss": format!("{}/", provider.url), "sub": "member", "aud": "terrier", "iat": now,
        "exp": now + 300,
        "nonce": exchange.nonce, "email": "member@example.com", "name": "Member", "email_verified": true,
    });
    claims
        .as_object_mut()
        .unwrap()
        .extend(exchange.overrides.clone());
    let token = axum_oidc::IdToken::<Claims>::new(
        serde_json::from_value(claims).unwrap(),
        &CoreHmacKey::new(CLIENT_SECRET),
        CoreJwsSigningAlgorithm::HmacSha256,
        None,
        None,
    )
    .unwrap();
    Json(json!({"access_token": "test-access-token", "token_type": "Bearer", "refresh_token": "test-refresh-token", "id_token": token.to_string()})).into_response()
}
