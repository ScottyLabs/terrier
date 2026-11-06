use super::*;
use axum::{
    Form, Json,
    body::{Body, to_bytes},
    http::{
        Request,
        header::{COOKIE, LOCATION, SET_COOKIE},
    },
};
use axum_oidc::openidconnect::{
    PkceCodeChallenge, PkceCodeVerifier,
    core::{CoreHmacKey, CoreJwsSigningAlgorithm},
};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Mutex};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionStore, cookie::time::OffsetDateTime, session::Id};

const CLIENT_SECRET: &str = "oidc-test-client-secret";

#[derive(Default)]
struct Exchange {
    nonce: String,
    challenge: String,
    overrides: serde_json::Map<String, Value>,
    refresh_status: Option<StatusCode>,
    refreshes: usize,
}

struct Provider {
    url: String,
    exchange: Mutex<Exchange>,
}

struct Fixture {
    provider: Arc<Provider>,
    task: tokio::task::JoinHandle<()>,
    app: Router,
    store: MemoryStore,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Fixture {
    async fn new() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let provider = Arc::new(Provider {
            url: format!("http://{}", listener.local_addr().unwrap()),
            exchange: Mutex::new(Exchange::default()),
        });
        let metadata = provider.clone();
        let mock = Router::new()
            .route(
                "/.well-known/openid-configuration",
                get(move || async move {
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
        let config = Arc::new(AuthConfig {
            app_url: "https://terrier.example/".parse().unwrap(),
            issuer: provider.url.parse().unwrap(),
            relay_url: "https://relay.example/callback".parse().unwrap(),
            client_id: "terrier".into(),
            client_secret: CLIENT_SECRET.into(),
            instance_admin_subjects: Default::default(),
        });
        let client = client(&config).await.unwrap();
        let probe = Router::new()
            .route(
                "/probe",
                get(|claims: Option<OidcClaims<Claims>>| async move {
                    if claims.is_some() {
                        StatusCode::OK
                    } else {
                        StatusCode::UNAUTHORIZED
                    }
                }),
            )
            .layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(oidc_error))
                    .layer(OidcAuthLayer::<Claims, OidcSessionStore>::new(
                        client.clone(),
                    )),
            );
        let store = MemoryStore::default();
        let unavailable = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = unavailable.local_addr().unwrap();
        drop(unavailable);
        let mut options = sea_orm::ConnectOptions::new(format!("postgres://test@{address}/test"));
        options
            .connect_lazy(true)
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_millis(100))
            .connect_timeout(std::time::Duration::from_millis(100));
        let db = sea_orm::Database::connect(options).await.unwrap();
        let app = routes(config, client)
            .merge(probe)
            .layer(super::super::session::layer(store.clone(), true))
            .layer(middleware::from_fn(super::super::session::error_response))
            .with_state(AppState { db });
        Self {
            provider,
            task,
            app,
            store,
        }
    }

    async fn record(&self, cookie: &str) -> tower_sessions::session::Record {
        self.store.load(&session_id(cookie)).await.unwrap().unwrap()
    }

