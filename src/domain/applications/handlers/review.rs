use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::core::auth::{
    context::RequestContext, middleware::SyncedUser, permissions::Permissions,
};
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
    use crate::domain::applications::repository::ApplicationRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;

    let hackathon = ctx.hackathon()?;

    // Update all applications to accepted status
    let app_repo = ApplicationRepository::new(&ctx.state.db);
    let applications = app_repo
        .find_by_ids_and_hackathon(application_ids, hackathon.id)
        .await?;

    for app in applications {
        let mut app: crate::entities::applications::ActiveModel = app.into();
        app.status = Set("accepted".to_string());
        app.updated_at = Set(Utc::now().naive_utc());
        app.update(&ctx.state.db)
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
    use crate::domain::applications::repository::ApplicationRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;

    let hackathon = ctx.hackathon()?;

    // Update all applications to rejected status
    let app_repo = ApplicationRepository::new(&ctx.state.db);
    let applications = app_repo
        .find_by_ids_and_hackathon(application_ids, hackathon.id)
        .await?;

    for app in applications {
        let mut app: crate::entities::applications::ActiveModel = app.into();
        app.status = Set("rejected".to_string());
        app.updated_at = Set(Utc::now().naive_utc());
        app.update(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update application: {}", e)))?;
    }

    Ok(())
}
