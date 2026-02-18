pub mod federation_index;

use crate::error::Error;
use crate::state::AppState;
use askama::Template;
use axum::Json;
use axum::extract::{Query, State};
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "select.html")]
struct SelectTemplate {
    session_id: String,
}

impl IntoResponse for SelectTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!(error = %e, "failed to render template");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "template error",
                )
                    .into_response()
            }
        }
    }
}

#[derive(Deserialize)]
pub struct DiscoveryParams {
    pub session: String,
}

#[derive(Deserialize)]
pub struct DiscoveryForm {
    pub session_id: String,
    pub entity_id: String,
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
}

/// Renders the university selection form. The user searches for and selects
/// their university, which POSTs back to this module for session update.
pub async fn discovery_page(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DiscoveryParams>,
) -> Result<impl IntoResponse, Error> {
    let _session = state
        .sessions
        .get(&params.session)
        .ok_or_else(|| Error::SessionNotFound(params.session.clone()))?;

    Ok(SelectTemplate {
        session_id: params.session,
    })
}

/// Receives the university selection form submission, updates the session with
/// the chosen entity ID, and redirects to the SP initiation endpoint to begin
/// authentication with the university IdP.
pub async fn discovery_submit(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<DiscoveryForm>,
) -> Result<impl IntoResponse, Error> {
    if !state
        .sessions
        .update_university(&form.session_id, form.entity_id)
    {
        return Err(Error::SessionNotFound(form.session_id));
    }

    Ok(Redirect::to(&format!(
        "/sp/initiate?session={}",
        form.session_id
    )))
}

const SEARCH_RESULT_LIMIT: usize = 20;

/// Searches InCommon IdP entities by display name, returning matching
/// institutions as JSON for the discovery UI's typeahead. Results come from
/// an in-memory index refreshed periodically from the MDQ aggregate.
pub async fn search_entities(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<Response, Error> {
    let results = state
        .federation_index
        .search(&params.q, SEARCH_RESULT_LIMIT)
        .await;
    Ok(Json(results).into_response())
}
