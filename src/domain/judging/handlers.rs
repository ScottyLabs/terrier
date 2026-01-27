use super::types::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::core::auth::{
    context::RequestContext, middleware::SyncedUser, permissions::Permissions,
};

/// Close submissions for a hackathon (prerequisite for starting judging)
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/close-submissions",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Submissions closed successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/close-submissions", user: SyncedUser)]
pub async fn close_submissions(slug: String) -> Result<(), ServerFnError> {
    use crate::entities::hackathons;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;
    let hackathon = ctx.hackathon()?;

    let mut active: hackathons::ActiveModel = hackathons::Entity::find_by_id(hackathon.id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?
        .into();

    active.submissions_closed = Set(true);

    active
        .update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to close submissions: {}", e)))?;

    Ok(())
}

/// Start judging for a hackathon (requires submissions to be closed)
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/start",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Judging started successfully"),
        (status = 400, description = "Submissions must be closed first"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/start", user: SyncedUser)]
pub async fn start_judging(slug: String) -> Result<(), ServerFnError> {
    use crate::entities::hackathons;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;
    let hackathon = ctx.hackathon()?;

    if !hackathon.submissions_closed {
        return Err(ServerFnError::new(
            "Submissions must be closed before starting judging",
        ));
    }

    let mut active: hackathons::ActiveModel = hackathons::Entity::find_by_id(hackathon.id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?
        .into();

    active.judging_started = Set(true);

    active
        .update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to start judging: {}", e)))?;

    Ok(())
}

/// Stop judging for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/stop",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Judging stopped successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/stop", user: SyncedUser)]
pub async fn stop_judging(slug: String) -> Result<(), ServerFnError> {
    use crate::entities::hackathons;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;
    let hackathon = ctx.hackathon()?;

    let mut active: hackathons::ActiveModel = hackathons::Entity::find_by_id(hackathon.id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?
        .into();

    active.judging_started = Set(false);

    active
        .update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to stop judging: {}", e)))?;

    Ok(())
}

/// Reset judging for a hackathon (clears all judging data)
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/reset",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Judging reset successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/reset", user: SyncedUser)]
pub async fn reset_judging(slug: String) -> Result<(), ServerFnError> {
    use crate::entities::{feature, hackathons, pairwise_comparison, project_visit};
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
    };

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;
    let hackathon = ctx.hackathon()?;

    // Start transaction
    let txn = ctx
        .state
        .db
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to start transaction: {}", e)))?;

    // 1. Reset hackathon state (stop judging if started)
    let mut active: hackathons::ActiveModel = hackathons::Entity::find_by_id(hackathon.id)
        .one(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?
        .into();

    active.judging_started = Set(false);

    active
        .update(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update hackathon: {}", e)))?;

    // 2. Delete all project visits for this hackathon
    project_visit::Entity::delete_many()
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .exec(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete visits: {}", e)))?;

    // 3. Delete all pairwise comparisons for this hackathon's features
    // First get feature IDs
    let feature_ids: Vec<i32> = feature::Entity::find()
        .filter(feature::Column::HackathonId.eq(hackathon.id))
        .select_only()
        .column(feature::Column::Id)
        .into_tuple::<i32>()
        .all(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch features: {}", e)))?;

    if !feature_ids.is_empty() {
        pairwise_comparison::Entity::delete_many()
            .filter(pairwise_comparison::Column::FeatureId.is_in(feature_ids.clone()))
            .exec(&txn)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to delete pairwise comparisons: {}", e))
            })?;

        // 4. Delete all project feature scores
        crate::entities::project_feature_score::Entity::delete_many()
            .filter(
                crate::entities::project_feature_score::Column::FeatureId
                    .is_in(feature_ids.clone()),
            )
            .exec(&txn)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to delete project feature scores: {}", e))
            })?;

        // 5. Reset judge assignments (clear best submission and notes)
        // This is important so that judges see the "first project" view again
        use crate::entities::judge_feature_assignment;
        use sea_orm::QueryOrder; // Added manually if needed, but not for update_many
        use sea_orm::Statement; // Not needed
        use sea_orm::sea_query::{Expr, Query}; // For update_many col_expr if needed, but we can use ActiveModel

        // SeaORM update_many with col_expr is a bit complex with Option,
        // let's just update them all.
        // Actually, let's use a simpler way: find all and update
        let assignments = judge_feature_assignment::Entity::find()
            .filter(judge_feature_assignment::Column::FeatureId.is_in(feature_ids))
            .all(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch assignments: {}", e)))?;

        for assignment in assignments {
            let mut active: judge_feature_assignment::ActiveModel = assignment.into();
            active.current_best_submission_id = Set(None);
            active.notes = Set(None);
            active.update(&txn).await.map_err(|e| {
                ServerFnError::new(format!("Failed to reset judge assignment: {}", e))
            })?;
        }
    }

    // Commit transaction
    txn.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to commit transaction: {}", e)))?;

    Ok(())
}

/// Get judging status for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/judging/status",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Judging status retrieved", body = JudgingStatus),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[get("/api/hackathons/:slug/judging/status", user: SyncedUser)]
pub async fn get_judging_status(slug: String) -> Result<JudgingStatus, ServerFnError> {
    use crate::entities::{feature, pairwise_comparison, project_visit, submission, teams};
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Count total submissions (via teams belonging to this hackathon)
    let hackathon_team_ids: Vec<i32> = teams::Entity::find()
        .filter(teams::Column::HackathonId.eq(hackathon.id))
        .select_only()
        .column(teams::Column::Id)
        .into_tuple::<i32>()
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch teams: {}", e)))?;

    let total_submissions = if hackathon_team_ids.is_empty() {
        0u64
    } else {
        submission::Entity::find()
            .filter(submission::Column::TeamId.is_in(hackathon_team_ids.clone()))
            .count(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to count submissions: {}", e)))?
    };

    // Count submissions with at least one visit
    let visited_submissions = project_visit::Entity::find()
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .select_only()
        .column(project_visit::Column::SubmissionId)
        .distinct()
        .count(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to count visited: {}", e)))?;

    // Count total visits
    let total_visits = project_visit::Entity::find()
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .count(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to count visits: {}", e)))?;

    // Count total comparisons (via features belonging to this hackathon)
    // First get feature IDs for this hackathon
    let hackathon_feature_ids: Vec<i32> = feature::Entity::find()
        .filter(feature::Column::HackathonId.eq(hackathon.id))
        .select_only()
        .column(feature::Column::Id)
        .into_tuple::<i32>()
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch features: {}", e)))?;

    let total_comparisons = if hackathon_feature_ids.is_empty() {
        0u64
    } else {
        pairwise_comparison::Entity::find()
            .filter(pairwise_comparison::Column::FeatureId.is_in(hackathon_feature_ids))
            .count(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to count comparisons: {}", e)))?
    };

    Ok(JudgingStatus {
        submissions_closed: hackathon.submissions_closed,
        judging_started: hackathon.judging_started,
        total_submissions: total_submissions as i64,
        visited_submissions: visited_submissions as i64,
        total_visits: total_visits as i64,
        total_comparisons: total_comparisons as i64,
    })
}

// ============================================================================
// Judge Assignment Endpoints
// ============================================================================

/// Request a new assignment for a judge
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/request-assignment",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Assignment created", body = JudgeAssignment),
        (status = 204, description = "No more projects available"),
        (status = 400, description = "Judging not active or judge has active assignment"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/request-assignment", user: SyncedUser)]
pub async fn request_assignment(slug: String) -> Result<Option<JudgeAssignment>, ServerFnError> {
    use crate::entities::{project_visit, submission, teams};
    use sea_orm::{
        ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, PaginatorTrait,
        QueryFilter, QuerySelect, Set, TransactionTrait,
    };

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Check if judging is active
    if !hackathon.judging_started {
        return Err(ServerFnError::new("Judging has not started yet"));
    }

    // Check if judge already has an active assignment
    let active_visit = project_visit::Entity::find()
        .filter(project_visit::Column::JudgeId.eq(ctx.user.id))
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .filter(project_visit::Column::IsActive.eq(true))
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to check active visits: {}", e)))?;

    if let Some(visit) = active_visit {
        // Return the existing active assignment
        let sub = submission::Entity::find_by_id(visit.submission_id)
            .one(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch submission: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Submission not found"))?;

        let team = teams::Entity::find_by_id(sub.team_id)
            .one(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Team not found"))?;

        let timeout_seconds = hackathon.judge_session_timeout_minutes as i64 * 60;
        let elapsed = chrono::Utc::now()
            .naive_utc()
            .signed_duration_since(visit.start_time)
            .num_seconds();
        let remaining = (timeout_seconds - elapsed).max(0);

        return Ok(Some(JudgeAssignment {
            visit_id: visit.id,
            submission_id: sub.id,
            team_name: team.name,
            submission_data: sub.submission_data,
            start_time: visit.start_time.to_string(),
            time_remaining_seconds: remaining,
        }));
    }

    // Find an available submission within a transaction
    let txn = ctx
        .state
        .db
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to start transaction: {}", e)))?;

    // Get all submissions for this hackathon
    // First get team IDs for this hackathon
    let team_ids: Vec<i32> = teams::Entity::find()
        .filter(teams::Column::HackathonId.eq(hackathon.id))
        .select_only()
        .column(teams::Column::Id)
        .into_tuple::<i32>()
        .all(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch teams: {}", e)))?;

    let available_submissions = if team_ids.is_empty() {
        Vec::new()
    } else {
        submission::Entity::find()
            .filter(submission::Column::TeamId.is_in(team_ids))
            .all(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch submissions: {}", e)))?
    };

    // Filter out submissions already visited by this judge or currently being visited
    let mut candidate = None;
    for sub in available_submissions {
        // Check if this judge has already visited this submission
        let already_visited = project_visit::Entity::find()
            .filter(project_visit::Column::SubmissionId.eq(sub.id))
            .filter(project_visit::Column::JudgeId.eq(ctx.user.id))
            .one(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to check visit: {}", e)))?;

        if already_visited.is_some() {
            continue;
        }

        // Check if another judge is currently visiting this submission
        let active_visits = project_visit::Entity::find()
            .filter(project_visit::Column::SubmissionId.eq(sub.id))
            .filter(project_visit::Column::IsActive.eq(true))
            .count(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to count active visits: {}", e)))?;

        if active_visits > 0 {
            continue;
        }

        // This submission is available
        candidate = Some(sub);
        break;
    }

    let sub = match candidate {
        Some(s) => s,
        None => {
            txn.commit()
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to commit: {}", e)))?;
            return Ok(None); // No available projects
        }
    };

    // Create the visit
    let now = chrono::Utc::now().naive_utc();
    let new_visit = project_visit::ActiveModel {
        id: NotSet,
        submission_id: Set(sub.id),
        judge_id: Set(ctx.user.id),
        hackathon_id: Set(hackathon.id),
        notes: Set(None),
        start_time: Set(now),
        completion_time: Set(None),
        is_active: Set(true),
    };

    let visit = new_visit
        .insert(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create visit: {}", e)))?;

    txn.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to commit: {}", e)))?;

    // Get team name
    let team = teams::Entity::find_by_id(sub.team_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    let timeout_seconds = hackathon.judge_session_timeout_minutes as i64 * 60;

    Ok(Some(JudgeAssignment {
        visit_id: visit.id,
        submission_id: sub.id,
        team_name: team.name,
        submission_data: sub.submission_data,
        start_time: visit.start_time.to_string(),
        time_remaining_seconds: timeout_seconds,
    }))
}

/// Complete a visit with notes
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/visits/{visit_id}/complete",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("visit_id" = i32, Path, description = "Visit ID")
    ),
    request_body = CompleteVisitRequest,
    responses(
        (status = 200, description = "Visit completed successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not your visit"),
        (status = 404, description = "Visit not found"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/visits/:visit_id/complete", user: SyncedUser)]
pub async fn complete_visit(
    slug: String,
    visit_id: i32,
    request: CompleteVisitRequest,
) -> Result<(), ServerFnError> {
    use crate::entities::project_visit;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let visit = project_visit::Entity::find_by_id(visit_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch visit: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Visit not found"))?;

    // Verify ownership
    if visit.judge_id != ctx.user.id {
        return Err(ServerFnError::new("This is not your visit"));
    }

    let mut active: project_visit::ActiveModel = visit.into();
    active.is_active = Set(false);
    active.completion_time = Set(Some(chrono::Utc::now().naive_utc()));
    active.notes = Set(request.notes);

    active
        .update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to complete visit: {}", e)))?;

    Ok(())
}

/// Get current assignment for a judge
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/judging/current-assignment",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Current assignment", body = Option<JudgeAssignment>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[get("/api/hackathons/:slug/judging/current-assignment", user: SyncedUser)]
pub async fn get_current_assignment(
    slug: String,
) -> Result<Option<JudgeAssignment>, ServerFnError> {
    use crate::entities::{project_visit, submission, teams};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    let active_visit = project_visit::Entity::find()
        .filter(project_visit::Column::JudgeId.eq(ctx.user.id))
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .filter(project_visit::Column::IsActive.eq(true))
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch active visit: {}", e)))?;

    match active_visit {
        Some(visit) => {
            let sub = submission::Entity::find_by_id(visit.submission_id)
                .one(&ctx.state.db)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to fetch submission: {}", e)))?
                .ok_or_else(|| ServerFnError::new("Submission not found"))?;

            let team = teams::Entity::find_by_id(sub.team_id)
                .one(&ctx.state.db)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
                .ok_or_else(|| ServerFnError::new("Team not found"))?;

            let timeout_seconds = hackathon.judge_session_timeout_minutes as i64 * 60;
            let elapsed = chrono::Utc::now()
                .naive_utc()
                .signed_duration_since(visit.start_time)
                .num_seconds();
            let remaining = (timeout_seconds - elapsed).max(0);

            Ok(Some(JudgeAssignment {
                visit_id: visit.id,
                submission_id: sub.id,
                team_name: team.name,
                submission_data: sub.submission_data,
                start_time: visit.start_time.to_string(),
                time_remaining_seconds: remaining,
            }))
        }
        None => Ok(None),
    }
}

/// Submit a pairwise comparison
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/compare",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = PairwiseComparisonRequest,
    responses(
        (status = 200, description = "Comparison submitted successfully"),
        (status = 400, description = "Invalid comparison"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/compare", user: SyncedUser)]
pub async fn submit_comparison(
    slug: String,
    request: PairwiseComparisonRequest,
) -> Result<(), ServerFnError> {
    use crate::entities::pairwise_comparison;
    use sea_orm::{ActiveModelTrait, ActiveValue::NotSet, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Check if judging is active
    if !hackathon.judging_started {
        return Err(ServerFnError::new("Judging has not started yet"));
    }

    let new_comparison = pairwise_comparison::ActiveModel {
        id: NotSet,
        feature_id: Set(request.feature_id),
        judge_id: Set(ctx.user.id),
        submission_a_id: Set(request.submission_a_id),
        submission_b_id: Set(request.submission_b_id),
        winner_id: Set(request.winner_id),
        created_at: Set(chrono::Utc::now().naive_utc()),
    };

    new_comparison
        .insert(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to submit comparison: {}", e)))?;

    Ok(())
}

// ============================================================================
// Feature Management Endpoints
// ============================================================================

/// Get all features for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/judging/features",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Features retrieved", body = Vec<FeatureInfo>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[get("/api/hackathons/:slug/judging/features", user: SyncedUser)]
pub async fn get_features(slug: String) -> Result<Vec<FeatureInfo>, ServerFnError> {
    use crate::entities::feature;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    let features = feature::Entity::find()
        .filter(feature::Column::HackathonId.eq(hackathon.id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch features: {}", e)))?;

    Ok(features
        .into_iter()
        .map(|f| FeatureInfo {
            id: f.id,
            name: f.name,
            description: f.description,
        })
        .collect())
}

/// Create a new feature for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/features",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = CreateFeatureRequest,
    responses(
        (status = 200, description = "Feature created", body = FeatureInfo),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/features", user: SyncedUser)]
pub async fn create_feature(
    slug: String,
    request: CreateFeatureRequest,
) -> Result<FeatureInfo, ServerFnError> {
    use crate::entities::feature;
    use sea_orm::{ActiveModelTrait, ActiveValue::NotSet, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;
    let hackathon = ctx.hackathon()?;

    let new_feature = feature::ActiveModel {
        id: NotSet,
        hackathon_id: Set(hackathon.id),
        name: Set(request.name),
        description: Set(request.description),
    };

    let feature = new_feature
        .insert(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create feature: {}", e)))?;

    Ok(FeatureInfo {
        id: feature.id,
        name: feature.name,
        description: feature.description,
    })
}

/// Update a feature
#[cfg_attr(feature = "server", utoipa::path(
    put,
    path = "/api/hackathons/{slug}/judging/features/{feature_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("feature_id" = i32, Path, description = "Feature ID")
    ),
    request_body = UpdateFeatureRequest,
    responses(
        (status = 200, description = "Feature updated", body = FeatureInfo),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Feature not found"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[put("/api/hackathons/:slug/judging/features/:feature_id", user: SyncedUser)]
pub async fn update_feature(
    slug: String,
    feature_id: i32,
    request: UpdateFeatureRequest,
) -> Result<FeatureInfo, ServerFnError> {
    use crate::entities::feature;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Find the feature
    let existing = feature::Entity::find_by_id(feature_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to find feature: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Feature not found"))?;

    // Verify it belongs to this hackathon
    if existing.hackathon_id != hackathon.id {
        return Err(ServerFnError::new("Feature not found in this hackathon"));
    }

    let mut active: feature::ActiveModel = existing.into();
    active.name = Set(request.name);
    active.description = Set(request.description);

    let updated = active
        .update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update feature: {}", e)))?;

    Ok(FeatureInfo {
        id: updated.id,
        name: updated.name,
        description: updated.description,
    })
}

/// Delete a feature
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/judging/features/{feature_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("feature_id" = i32, Path, description = "Feature ID")
    ),
    responses(
        (status = 200, description = "Feature deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Feature not found"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[delete("/api/hackathons/:slug/judging/features/:feature_id", user: SyncedUser)]
pub async fn delete_feature(slug: String, feature_id: i32) -> Result<(), ServerFnError> {
    use crate::entities::feature;
    use sea_orm::{EntityTrait, ModelTrait};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Find the feature
    let existing = feature::Entity::find_by_id(feature_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to find feature: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Feature not found"))?;

    // Verify it belongs to this hackathon
    if existing.hackathon_id != hackathon.id {
        return Err(ServerFnError::new("Feature not found in this hackathon"));
    }

    existing
        .delete(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete feature: {}", e)))?;

    Ok(())
}

/// Re-open submissions for a hackathon
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/reopen-submissions",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Submissions re-opened successfully"),
        (status = 400, description = "Judging already started"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/reopen-submissions", user: SyncedUser)]
pub async fn reopen_submissions(slug: String) -> Result<(), ServerFnError> {
    use crate::entities::hackathons;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Cannot re-open if judging has already started
    if hackathon.judging_started {
        return Err(ServerFnError::new(
            "Cannot re-open submissions while judging is active",
        ));
    }

    let mut active: hackathons::ActiveModel = hackathons::Entity::find_by_id(hackathon.id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch hackathon: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?
        .into();

    active.submissions_closed = Set(false);

    active
        .update(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to re-open submissions: {}", e)))?;

    Ok(())
}

// ============================================================================
// Unified Judging Mode Endpoints
// ============================================================================

/// Get the full unified judging state for a judge
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/judging/state",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Unified judging state", body = UnifiedJudgingState),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[get("/api/hackathons/:slug/judging/state", user: SyncedUser)]
pub async fn get_unified_state(slug: String) -> Result<UnifiedJudgingState, ServerFnError> {
    use crate::entities::{feature, judge_feature_assignment, project_visit, submission, teams};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get judge's assigned features
    let assignments = judge_feature_assignment::Entity::find()
        .filter(judge_feature_assignment::Column::JudgeId.eq(ctx.user.id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch assignments: {}", e)))?;

    // Get feature details for each assignment
    let mut features = Vec::new();
    for assignment in assignments {
        let feature_model = feature::Entity::find_by_id(assignment.feature_id)
            .one(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch feature: {}", e)))?;

        if let Some(feat) = feature_model {
            // Only include features for this hackathon
            if feat.hackathon_id != hackathon.id {
                continue;
            }

            let mut current_best_team_name = None;
            let mut current_best_description = None;

            if let Some(best_sub_id) = assignment.current_best_submission_id {
                if let Ok(Some(sub)) = submission::Entity::find_by_id(best_sub_id)
                    .one(&ctx.state.db)
                    .await
                {
                    if let Ok(Some(team)) = teams::Entity::find_by_id(sub.team_id)
                        .one(&ctx.state.db)
                        .await
                    {
                        current_best_team_name = Some(team.name);
                    }
                    // Extract description from submission_data if available
                    current_best_description = sub
                        .submission_data
                        .get("description")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string());
                }
            }

            features.push(JudgeFeatureState {
                feature_id: feat.id,
                feature_name: feat.name,
                feature_description: feat.description,
                current_best_submission_id: assignment.current_best_submission_id,
                current_best_team_name,
                current_best_description,
                notes: assignment.notes,
            });
        }
    }

    // Get current active visit
    let active_visit = project_visit::Entity::find()
        .filter(project_visit::Column::JudgeId.eq(ctx.user.id))
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .filter(project_visit::Column::IsActive.eq(true))
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch active visit: {}", e)))?;

    let current_project = if let Some(visit) = active_visit {
        let sub = submission::Entity::find_by_id(visit.submission_id)
            .one(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch submission: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Submission not found"))?;

        let team = teams::Entity::find_by_id(sub.team_id)
            .one(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Team not found"))?;

        Some(CurrentProject {
            visit_id: visit.id,
            submission_id: sub.id,
            team_name: team.name.clone(),
            project_name: sub
                .submission_data
                .get("projectName")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string()),
            location: None, // Table number not tracked in teams
            description: sub
                .submission_data
                .get("description")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string()),
            submission_data: sub.submission_data,
        })
    } else {
        None
    };

    Ok(UnifiedJudgingState {
        current_project,
        features,
        judging_started: hackathon.judging_started,
    })
}

/// Request the next project to judge using 2-phase algorithm
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/next-project",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Next project assigned", body = Option<CurrentProject>),
        (status = 400, description = "Judging not active or already has active assignment"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/next-project", user: SyncedUser)]
pub async fn request_next_project(slug: String) -> Result<Option<CurrentProject>, ServerFnError> {
    use crate::entities::{project_feature_score, project_visit, submission, teams};
    use rand::prelude::*;
    use sea_orm::{
        ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, PaginatorTrait,
        QueryFilter, QuerySelect, Set, TransactionTrait,
    };

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Check if judging is active
    if !hackathon.judging_started {
        return Err(ServerFnError::new("Judging has not started yet"));
    }

    // Check if judge already has an active assignment
    let existing_active = project_visit::Entity::find()
        .filter(project_visit::Column::JudgeId.eq(ctx.user.id))
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .filter(project_visit::Column::IsActive.eq(true))
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to check active visits: {}", e)))?;

    if existing_active.is_some() {
        return Err(ServerFnError::new(
            "You already have an active project. Complete it first.",
        ));
    }

    // Start transaction for atomic assignment
    let txn = ctx
        .state
        .db
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to start transaction: {}", e)))?;

    // Get all team IDs for this hackathon
    let team_ids: Vec<i32> = teams::Entity::find()
        .filter(teams::Column::HackathonId.eq(hackathon.id))
        .select_only()
        .column(teams::Column::Id)
        .into_tuple::<i32>()
        .all(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch teams: {}", e)))?;

    if team_ids.is_empty() {
        txn.commit().await.ok();
        return Ok(None);
    }

    // Get all submissions
    let all_submissions = submission::Entity::find()
        .filter(submission::Column::TeamId.is_in(team_ids.clone()))
        .all(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch submissions: {}", e)))?;

    if all_submissions.is_empty() {
        txn.commit().await.ok();
        return Ok(None);
    }

    // Get IDs of submissions this judge has already visited
    let visited_ids: Vec<i32> = project_visit::Entity::find()
        .filter(project_visit::Column::JudgeId.eq(ctx.user.id))
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .select_only()
        .column(project_visit::Column::SubmissionId)
        .into_tuple::<i32>()
        .all(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch visited: {}", e)))?;

    // Get IDs of submissions currently being visited by any judge
    let locked_ids: Vec<i32> = project_visit::Entity::find()
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .filter(project_visit::Column::IsActive.eq(true))
        .select_only()
        .column(project_visit::Column::SubmissionId)
        .into_tuple::<i32>()
        .all(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch locked: {}", e)))?;

    // Filter to available submissions
    let available_submissions: Vec<_> = all_submissions
        .iter()
        .filter(|s| !visited_ids.contains(&s.id) && !locked_ids.contains(&s.id))
        .collect();

    if available_submissions.is_empty() {
        txn.commit().await.ok();
        return Ok(None);
    }

    // Get visit counts for each submission
    let mut submission_visit_counts: std::collections::HashMap<i32, u64> =
        std::collections::HashMap::new();
    for sub in &available_submissions {
        let count = project_visit::Entity::find()
            .filter(project_visit::Column::SubmissionId.eq(sub.id))
            .filter(project_visit::Column::HackathonId.eq(hackathon.id))
            .count(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to count visits: {}", e)))?;
        submission_visit_counts.insert(sub.id, count);
    }

    let mut rng = rand::thread_rng();
    let selected_sub;

    // Phase 1: Prioritize submissions with < 2 visits
    let under_visited: Vec<_> = available_submissions
        .iter()
        .filter(|s| submission_visit_counts.get(&s.id).copied().unwrap_or(0) < 2)
        .collect();

    if !under_visited.is_empty() {
        // Random selection from under-visited
        selected_sub = *under_visited.choose(&mut rng).unwrap();
    } else {
        // Phase 2: Softmax-weighted selection based on average feature scores
        let mut weights: Vec<f64> = Vec::new();

        for sub in &available_submissions {
            // Get average score across all features
            let score_records = project_feature_score::Entity::find()
                .filter(project_feature_score::Column::SubmissionId.eq(sub.id))
                .all(&txn)
                .await
                .unwrap_or_default();

            let scores: Vec<f32> = score_records.iter().filter_map(|r| r.score).collect();

            let avg_score = if scores.is_empty() {
                0.5 // Default score for unscored projects
            } else {
                scores.iter().sum::<f32>() / scores.len() as f32
            };

            // Softmax weight (higher scores get higher probability)
            weights.push((avg_score as f64).exp());
        }

        // Normalize weights
        let total: f64 = weights.iter().sum();
        if total > 0.0 {
            for w in &mut weights {
                *w /= total;
            }
        } else {
            // Equal weights if all zero
            let equal = 1.0 / weights.len() as f64;
            for w in &mut weights {
                *w = equal;
            }
        }

        // Sample using cumulative distribution
        let mut cumsum = 0.0;
        let sample: f64 = rng.random();
        let mut selected_idx = 0;
        for (i, &w) in weights.iter().enumerate() {
            cumsum += w;
            if sample <= cumsum {
                selected_idx = i;
                break;
            }
        }
        selected_sub = &available_submissions[selected_idx];
    }

    // Create the visit
    let now = chrono::Utc::now().naive_utc();
    let new_visit = project_visit::ActiveModel {
        id: NotSet,
        submission_id: Set(selected_sub.id),
        judge_id: Set(ctx.user.id),
        hackathon_id: Set(hackathon.id),
        notes: Set(None),
        start_time: Set(now),
        completion_time: Set(None),
        is_active: Set(true),
    };

    let visit = new_visit
        .insert(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create visit: {}", e)))?;

    txn.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to commit: {}", e)))?;

    // Get team details
    let team = teams::Entity::find_by_id(selected_sub.team_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    Ok(Some(CurrentProject {
        visit_id: visit.id,
        submission_id: selected_sub.id,
        team_name: team.name.clone(),
        project_name: selected_sub
            .submission_data
            .get("projectName")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string()),
        location: None, // Table number not tracked in teams
        description: selected_sub
            .submission_data
            .get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string()),
        submission_data: selected_sub.submission_data.clone(),
    }))
}

/// Submit comparisons for all features at once
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/submit",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    request_body = SubmitComparisonsRequest,
    responses(
        (status = 200, description = "Comparisons submitted successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/submit", user: SyncedUser)]
pub async fn submit_comparisons(
    slug: String,
    request: SubmitComparisonsRequest,
) -> Result<(), ServerFnError> {
    use crate::entities::{judge_feature_assignment, pairwise_comparison, project_visit};
    use sea_orm::{
        ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, QueryFilter, Set,
        TransactionTrait,
    };

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    if !hackathon.judging_started {
        return Err(ServerFnError::new("Judging has not started yet"));
    }

    // Verify the visit belongs to this judge
    let visit = project_visit::Entity::find_by_id(request.visit_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch visit: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Visit not found"))?;

    if visit.judge_id != ctx.user.id {
        return Err(ServerFnError::new("This is not your visit"));
    }

    if !visit.is_active {
        return Err(ServerFnError::new("This visit is already completed"));
    }

    let txn = ctx
        .state
        .db
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to start transaction: {}", e)))?;

    let current_submission_id = visit.submission_id;
    let now = chrono::Utc::now().naive_utc();

    for comparison in request.comparisons {
        // Get the judge's assignment for this feature
        let assignment = judge_feature_assignment::Entity::find()
            .filter(judge_feature_assignment::Column::JudgeId.eq(ctx.user.id))
            .filter(judge_feature_assignment::Column::FeatureId.eq(comparison.feature_id))
            .one(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch assignment: {}", e)))?;

        let assignment = match assignment {
            Some(a) => a,
            None => continue, // Skip if not assigned to this feature
        };

        let old_best_id = assignment.current_best_submission_id;

        // Record pairwise comparison if there was a previous best
        if let Some(prev_best_id) = old_best_id {
            let new_comparison = pairwise_comparison::ActiveModel {
                id: NotSet,
                feature_id: Set(comparison.feature_id),
                judge_id: Set(ctx.user.id),
                submission_a_id: Set(current_submission_id),
                submission_b_id: Set(prev_best_id),
                winner_id: Set(Some(comparison.winner_submission_id)),
                created_at: Set(now),
            };

            new_comparison
                .insert(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to save comparison: {}", e)))?;
        }

        // Update the assignment with the new best and notes
        let mut active: judge_feature_assignment::ActiveModel = assignment.into();
        active.current_best_submission_id = Set(Some(comparison.winner_submission_id));
        if let Some(notes) = comparison.notes {
            active.notes = Set(Some(notes));
        }

        active
            .update(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update assignment: {}", e)))?;
    }

    // Mark visit as complete
    let mut visit_active: project_visit::ActiveModel = visit.into();
    visit_active.is_active = Set(false);
    visit_active.completion_time = Set(Some(now));
    if let Some(notes) = request.notes {
        visit_active.notes = Set(Some(notes));
    }

    visit_active
        .update(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to complete visit: {}", e)))?;

    txn.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to commit: {}", e)))?;

    Ok(())
}

// ============================================================================
// Judge Assignment Management (Admin)
// ============================================================================

/// Get judges assigned to a feature
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/judging/features/{feature_id}/judges",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("feature_id" = i32, Path, description = "Feature ID")
    ),
    responses(
        (status = 200, description = "Judges retrieved", body = Vec<JudgeInfo>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[get("/api/hackathons/:slug/judging/features/:feature_id/judges", user: SyncedUser)]
pub async fn get_feature_judges(
    slug: String,
    feature_id: i32,
) -> Result<Vec<JudgeInfo>, ServerFnError> {
    use crate::entities::{judge_feature_assignment, users};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;

    let assignments = judge_feature_assignment::Entity::find()
        .filter(judge_feature_assignment::Column::FeatureId.eq(feature_id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch assignments: {}", e)))?;

    let mut judges = Vec::new();
    for assignment in assignments {
        if let Some(user_model) = users::Entity::find_by_id(assignment.judge_id)
            .one(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch user: {}", e)))?
        {
            judges.push(JudgeInfo {
                user_id: user_model.id,
                name: user_model.name.unwrap_or_else(|| "Unknown".to_string()),
                email: Some(user_model.email),
            });
        }
    }

    Ok(judges)
}

/// Assign judges to a feature
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/features/{feature_id}/judges",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("feature_id" = i32, Path, description = "Feature ID")
    ),
    request_body = AssignJudgesRequest,
    responses(
        (status = 200, description = "Judges assigned"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/features/:feature_id/judges", user: SyncedUser)]
pub async fn assign_judges(
    slug: String,
    feature_id: i32,
    request: AssignJudgesRequest,
) -> Result<(), ServerFnError> {
    use crate::entities::judge_feature_assignment;
    use sea_orm::{ActiveModelTrait, ActiveValue::NotSet, Set};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;

    for judge_id in request.judge_ids {
        let new_assignment = judge_feature_assignment::ActiveModel {
            id: NotSet,
            judge_id: Set(judge_id),
            feature_id: Set(feature_id),
            current_best_submission_id: Set(None),
            notes: Set(None),
            created_at: Set(chrono::Utc::now().naive_utc()),
        };

        // Use insert, ignoring conflicts (already assigned)
        let _ = new_assignment.insert(&ctx.state.db).await;
    }

    Ok(())
}

/// Unassign a judge from a feature
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/judging/features/{feature_id}/judges/{judge_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("feature_id" = i32, Path, description = "Feature ID"),
        ("judge_id" = i32, Path, description = "Judge user ID")
    ),
    responses(
        (status = 200, description = "Judge unassigned"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[delete("/api/hackathons/:slug/judging/features/:feature_id/judges/:judge_id", user: SyncedUser)]
pub async fn unassign_judge(
    slug: String,
    feature_id: i32,
    judge_id: i32,
) -> Result<(), ServerFnError> {
    use crate::entities::judge_feature_assignment;
    use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    Permissions::require_admin_or_organizer(&ctx).await?;

    let assignment = judge_feature_assignment::Entity::find()
        .filter(judge_feature_assignment::Column::FeatureId.eq(feature_id))
        .filter(judge_feature_assignment::Column::JudgeId.eq(judge_id))
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to find assignment: {}", e)))?;

    if let Some(a) = assignment {
        a.delete(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete assignment: {}", e)))?;
    }

    Ok(())
}

/// Get all features with their assigned judges (for admin view)
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/judging/features-with-judges",
    params(
        ("slug" = String, Path, description = "Hackathon slug")
    ),
    responses(
        (status = 200, description = "Features with judges", body = Vec<FeatureWithJudges>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[get("/api/hackathons/:slug/judging/features-with-judges", user: SyncedUser)]
pub async fn get_features_with_judges(
    slug: String,
) -> Result<Vec<FeatureWithJudges>, ServerFnError> {
    use crate::entities::{feature, judge_feature_assignment, users};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    let features = feature::Entity::find()
        .filter(feature::Column::HackathonId.eq(hackathon.id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch features: {}", e)))?;

    let mut result = Vec::new();
    for feat in features {
        let assignments = judge_feature_assignment::Entity::find()
            .filter(judge_feature_assignment::Column::FeatureId.eq(feat.id))
            .all(&ctx.state.db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch assignments: {}", e)))?;

        let mut judges = Vec::new();
        for assignment in assignments {
            if let Some(user_model) = users::Entity::find_by_id(assignment.judge_id)
                .one(&ctx.state.db)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to fetch user: {}", e)))?
            {
                judges.push(JudgeInfo {
                    user_id: user_model.id,
                    name: user_model.name.unwrap_or_else(|| "Unknown".to_string()),
                    email: Some(user_model.email),
                });
            }
        }

        result.push(FeatureWithJudges {
            feature: FeatureInfo {
                id: feat.id,
                name: feat.name,
                description: feat.description,
            },
            judges,
        });
    }

    Ok(result)
}

// ============================================================================
// Results Page Endpoints
// ============================================================================

/// Get results for a specific prize track
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/judging/results/{prize_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("prize_id" = i32, Path, description = "Prize track ID")
    ),
    responses(
        (status = 200, description = "Prize track results", body = PrizeTrackResults),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Prize not found"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[get("/api/hackathons/:slug/judging/results/:prize_id", user: SyncedUser)]
pub async fn get_prize_track_results(
    slug: String,
    prize_id: i32,
) -> Result<PrizeTrackResults, ServerFnError> {
    use crate::entities::{feature, prize, prize_feature_weight, submission, teams};
    use rand::Rng;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get the prize
    let prize_model = prize::Entity::find_by_id(prize_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch prize: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Prize not found"))?;

    // Get all features for this hackathon
    let features = feature::Entity::find()
        .filter(feature::Column::HackathonId.eq(hackathon.id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch features: {}", e)))?;

    let feature_infos: Vec<FeatureInfo> = features
        .iter()
        .map(|f| FeatureInfo {
            id: f.id,
            name: f.name.clone(),
            description: f.description.clone(),
        })
        .collect();

    // Get feature weights for this prize
    let weights = prize_feature_weight::Entity::find()
        .filter(prize_feature_weight::Column::PrizeId.eq(prize_id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch weights: {}", e)))?;

    let weight_map: std::collections::HashMap<i32, f32> =
        weights.iter().map(|w| (w.feature_id, w.weight)).collect();

    // Get all team IDs for this hackathon
    let team_ids: Vec<i32> = teams::Entity::find()
        .filter(teams::Column::HackathonId.eq(hackathon.id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch teams: {}", e)))?
        .iter()
        .map(|t| t.id)
        .collect();

    // Get all submissions for these teams
    let submissions = submission::Entity::find()
        .filter(submission::Column::TeamId.is_in(team_ids.clone()))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch submissions: {}", e)))?;

    // Get team info for each submission
    let teams_list = teams::Entity::find()
        .filter(teams::Column::HackathonId.eq(hackathon.id))
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch teams: {}", e)))?;

    let team_map: std::collections::HashMap<i32, String> =
        teams_list.iter().map(|t| (t.id, t.name.clone())).collect();

    // Build project results with mocked random scores
    // NOTE: In production, this would use the actual pairwise comparison algorithm
    let mut rng = rand::thread_rng();
    let mut projects: Vec<ProjectResultInfo> = Vec::new();

    for sub in &submissions {
        let team_name = team_map
            .get(&sub.team_id)
            .cloned()
            .unwrap_or_else(|| "Unknown Team".to_string());

        // Extract data from submission_data JSON
        let project_name = sub
            .submission_data
            .get("projectName")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        let description = sub
            .submission_data
            .get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        let repo_url = sub
            .submission_data
            .get("repoUrl")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        let presentation_url = sub
            .submission_data
            .get("presentationUrl")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        let video_url = sub
            .submission_data
            .get("videoUrl")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        // Generate mocked random scores for each feature
        // In production, these would come from the pairwise comparison algorithm
        let mut feature_ranks: Vec<FeatureRankInfo> = Vec::new();
        let mut weighted_score: f32 = 0.0;

        for feat in &features {
            // Mock: random score between 0.0 and 1.0
            let score: f32 = rng.r#gen();
            let weight = weight_map
                .get(&feat.id)
                .copied()
                .unwrap_or(1.0 / features.len() as f32);
            weighted_score += score * weight;

            feature_ranks.push(FeatureRankInfo {
                feature_id: feat.id,
                feature_name: feat.name.clone(),
                rank: None, // Will be computed after sorting
            });
        }

        // Mock AI summary with Lorem Ipsum
        let ai_summary = Some(
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
             Fusce consequat tincidunt urna placerat pulvinar. \
             Sed facilisis felis sed vehicula consequat. \
             Sed nisl augue, sollicitudin vel diam quis, lobortis posuere diam."
                .to_string(),
        );

        projects.push(ProjectResultInfo {
            submission_id: sub.id,
            project_name,
            team_name,
            weighted_score: Some(weighted_score),
            rank: 0, // Will be set after sorting
            feature_ranks,
            description,
            repo_url,
            presentation_url,
            video_url,
            ai_summary,
        });
    }

    // Sort projects by weighted score (descending)
    projects.sort_by(|a, b| {
        b.weighted_score
            .unwrap_or(0.0)
            .partial_cmp(&a.weighted_score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Assign overall ranks
    for (idx, project) in projects.iter_mut().enumerate() {
        project.rank = (idx + 1) as i32;
    }

    // Compute feature-specific ranks
    // For each feature, rank projects by their score in that feature
    for feat_idx in 0..features.len() {
        // Get scores for this feature (use random for mock)
        let mut feature_scores: Vec<(usize, f32)> = projects
            .iter()
            .enumerate()
            .map(|(i, _)| (i, rng.r#gen::<f32>()))
            .collect();

        feature_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Assign ranks
        for (rank, (proj_idx, _)) in feature_scores.iter().enumerate() {
            if feat_idx < projects[*proj_idx].feature_ranks.len() {
                projects[*proj_idx].feature_ranks[feat_idx].rank = Some((rank + 1) as i32);
            }
        }
    }

    Ok(PrizeTrackResults {
        prize_id: prize_model.id,
        prize_name: prize_model.name,
        features: feature_infos,
        projects,
    })
}

/// Get the current user's visit notes for a specific project
#[cfg_attr(feature = "server", utoipa::path(
    get,
    path = "/api/hackathons/{slug}/judging/my-notes/{submission_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("submission_id" = i32, Path, description = "Submission ID")
    ),
    responses(
        (status = 200, description = "Visit notes", body = JudgeVisitNotes),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[get("/api/hackathons/:slug/judging/my-notes/:submission_id", user: SyncedUser)]
pub async fn get_my_visit_notes(
    slug: String,
    submission_id: i32,
) -> Result<JudgeVisitNotes, ServerFnError> {
    use crate::entities::project_visit;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Find the most recent visit by this judge for this submission
    let visit = project_visit::Entity::find()
        .filter(project_visit::Column::JudgeId.eq(ctx.user.id))
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .filter(project_visit::Column::SubmissionId.eq(submission_id))
        .order_by_desc(project_visit::Column::StartTime)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch visit: {}", e)))?;

    match visit {
        Some(v) => Ok(JudgeVisitNotes {
            visited: true,
            notes: v.notes,
        }),
        None => Ok(JudgeVisitNotes {
            visited: false,
            notes: None,
        }),
    }
}

/// Generate an AI summary for a project based on all judge notes and description
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/judging/generate-summary/{submission_id}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("submission_id" = i32, Path, description = "Submission ID")
    ),
    responses(
        (status = 200, description = "AI summary generated", body = AiSummaryResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "judging"
))]
#[post("/api/hackathons/:slug/judging/generate-summary/:submission_id", user: SyncedUser)]
pub async fn generate_ai_summary(
    slug: String,
    submission_id: i32,
) -> Result<AiSummaryResponse, ServerFnError> {
    use crate::entities::{project_visit, submission, teams};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get the submission
    let sub = submission::Entity::find_by_id(submission_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch submission: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Submission not found"))?;

    // Get team name
    let team = teams::Entity::find_by_id(sub.team_id)
        .one(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Team not found"))?;

    // Get project description
    let description = sub
        .submission_data
        .get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("No description provided.");

    let project_name = sub
        .submission_data
        .get("projectName")
        .and_then(|n| n.as_str())
        .unwrap_or("Untitled Project");

    // Get all judge visits/notes for this submission
    let visits = project_visit::Entity::find()
        .filter(project_visit::Column::HackathonId.eq(hackathon.id))
        .filter(project_visit::Column::SubmissionId.eq(submission_id))
        .filter(project_visit::Column::CompletionTime.is_not_null())
        .all(&ctx.state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch visits: {}", e)))?;

    // Collect all judge notes
    let judge_notes: Vec<String> = visits
        .iter()
        .filter_map(|v| v.notes.clone())
        .filter(|n| !n.trim().is_empty())
        .collect();

    // Check if we have an API key
    let api_key = match &ctx.state.config.openrouter_api_key {
        Some(key) if !key.is_empty() => key.clone(),
        _ => {
            // Return a fallback summary if no API key
            return Ok(AiSummaryResponse {
                summary: format!(
                    "Project '{}' by team '{}'. {} judge(s) have reviewed this project. \
                     Configure OPENROUTER_API_KEY to enable AI summaries.",
                    project_name,
                    team.name,
                    visits.len()
                ),
            });
        }
    };

    // Build the prompt
    let notes_text = if judge_notes.is_empty() {
        "No judge notes available.".to_string()
    } else {
        judge_notes
            .iter()
            .enumerate()
            .map(|(i, n)| format!("Judge {}: {}", i + 1, n))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let prompt = format!(
        "You are summarizing a hackathon project for judges and organizers. \
         Be concise but informative. Focus on the key aspects and any feedback from judges.\n\n\
         Project Name: {}\n\
         Team Name: {}\n\n\
         Project Description:\n{}\n\n\
         Judge Notes:\n{}\n\n\
         Please provide a brief summary (2-3 sentences) of this project and the key points from the judge feedback.",
        project_name, team.name, description, notes_text
    );

    // Call OpenRouter API
    let client = reqwest::Client::new();
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "openai/gpt-4o-mini",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 300,
            "temperature": 0.7
        }))
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to call OpenRouter: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!(
            "OpenRouter API error: {}",
            error_text
        )));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to parse OpenRouter response: {}", e)))?;

    let summary = response_json["choices"]
        .get(0)
        .and_then(|c| c["message"]["content"].as_str())
        .unwrap_or("Failed to generate summary.")
        .to_string();

    Ok(AiSummaryResponse { summary })
}
