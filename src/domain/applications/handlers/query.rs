use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::core::auth::{
    context::RequestContext, middleware::SyncedUser, permissions::Permissions,
};

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
    use crate::domain::applications::repository::ApplicationRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;

    let hackathon = ctx.hackathon()?;

    // Fetch all applications with user information
    let app_repo = ApplicationRepository::new(&ctx.state.db);
    app_repo
        .find_all_with_users_by_hackathon(hackathon.id)
        .await
}
