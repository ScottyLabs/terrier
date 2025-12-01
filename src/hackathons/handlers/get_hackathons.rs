use dioxus::prelude::*;

use crate::hackathons::HackathonInfo;

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
    use dioxus::fullstack::{FullstackContext, extract::State};
    use sea_orm::EntityTrait;

    // Extract state from context
    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch all hackathons
    let hackathons = crate::entities::prelude::Hackathons::find()
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathons: {}", e)))?;

    Ok(hackathons
        .into_iter()
        .map(|h| HackathonInfo {
            id: h.id,
            name: h.name,
            slug: h.slug,
            description: h.description,
            start_date: h.start_date,
            end_date: h.end_date,
            is_active: h.is_active,
            max_team_size: h.max_team_size,
            banner_url: h.banner_url,
            updated_at: h.updated_at,
            form_config: h.form_config,
        })
        .collect())
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
    use dioxus::fullstack::{FullstackContext, extract::State};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // Extract state from context
    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon by slug
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?;

    Ok(hackathon.map(|h| HackathonInfo {
        id: h.id,
        name: h.name,
        slug: h.slug,
        description: h.description,
        start_date: h.start_date,
        end_date: h.end_date,
        is_active: h.is_active,
        max_team_size: h.max_team_size,
        banner_url: h.banner_url,
        form_config: h.form_config,
        updated_at: h.updated_at,
    }))
}
