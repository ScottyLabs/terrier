use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::{AppState, auth::middleware::SyncedUser};

/// Remove a user from a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/people/{user_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("user_id" = i32, Path, description = "User ID to remove")
    ),
    responses(
        (status = 200, description = "User removed successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Requires admin role"),
        (status = 404, description = "Hackathon or user not found"),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[delete("/api/hackathons/:slug/people/:user_id", user: SyncedUser)]
pub async fn remove_hackathon_person(slug: String, user_id: i32) -> Result<(), ServerFnError> {
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

    let is_admin = user_role
        .as_ref()
        .map(|r| r.role == "admin")
        .unwrap_or(false);

    if !is_global_admin && !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    // Delete the user's role entry for this hackathon
    crate::entities::prelude::UserHackathonRoles::delete_many()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .exec(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to remove user: {}", e)))?;

    Ok(())
}
