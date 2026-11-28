use std::collections::BTreeMap;

use anyhow::Context;

use axum::{
    Json, Router,
    extract::{
        Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    routing::get,
};
use sea_orm::{
    ActiveEnum, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Select, Statement,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use crate::{
    audit::{Audit, Scope},
    audited,
    auth::policy::{
        Applicant, ApplicationAccess, Auth, ListApplications, OwnApplication, ReadApplication,
    },
    entities::{
        application,
        sea_orm_active_enums::{self as db_enum, HackathonRole},
        user_hackathon_role,
    },
    error::ApiError,
    files::{
        Attachment, FileFields, FileInfo, mark_submitted, parse_file_fields, referenced_files,
        visible_files,
    },
    state::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(list_applications, read_application, save_application))]
pub struct ApplicationsApiDoc;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/hackathons/{slug}/applications", get(list_applications))
        .route(
            "/hackathons/{slug}/applications/{user_id}",
            get(read_application).put(save_application),
        )
}

#[derive(Serialize, ToSchema)]
struct Application {
    id: String,
    user_id: String,
    /// `pending`, `accepted`, `rejected` or `waitlisted`; `null` while still a draft.
    status: Option<String>,
    /// The latest submitted revision. Empty until the first submission.
    #[schema(value_type = Object)]
    data: Value,
    /// The applicant's saved working copy. Returned only to the applicant.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    draft_data: Option<Value>,
    submitted_at: Option<String>,
    /// `pending`, `confirmed`, `declined`, `expired` or `revoked`; `null` until accepted.
    attendance_status: Option<String>,
    confirmation_deadline: Option<String>,
    /// Uploaded files. Application managers see only files used by a submitted revision.
    files: Vec<FileInfo>,
}

impl Application {
    fn view(model: application::Model, access: ApplicationAccess) -> Result<Self, ApiError> {
        Ok(Self {
            files: visible_files(&model, access)?,
            id: model.id.to_string(),
            user_id: model.user_id.to_string(),
            status: model.status.map(|status| status.to_value()),
            data: model.data,
            draft_data: (access == ApplicationAccess::Owner).then_some(model.draft_data),
            submitted_at: model.submitted_at.map(|at| at.to_rfc3339()),
            attendance_status: model.attendance_status.map(|status| status.to_value()),
            confirmation_deadline: model.confirmation_deadline.map(|at| at.to_rfc3339()),
        })
    }
}

#[derive(Serialize, ToSchema)]
struct ApplicationList {
    applications: Vec<Application>,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ApplicationFilter {
    /// Only applications with this status: `pending`, `accepted`, `rejected` or `waitlisted`.
    status: Option<String>,
}

/// An applicant's draft save or submission.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct SaveApplication {
    /// Answers to the hackathon's application form, as a JSON object.
    #[schema(value_type = Object)]
    data: Value,
    /// `true` submits `data` as the next revision; `false` only saves the working copy.
    #[serde(default)]
    submit: bool,
}

pub(crate) fn applications_in(
    hackathon_id: Uuid,
    access: ApplicationAccess,
) -> Select<application::Entity> {
    let query = application::Entity::find()
        .filter(application::Column::HackathonId.eq(hackathon_id))
        .filter(application::Column::IsActive.eq(true));
    match access {
        ApplicationAccess::Owner => query,
        ApplicationAccess::Manager => query.filter(application::Column::Status.is_not_null()),
        ApplicationAccess::Organizer => query
            .filter(application::Column::Status.eq(db_enum::ApplicationStatus::Accepted))
            .filter(application::Column::AttendanceStatus.eq(db_enum::AttendanceStatus::Confirmed)),
    }
}

