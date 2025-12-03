use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::hackathons::HackathonInfo;

#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct CreateHackathonRequest {
    pub name: String,
    pub description: String,
    pub start_date: String,
    pub end_date: String,
    pub max_team_size: Option<i32>,
}

#[cfg(feature = "server")]
use crate::{AppState, auth::middleware::SyncedUser};
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, Set};

/// Create a new hackathon
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons",
    request_body = CreateHackathonRequest,
    responses(
        (status = 200, description = "Hackathon created successfully", body = HackathonInfo),
        (status = 401, description = "Admin access required"),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[post("/api/hackathons", user: SyncedUser)]
pub async fn create_hackathon(req: CreateHackathonRequest) -> Result<HackathonInfo, ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    // Extract state from context
    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Check if user is admin
    if !state
        .config
        .admin_emails
        .contains(&user.0.email.to_lowercase())
    {
        return Err(ServerFnError::new("Admin access required"));
    }

    if req.name.is_empty() {
        return Err(ServerFnError::new("Hackathon name is required"));
    }

    if req.description.is_empty() {
        return Err(ServerFnError::new("Description is required"));
    }

    // Generate slug from name
    let slug = req.name.to_lowercase().replace(" ", "-");

    // Parse dates
    let start_date = chrono::NaiveDateTime::parse_from_str(&req.start_date, "%Y-%m-%dT%H:%M")
        .map_err(|e| ServerFnError::new(format!("Invalid start date format: {}", e)))?;
    let end_date = chrono::NaiveDateTime::parse_from_str(&req.end_date, "%Y-%m-%dT%H:%M")
        .map_err(|e| ServerFnError::new(format!("Invalid end date format: {}", e)))?;

    if end_date <= start_date {
        return Err(ServerFnError::new("End date must be after start date"));
    }

    // Create hackathon
    let now = Utc::now().naive_utc();
    let hackathon = crate::entities::hackathons::ActiveModel {
        name: Set(req.name.clone()),
        slug: Set(slug.clone()),
        description: Set(Some(req.description.clone())),
        start_date: Set(start_date),
        end_date: Set(end_date),
        is_active: Set(false),
        max_team_size: Set(req.max_team_size.unwrap_or(4)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let hackathon = hackathon
        .insert(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create hackathon: {}", e)))?;

    // Assign creator as admin
    let user_hackathon_role = crate::entities::user_hackathon_roles::ActiveModel {
        user_id: Set(user.0.id),
        hackathon_id: Set(hackathon.id),
        role: Set("admin".to_string()),
        ..Default::default()
    };

    user_hackathon_role
        .insert(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to assign admin role: {}", e)))?;

    Ok(HackathonInfo {
        id: hackathon.id,
        name: hackathon.name,
        slug: hackathon.slug,
        description: hackathon.description,
        start_date: hackathon.start_date,
        end_date: hackathon.end_date,
        is_active: hackathon.is_active,
        max_team_size: hackathon.max_team_size,
        banner_url: hackathon.banner_url,
        updated_at: hackathon.updated_at,
        form_config: hackathon.form_config,
    })
}

/// Upload a banner for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}/banner",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Banner uploaded successfully", body = String),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "hackathons"
))]
#[put("/api/hackathons/:slug/banner", user: SyncedUser)]
pub async fn upload_banner(
    slug: String,
    file_data: Vec<u8>,
    content_type: String,
) -> Result<String, ServerFnError> {
    use crate::AppState;
    use dioxus::fullstack::{FullstackContext, extract::State};

    // Extract state from context
    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Use shared implementation
    upload_banner_impl(&state, &slug, &user, file_data, content_type)
        .await
        .map_err(ServerFnError::new)
}

/// Shared implementation for banner upload logic
#[cfg(feature = "server")]
async fn upload_banner_impl(
    state: &AppState,
    slug: &str,
    user: &SyncedUser,
    file_data: Vec<u8>,
    content_type: String,
) -> Result<String, String> {
    use minio::s3::args::{PutObjectArgs, RemoveObjectArgs};
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
    use std::io::Cursor;

    // Fetch hackathon by slug
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(slug))
        .one(&state.db)
        .await
        .map_err(|e| format!("Failed to fetch hackathon: {}", e))?
        .ok_or_else(|| "Hackathon not found".to_string())?;

    // Check if user is global admin or hackathon admin
    let is_global_admin = state
        .config
        .admin_emails
        .contains(&user.0.email.to_lowercase());

    let user_role = crate::entities::prelude::UserHackathonRoles::find()
        .filter(crate::entities::user_hackathon_roles::Column::UserId.eq(user.0.id))
        .filter(crate::entities::user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| format!("Failed to fetch user role: {}", e))?;

    let is_hackathon_admin = user_role
        .as_ref()
        .map(|r| r.role == "admin")
        .unwrap_or(false);

    if !is_global_admin && !is_hackathon_admin {
        return Err("Admin access required".to_string());
    }

    // Delete old banner if exists
    if let Some(old_url) = &hackathon.banner_url {
        tracing::info!("Deleting old banner: {}", old_url);
        let url_parts: Vec<&str> = old_url.split('/').collect();
        if url_parts.len() >= 2 {
            let object_key = url_parts[url_parts.len() - 2..].join("/");

            if let Ok(remove_args) = RemoveObjectArgs::new(&state.config.minio_bucket, &object_key)
            {
                let _ = state.s3.remove_object(&remove_args).await;
                tracing::info!("Old banner deleted: {}", object_key);
            }
        }
    }

    // Upload new banner
    let extension = match content_type.as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "jpg",
    };

    let object_key = format!("{}/banner.{}", slug, extension);
    tracing::info!(
        "Uploading banner: key={}, type={}, size={}",
        object_key,
        content_type,
        file_data.len()
    );

    let file_size = file_data.len();
    let mut cursor = Cursor::new(file_data);
    let mut put_args = PutObjectArgs::new(
        &state.config.minio_bucket,
        &object_key,
        &mut cursor,
        Some(file_size),
        None,
    )
    .map_err(|e| format!("Failed to create put args: {}", e))?;

    put_args.content_type = content_type.as_str();

    state
        .s3
        .put_object(&mut put_args)
        .await
        .map_err(|e| format!("Failed to upload to MinIO: {}", e))?;

    let banner_url = format!(
        "{}/{}/{}",
        state.config.minio_public_endpoint, state.config.minio_bucket, object_key
    );

    // Update hackathon with banner URL
    let mut active_hackathon: crate::entities::hackathons::ActiveModel = hackathon.into();
    active_hackathon.banner_url = Set(Some(banner_url.clone()));
    active_hackathon.updated_at = Set(Utc::now().naive_utc());
    active_hackathon
        .update(&state.db)
        .await
        .map_err(|e| format!("Failed to update banner URL: {}", e))?;

    Ok(banner_url)
}
