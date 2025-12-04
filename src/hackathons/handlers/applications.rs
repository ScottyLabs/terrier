use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[cfg(feature = "server")]
use crate::{AppState, auth::middleware::SyncedUser};
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, ActiveValue::NotSet, Set};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct ApplicationData {
    pub form_data: JsonValue,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct ApplicationWithUser {
    pub id: i32,
    pub user_id: i32,
    pub user_name: Option<String>,
    pub user_email: String,
    pub user_picture: Option<String>,
    pub form_data: JsonValue,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Update application (draft/auto-save)
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}/application",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = JsonValue,
    responses(
        (status = 200, description = "Application updated successfully", body = ApplicationData),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[put("/api/hackathons/:slug/application", user: SyncedUser)]
pub async fn update_application(
    slug: String,
    form_data: JsonValue,
) -> Result<ApplicationData, ServerFnError> {
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

    // Check if application already exists
    let existing_application = crate::entities::prelude::Applications::find()
        .filter(crate::entities::applications::Column::UserId.eq(user.0.id))
        .filter(crate::entities::applications::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch application: {}", e)))?;

    let updated_at = Utc::now().naive_utc();

    let application = if let Some(existing) = existing_application {
        // Update existing application
        let mut app: crate::entities::applications::ActiveModel = existing.into();
        app.form_data = Set(form_data.clone());
        app.updated_at = Set(updated_at);

        app.update(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update application: {}", e)))?
    } else {
        // Create new application with "draft" status
        let app = crate::entities::applications::ActiveModel {
            id: NotSet,
            hackathon_id: Set(hackathon.id),
            user_id: Set(user.0.id),
            form_data: Set(form_data.clone()),
            status: Set("draft".to_string()),
            created_at: Set(updated_at),
            updated_at: Set(updated_at),
        };

        app.insert(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to create application: {}", e)))?
    };

    Ok(ApplicationData {
        form_data: application.form_data,
        status: application.status,
        updated_at: application.updated_at.to_string(),
    })
}

/// Submit application
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/application/submit",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = JsonValue,
    responses(
        (status = 200, description = "Application submitted successfully", body = ApplicationData),
        (status = 400, description = "Application already submitted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[post("/api/hackathons/:slug/application/submit", user: SyncedUser)]
pub async fn submit_application(
    slug: String,
    form_data: JsonValue,
) -> Result<ApplicationData, ServerFnError> {
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

    // Check if application already exists
    let existing_application = crate::entities::prelude::Applications::find()
        .filter(crate::entities::applications::Column::UserId.eq(user.0.id))
        .filter(crate::entities::applications::Column::HackathonId.eq(hackathon.id))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch application: {}", e)))?;

    let updated_at = Utc::now().naive_utc();

    let application = if let Some(existing) = existing_application {
        // Check if already submitted
        if existing.status == "pending" || existing.status == "accepted" || existing.status == "rejected" {
            return Err(ServerFnError::new(
                "Application has already been submitted and cannot be modified",
            ));
        }

        // Update existing application and mark as submitted
        let mut app: crate::entities::applications::ActiveModel = existing.into();
        app.form_data = Set(form_data.clone());
        app.status = Set("pending".to_string());
        app.updated_at = Set(updated_at);

        app.update(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to submit application: {}", e)))?
    } else {
        // Create new application with "pending" status
        let app = crate::entities::applications::ActiveModel {
            id: NotSet,
            hackathon_id: Set(hackathon.id),
            user_id: Set(user.0.id),
            form_data: Set(form_data.clone()),
            status: Set("pending".to_string()),
            created_at: Set(updated_at),
            updated_at: Set(updated_at),
        };

        app.insert(&state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to create application: {}", e)))?
    };

    Ok(ApplicationData {
        form_data: application.form_data,
        status: application.status,
        updated_at: application.updated_at.to_string(),
    })
}

/// Get the user's application for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/application",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Application retrieved successfully", body = ApplicationData),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Application not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[get("/api/hackathons/:slug/application", user: SyncedUser)]
pub async fn get_application(slug: String) -> Result<ApplicationData, ServerFnError> {
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

    Ok(ApplicationData {
        form_data: application.form_data,
        status: application.status,
        updated_at: application.updated_at.to_string(),
    })
}

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
        return Err(ServerFnError::new(
            "Admin or organizer access required",
        ));
    }

    // Fetch all applications with user information
    let applications = crate::entities::prelude::Applications::find()
        .filter(crate::entities::applications::Column::HackathonId.eq(hackathon.id))
        .find_also_related(crate::entities::prelude::Users)
        .all(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch applications: {}", e)))?;

    let results = applications
        .into_iter()
        .filter_map(|(app, user_opt)| {
            user_opt.map(|user| ApplicationWithUser {
                id: app.id,
                user_id: app.user_id,
                user_name: user.name,
                user_email: user.email,
                user_picture: user.picture,
                form_data: app.form_data,
                status: app.status,
                created_at: app.created_at.to_string(),
                updated_at: app.updated_at.to_string(),
            })
        })
        .collect();

    Ok(results)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct BulkUpdateApplicationsRequest {
    pub application_ids: Vec<i32>,
}

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
        return Err(ServerFnError::new(
            "Requires admin or organizer role",
        ));
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
        return Err(ServerFnError::new(
            "Requires admin or organizer role",
        ));
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

/// Unsubmit an application (change from pending back to draft)
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}/application/unsubmit",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Application unsubmitted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Application not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[put("/api/hackathons/:slug/application/unsubmit", user: SyncedUser)]
pub async fn unsubmit_application(slug: String) -> Result<(), ServerFnError> {
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

    // Only allow unsubmitting pending applications
    if application.status != "pending" {
        return Err(ServerFnError::new("Can only unsubmit pending applications"));
    }

    // Update status to draft
    let mut app: crate::entities::applications::ActiveModel = application.into();
    app.status = Set("draft".to_string());
    app.updated_at = Set(Utc::now().naive_utc());

    app.update(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to unsubmit application: {}", e)))?;

    Ok(())
}

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
