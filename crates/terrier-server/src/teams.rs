use axum::{
    Json,
    Router,
    extract::{
        Query,
        State,
        rejection::QueryRejection,
        // rejection::JsonRejection,
    },
    // http::StatusCode,
    routing::get,
    // routing::patch,
};

// use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait,
    EntityTrait,
    ExprTrait,
    PaginatorTrait,
    QueryFilter,
    QueryOrder,
    QuerySelect,
    QueryTrait,
    // Set,
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

use crate::{
    // audit::{Audit, Scope},
    // audited,
    auth::policy::{
        Auth,
        HackathonMember,
        ViewTeamDetails,
        // CanRemoveTeamMember,
    },
    entities::{
        // hackathon,
        project,
        sea_orm_active_enums::{ProjectStatus, TeamMemberStatus},
        team,
        team_member,
        user,
    },
    error::ApiError,
    state::AppState,
};

const PAGE_SIZE: u64 = 20;

// --------------------------------------------------
// Team-list request and response types
// --------------------------------------------------

#[derive(Deserialize)]
struct ListTeamsQuery {
    page: Option<u64>,
    seeking_members: Option<bool>,
}

#[derive(Serialize, ToSchema)]
struct TeamSummary {
    id: String,
    name: String,
    description: String,
}

#[derive(Serialize, ToSchema)]
struct ListTeamsResponse {
    teams: Vec<TeamSummary>,
    page: u64,
    page_size: u64,
}

// --------------------------------------------------
// Team-details response types
// --------------------------------------------------

#[derive(Clone, Serialize, ToSchema)]
struct TeamMemberSummary {
    user_id: String,
    display_name: String,
    is_leader: bool,
}

#[derive(Serialize, ToSchema)]
struct SubmittedProject {
    id: String,
    data: serde_json::Value,
    submitted_at: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct RestrictedTeamDetails {
    members: Vec<TeamMemberSummary>,
    latest_submission: Option<SubmittedProject>,
}

#[derive(Serialize, ToSchema)]
struct TeamDetailsResponse {
    id: String,
    name: String,
    description: String,
    member_count: u64,
    leader: Option<TeamMemberSummary>,

    // Omitted for eligible hackers outside the team.
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<RestrictedTeamDetails>,
}

// --------------------------------------------------
// GET /api/hackathons/{slug}/teams
// --------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/hackathons/{slug}/teams",
    tag = "teams",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("page" = Option<u64>, Query,
            description = "Page number starting at 1; defaults to 1"),
        ("seeking_members" = Option<bool>, Query,
            description = "Only teams below capacity; defaults to false")
    ),
    responses(
        (status = 200, body = ListTeamsResponse),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)
    )
)]
async fn list_teams(
    auth: Auth<HackathonMember>,
    State(state): State<AppState>,
    query: Result<Query<ListTeamsQuery>, QueryRejection>,
) -> Result<Json<ListTeamsResponse>, ApiError> {
    let Query(query) =
        query.map_err(|_| ApiError::BadRequest("Invalid query parameters.".into()))?;

    let page = query.page.unwrap_or(1);
    let seeking_members = query.seeking_members.unwrap_or(false);

    let offset = page
        .checked_sub(1)
        .and_then(|index| index.checked_mul(PAGE_SIZE))
        .ok_or_else(|| ApiError::BadRequest("Page must be at least 1 and within range.".into()))?;

    let hackathon_id = auth.data.hackathon.id;
    let max_team_size = auth.data.hackathon.max_team_size;

    if seeking_members && max_team_size <= 0 {
        return Ok(Json(ListTeamsResponse {
            teams: Vec::new(),
            page,
            page_size: PAGE_SIZE,
        }));
    }

    // Only active teams in the authorized hackathon.
    let mut teams_query = team::Entity::find()
        .filter(team::Column::HackathonId.eq(hackathon_id))
        .filter(team::Column::IsActive.eq(true));

    if seeking_members {
        // Find teams at or above capacity, counting active members.
        let full_teams = team_member::Entity::find()
            .select_only()
            .column(team_member::Column::TeamId)
            .filter(team_member::Column::HackathonId.eq(hackathon_id))
            .filter(team_member::Column::Status.eq(TeamMemberStatus::Active))
            .group_by(team_member::Column::TeamId)
            .having(
                team_member::Column::UserId
                    .count()
                    .gte(i64::from(max_team_size)),
            )
            .into_query();

        // Exclude full teams before pagination.
        teams_query = teams_query.filter(team::Column::Id.not_in_subquery(full_teams));
    }

    let rows = teams_query
        .order_by_asc(team::Column::Name)
        .order_by_asc(team::Column::Id)
        .offset(offset)
        .limit(PAGE_SIZE)
        .all(&state.db)
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;

    let teams = rows
        .into_iter()
        .map(|row| TeamSummary {
            id: row.id.to_string(),
            name: row.name,
            description: row.description,
        })
        .collect();

    Ok(Json(ListTeamsResponse {
        teams,
        page,
        page_size: PAGE_SIZE,
    }))
}

