use dioxus::fullstack::reqwest;
use dioxus::prelude::*;
use dioxus::document::eval;
mod auth;
mod config;
#[cfg(feature = "server")]
mod entities;
mod hackathons;
mod pages;
mod types;

mod backend;

pub use pages::Home;
pub use pages::Test;
pub use pages::Application;

#[cfg(feature = "server")]
use config::Config;

#[cfg(feature = "server")]
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: sea_orm::DatabaseConnection,
}

use crate::dioxus_fullstack::FullstackContext;
use crate::dioxus_fullstack::extract::FromRef;
use dioxus::fullstack::extract::State;

#[cfg(feature = "server")]
impl FromRef<FullstackContext> for AppState {
    fn from_ref(state: &FullstackContext) -> Self {
        state.extension::<AppState>().unwrap()
    }
}

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/test")]
    Test {},
    #[route("/hackathon/:id/application")]
    Application {
        id: String,
    },
}

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    #[cfg(feature = "server")]
    {
        use axum::{
            Extension, Router, middleware,
            response::IntoResponse,
            routing::{get, post},
        };
        use axum_oidc::{
            EmptyAdditionalClaims, OidcAuthLayer, OidcClient, OidcLoginLayer,
            error::MiddlewareError, handle_oidc_redirect,
        };
        use dioxus::prelude::{DioxusRouterExt, ServeConfig};
        use http::Uri;
        use openidconnect::{ClientId, ClientSecret};

        use sea_orm::Database;
        use tower::ServiceBuilder;
        use tower_sessions::{
            Expiry, MemoryStore, SessionManagerLayer,
            cookie::{SameSite, time::Duration},
        };
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                // Initialize tracing
                tracing_subscriber::registry()
                    .with(
                        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(
                            |_| "info,terrier=debug,axum_oidc=debug,tower_sessions=debug".into(),
                        ),
                    )
                    .with(tracing_subscriber::fmt::layer())
                    .init();

                // Load configuration
                let config = Config::from_env().expect("Failed to load configuration");
                tracing::info!("Configuration loaded successfully");

                // Connect to database
                let db = Database::connect(&config.database_url)
                    .await
                    .expect("Failed to connect to database");
                tracing::info!("Database connected successfully");

                // Create app state
                let app_state = AppState {
                    config: config.clone(),
                    db,
                };

                // Session Management Setup
                let session_store = MemoryStore::default();
                let session_layer = SessionManagerLayer::new(session_store)
                    .with_secure(false) // Set to true in production with HTTPS
                    .with_same_site(SameSite::Lax)
                    .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

                // OIDC Client Configuration
                let redirect_url = format!("{}/auth/callback", app_state.config.api_url);
                tracing::info!("OIDC redirect URL: {}", redirect_url);

                let oidc_client = OidcClient::<EmptyAdditionalClaims>::builder()
                    .with_default_http_client()
                    .with_redirect_url(Uri::try_from(redirect_url).expect("valid API_URL"))
                    .with_client_id(ClientId::new(app_state.config.oidc_client_id.clone()))
                    .with_client_secret(ClientSecret::new(
                        app_state.config.oidc_client_secret.clone(),
                    ))
                    .with_scopes(["openid", "email", "profile"].into_iter())
                    .discover(app_state.config.oidc_issuer.clone())
                    .await
                    .expect("Failed to discover OIDC provider")
                    .build();

                tracing::info!("OIDC client configured successfully");

                // OIDC Login Layer
                let oidc_login_service = ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        |e: MiddlewareError| async {
                            tracing::error!("OIDC Login error: {:?}", e);
                            e.into_response()
                        },
                    ))
                    .layer(OidcLoginLayer::<EmptyAdditionalClaims>::new());

                // OIDC Auth Layer
                let oidc_auth_service = ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        |e: MiddlewareError| async {
                            tracing::error!("OIDC Auth error: {:?}", e);
                            e.into_response()
                        },
                    ))
                    .layer(OidcAuthLayer::new(oidc_client));

                // Build API router with protected routes
                let api_router = Router::new()
                    // Protected routes (require authentication)
                    .route("/auth/login", get(auth::handlers::login))
                    .route("/auth/logout", get(auth::handlers::logout))
                    // .route("/hackathons", post(hackathons::handlers::create_hackathon))
                    // .route(
                    //     "/hackathons/{slug}/role",
                    //     get(hackathons::handlers::get_user_role),
                    // )
                    // User sync middleware (runs on authenticated requests)
                    .layer(middleware::from_fn_with_state(
                        app_state.clone(),
                        auth::middleware::sync_user_middleware,
                    ))
                    .layer(oidc_login_service.clone())
                    // Public routes (no authentication required)
                    // .route("/auth/status", get(auth::handlers::status))
                    .route(
                        "/auth/callback",
                        get(handle_oidc_redirect::<EmptyAdditionalClaims>),
                    )
                    // .route(
                    //     "/hackathons/public",
                    //     get(hackathons::handlers::list_public_hackathons),
                    // )
                    .route("/health", get(|| async { "OK" }))
                    // Apply OIDC auth and session layers
                    .layer(oidc_auth_service.clone())
                    .layer(session_layer.clone())
                    .with_state(app_state.clone());

                // Create the main router with API routes under /api and Dioxus app
                // Add Extension layer to share state with Dioxus server functions
                let dioxus_router = Router::new()
                    .serve_dioxus_application(ServeConfig::default(), App)
                    .layer(middleware::from_fn_with_state(
                        app_state.clone(),
                        auth::middleware::sync_user_middleware,
                    ))
                    .layer(oidc_login_service.clone())
                    .layer(oidc_auth_service.clone())
                    .layer(session_layer.clone())
                    .layer(Extension(app_state.clone()));

                let router = Router::new().nest("/api", api_router).merge(dioxus_router);

                // Get address from CLI config or default to localhost:8080
                let address = dioxus::cli_config::fullstack_address_or_localhost();
                tracing::info!("Starting server at {}", address);

                let listener = tokio::net::TcpListener::bind(address)
                    .await
                    .expect("Failed to bind to address");

                axum::serve(listener, router.into_make_service())
                    .await
                    .expect("Failed to start server");
            });
    }

    #[cfg(not(feature = "server"))]
    {
        dioxus_logger::init(dioxus_logger::tracing::Level::DEBUG).expect("failed to init logger");
        dioxus::launch(App);
    }
}

#[component]
fn App() -> Element {

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}
