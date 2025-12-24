use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::{AppState, auth::middleware::SyncedUser};
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, Set};

/// Decline attendance (change status to declined)
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}/application/decline",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Attendance declined successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Application not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[put("/api/hackathons/:slug/application/decline", user: SyncedUser)]
pub async fn decline_attendance(slug: String) -> Result<(), ServerFnError> {
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

    // Fetch application
    let application = crate::entities::prelude::Applications::find()
        .filter(crate::entities::applications::Column::UserId.eq(user.0.id))
        .filter(crate::entities::applications::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Application not found"))?;

    // Only allow declining accepted applications
    if application.status != "accepted" {
        return Err(ServerFnError::new("Can only decline accepted applications"));
    }

    // Update status to declined
    let mut app: crate::entities::applications::ActiveModel = application.into();
    app.status = Set("declined".to_string());
    app.updated_at = Set(Utc::now().naive_utc());

    app.update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to decline attendance: {}", e)))?;

    Ok(())
}

/// Confirm attendance (change status to confirmed and user role to participant)
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/application/confirm",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Attendance confirmed successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Application not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[post("/api/hackathons/:slug/application/confirm", user: SyncedUser)]
pub async fn confirm_attendance(slug: String) -> Result<(), ServerFnError> {
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

    // Fetch application
    let application = crate::entities::prelude::Applications::find()
        .filter(crate::entities::applications::Column::UserId.eq(user.0.id))
        .filter(crate::entities::applications::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Application not found"))?;

    // Only allow confirming accepted applications
    if application.status != "accepted" {
        return Err(ServerFnError::new("Can only confirm accepted applications"));
    }

    // Update status to confirmed
    let mut app: crate::entities::applications::ActiveModel = application.into();
    app.status = Set("confirmed".to_string());
    app.updated_at = Set(Utc::now().naive_utc());

    app.update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to confirm attendance: {}", e)))?;

    // Change user's role to participant (only if they were applicant, not organizer/admin)
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?;

    if let Some(role) = user_role {
        if role.role == "applicant" {
            let mut role: crate::entities::user_hackathon_roles::ActiveModel = role.into();
            role.role = Set("participant".to_string());
            role.update(&state.db)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to update user role: {}", e)))?;
        }
    }

    Ok(())
}

/// Undo confirmation (change status from confirmed back to accepted)
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}/application/undo-confirmation",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Confirmation undone successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Application not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[put("/api/hackathons/:slug/application/undo-confirmation", user: SyncedUser)]
pub async fn undo_confirmation(slug: String) -> Result<(), ServerFnError> {
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

    // Fetch application
    let application = crate::entities::prelude::Applications::find()
        .filter(crate::entities::applications::Column::UserId.eq(user.0.id))
        .filter(crate::entities::applications::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Application not found"))?;

    // Only allow undoing confirmed applications
    if application.status != "confirmed" {
        return Err(ServerFnError::new("Can only undo confirmed applications"));
    }

    // Update status back to accepted
    let mut app: crate::entities::applications::ActiveModel = application.into();
    app.status = Set("accepted".to_string());
    app.updated_at = Set(Utc::now().naive_utc());

    app.update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to undo confirmation: {}", e)))?;

    // Change user's role back to applicant (only if they were participant)
    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user role: {}", e)))?;

    if let Some(role) = user_role {
        if role.role == "participant" {
            let mut role: crate::entities::user_hackathon_roles::ActiveModel = role.into();
            role.role = Set("applicant".to_string());
            role.update(&state.db)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to update user role: {}", e)))?;
        }
    }

    Ok(())
}