// --------------------------------------------------
// GET /api/hackathons/{slug}/teams/{team_id}
// --------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/hackathons/{slug}/teams/{team_id}",
    tag = "teams",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("team_id" = String, Path, description = "Team UUID")
    ),
    responses(
        (status = 200, body = TeamDetailsResponse),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)
    )
)]
async fn get_team_details(
    auth: Auth<ViewTeamDetails>,
    State(state): State<AppState>,
) -> Result<Json<TeamDetailsResponse>, ApiError> {
    let access = auth.data;

    // The policy already checked team activity and hackathon scope.
    let team = access.team;
    let hackathon_id = access.membership.hackathon.id;
    let can_view_details = access.can_view_restricted_details;

    let active_members = team_member::Entity::find()
        .filter(team_member::Column::TeamId.eq(team.id))
        .filter(team_member::Column::HackathonId.eq(hackathon_id))
        .filter(team_member::Column::Status.eq(TeamMemberStatus::Active));

    // Outside viewers may see the count, but not member identities.
    let outside_member_count = if can_view_details {
        None
    } else {
        Some(
            active_members
                .clone()
                .count(&state.db)
                .await
                .map_err(|error| ApiError::Internal(error.into()))?,
        )
    };

    let mut visible_members_query = active_members;

    if !can_view_details {
        // Outside hackers may see only the active leader.
        visible_members_query =
            visible_members_query.filter(team_member::Column::IsLeader.eq(true));
    }

    // Join users in one query to obtain their display names.
    let member_rows = visible_members_query
        .order_by_desc(team_member::Column::IsLeader)
        .order_by_asc(team_member::Column::UserId)
        .find_also_related(user::Entity)
        .all(&state.db)
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;

    let mut visible_members = Vec::with_capacity(member_rows.len());

    for (member, related_user) in member_rows {
        let related_user = related_user.ok_or_else(|| {
            ApiError::Internal(
                std::io::Error::other("Team membership references a missing user.").into(),
            )
        })?;

        visible_members.push(TeamMemberSummary {
            user_id: member.user_id.to_string(),
            display_name: related_user.display_name,
            is_leader: member.is_leader,
        });
    }

    let member_count = outside_member_count.unwrap_or(visible_members.len() as u64);

    // Leader information is available to every authorized viewer.
    let leader = visible_members
        .iter()
        .find(|member| member.is_leader)
        .cloned();

    let details = if can_view_details {
        // Fetch submission data only for authorized callers.
        let submission = project::Entity::find()
            .filter(project::Column::TeamId.eq(team.id))
            .filter(project::Column::HackathonId.eq(hackathon_id))
            .filter(project::Column::Status.eq(ProjectStatus::Submitted))
            .filter(project::Column::SubmittedAt.is_not_null())
            .order_by_desc(project::Column::SubmittedAt)
            .order_by_desc(project::Column::Id)
            .one(&state.db)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?;

        let latest_submission = submission.map(|submission| SubmittedProject {
            id: submission.id.to_string(),
            data: submission.data,
            submitted_at: submission.submitted_at.map(|time| time.to_rfc3339()),
        });

        Some(RestrictedTeamDetails {
            members: visible_members,
            latest_submission,
        })
    } else {
        None
    };

    Ok(Json(TeamDetailsResponse {
        id: team.id.to_string(),
        name: team.name,
        description: team.description,
        member_count,
        leader,
        details,
    }))
}

// #[derive(Deserialize, ToSchema)]
// #[serde(deny_unknown_fields)]
// struct RemoveTeamMemberInput {
//     is_active: bool,
// }

// #[utoipa::path(
//     patch,
//     path = "/api/hackathons/{slug}/teams/{team_id}/members/{user_id}",
//     tag = "teams",
//     params(
//         ("slug" = String, Path, description = "Hackathon slug"),
//         ("team_id" = String, Path, description = "Team UUID"),
//         ("user_id" = String, Path,
//             description = "User to remove; use your own ID to leave")
//     ),
//     request_body = RemoveTeamMemberInput,
//     responses(
//         (status = 204, description = "Membership removed"),
//         (status = 400, body = crate::error::ErrorResponse),
//         (status = 401, body = crate::error::ErrorResponse),
//         (status = 403, body = crate::error::ErrorResponse),
//         (status = 404, body = crate::error::ErrorResponse),
//         (status = 500, body = crate::error::ErrorResponse)
//     )
// )] async fn remove_team_member(
//     auth: Auth<CanRemoveTeamMember>,
//     audit: Audit,
//     input: Result<Json<RemoveTeamMemberInput>, JsonRejection>,
// ) -> Result<StatusCode, ApiError> {
//     let Json(input) = input.map_err(|_| {
//         ApiError::BadRequest("Invalid JSON request.".into())
//     })?;

//     // This endpoint supports removal, not restoration.
//     if input.is_active {
//         return Err(ApiError::BadRequest(
//             "Set is_active to false to leave or remove a member.".into(),
//         ));
//     }

