use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schemas::FormSchema;

#[cfg(feature = "server")]
use crate::{AppState, auth::middleware::SyncedUser};
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, Set};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct SetFormConfigRequest {
    pub form_config: FormSchema,
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
    use dioxus::fullstack::{FullstackContext, extract::State};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // Extract state from context
    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Validate form schema
    form_config
        .validate()
        .map_err(|e| ServerFnError::new(format!("Invalid form schema: {}", e)))?;

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

    // Serialize form config to JSON
    let form_config_json = serde_json::to_value(&form_config)
        .map_err(|e| ServerFnError::new(format!("Failed to serialize form config: {}", e)))?;

    // Update hackathon with form config
    let mut hackathon: crate::entities::hackathons::ActiveModel = hackathon.into();
    hackathon.form_config = Set(Some(form_config_json));
    hackathon.updated_at = Set(Utc::now().naive_utc());

    hackathon
        .update(&state.db)
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

    // Get form config
    let form_config_json = hackathon
        .form_config
        .ok_or_else(|| ServerFnError::new("No form config set for this hackathon"))?;

    // Deserialize form config
    let form_config: FormSchema = serde_json::from_value(form_config_json)
        .map_err(|e| ServerFnError::new(format!("Failed to deserialize form config: {}", e)))?;

    Ok(form_config)
}
