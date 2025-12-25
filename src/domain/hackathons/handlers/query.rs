use dioxus::prelude::*;

use crate::domain::hackathons::types::HackathonInfo;

#[cfg(feature = "server")]
use crate::AppState;

/// Get all hackathons
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons",
    responses(
        (status = 200, description = "List of hackathons", body = Vec<HackathonInfo>),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[get("/api/hackathons")]
pub async fn get_hackathons() -> Result<Vec<HackathonInfo>, ServerFnError> {
    use crate::domain::hackathons::repository::HackathonRepository;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>().await?;
    let repo = HackathonRepository::new(&state.db);

    repo.get_all().await
}

/// Get a hackathon by slug
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Hackathon found", body = Option<HackathonInfo>),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[get("/api/hackathons/:slug")]
pub async fn get_hackathon_by_slug(slug: String) -> Result<Option<HackathonInfo>, ServerFnError> {
    use crate::domain::hackathons::repository::HackathonRepository;
    use dioxus::fullstack::{FullstackContext, extract::State};

    let State(state) = FullstackContext::extract::<State<AppState>, _>().await?;
    let repo = HackathonRepository::new(&state.db);

    repo.find_by_slug(&slug).await.map(|h| h.map(Into::into))
}
