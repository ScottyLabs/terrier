use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::{AppState, auth::middleware::SyncedUser};
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, Set};

use crate::domain::applications::types::BulkUpdateApplicationsRequest;

/// Accept multiple applications
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/applications/accept",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = BulkUpdateApplicationsRequest,
    responses(
        (status = 200, description = "Applications accepted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Requires admin or organizer role"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[post("/api/hackathons/:slug/applications/accept", user: SyncedUser)]
pub async fn accept_applications(
    slug: String,
    application_ids: Vec<i32>,
) -> Result<(), ServerFnError> {
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
        return Err(ServerFnError::new("Requires admin or organizer role"));
    }

    // Update all applications to accepted status
    let applications = crate::entities::prelude::Applications::find()
        .filter(crate::entities::applications::Column::Id.is_in(application_ids))
        .filter(crate::entities::applications::Column::HackathonId.eq(hackathon.id))
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch applications: {}", e)))?;

    for app in applications {
        let mut app: crate::entities::applications::ActiveModel = app.into();
        app.status = Set("accepted".to_string());
        app.updated_at = Set(Utc::now().naive_utc());
        app.update(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update application: {}", e)))?;
    }

    Ok(())
}

/// Reject multiple applications
/// Only admins and organizers can reject applications
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/applications/reject",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = BulkUpdateApplicationsRequest,
    responses(
        (status = 200, description = "Applications rejected successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Requires admin or organizer role"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[post("/api/hackathons/:slug/applications/reject", user: SyncedUser)]
pub async fn reject_applications(
    slug: String,
    application_ids: Vec<i32>,
) -> Result<(), ServerFnError> {
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
        return Err(ServerFnError::new("Requires admin or organizer role"));
    }

    // Update all applications to rejected status
    let applications = crate::entities::prelude::Applications::find()
        .filter(crate::entities::applications::Column::Id.is_in(application_ids))
        .filter(crate::entities::applications::Column::HackathonId.eq(hackathon.id))
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch applications: {}", e)))?;

    for app in applications {
        let mut app: crate::entities::applications::ActiveModel = app.into();
        app.status = Set("rejected".to_string());
        app.updated_at = Set(Utc::now().naive_utc());
        app.update(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update application: {}", e)))?;
    }

    Ok(())
}
