use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[cfg(feature = "server")]
use crate::core::auth::{context::RequestContext, middleware::SyncedUser};

#[cfg(feature = "server")]
use utoipa::ToSchema;

/// Submission data returned from handlers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct SubmissionData {
    pub id: i32,
    pub team_id: i32,
    pub submission_data: JsonValue,
    pub prize_track_ids: Vec<i32>,
    pub submitted_at: String,
}

/// Request payload for submitting a project
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct SubmitProjectRequest {
    pub submission_data: JsonValue,
    pub prize_track_ids: Vec<i32>,
}

/// Request payload for updating prize tracks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct UpdatePrizeTracksRequest {
    pub prize_track_ids: Vec<i32>,
}

/// Get the user's team submission for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/submission",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Submission retrieved successfully", body = SubmissionData),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Submission not found"),
        (status = 500, description = "Server error")
    ),
    tag = "submissions"
))]
#[get("/api/hackathons/:slug/submission", user: SyncedUser)]
pub async fn get_submission(slug: String) -> Result<Option<SubmissionData>, ServerFnError> {
    use crate::domain::teams::repository::TeamRepository;
    use crate::entities::{prize_track_entry, submission};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get user's team
    let team_repo = TeamRepository::new(&ctx.state.db);
    let team_id = match team_repo.find_user_team(ctx.user.id, hackathon.id).await? {
        Some(id) => id,
        None => return Ok(None), // User has no team, so no submission
    };

    // Find submission for this team
    let submission_opt = submission::Entity::find()
        .filter(submission::Column::TeamId.eq(team_id))
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch submission: {}", e)))?;

    match submission_opt {
        Some(sub) => {
            // Fetch prize track entries
            let prize_entries = prize_track_entry::Entity::find()
                .filter(prize_track_entry::Column::SubmissionId.eq(sub.id))
                .all(&ctx.state.db)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to fetch prize tracks: {}", e)))?;

            let prize_track_ids: Vec<i32> = prize_entries.iter().map(|e| e.prize_id).collect();

            Ok(Some(SubmissionData {
                id: sub.id,
                team_id: sub.team_id,
                submission_data: sub.submission_data,
                prize_track_ids,
                submitted_at: sub.submitted_at.to_string(),
            }))
        }
        None => Ok(None),
    }
}

/// Submit or update a project
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/submission",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = SubmitProjectRequest,
    responses(
        (status = 200, description = "Project submitted successfully", body = SubmissionData),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "User must be on a team to submit"),
        (status = 500, description = "Server error")
    ),
    tag = "submissions"
))]
#[post("/api/hackathons/:slug/submission", user: SyncedUser)]
pub async fn submit_project(
    slug: String,
    request: SubmitProjectRequest,
) -> Result<SubmissionData, ServerFnError> {
    use crate::domain::teams::repository::TeamRepository;
    use crate::entities::{prize_track_entry, submission};
    use sea_orm::{
        ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, QueryFilter, Set,
    };

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get user's team
    let team_repo = TeamRepository::new(&ctx.state.db);
    let team_id = team_repo
        .find_user_team(ctx.user.id, hackathon.id)
        .await?
        .ok_or_else(|| ServerFnError::new("You must be on a team to submit a project"))?;

    // Check if submission already exists
    let existing = submission::Entity::find()
        .filter(submission::Column::TeamId.eq(team_id))
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to check existing submission: {}", e)))?;

    let sub = if let Some(existing_sub) = existing {
        // Update existing submission
        let mut active: submission::ActiveModel = existing_sub.into();
        active.submission_data = Set(request.submission_data.clone());

        active
            .update(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update submission: {}", e)))?
    } else {
        // Create new submission
        let now = chrono::Utc::now().naive_utc();
        let new_sub = submission::ActiveModel {
            id: NotSet,
            team_id: Set(team_id),
            submission_data: Set(request.submission_data.clone()),
            submitted_at: Set(now),
        };

        new_sub
            .insert(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to create submission: {}", e)))?
    };

    // Update prize track entries
    // First, delete existing entries
    prize_track_entry::Entity::delete_many()
        .filter(prize_track_entry::Column::SubmissionId.eq(sub.id))
        .exec(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to clear prize tracks: {}", e)))?;

    // Insert new entries
    for prize_id in &request.prize_track_ids {
        let entry = prize_track_entry::ActiveModel {
            id: NotSet,
            submission_id: Set(sub.id),
            prize_id: Set(*prize_id),
        };

        entry
            .insert(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to add prize track: {}", e)))?;
    }

    Ok(SubmissionData {
        id: sub.id,
        team_id: sub.team_id,
        submission_data: sub.submission_data,
        prize_track_ids: request.prize_track_ids,
        submitted_at: sub.submitted_at.to_string(),
    })
}

/// Update prize tracks for an existing submission
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}/submission/prize-tracks",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = UpdatePrizeTracksRequest,
    responses(
        (status = 200, description = "Prize tracks updated successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Submission not found"),
        (status = 500, description = "Server error")
    ),
    tag = "submissions"
))]
#[put("/api/hackathons/:slug/submission/prize-tracks", user: SyncedUser)]
pub async fn update_prize_tracks(
    slug: String,
    request: UpdatePrizeTracksRequest,
) -> Result<(), ServerFnError> {
    use crate::domain::teams::repository::TeamRepository;
    use crate::entities::{prize_track_entry, submission};
    use sea_orm::{
        ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, QueryFilter, Set,
    };

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get user's team
    let team_repo = TeamRepository::new(&ctx.state.db);
    let team_id = team_repo
        .find_user_team(ctx.user.id, hackathon.id)
        .await?
        .ok_or_else(|| ServerFnError::new("You must be on a team to update prize tracks"))?;

    // Find submission for this team
    let sub = submission::Entity::find()
        .filter(submission::Column::TeamId.eq(team_id))
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch submission: {}", e)))?
        .ok_or_else(|| ServerFnError::new("No submission found. Submit a project first."))?;

    // Delete existing entries
    prize_track_entry::Entity::delete_many()
        .filter(prize_track_entry::Column::SubmissionId.eq(sub.id))
        .exec(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to clear prize tracks: {}", e)))?;

    // Insert new entries
    for prize_id in &request.prize_track_ids {
        let entry = prize_track_entry::ActiveModel {
            id: NotSet,
            submission_id: Set(sub.id),
            prize_id: Set(*prize_id),
        };

        entry
            .insert(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to add prize track: {}", e)))?;
    }

    Ok(())
}
