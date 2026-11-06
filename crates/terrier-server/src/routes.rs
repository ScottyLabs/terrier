use axum::Router;
use utoipa::{OpenApi, openapi};

use crate::{profile, state::AppState};

pub fn routes() -> (Router<AppState>, openapi::OpenApi) {
    [(profile::routes(), profile::ProfileApiDoc::openapi())]
        .into_iter()
        .fold(
            (Router::new(), openapi::OpenApi::default()),
            |(router, mut api), (routes, document)| {
                api.merge(document);
                (router.merge(routes), api)
            },
        )
}
