use std::collections::HashMap;

use axum::{Json, Router, extract::State, routing::get};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::{
    auth::policy::{Auth, ViewOwnJudgeAssignments},
    entities::{
        audit_event, evaluation,
        sea_orm_active_enums::{AuditResource, EvaluationStatus, JudgingMethod},
    },
    error::ApiError,
    state::AppState,
};

// Fields read from the stored project snapshot.
// Extra snapshot fields are ignored.
#[derive(Deserialize)]
struct ProjectSnapshot {
    id: Uuid,
    hackathon_id: Uuid,
    status: String,
    track_ids: Vec<Uuid>,
    data: Value,
}

#[derive(Serialize, ToSchema)]
struct AssignedProject {
    id: String,
    snapshot_id: Option<String>,
    data: Option<Value>,
}

#[derive(Serialize, ToSchema)]
struct JudgeAssignment {
    evaluation_id: String,
    project: AssignedProject,
    status: String,
    judging_method: String,
    rubric: Value,
    created_at: String,
}

#[derive(Serialize, ToSchema)]
struct JudgeAssignmentsResponse {
    hackathon_id: String,
    track_id: String,
    judge_id: String,
    assignments: Vec<JudgeAssignment>,
}

fn invalid_snapshot() -> ApiError {
    ApiError::Internal(
        std::io::Error::other("Evaluation references a missing or invalid project snapshot.")
            .into(),
    )
}

#[utoipa::path(
    get,
    path = "/api/hackathons/{slug}/tracks/{track_id}/judges/{judge_id}/assignments",
    tag = "judging",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("track_id" = String, Path, description = "Track UUID"),
        ("judge_id" = String, Path,
            description = "Authenticated judge's user UUID")
    ),
    responses(
        (status = 200, body = JudgeAssignmentsResponse),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)
    )
)]
async fn list_judge_assignments(
    auth: Auth<ViewOwnJudgeAssignments>,
    State(state): State<AppState>,
) -> Result<Json<JudgeAssignmentsResponse>, ApiError> {
    let access = auth.data;

    // Return all current pending assignments for this judge/track.
    let evaluations = evaluation::Entity::find()
        .filter(evaluation::Column::HackathonId.eq(access.hackathon_id))
        .filter(evaluation::Column::TrackId.eq(access.track_id))
        .filter(evaluation::Column::JudgeId.eq(access.judge_id))
        .filter(evaluation::Column::Status.eq(EvaluationStatus::Pending))
        .filter(evaluation::Column::EndedAt.is_null())
        .order_by_asc(evaluation::Column::CreatedAt)
        .order_by_asc(evaluation::Column::Id)
        .all(&state.db)
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;

    // Fetch referenced snapshots together instead of one per project.
    let snapshot_ids: Vec<Uuid> = evaluations
        .iter()
        .filter_map(|evaluation| evaluation.project_audit_id)
        .collect();

    let mut snapshots: HashMap<Uuid, ProjectSnapshot> = HashMap::new();

    if !snapshot_ids.is_empty() {
        let snapshot_rows = audit_event::Entity::find()
            .filter(audit_event::Column::Id.is_in(snapshot_ids))
            .filter(audit_event::Column::HackathonId.eq(access.hackathon_id))
            .filter(audit_event::Column::ResourceType.eq(AuditResource::Project))
            .all(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?;

        for row in snapshot_rows {
            let payload = row.after_data.ok_or_else(invalid_snapshot)?;

            let snapshot: ProjectSnapshot =
                serde_json::from_value(payload).map_err(|_| invalid_snapshot())?;

            snapshots.insert(row.id, snapshot);
        }
    }

    let mut assignments = Vec::with_capacity(evaluations.len());

    for evaluation in evaluations {
        let project_data = if let Some(snapshot_id) = evaluation.project_audit_id {
            let snapshot = snapshots.get(&snapshot_id).ok_or_else(invalid_snapshot)?;

            // Confirm the historical content matches this assignment.
            if snapshot.id != evaluation.project_id
                || snapshot.hackathon_id != access.hackathon_id
                || snapshot.status != "submitted"
                || !snapshot.track_ids.contains(&access.track_id)
            {
                return Err(invalid_snapshot());
            }

            Some(snapshot.data.clone())
        } else {
            // No historical snapshot is referenced.
            // Do not silently substitute the live project's content.
            None
        };

        let judging_method = match evaluation.judging_method {
            JudgingMethod::Scoring => "scoring",
            JudgingMethod::Expo => "expo",
        };

        assignments.push(JudgeAssignment {
            evaluation_id: evaluation.id.to_string(),
            project: AssignedProject {
                id: evaluation.project_id.to_string(),
                snapshot_id: evaluation.project_audit_id.map(|id| id.to_string()),
                data: project_data,
            },
            status: "pending".to_owned(),
            judging_method: judging_method.to_owned(),
            rubric: evaluation.rubric,
            created_at: evaluation.created_at.to_string(),
        });
    }

    Ok(Json(JudgeAssignmentsResponse {
        hackathon_id: access.hackathon_id.to_string(),
        track_id: access.track_id.to_string(),
        judge_id: access.judge_id.to_string(),
        assignments,
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/hackathons/{slug}/tracks/{track_id}/judges/{judge_id}/assignments",
        get(list_judge_assignments),
    )
}

#[derive(OpenApi)]
#[openapi(paths(list_judge_assignments))]
pub struct JudgingApiDoc;
