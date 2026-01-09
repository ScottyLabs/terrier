use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::core::auth::{
    context::RequestContext, middleware::SyncedUser, permissions::Permissions,
};
#[cfg(feature = "server")]
use utoipa::ToSchema;

/// Attendee info for organizer view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct Attendee {
    pub user_id: i32,
    pub name: String,
    pub email: String,
    pub checked_in_at: chrono::NaiveDateTime,
}

/// User points info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct UserPoints {
    pub user_id: i32,
    pub total_points: i32,
}

/// Participant info for confirmation modal
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct ParticipantInfo {
    pub user_id: i32,
    pub name: String,
    pub email: String,
}

/// Self check-in to an event (participant action)
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/events/{event_id}/checkin",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("event_id" = i32, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "Checked in successfully"),
        (status = 400, description = "Event requires organizer scan"),
        (status = 404, description = "Event not found"),
        (status = 500, description = "Server error")
    ),
    tag = "checkin"
))]
#[post("/api/hackathons/:slug/events/:event_id/checkin", user: SyncedUser)]
pub async fn self_checkin(slug: String, event_id: i32) -> Result<(), ServerFnError> {
    use crate::entities::{event_checkins, events};
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    // Find the event
    let event = events::Entity::find_by_id(event_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to find event: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Event not found"))?;

    // Verify it's a self-checkin event
    if event.checkin_type != "self_checkin" {
        return Err(ServerFnError::new(
            "This event requires QR scan by an organizer",
        ));
    }

    // Create check-in record
    let now = Utc::now().naive_utc();
    let checkin = event_checkins::ActiveModel {
        event_id: Set(event_id),
        user_id: Set(ctx.user.id),
        checked_in_at: Set(now),
        checked_in_by: Set(None), // Self check-in
        ..Default::default()
    };

    // Insert (ignore if already exists)
    let _ = checkin.insert(&ctx.state.db).await;

    Ok(())
}

/// Remove self check-in (participant action)
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/events/{event_id}/checkin",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("event_id" = i32, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "Check-in removed"),
        (status = 404, description = "Check-in not found"),
        (status = 500, description = "Server error")
    ),
    tag = "checkin"
))]
#[delete("/api/hackathons/:slug/events/:event_id/checkin", user: SyncedUser)]
pub async fn remove_self_checkin(slug: String, event_id: i32) -> Result<(), ServerFnError> {
    use crate::entities::event_checkins;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    // Delete the check-in record
    event_checkins::Entity::delete_many()
        .filter(event_checkins::Column::EventId.eq(event_id))
        .filter(event_checkins::Column::UserId.eq(ctx.user.id))
        .exec(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to remove check-in: {}", e)))?;

    Ok(())
}

/// Check in a user by organizer (QR scan or manual entry)
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/events/{event_id}/checkin/{target_user_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("event_id" = i32, Path, description = "Event ID"),
        ("target_user_id" = i32, Path, description = "User ID to check in")
    ),
    responses(
        (status = 200, description = "User checked in successfully"),
        (status = 400, description = "User already checked in"),
        (status = 403, description = "Not authorized to check in users"),
        (status = 404, description = "Event or user not found"),
        (status = 500, description = "Server error")
    ),
    tag = "checkin"
))]
#[post("/api/hackathons/:slug/events/:event_id/checkin/:target_user_id", user: SyncedUser)]
pub async fn organizer_checkin(
    slug: String,
    event_id: i32,
    target_user_id: i32,
) -> Result<(), ServerFnError> {
    use crate::domain::people::repository::UserRoleRepository;
    use crate::entities::event_checkins;
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Verify user is organizer, admin, or global admin
    let is_global_admin = Permissions::is_global_admin(&ctx);
    let role_repo = UserRoleRepository::new(&ctx.state.db);
    let is_admin = role_repo.is_admin(ctx.user.id, hackathon.id).await?;
    let is_organizer = role_repo.is_organizer(ctx.user.id, hackathon.id).await?;

    if !is_global_admin && !is_admin && !is_organizer {
        return Err(ServerFnError::new(
            "Only organizers can check in participants",
        ));
    }

    // Check if user is already checked in
    let existing_checkin = event_checkins::Entity::find()
        .filter(event_checkins::Column::EventId.eq(event_id))
        .filter(event_checkins::Column::UserId.eq(target_user_id))
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to check existing check-in: {}", e)))?;

    if existing_checkin.is_some() {
        return Err(ServerFnError::new("ALREADY_CHECKED_IN"));
    }

    // Create check-in record
    let now = Utc::now().naive_utc();
    let checkin = event_checkins::ActiveModel {
        event_id: Set(event_id),
        user_id: Set(target_user_id),
        checked_in_at: Set(now),
        checked_in_by: Set(Some(ctx.user.id)), // Organizer who scanned
        ..Default::default()
    };

    checkin
        .insert(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to check in user: {}", e)))?;

    Ok(())
}

/// Remove a user's check-in (organizer action)
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/events/{event_id}/checkin/{target_user_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("event_id" = i32, Path, description = "Event ID"),
        ("target_user_id" = i32, Path, description = "User ID to remove")
    ),
    responses(
        (status = 200, description = "Check-in removed"),
        (status = 403, description = "Not authorized"),
        (status = 500, description = "Server error")
    ),
    tag = "checkin"
))]
#[delete("/api/hackathons/:slug/events/:event_id/checkin/:target_user_id", user: SyncedUser)]
pub async fn organizer_remove_checkin(
    slug: String,
    event_id: i32,
    target_user_id: i32,
) -> Result<(), ServerFnError> {
    use crate::domain::people::repository::UserRoleRepository;
    use crate::entities::event_checkins;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Verify user is organizer, admin, or global admin
    let is_global_admin = Permissions::is_global_admin(&ctx);
    let role_repo = UserRoleRepository::new(&ctx.state.db);
    let is_admin = role_repo.is_admin(ctx.user.id, hackathon.id).await?;
    let is_organizer = role_repo.is_organizer(ctx.user.id, hackathon.id).await?;

    if !is_global_admin && !is_admin && !is_organizer {
        return Err(ServerFnError::new("Only organizers can remove check-ins"));
    }

    // Delete the check-in record
    event_checkins::Entity::delete_many()
        .filter(event_checkins::Column::EventId.eq(event_id))
        .filter(event_checkins::Column::UserId.eq(target_user_id))
        .exec(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to remove check-in: {}", e)))?;

    Ok(())
}

/// Get attendees for an event (organizer action)
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/events/{event_id}/attendees",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("event_id" = i32, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "Attendees list", body = Vec<Attendee>),
        (status = 403, description = "Not authorized"),
        (status = 500, description = "Server error")
    ),
    tag = "checkin"
))]
#[get("/api/hackathons/:slug/events/:event_id/attendees", user: SyncedUser)]
pub async fn get_attendees(slug: String, event_id: i32) -> Result<Vec<Attendee>, ServerFnError> {
    use crate::domain::people::repository::UserRoleRepository;
    use crate::entities::{event_checkins, users};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Verify user is organizer, admin, or global admin
    let is_global_admin = Permissions::is_global_admin(&ctx);
    let role_repo = UserRoleRepository::new(&ctx.state.db);
    let is_admin = role_repo.is_admin(ctx.user.id, hackathon.id).await?;
    let is_organizer = role_repo.is_organizer(ctx.user.id, hackathon.id).await?;

    if !is_global_admin && !is_admin && !is_organizer {
        return Err(ServerFnError::new("Only organizers can view attendees"));
    }

    // Fetch checkins for this event
    let checkins = event_checkins::Entity::find()
        .filter(event_checkins::Column::EventId.eq(event_id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch attendees: {}", e)))?;

    // Get user IDs
    let user_ids: Vec<i32> = checkins.iter().map(|c| c.user_id).collect();

    // Fetch users
    let users_list = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch users: {}", e)))?;

    // Build attendee list
    let user_map: std::collections::HashMap<i32, &users::Model> =
        users_list.iter().map(|u| (u.id, u)).collect();

    let attendees = checkins
        .iter()
        .filter_map(|c| {
            user_map.get(&c.user_id).map(|u| Attendee {
                user_id: u.id,
                name: u.name.clone().unwrap_or_else(|| "Unknown".to_string()),
                email: u.email.clone(),
                checked_in_at: c.checked_in_at,
            })
        })
        .collect();

    Ok(attendees)
}

/// Get user's total points from check-ins
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/user/points",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "User points", body = UserPoints),
        (status = 500, description = "Server error")
    ),
    tag = "checkin"
))]
#[get("/api/hackathons/:slug/user/points", user: SyncedUser)]
pub async fn get_user_points(slug: String) -> Result<UserPoints, ServerFnError> {
    use crate::entities::{event_checkins, events};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Fetch user's check-ins
    let checkins = event_checkins::Entity::find()
        .filter(event_checkins::Column::UserId.eq(ctx.user.id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch checkins: {}", e)))?;

    let event_ids: Vec<i32> = checkins.iter().map(|c| c.event_id).collect();

    // Fetch events to get points
    let events_list = events::Entity::find()
        .filter(events::Column::Id.is_in(event_ids))
        .filter(events::Column::HackathonId.eq(hackathon.id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch events: {}", e)))?;

    // Sum points
    let total_points: i32 = events_list.iter().filter_map(|e| e.points).sum();

    Ok(UserPoints {
        user_id: ctx.user.id,
        total_points,
    })
}

/// Get participant info by user ID (for check-in confirmation)
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/participant/{user_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("user_id" = i32, Path, description = "User ID to look up")
    ),
    responses(
        (status = 200, description = "Participant info", body = ParticipantInfo),
        (status = 403, description = "Not authorized"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Server error")
    ),
    tag = "checkin"
))]
#[get("/api/hackathons/:slug/participant/:user_id", user: SyncedUser)]
pub async fn get_participant_info(
    slug: String,
    user_id: i32,
) -> Result<ParticipantInfo, ServerFnError> {
    use crate::domain::people::repository::UserRoleRepository;
    use crate::entities::users;
    use sea_orm::EntityTrait;

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Verify user is organizer, admin, or global admin
    let is_global_admin = Permissions::is_global_admin(&ctx);
    let role_repo = UserRoleRepository::new(&ctx.state.db);
    let is_admin = role_repo.is_admin(ctx.user.id, hackathon.id).await?;
    let is_organizer = role_repo.is_organizer(ctx.user.id, hackathon.id).await?;

    if !is_global_admin && !is_admin && !is_organizer {
        return Err(ServerFnError::new(
            "Only organizers can look up participants",
        ));
    }

    // Fetch the user
    let user_record = users::Entity::find_by_id(user_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User not found"))?;

    Ok(ParticipantInfo {
        user_id: user_record.id,
        name: user_record.name.unwrap_or_else(|| "Unknown".to_string()),
        email: user_record.email,
    })
}
