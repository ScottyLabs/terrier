//! Unified judging mode: two-phase project selection and batch comparison submission.

use crate::domain::judging::types::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::core::auth::{context::RequestContext, middleware::SyncedUser};

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
            location: None, // Deprecated in favor of table_number
            table_number: sub.table_number.clone(),
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

    // Get all submissions with a table number assigned
    let all_submissions = submission::Entity::find()
        .filter(submission::Column::TeamId.is_in(team_ids.clone()))
        .filter(submission::Column::TableNumber.is_not_null())
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

    let mut rng = rand::rng();
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
        location: None,
        table_number: selected_sub.table_number.clone(),
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
