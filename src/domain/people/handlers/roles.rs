use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::core::auth::{
    context::RequestContext, middleware::SyncedUser, permissions::Permissions,
};

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
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin(&ctx).await?;

    let hackathon = ctx.hackathon()?;

    // Delete the user's role entry for this hackathon
    crate::entities::prelude::UserHackathonRoles::delete_many()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user_id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .exec(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to remove user: {}", e)))?;

    Ok(())
}
