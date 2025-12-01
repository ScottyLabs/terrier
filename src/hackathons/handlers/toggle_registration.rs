use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::{AppState, auth::middleware::SyncedUser};
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, Set};

/// Toggle registration status for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/toggle-registration",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Registration status toggled successfully", body = bool),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Requires admin or organizer role"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[post("/api/hackathons/:slug/toggle-registration", user: SyncedUser)]
pub async fn toggle_registration(slug: String) -> Result<bool, ServerFnError> {
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

    // Check if user is global admin
    let is_global_admin = state
        .config
        .admin_emails
        .contains(&user.0.email.to_lowercase());

    // Check user's role in this hackathon
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?;

    let is_admin_or_organizer = user_role
        .as_ref()
        .map(|r| r.role == "admin" || r.role == "organizer")
        .unwrap_or(false);

    if !is_global_admin && !is_admin_or_organizer {
        return Err(ServerFnError::new(
            "Admin or organizer access required",
        ));
    }

    // Toggle is_active
    let new_status = !hackathon.is_active;
    let mut hackathon: crate::entities::hackathons::ActiveModel = hackathon.into();
    hackathon.is_active = Set(new_status);
    hackathon.updated_at = Set(Utc::now().naive_utc());

    hackathon
        .update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update hackathon: {}", e)))?;

    Ok(new_status)
}