    async fn begin(&self) -> (String, String) {
        let response = self
            .request("/api/auth/login?return_to=%2Fdashboard", None)
            .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let cookie_header = response.headers()[SET_COOKIE].to_str().unwrap();
        for attribute in [
            "HttpOnly",
            "Secure",
            "SameSite=Lax",
            "Path=/",
            "Max-Age=172800",
        ] {
            assert!(cookie_header.contains(attribute), "{cookie_header}");
        }
        let cookie = cookie_header.split(';').next().unwrap().to_owned();
        let url: url::Url = response.headers()[LOCATION]
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query["redirect_uri"], "https://relay.example/callback");
        assert_eq!(query["code_challenge_method"], "S256");
        let relay: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(&query["state"]).unwrap()).unwrap();
        assert_eq!(
            relay["return_to"],
            "https://terrier.example/api/auth/callback"
        );
        let mut exchange = self.provider.exchange.lock().unwrap();
        exchange.nonce = query["nonce"].clone();
        exchange.challenge = query["code_challenge"].clone();
        (cookie, query["state"].clone())
    }

    async fn request(&self, path: &str, cookie: Option<&str>) -> Response {
        let mut request = Request::get(path);
        if let Some(cookie) = cookie {
            request = request.header(COOKIE, cookie);
        }
        self.app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn callback(&self, cookie: &str, state: &str) -> Response {
        self.request(
            &format!("/api/auth/callback?code=test-code&state={state}"),
            Some(cookie),
        )
        .await
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

fn cookie(response: &Response) -> String {
    response.headers()[SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

fn session_id(cookie: &str) -> Id {
    cookie.split_once('=').unwrap().1.parse().unwrap()
}

#[tokio::test]
async fn callback_rotates_session_and_preserves_destination() {
    let fixture = Fixture::new().await;
    let (pending, state) = fixture.begin().await;
    let response = fixture.callback(&pending, &state).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[LOCATION], "/api/auth/login");
    let authenticated = cookie(&response);
    assert_ne!(pending, authenticated);
    assert!(
        fixture
            .store
            .load(&session_id(&pending))
            .await
            .unwrap()
            .is_none()
    );
    let record = fixture.record(&authenticated).await;
    assert_eq!(record.data[RETURN_TO], "/dashboard");
    assert!(record.data[CSRF].as_str().is_some());
    let lifetime = record.expiry_date - OffsetDateTime::now_utc();
    assert!(lifetime.whole_seconds() >= 172798 && lifetime.whole_seconds() <= 172800);
    let mut aging = record;
    aging.expiry_date -= tower_sessions::cookie::time::Duration::hours(1);
    fixture.store.save(&aging).await.unwrap();
    assert_eq!(
        fixture
            .request("/probe", Some(&authenticated))
            .await
            .status(),
        StatusCode::OK
    );
    let renewed = fixture.record(&authenticated).await;
    assert!(
        renewed.expiry_date
            > aging.expiry_date + tower_sessions::cookie::time::Duration::minutes(59)
    );
    assert_eq!(
        fixture.request("/probe", Some(&pending)).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let sync_failure = fixture
        .request("/api/auth/login", Some(&authenticated))
        .await;
    assert_eq!(sync_failure.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(sync_failure.into_body(), 4096).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap()["code"],
        "internal_error"
    );
    assert_eq!(
        fixture.callback(&authenticated, &state).await.status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn explicit_destination_starts_a_fresh_login_for_an_authenticated_session() {
    let fixture = Fixture::new().await;
    let (pending, state) = fixture.begin().await;
    let response = fixture.callback(&pending, &state).await;
    let authenticated = cookie(&response);
    let response = fixture
        .request(
            "/api/auth/login?return_to=%2Fsettings",
            Some(&authenticated),
        )
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(
        response.headers()[LOCATION]
            .to_str()
            .unwrap()
            .starts_with(&fixture.provider.url)
    );
    let record = fixture.record(&cookie(&response)).await;
    assert_eq!(record.data[RETURN_TO], "/settings");
}

#[tokio::test]
async fn rejects_state_nonce_issuer_audience_and_expired_tokens() {
    let fixture = Fixture::new().await;
    let (pending, _) = fixture.begin().await;
    assert_eq!(
        fixture.callback(&pending, "forged-state").await.status(),
        StatusCode::BAD_REQUEST
    );
    for (claim, value) in [
        ("nonce", json!("wrong")),
        ("iss", json!("https://other.example")),
        ("aud", json!("other-client")),
        ("exp", json!(0)),
    ] {
        let fixture = Fixture::new().await;
        let (pending, state) = fixture.begin().await;
        fixture
            .provider
            .exchange
            .lock()
            .unwrap()
            .overrides
            .insert(claim.into(), value);
        let response = fixture.callback(&pending, &state).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{claim}");
        assert_eq!(
            fixture.request("/probe", Some(&pending)).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
}

#[tokio::test]
async fn refresh_rejection_is_anonymous_but_provider_failure_is_an_error() {
    for (failure, expected) in [
        (None, StatusCode::OK),
        (Some(StatusCode::BAD_REQUEST), StatusCode::UNAUTHORIZED),
        (
            Some(StatusCode::SERVICE_UNAVAILABLE),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
    ] {
        let fixture = Fixture::new().await;
        let (pending, state) = fixture.begin().await;
        let response = fixture.callback(&pending, &state).await;
        let authenticated = cookie(&response);
        // Replace only the signed token's expiry, keeping the actual session and refresh token.
        let mut record = fixture.record(&authenticated).await;
        let mut exchange = fixture.provider.exchange.lock().unwrap();
        exchange.refresh_status = failure;
        drop(exchange);
        let raw = record
            .data
            .get_mut(OIDC_KEY)
            .unwrap()
            .get_mut("Authenticated")
            .unwrap();
        let encoded = raw["authenticated"]["id_token"].as_str().unwrap();
        let payload = URL_SAFE_NO_PAD
            .decode(encoded.split('.').nth(1).unwrap())
            .unwrap();
        let mut claims: Value = serde_json::from_slice(&payload).unwrap();
        claims["exp"] = json!(0);
        let expired = axum_oidc::IdToken::<Claims>::new(
            serde_json::from_value(claims).unwrap(),
            &CoreHmacKey::new(CLIENT_SECRET),
            CoreJwsSigningAlgorithm::HmacSha256,
            None,
            None,
        )
        .unwrap();
        raw["authenticated"]["id_token"] = json!(expired.to_string());
        fixture.store.save(&record).await.unwrap();
        let response = fixture.request("/probe", Some(&authenticated)).await;
        assert_eq!(response.status(), expected);
        assert_eq!(fixture.provider.exchange.lock().unwrap().refreshes, 1);
    }
}

#[tokio::test]
async fn logout_requires_csrf_and_deletes_the_session() {
    let fixture = Fixture::new().await;
    let (pending, state) = fixture.begin().await;
    let response = fixture.callback(&pending, &state).await;
    let authenticated = cookie(&response);
    let record = fixture.record(&authenticated).await;
    let csrf = record.data[CSRF].as_str().unwrap();
    for (origin, supplied, expected) in [
        ("https://evil.example", csrf, StatusCode::FORBIDDEN),
        ("https://terrier.example", "forged", StatusCode::FORBIDDEN),
        ("https://terrier.example", csrf, StatusCode::NO_CONTENT),
    ] {
        let response = fixture
            .app
            .clone()
            .oneshot(
                Request::post("/api/auth/logout")
                    .header(COOKIE, &authenticated)
                    .header(header::ORIGIN, origin)
                    .header("x-csrf-token", supplied)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert!(
        fixture
            .store
            .load(&session_id(&authenticated))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fixture
            .request("/probe", Some(&authenticated))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn rejects_external_and_ambiguous_return_paths() {
    for path in [
        "https://evil.example",
        "//evil.example",
        "/\\evil.example",
        "dashboard",
        "/api/auth/login",
        "/#fragment",
        "/\r\nLocation:evil",
    ] {
        assert!(local_return_path(path).is_err(), "{path:?}");
    }
    assert_eq!(
        local_return_path("/dashboard?tab=profile").unwrap(),
        "/dashboard?tab=profile"
    );
}
