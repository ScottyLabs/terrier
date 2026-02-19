pub mod attributes;
pub mod config;
pub mod discovery;
pub mod error;
pub mod idp;
pub mod session;
pub mod sp;
pub mod state;

use axum::Router;
use state::AppState;
use std::path::Path;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

pub fn app(state: Arc<AppState>, static_dir: impl AsRef<Path>) -> Router {
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
        .with_state(state)
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(TraceLayer::new_for_http())
}