#[utoipa::path(get, path = "/api/hackathons/{slug}/applications", tag = "applications",
    params(("slug" = String, Path, description = "Hackathon slug"), ApplicationFilter),
    responses((status = 200, body = ApplicationList),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn list_applications(
    auth: Auth<ListApplications>,
    State(state): State<AppState>,
    filter: Result<Query<ApplicationFilter>, QueryRejection>,
) -> Result<Json<ApplicationList>, ApiError> {
    let Query(filter) =
        filter.map_err(|_error| ApiError::BadRequest("Invalid application filter.".into()))?;
    let (hackathon_id, access) = auth.data;
    let mut query = applications_in(hackathon_id, access);
    if let Some(status) = filter.status {
        let status = db_enum::ApplicationStatus::try_from_value(&status)
            .map_err(|_error| ApiError::BadRequest("Unknown application status.".into()))?;
        query = query.filter(application::Column::Status.eq(status));
    }
    let applications = query
        .order_by_asc(application::Column::SubmittedAt)
        .order_by_asc(application::Column::Id)
        .all(&state.db)
        .await?
        .into_iter()
        .map(|model| Application::view(model, access))
        .collect::<Result<_, _>>()?;
    Ok(Json(ApplicationList { applications }))
}

#[utoipa::path(get, path = "/api/hackathons/{slug}/applications/{user_id}", tag = "applications",
    params(("slug" = String, Path, description = "Hackathon slug"),
        ("user_id" = String, Path, description = "Applicant's Terrier user UUID")),
    responses((status = 200, body = Application),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn read_application(
    auth: Auth<ReadApplication>,
    State(state): State<AppState>,
) -> Result<Json<Application>, ApiError> {
    let target = auth.data;
    let model = applications_in(target.hackathon_id, target.access)
        .filter(application::Column::UserId.eq(target.applicant_id))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(Application::view(model, target.access)?))
}

#[utoipa::path(put, path = "/api/hackathons/{slug}/applications/{user_id}", tag = "applications",
    params(("slug" = String, Path, description = "Hackathon slug"),
        ("user_id" = String, Path, description = "The signed-in user's Terrier UUID")),
    request_body = SaveApplication,
    responses((status = 200, body = Application),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 409, body = crate::error::ErrorResponse),
        (status = 422, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn save_application(
    auth: Auth<OwnApplication>,
    audit: Audit,
    input: Result<Json<SaveApplication>, JsonRejection>,
) -> Result<Json<Application>, ApiError> {
    let Json(input) =
        input.map_err(|_error| ApiError::BadRequest("Invalid JSON request.".into()))?;
    if !input.data.is_object() {
        return Err(ApiError::Validation(BTreeMap::from([(
            "data".into(),
            "Application data must be a JSON object.".into(),
        )])));
    }
    // TODO: validate `data` against `hackathon.application_schema` once its format is defined
    let Applicant {
        user_id,
        hackathon_id,
        has_role,
    } = auth.data;
    let scope = if has_role {
        Scope::Hackathon(hackathon_id)
    } else {
        Scope::HackathonExclusive(hackathon_id)
    };
    let saved = audited!(audit, scope, |db| {
        write_application(db, user_id, hackathon_id, !has_role, input).await
    })
    .await?;
    Ok(Json(Application::view(saved, ApplicationAccess::Owner)?))
}

pub(crate) fn statement(sql: &str, values: impl IntoIterator<Item = sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

async fn write_application(
    db: &DatabaseTransaction,
    user_id: Uuid,
    hackathon_id: Uuid,
    create_role: bool,
    input: SaveApplication,
) -> Result<application::Model, ApiError> {
    if create_role {
        db.execute_raw(statement(
            "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) VALUES($1,$2,'hacker')
                ON CONFLICT (user_id,hackathon_id) DO NOTHING",
            [user_id.into(), hackathon_id.into()],
        ))
        .await?;
    }
    let role = user_hackathon_role::Entity::find_by_id((user_id, hackathon_id))
        .one(db)
        .await?
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("hackathon role is missing")))?;
    if !role.is_active {
        return Err(ApiError::Forbidden);
    }
    if role.role != HackathonRole::Hacker {
        return Err(ApiError::Conflict(
            "Your account has a different role in this hackathon.".into(),
        ));
    }

    let existing = application::Entity::find()
        .filter(application::Column::UserId.eq(user_id))
        .filter(application::Column::HackathonId.eq(hackathon_id))
        .lock_exclusive()
        .one(db)
        .await?;

    let file_fields = open_hackathon(db, hackathon_id).await?;
    let mut attachments = match &existing {
        Some(current) => {
            ensure_editable(current)?;
            Attachment::list(current)?
        }
        None => Vec::new(),
    };
    let referenced = referenced_files(&file_fields, &attachments, &input.data)?;

    let id = match existing {
        None => {
            let id = Uuid::new_v4();
            let sql = if input.submit {
                "INSERT INTO application(id,user_id,hackathon_id,status,data,draft_data,submitted_at)
                    VALUES($1,$2,$3,'pending',$4,$4,clock_timestamp())
                    ON CONFLICT (user_id,hackathon_id) DO NOTHING RETURNING id"
            } else {
                "INSERT INTO application(id,user_id,hackathon_id,data,draft_data)
                    VALUES($1,$2,$3,'{}',$4)
                    ON CONFLICT (user_id,hackathon_id) DO NOTHING RETURNING id"
            };
            db.query_one_raw(statement(
                sql,
                [
                    id.into(),
                    user_id.into(),
                    hackathon_id.into(),
                    input.data.into(),
                ],
            ))
            .await?
            .ok_or_else(|| {
                ApiError::Conflict("The application was saved in another tab. Try again.".into())
            })?;
            id
        }
        Some(current) => {
            let new_revision =
                input.submit && (current.status.is_none() || current.data != input.data);
            if new_revision {
                mark_submitted(&mut attachments, &referenced);
                let attachments = serde_json::to_value(&attachments)
                    .context("serialize attachments")
                    .map_err(ApiError::Internal)?;
                db.execute_raw(statement(
                    "UPDATE application SET status=coalesce(status,'pending'), data=$3,
                        draft_data=$3, submitted_at=clock_timestamp(), attachments=$4
                        WHERE id=$1 AND hackathon_id=$2",
                    [
                        current.id.into(),
                        hackathon_id.into(),
                        input.data.into(),
                        attachments.into(),
                    ],
                ))
                .await?;
            } else if current.draft_data != input.data {
                db.execute_raw(statement(
                    "UPDATE application SET draft_data=$3 WHERE id=$1 AND hackathon_id=$2",
                    [current.id.into(), hackathon_id.into(), input.data.into()],
                ))
                .await?;
            }
            current.id
        }
    };

    application::Entity::find_by_id(id)
        .filter(application::Column::HackathonId.eq(hackathon_id))
        .one(db)
        .await?
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("saved application is missing")))
}

/// Rejects changes to deactivated or decided applications.
pub(crate) fn ensure_editable(current: &application::Model) -> Result<(), ApiError> {
    if !current.is_active {
        return Err(ApiError::Conflict(
            "This application was deactivated. Contact the organizers.".into(),
        ));
    }
    if !matches!(
        current.status,
        None | Some(db_enum::ApplicationStatus::Pending)
    ) {
        return Err(ApiError::Conflict(
            "This application can no longer be edited.".into(),
        ));
    }
    Ok(())
}

/// Returns the hackathon's file fields while applications are open.
pub(crate) async fn open_hackathon(
    db: &DatabaseTransaction,
    hackathon_id: Uuid,
) -> Result<FileFields, ApiError> {
    let row = db
        .query_one_raw(statement(
            "SELECT application_deadline > clock_timestamp() AS open, file_fields
                FROM hackathon WHERE id=$1 AND is_active",
            [hackathon_id.into()],
        ))
        .await?
        .ok_or(ApiError::NotFound)?;
    if !row.try_get::<bool>("", "open")? {
        return Err(ApiError::Conflict(
            "The application deadline has passed.".into(),
        ));
    }
    parse_file_fields(row.try_get("", "file_fields")?)
}
