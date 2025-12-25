use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::core::auth::{
    context::RequestContext, middleware::SyncedUser, permissions::Permissions,
};

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
    use crate::domain::people::repository::UserRoleRepository;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;

    let hackathon = ctx.hackathon()?;

    // Fetch all user-hackathon roles for this hackathon excluding applicants
    let role_repo = UserRoleRepository::new(&ctx.state.db);
    let roles = role_repo
        .find_all_roles_for_hackathon_excluding_role(hackathon.id, "applicant")
        .await?;

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
