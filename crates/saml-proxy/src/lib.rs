pub mod attributes;
pub mod config;
pub mod discovery;
pub mod error;
pub mod idp;
pub mod session;
pub mod sp;
pub mod state;

use axum::Router;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use state::AppState;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

static DISCOVERY_JS: &str = include_str!("../static/discovery.js");

async fn discovery_js() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/javascript")],
        DISCOVERY_JS,
    )
}

pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/saml", idp::router())
        .nest("/sp", sp::router())
        .route("/discovery", axum::routing::get(discovery::discovery_page))
        .route(
            "/discovery",
            axum::routing::post(discovery::discovery_submit),
        )
        .route(
            "/api/entities/search",
            axum::routing::get(discovery::search_entities),
        )
        .route("/static/discovery.js", axum::routing::get(discovery_js))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
