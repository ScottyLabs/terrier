pub mod metadata;
pub mod slo;
pub mod sso;

use crate::state::AppState;
use axum::Router;
use axum::routing::{get, post};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/metadata", get(metadata::metadata))
        .route("/sso", get(sso::sso_redirect).post(sso::sso_post))
        .route("/slo", post(slo::slo_post))
}
