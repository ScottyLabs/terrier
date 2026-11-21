use axum::Router;
use utoipa::{OpenApi, openapi};

use crate::{judging, profile, state::AppState, teams};

pub fn routes() -> (Router<AppState>, openapi::OpenApi) {
    [
        (profile::routes(), profile::ProfileApiDoc::openapi()),
        (teams::routes(), teams::TeamsApiDoc::openapi()),
        (judging::routes(), judging::JudgingApiDoc::openapi()),
    ]
    .into_iter()
    .fold(
        (Router::new(), openapi::OpenApi::default()),
        |(router, mut api), (routes, document)| {
            api.merge(document);
            (router.merge(routes), api)
        },
    )
}
