use super::*;
use crate::auth::session::OIDC_KEY;
use axum::{
    body::{Body, to_bytes},
    http::{
        Request,
        header::{COOKIE, LOCATION, SET_COOKIE},
    },
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use openidconnect::{
    core::{CoreHmacKey, CoreJwsSigningAlgorithm},
    core::{CoreIdToken, CoreIdTokenClaims},
};
use serde_json::{Value, json};
use std::collections::HashMap;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionStore, cookie::time::OffsetDateTime, session::Id};

#[path = "../../../tests/common/provider.rs"]
mod provider;
use provider::{CLIENT_SECRET, Provider};

struct Fixture {
    provider: Arc<Provider>,
    client: OidcClient,
    task: tokio::task::JoinHandle<()>,
    app: Router,
    store: MemoryStore,
    relay: bool,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Fixture {
    async fn new() -> Self {
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
        Self::with_db(db).await
    }

    async fn with_db(db: sea_orm::DatabaseConnection) -> Self {
        Self::with_relay(db, true).await
    }

    async fn with_relay(db: sea_orm::DatabaseConnection, relay: bool) -> Self {
        let (provider, task) = Provider::start().await;
        let config = Arc::new(AuthConfig {
            app_url: "https://terrier.example/".parse().unwrap(),
            issuer: provider.url.parse().unwrap(),
            relay_url: relay.then(|| "https://relay.example/callback".parse().unwrap()),
            client_id: "terrier".into(),
            client_secret: CLIENT_SECRET.into(),
            instance_admin_subjects: Default::default(),
        });
        let client = client(&config).await.unwrap();
        let probe = Router::new()
            .route(
                "/probe",
                get(|claims: Option<Extension<CoreIdTokenClaims>>| async move {
                    if claims.is_some() {
                        StatusCode::OK
                    } else {
                        StatusCode::UNAUTHORIZED
                    }
                }),
            )
            .layer(middleware::from_fn_with_state(client.clone(), authenticate));
        let store = MemoryStore::default();
        let application = super::super::middleware::application_routes(
            crate::routes::routes().0.route(
                "/test",
                get(|user: CurrentUser| async move { user.user().id.to_string() }).post(
                    |audit: crate::audit::Audit, user: CurrentUser| async move {
                        use sea_orm::{ActiveModelTrait, Set};
                        let id = user.user().id;
                        crate::audited!(audit, crate::audit::Scope::Global, |db| {
                            crate::entities::user::ActiveModel {
                                id: Set(id),
                                display_name: Set("Updated member".into()),
                                ..Default::default()
                            }
                            .update(db)
                            .await?;
                            Ok(())
                        })
                        .await?;
                        Ok::<_, ApiError>(StatusCode::NO_CONTENT)
                    },
                ),
            ),
            AppState { db: db.clone() },
            config.clone(),
            client.clone(),
        );
        let app = routes(config, client.clone(), AppState { db: db.clone() })
            .nest("/api", application)
            .merge(probe)
            .layer(super::super::session::layer(store.clone(), true))
            .layer(middleware::from_fn_with_state(
                crate::auth::session::lock::MemoryLocks::default(),
                crate::auth::session::lock::serialize::<crate::auth::session::lock::MemoryLocks>,
            ))
            .layer(middleware::from_fn(super::super::session::error_response))
            .with_state(AppState { db });
        Self {
            provider,
            client,
            task,
            app,
            store,
            relay,
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
        assert_eq!(
            query["redirect_uri"],
            if self.relay {
                "https://relay.example/callback"
            } else {
                "https://terrier.example/api/auth/callback"
            }
        );
        assert_eq!(query["code_challenge_method"], "S256");
        if self.relay {
            let relay: Value =
                serde_json::from_slice(&URL_SAFE_NO_PAD.decode(&query["state"]).unwrap()).unwrap();
            assert_eq!(
                relay["return_to"],
                "https://terrier.example/api/auth/callback"
            );
        }
        let mut exchange = self.provider.exchange.lock().unwrap();
        exchange.redirect_uri = query["redirect_uri"].clone();
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

    async fn expire(&self, authenticated: &str) {
        let mut record = self.record(authenticated).await;
        let raw = record
            .data
            .get_mut(OIDC_KEY)
            .unwrap()
            .get_mut("id_token")
            .unwrap();
        let encoded = raw.as_str().unwrap();
        let payload = URL_SAFE_NO_PAD
            .decode(encoded.split('.').nth(1).unwrap())
            .unwrap();
        let mut claims: Value = serde_json::from_slice(&payload).unwrap();
        claims["exp"] = json!(0);
        let expired = CoreIdToken::new(
            serde_json::from_value(claims).unwrap(),
            &CoreHmacKey::new(CLIENT_SECRET),
            CoreJwsSigningAlgorithm::HmacSha256,
            None,
            None,
        )
        .unwrap();
        *raw = json!(expired.to_string());
        self.store.save(&record).await.unwrap();
    }

    async fn callback(&self, cookie: &str, state: &str) -> Response {
        self.request(
            &format!("/api/auth/callback?code=test-code&state={state}"),
            Some(cookie),
        )
        .await
    }
}

#[tokio::test]
async fn provider_metadata_is_refreshed_after_the_cache_interval() {
    let fixture = Fixture::new().await;
    fixture.provider.discovered.notified().await;
    tokio::time::pause();
    let refresh = tokio::spawn(refresh_metadata(fixture.client.clone()));
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    tokio::time::advance(std::time::Duration::from_secs(15 * 60)).await;
    tokio::time::resume();
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        fixture.provider.discovered.notified(),
    )
    .await
    .unwrap();
    refresh.abort();
}

#[test]
fn session_errors_keep_backend_details_but_redact_stored_values() {
    let backend = tower_sessions::session::Error::Store(
        tower_sessions::session_store::Error::Backend("connection refused".into()),
    );
    let ApiError::Internal(error) = session_error(backend) else {
        panic!("expected internal error")
    };
    assert!(format!("{error:#}").contains("connection refused"));
    let error = serde_json::from_str::<bool>("\"private-token\"").unwrap_err();
    let ApiError::Internal(error) = session_error(tower_sessions::session::Error::SerdeJson(error))
    else {
        panic!("expected internal error")
    };
    assert!(!format!("{error:#}").contains("private-token"));
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
        ("azp", json!("other-client")),
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
        fixture.provider.exchange.lock().unwrap().refresh_status = failure;
        fixture.expire(&authenticated).await;
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
                    .header(axum::http::header::ORIGIN, origin)
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

#[tokio::test]
async fn anonymous_status_and_application_requests_have_distinct_responses() {
    let fixture = Fixture::new().await;
    let response = fixture.request("/api/auth/status", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[axum::http::header::CACHE_CONTROL],
        "no-store"
    );
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap(),
        json!({"user": null, "csrf_token": null})
    );
    assert_eq!(
        fixture.request("/api/test", None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let response = fixture.request("/api/missing", None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let (pending, state) = fixture.begin().await;
    let response = fixture.callback(&pending, &state).await;
    let authenticated = cookie(&response);
    assert_eq!(
        fixture
            .request("/api/auth/status", Some(&authenticated))
            .await
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        fixture
            .request("/api/test", Some(&authenticated))
            .await
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn authenticated_requests_enforce_ownership_csrf_and_audit() -> anyhow::Result<()> {
    use sea_orm::ConnectionTrait;
    let url = std::env::var("DATABASE_URL")?;
    let admin = sea_orm::Database::connect(&url).await?;
    let schema = format!("auth_{}", uuid::Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await?;
    let mut options = sea_orm::ConnectOptions::new(url);
    options.set_schema_search_path(&schema);
    let state = AppState::connect(options).await?;
    let fixture = Fixture::with_db(state.db).await;
    let (pending, state) = fixture.begin().await;
    fixture
        .provider
        .exchange
        .lock()
        .unwrap()
        .overrides
        .insert("email_verified".into(), json!(false));
    let response = fixture.callback(&pending, &state).await;
    let authenticated = cookie(&response);
    let status = fixture
        .request("/api/auth/status", Some(&authenticated))
        .await;
    assert_eq!(status.status(), StatusCode::OK);
    let body = to_bytes(status.into_body(), 4096).await?;
    let status: Value = serde_json::from_slice(&body)?;
    assert!(status["user"].get("oidc_subject").is_none());
    let response = fixture.request("/api/test", Some(&authenticated)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let own_path = format!("/api/users/{}", status["user"]["id"].as_str().unwrap());
    let own = fixture.request(&own_path, Some(&authenticated)).await;
    assert_eq!(own.status(), StatusCode::OK);
    let own: Value = serde_json::from_slice(&to_bytes(own.into_body(), 4096).await?)?;
    assert_eq!(
        own,
        json!({"id": status["user"]["id"], "email": "member@example.com", "display_name": "Member"})
    );
    assert_eq!(
        fixture.request(&own_path, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        fixture
            .request(
                &format!("/api/users/{}", uuid::Uuid::new_v4()),
                Some(&authenticated)
            )
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        fixture
            .request("/api/users/not-a-uuid", Some(&authenticated))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    for (origin, csrf, expected) in [
        ("https://terrier.example", "forged", StatusCode::FORBIDDEN),
        (
            "https://evil.example",
            status["csrf_token"].as_str().unwrap(),
            StatusCode::FORBIDDEN,
        ),
        (
            "https://terrier.example",
            status["csrf_token"].as_str().unwrap(),
            StatusCode::NO_CONTENT,
        ),
    ] {
        let response = fixture
            .app
            .clone()
            .oneshot(
                Request::post("/api/test")
                    .header(COOKIE, &authenticated)
                    .header("origin", origin)
                    .header("x-csrf-token", csrf)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), expected);
    }
    let audit = admin
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            format!("SELECT count(*) AS count FROM {schema}.audit_event WHERE actor_user_id IS NOT NULL AND resource_type = 'user' AND after_data->>'display_name' = 'Updated member'"),
        ))
        .await?
        .expect("count query returns one row");
    assert_eq!(audit.try_get::<i64>("", "count")?, 1);
    drop(fixture);
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await?;
    Ok(())
}

#[tokio::test]
async fn direct_callbacks_do_not_require_a_relay() {
    let fixture = Fixture::with_relay(sea_orm::DatabaseConnection::default(), false).await;
    let (pending, state) = fixture.begin().await;
    let response = fixture.callback(&pending, &state).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        fixture
            .request("/probe", Some(&cookie(&response)))
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn expired_and_denied_logins_cannot_be_completed() {
    let fixture = Fixture::new().await;
    let (pending, state) = fixture.begin().await;
    let mut record = fixture.record(&pending).await;
    record.data.get_mut(client::PENDING).unwrap()["created_at"] =
        json!(OffsetDateTime::now_utc().unix_timestamp() - 600);
    fixture.store.save(&record).await.unwrap();
    assert_eq!(
        fixture.callback(&pending, &state).await.status(),
        StatusCode::BAD_REQUEST
    );

    let (pending, state) = fixture.begin().await;
    assert_eq!(
        fixture
            .request(
                &format!("/api/auth/callback?error=access_denied&state={state}"),
                Some(&pending)
            )
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        fixture.callback(&pending, &state).await.status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn callback_rejects_invalid_signatures_hashes_and_missing_id_tokens() {
    for failure in ["signature", "hash", "missing"] {
        let fixture = Fixture::new().await;
        let (pending, state) = fixture.begin().await;
        {
            let mut exchange = fixture.provider.exchange.lock().unwrap();
            match failure {
                "signature" => exchange.signing_secret = Some("other-secret".into()),
                "hash" => {
                    exchange
                        .overrides
                        .insert("at_hash".into(), json!("invalid-hash"));
                }
                _ => exchange.omit_id_token = true,
            }
        }
        let expected = if failure == "missing" {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::BAD_REQUEST
        };
        assert_eq!(
            fixture.callback(&pending, &state).await.status(),
            expected,
            "{failure}"
        );
        assert_eq!(
            fixture.request("/probe", Some(&pending)).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            fixture.callback(&pending, &state).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn refresh_rejects_changed_identity_nonce_and_invalid_tokens() {
    for (claim, value) in [
        ("sub", json!("another-user")),
        ("auth_time", json!(123)),
        ("nonce", json!("another-login")),
        ("iss", json!("https://other.example")),
        ("aud", json!("other-client")),
        ("azp", json!("other-client")),
        ("at_hash", json!("invalid-hash")),
    ] {
        let fixture = Fixture::new().await;
        let (pending, state) = fixture.begin().await;
        let response = fixture.callback(&pending, &state).await;
        let authenticated = cookie(&response);
        fixture.expire(&authenticated).await;
        fixture
            .provider
            .exchange
            .lock()
            .unwrap()
            .overrides
            .insert(claim.into(), value);
        assert_eq!(
            fixture
                .request("/probe", Some(&authenticated))
                .await
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "{claim}"
        );
    }
}

#[tokio::test]
async fn refresh_allows_omitted_nonce_and_preserves_an_unrotated_refresh_token() {
    let fixture = Fixture::new().await;
    let (pending, state) = fixture.begin().await;
    let response = fixture.callback(&pending, &state).await;
    let authenticated = cookie(&response);
    fixture.expire(&authenticated).await;
    {
        let mut exchange = fixture.provider.exchange.lock().unwrap();
        exchange.omit_nonce = true;
        exchange.omit_refresh_token = true;
    }
    for _ in 0..2 {
        assert_eq!(
            fixture
                .request("/probe", Some(&authenticated))
                .await
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(fixture.provider.exchange.lock().unwrap().refreshes, 1);
    fixture.expire(&authenticated).await;
    assert_eq!(
        fixture
            .request("/probe", Some(&authenticated))
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(fixture.provider.exchange.lock().unwrap().refreshes, 2);
}

#[tokio::test]
async fn refresh_distinguishes_expired_keycloak_tokens_from_client_misconfiguration() {
    for (error, expected) in [
        ("JwtToken", StatusCode::UNAUTHORIZED),
        ("invalid_client", StatusCode::INTERNAL_SERVER_ERROR),
    ] {
        let fixture = Fixture::new().await;
        let (pending, state) = fixture.begin().await;
        let response = fixture.callback(&pending, &state).await;
        let authenticated = cookie(&response);
        fixture.expire(&authenticated).await;
        {
            let mut exchange = fixture.provider.exchange.lock().unwrap();
            exchange.refresh_status = Some(StatusCode::BAD_REQUEST);
            exchange.refresh_error = Some(error.into());
        }
        assert_eq!(
            fixture
                .request("/probe", Some(&authenticated))
                .await
                .status(),
            expected,
            "{error}"
        );
    }
}

#[tokio::test]
async fn concurrent_requests_share_one_token_refresh() {
    let fixture = Fixture::new().await;
    let (pending, state) = fixture.begin().await;
    let response = fixture.callback(&pending, &state).await;
    let authenticated = cookie(&response);
    fixture.expire(&authenticated).await;
    fixture.provider.exchange.lock().unwrap().refresh_delay = std::time::Duration::from_millis(50);
    let (first, second) = tokio::join!(
        fixture.request("/probe", Some(&authenticated)),
        fixture.request("/probe", Some(&authenticated)),
    );
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(fixture.provider.exchange.lock().unwrap().refreshes, 1);
}

#[tokio::test]
async fn callbacks_validate_the_response_issuer_and_discovery_requirement() {
    let fixture = Fixture::new().await;
    let (pending, state) = fixture.begin().await;
    assert_eq!(fixture.request(
        &format!("/api/auth/callback?code=test-code&state={state}&iss=https%3A%2F%2Fother.example"),
        Some(&pending),
    ).await.status(), StatusCode::BAD_REQUEST);
    fixture.provider.exchange.lock().unwrap().require_issuer = true;
    fixture.client.rediscover().await.unwrap();
    assert_eq!(
        fixture.callback(&pending, &state).await.status(),
        StatusCode::BAD_REQUEST
    );
    let issuer: String =
        url::form_urlencoded::byte_serialize(format!("{}/", fixture.provider.url).as_bytes())
            .collect();
    let response = fixture
        .request(
            &format!("/api/auth/callback?code=test-code&state={state}&iss={issuer}"),
            Some(&pending),
        )
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn rotated_refresh_tokens_are_persisted_and_used_by_the_next_refresh() {
    let fixture = Fixture::new().await;
    let (pending, state) = fixture.begin().await;
    let response = fixture.callback(&pending, &state).await;
    let authenticated = cookie(&response);
    fixture.provider.exchange.lock().unwrap().next_refresh_token = Some("rotated-token".into());
    for _ in 0..2 {
        fixture.expire(&authenticated).await;
        assert_eq!(
            fixture
                .request("/probe", Some(&authenticated))
                .await
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        fixture.provider.exchange.lock().unwrap().refresh_requests,
        ["test-refresh-token", "rotated-token"]
    );
}

#[tokio::test]
async fn concurrent_requests_with_encoded_cookie_names_share_the_session_lock() {
    let fixture = Fixture::new().await;
    let (pending, state) = fixture.begin().await;
    let response = fixture.callback(&pending, &state).await;
    let authenticated = cookie(&response);
    let encoded = authenticated.replace("terrier_session=", "terrier%5Fsession=");
    fixture.expire(&authenticated).await;
    fixture.provider.exchange.lock().unwrap().refresh_delay = std::time::Duration::from_millis(50);
    let (first, second) = tokio::join!(
        fixture.request("/probe", Some(&authenticated)),
        fixture.request("/probe", Some(&encoded)),
    );
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(fixture.provider.exchange.lock().unwrap().refreshes, 1);
}

#[tokio::test]
async fn logout_during_a_refresh_cannot_restore_the_authenticated_session() {
    let fixture = Fixture::new().await;
    let (pending, state) = fixture.begin().await;
    let response = fixture.callback(&pending, &state).await;
    let authenticated = cookie(&response);
    let record = fixture.record(&authenticated).await;
    let csrf = record.data[CSRF].as_str().unwrap();
    fixture.expire(&authenticated).await;
    fixture.provider.exchange.lock().unwrap().refresh_delay = std::time::Duration::from_millis(50);
    let logout = fixture.app.clone().oneshot(
        Request::post("/api/auth/logout")
            .header(COOKIE, &authenticated)
            .header("x-csrf-token", csrf)
            .body(Body::empty())
            .unwrap(),
    );
    let (_, logged_out) = tokio::join!(fixture.request("/probe", Some(&authenticated)), logout);
    assert_eq!(logged_out.unwrap().status(), StatusCode::NO_CONTENT);
    assert_eq!(
        fixture
            .request("/probe", Some(&authenticated))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert!(
        fixture
            .store
            .load(&session_id(&authenticated))
            .await
            .unwrap()
            .is_none()
    );
}
