use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::{AppState, auth::middleware::SyncedUser};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct HackathonPerson {
    pub user_id: i32,
    pub name: Option<String>,
    pub email: String,
    pub picture: Option<String>,
    pub role: String,
    pub team_id: Option<i32>,
}

/// Get all people associated with a hackathon, excluding applicants
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/people",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "People retrieved successfully", body = Vec<HackathonPerson>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Requires admin or organizer role"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[get("/api/hackathons/:slug/people", user: SyncedUser)]
pub async fn get_hackathon_people(slug: String) -> Result<Vec<HackathonPerson>, ServerFnError> {
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

    // Fetch all user-hackathon roles for this hackathon excluding applicants
    let roles = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .filter(crate::entities::user_hackathon_roles::Column::Role.ne("applicant"))
        .find_also_related(crate::entities::prelude::Users)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch roles: {}", e)))?;

    let results = roles
        .into_iter()
        .filter_map(|(role, user_opt)| {
            user_opt.map(|user| HackathonPerson {
                user_id: user.id,
                name: user.name,
                email: user.email,
                picture: user.picture,
                role: role.role,
                team_id: role.team_id,
            })
        })
        .collect();

    Ok(results)
}

// Role management functions moved to roles.rs
