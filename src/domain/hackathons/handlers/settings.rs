use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domain::applications::types::FormSchema;

#[cfg(feature = "server")]
use crate::{
    AppState,
    core::auth::{context::RequestContext, middleware::SyncedUser, permissions::Permissions},
};
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, Set};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct SetFormConfigRequest {
    pub form_config: FormSchema,
}

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
    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;

    let hackathon = ctx.hackathon()?;

    // Toggle is_active
    let new_status = !hackathon.is_active;
    let mut hackathon: crate::entities::hackathons::ActiveModel = hackathon.clone().into();
    hackathon.is_active = Set(new_status);
    hackathon.updated_at = Set(Utc::now().naive_utc());

    hackathon
        .update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update hackathon: {}", e)))?;

    Ok(new_status)
}

/// Set the application form configuration for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}/form-config",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = SetFormConfigRequest,
    responses(
        (status = 200, description = "Form config updated successfully"),
        (status = 400, description = "Invalid form schema"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Requires admin or organizer role"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[put("/api/hackathons/:slug/form-config", user: SyncedUser)]
pub async fn set_form_config(slug: String, form_config: FormSchema) -> Result<(), ServerFnError> {
    // Validate form schema
    form_config
        .validate()
        .map_err(|e| ServerFnError::new(format!("Invalid form schema: {}", e)))?;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;

    let hackathon = ctx.hackathon()?;

    // Serialize form config to JSON
    let form_config_json = serde_json::to_value(&form_config)
        .map_err(|e| ServerFnError::new(format!("Failed to serialize form config: {}", e)))?;

    // Update hackathon with form config
    let mut hackathon: crate::entities::hackathons::ActiveModel = hackathon.clone().into();
    hackathon.form_config = Set(Some(form_config_json));
    hackathon.updated_at = Set(Utc::now().naive_utc());

    hackathon
        .update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update hackathon: {}", e)))?;

    Ok(())
}

/// Get the application form configuration for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/form-config",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Form config retrieved successfully", body = FormSchema),
        (status = 404, description = "Hackathon not found or no form config set"),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[get("/api/hackathons/:slug/form-config")]
pub async fn get_form_config(slug: String) -> Result<FormSchema, ServerFnError> {
    use crate::domain::hackathons::repository::HackathonRepository;
    use dioxus::fullstack::{FullstackContext, extract::State};

    // Extract state from context
    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon by slug
    let hackathon_repo = HackathonRepository::new(&state.db);
    let hackathon = hackathon_repo.find_by_slug_or_error(&slug).await?;

    // Get form config
    let form_config_json = hackathon
        .form_config
        .ok_or_else(|| ServerFnError::new("No form config set for this hackathon"))?;

    // Deserialize form config
    let form_config: FormSchema = serde_json::from_value(form_config_json)
        .map_err(|e| ServerFnError::new(format!("Failed to deserialize form config: {}", e)))?;

    Ok(form_config)
}

/// Set the submission form configuration for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}/submission-form-config",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = SetFormConfigRequest,
    responses(
        (status = 200, description = "Submission form config updated successfully"),
        (status = 400, description = "Invalid form schema"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Requires admin or organizer role"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[put("/api/hackathons/:slug/submission-form-config", user: SyncedUser)]
pub async fn set_submission_form_config(
    slug: String,
    form_config: FormSchema,
) -> Result<(), ServerFnError> {
    // Validate form schema
    form_config
        .validate()
        .map_err(|e| ServerFnError::new(format!("Invalid form schema: {}", e)))?;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;

    let hackathon = ctx.hackathon()?;

    // Serialize form config to JSON
    let form_config_json = serde_json::to_value(&form_config)
        .map_err(|e| ServerFnError::new(format!("Failed to serialize form config: {}", e)))?;

    // Update hackathon with submission form config
    let mut hackathon: crate::entities::hackathons::ActiveModel = hackathon.clone().into();
    hackathon.submission_form = Set(Some(form_config_json));
    hackathon.updated_at = Set(Utc::now().naive_utc());

    hackathon
        .update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update hackathon: {}", e)))?;

    Ok(())
}
