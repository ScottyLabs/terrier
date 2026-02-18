pub mod acs;
pub mod initiate;

use crate::state::AppState;
use axum::Router;
use axum::routing::{get, post};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/initiate", get(initiate::initiate))
        .route("/acs", post(acs::assertion_consumer_service))
}