//     let access = auth.data;
//     let hackathon_id = access.hackathon.id;
//     let team_id = access.team.id;
//     let caller_id = access.user.user().id;
//     let target_id = access.target_membership.user_id;

//     audited!(audit, Scope::Hackathon(hackathon_id), |db| {
//         // Lock the team before checking and changing memberships.
//         let checked_team = team::Entity::find_by_id(team_id)
//             .filter(team::Column::HackathonId.eq(hackathon_id))
//             .filter(team::Column::IsActive.eq(true))
//             .lock_exclusive()
//             .one(db)
//             .await
//             .map_err(|error| ApiError::Internal(error.into()))?
//             .ok_or(ApiError::NotFound)?;

//         // Recheck the parent after waiting for the lock.
//         hackathon::Entity::find_by_id(hackathon_id)
//             .filter(hackathon::Column::IsActive.eq(true))
//             .one(db)
//             .await
//             .map_err(|error| ApiError::Internal(error.into()))?
//             .ok_or(ApiError::NotFound)?;

//         // Reload and lock active memberships.
//         // Earliest joined member is the first successor candidate.
//         let members = team_member::Entity::find()
//             .filter(team_member::Column::TeamId.eq(checked_team.id))
//             .filter(
//                 team_member::Column::HackathonId.eq(hackathon_id),
//             )
//             .filter(
//                 team_member::Column::Status
//                     .eq(TeamMemberStatus::Active),
//             )
//             .order_by_asc(team_member::Column::JoinedAt)
//             .order_by_asc(team_member::Column::UserId)
//             .lock_exclusive()
//             .all(db)
//             .await
//             .map_err(|error| ApiError::Internal(error.into()))?;

//         let caller = members
//             .iter()
//             .find(|member| member.user_id == caller_id)
//             .ok_or(ApiError::Forbidden)?;

//         // Recheck permissions using the freshly loaded membership.
//         if caller_id != target_id && !caller.is_leader {
//             return Err(ApiError::Forbidden);
//         }

//         let target = members
//             .iter()
//             .find(|member| member.user_id == target_id)
//             .ok_or(ApiError::NotFound)?;

//         let target_was_leader = target.is_leader;

//         let successor_id = members
//             .iter()
//             .find(|member| member.user_id != target_id)
//             .map(|member| member.user_id);

//         // Retain the membership row and end this membership.
//         team_member::Entity::update_many()
//             .set(team_member::ActiveModel {
//                 status: Set(TeamMemberStatus::Removed),
//                 is_leader: Set(false),
//                 ..Default::default()
//             })
//             .col_expr(
//                 team_member::Column::EndedAt,
//                 Expr::current_timestamp(),
//             )
//             .filter(team_member::Column::TeamId.eq(team_id))
//             .filter(
//                 team_member::Column::HackathonId.eq(hackathon_id),
//             )
//             .filter(team_member::Column::UserId.eq(target_id))
//             .filter(
//                 team_member::Column::Status
//                     .eq(TeamMemberStatus::Active),
//             )
//             .exec(db)
//             .await
//             .map_err(|error| ApiError::Internal(error.into()))?;

//         if let Some(successor_id) = successor_id {
//             if target_was_leader {
//                 // Transfer leadership after clearing the old leader.
//                 team_member::Entity::update_many()
//                     .set(team_member::ActiveModel {
//                         is_leader: Set(true),
//                         ..Default::default()
//                     })
//                     .filter(
//                         team_member::Column::TeamId.eq(team_id),
//                     )
//                     .filter(
//                         team_member::Column::HackathonId
//                             .eq(hackathon_id),
//                     )
//                     .filter(
//                         team_member::Column::UserId.eq(successor_id),
//                     )
//                     .filter(
//                         team_member::Column::Status
//                             .eq(TeamMemberStatus::Active),
//                     )
//                     .exec(db)
//                     .await
//                     .map_err(|error| {
//                         ApiError::Internal(error.into())
//                     })?;
//             }
//         } else {
//             // No active members remain. Retain but deactivate the team.
//             team::Entity::update_many()
//                 .set(team::ActiveModel {
//                     is_active: Set(false),
//                     ..Default::default()
//                 })
//                 .filter(team::Column::Id.eq(team_id))
//                 .filter(
//                     team::Column::HackathonId.eq(hackathon_id),
//                 )
//                 .exec(db)
//                 .await
//                 .map_err(|error| ApiError::Internal(error.into()))?;
//         }

//         Ok(())
//     })
//     .await?;

//     Ok(StatusCode::NO_CONTENT)
// }

// --------------------------------------------------
// Route and OpenAPI registration
// --------------------------------------------------

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/hackathons/{slug}/teams", get(list_teams))
        .route("/hackathons/{slug}/teams/{team_id}", get(get_team_details))
    // .route(
    //     "/hackathons/{slug}/teams/{team_id}/members/{user_id}",
    //     patch(remove_team_member),
    // )
}

#[derive(OpenApi)]
#[openapi(paths(
    list_teams,
    get_team_details,
    // remove_team_member
))]
pub struct TeamsApiDoc;
