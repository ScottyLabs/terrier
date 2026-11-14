use axum::Router;
use utoipa::{OpenApi, openapi};

use crate::{applications, directory, hackathons, organizers, profile, projects, state::AppState};

pub fn routes() -> (Router<AppState>, openapi::OpenApi) {
    [
        (profile::routes(), profile::ProfileApiDoc::openapi()),
        (
            applications::routes(),
            applications::ApplicationsApiDoc::openapi(),
        ),
        (organizers::routes(), organizers::OrganizerApiDoc::openapi()),
        (directory::routes(), directory::DirectoryApiDoc::openapi()),
        (hackathons::routes(), hackathons::HackathonApiDoc::openapi()),
        (projects::routes(), projects::ProjectsApiDoc::openapi()),
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
