use axum::{
    Form, Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use openidconnect::{
    PkceCodeChallenge, PkceCodeVerifier,
    core::{CoreHmacKey, CoreIdToken, CoreJwsSigningAlgorithm},
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
    pub refresh_delay: std::time::Duration,
    pub challenge: String,
    pub overrides: serde_json::Map<String, Value>,
    pub refresh_status: Option<StatusCode>,
    pub refresh_error: Option<String>,
    pub redirect_uri: String,
    pub refreshes: usize,
    pub refresh_requests: Vec<String>,
    pub next_refresh_token: Option<String>,
    pub signing_secret: Option<String>,
    pub require_issuer: bool,
    pub omit_nonce: bool,
    pub omit_id_token: bool,
    pub omit_refresh_token: bool,
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
                        "authorization_response_iss_parameter_supported": metadata.exchange.lock().unwrap().require_issuer,
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
    if form["grant_type"] == "refresh_token" {
        let delay = provider.exchange.lock().unwrap().refresh_delay;
        tokio::time::sleep(delay).await;
    }
    let mut exchange = provider.exchange.lock().unwrap();
    if form["grant_type"] == "authorization_code" {
        assert_eq!(form["redirect_uri"], exchange.redirect_uri);
        let verifier = PkceCodeVerifier::new(form["code_verifier"].clone());
        assert_eq!(
            PkceCodeChallenge::from_code_verifier_sha256(&verifier).as_str(),
            exchange.challenge
        );
    } else {
        exchange.refreshes += 1;
        exchange
            .refresh_requests
            .push(form["refresh_token"].clone());
        if let Some(status) = exchange.refresh_status {
            return (status, Json(json!({"error": exchange.refresh_error.as_deref().unwrap_or(if status == StatusCode::BAD_REQUEST {"invalid_grant"} else {"temporarily_unavailable"})}))).into_response();
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
    if exchange.omit_nonce {
        claims.as_object_mut().unwrap().remove("nonce");
    }
    let token = CoreIdToken::new(
        serde_json::from_value(claims).unwrap(),
        &CoreHmacKey::new(exchange.signing_secret.as_deref().unwrap_or(CLIENT_SECRET)),
        CoreJwsSigningAlgorithm::HmacSha256,
        None,
        None,
    )
    .unwrap();
    let mut response = json!({"access_token": "test-access-token", "token_type": "Bearer", "refresh_token": exchange.next_refresh_token.as_deref().unwrap_or("test-refresh-token"), "id_token": token.to_string()});
    if exchange.omit_id_token {
        response.as_object_mut().unwrap().remove("id_token");
    }
    if exchange.omit_refresh_token {
        response.as_object_mut().unwrap().remove("refresh_token");
    }
    Json(response).into_response()
}
