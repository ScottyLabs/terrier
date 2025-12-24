use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::{AppState, auth::middleware::SyncedUser};

use crate::domain::applications::types::ApplicationWithUser;

/// Get all applications for a hackathon for review
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/applications",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Applications retrieved successfully", body = Vec<ApplicationWithUser>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Requires admin or organizer role"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[get("/api/hackathons/:slug/applications", user: SyncedUser)]
pub async fn get_all_applications(slug: String) -> Result<Vec<ApplicationWithUser>, ServerFnError> {
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
        return Err(ServerFnError::new("Admin or organizer access required"));
    }

    // Fetch all applications with user information
    let applications = crate::entities::prelude::Applications::find()
        .filter(crate::entities::applications::Column::HackathonId.eq(hackathon.id))
        .find_also_related(crate::entities::prelude::Users)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch applications: {}", e)))?;

    let results = applications
        .into_iter()
        .filter_map(|(app, user_opt)| {
            user_opt.map(|user| ApplicationWithUser {
                id: app.id,
                user_id: app.user_id,
                user_name: user.name,
                user_email: user.email,
                user_picture: user.picture,
                form_data: app.form_data,
                status: app.status,
                created_at: app.created_at.to_string(),
                updated_at: app.updated_at.to_string(),
            })
        })
        .collect();

    Ok(results)
}
