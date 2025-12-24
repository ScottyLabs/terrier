use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domain::hackathons::types::HackathonInfo;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct UpdateHackathonRequest {
    pub name: String,
    pub description: String,
    pub max_team_size: i32,
}

#[cfg(feature = "server")]
use crate::auth::middleware::SyncedUser;
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, Set};

/// Update a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Hackathon updated successfully", body = HackathonInfo),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[put("/api/hackathons/:slug", user: SyncedUser)]
pub async fn update_hackathon(
    slug: String,
    req: UpdateHackathonRequest,
) -> Result<HackathonInfo, ServerFnError> {
    use crate::AppState;
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
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Check if user is global admin or hackathon admin
    let is_global_admin = state
        .config
        .admin_emails
        .contains(&user.0.email.to_lowercase());

    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?;

    let is_hackathon_admin = user_role
        .as_ref()
        .map(|r| r.role == "admin")
        .unwrap_or(false);

    if !is_global_admin && !is_hackathon_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    // Update hackathon
    let mut hackathon: crate::entities::hackathons::ActiveModel = hackathon.into();
    hackathon.name = Set(req.name.clone());
    hackathon.description = Set(Some(req.description.clone()));
    hackathon.max_team_size = Set(req.max_team_size);
    hackathon.updated_at = Set(Utc::now().naive_utc());

    let hackathon = hackathon
        .update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update hackathon: {}", e)))?;

    Ok(HackathonInfo {
        id: hackathon.id,
        name: hackathon.name,
        slug: hackathon.slug,
        description: hackathon.description,
        start_date: hackathon.start_date,
        end_date: hackathon.end_date,
        is_active: hackathon.is_active,
        max_team_size: hackathon.max_team_size,
        banner_url: hackathon.banner_url,
        background_url: hackathon.background_url,
        updated_at: hackathon.updated_at,
        form_config: hackathon.form_config,
    })
}

/// Delete a hackathon banner
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/banner",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Banner deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[delete("/api/hackathons/:slug/banner", user: SyncedUser)]
pub async fn delete_banner(slug: String) -> Result<(), ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // Extract state from context
    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon by slug
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Check if user is global admin or hackathon admin
    let is_global_admin = state
        .config
        .admin_emails
        .contains(&user.0.email.to_lowercase());

    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?;

    let is_hackathon_admin = user_role
        .as_ref()
        .map(|r| r.role == "admin")
        .unwrap_or(false);

    if !is_global_admin && !is_hackathon_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    // Delete banner if exists
    if let Some(old_url) = &hackathon.banner_url {
        let url_parts: Vec<&str> = old_url.split('/').collect();
        if url_parts.len() >= 2 {
            let object_key = url_parts[url_parts.len() - 2..].join("/");

            use minio::s3::args::RemoveObjectArgs;
            if let Ok(remove_args) = RemoveObjectArgs::new(&state.config.minio_bucket, &object_key)
            {
                state
                    .s3
                    .remove_object(&remove_args)
                    .await
                    .map_err(|e| ServerFnError::new(format!("Failed to delete banner: {}", e)))?;
            }
        }

        // Update hackathon to remove banner URL
        let mut active_hackathon: crate::entities::hackathons::ActiveModel = hackathon.into();
        active_hackathon.banner_url = Set(None);
        active_hackathon.updated_at = Set(Utc::now().naive_utc());
        active_hackathon
            .update(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update hackathon: {}", e)))?;
    }

    Ok(())
}
