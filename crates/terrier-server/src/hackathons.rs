use std::collections::BTreeMap;

use anyhow::Context;
use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::{get, patch, post},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Set,
    SqlErr, Statement, entity::prelude::DateTimeWithTimeZone,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::{
    audit::{Audit, Scope},
    audited,
    auth::{
        current_user::CurrentUser,
        policy::{Auth, InstanceAdmin, ManageOrganizers, active_hackathon, ensure_manager},
    },
    entities::{
        hackathon, organizer,
        sea_orm_active_enums::{Capability, HackathonRole},
        user_hackathon_role,
    },
    error::{ApiError, ErrorResponse},
    files::{FileFields, default_file_fields, parse_file_fields, validate_file_fields},
    state::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(create, read, update))]
pub struct HackathonApiDoc;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct Settings {
    name: String,
    description: String,
    location: String,
    contact_email: String,
    max_team_size: i32,
    max_participants: i32,
    #[schema(value_type = String, format = DateTime)]
    application_deadline: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    team_formation_deadline: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    submission_deadline: DateTimeWithTimeZone,
    application_schema: Value,
    /// Upload rules keyed by application field. Omitted fields default to PDFs up to 5 MB.
    #[serde(default = "default_file_fields")]
    file_fields: FileFields,
}

impl Settings {
    fn validate(&self) -> Result<(), ApiError> {
        let mut errors = BTreeMap::new();
        for (field, value) in [
            ("name", &self.name),
            ("description", &self.description),
            ("location", &self.location),
            ("contact_email", &self.contact_email),
        ] {
            if value.trim().is_empty() || value.chars().count() > 255 {
                errors.insert(
                    format!("settings.{field}"),
                    "Use 1 to 255 characters.".into(),
                );
            }
        }
        if self.max_team_size <= 0 || self.max_team_size > self.max_participants {
            errors.insert(
                "settings.max_team_size".into(),
                "Use a positive size that fits within the participant limit.".into(),
            );
        }
        if self.application_deadline > self.team_formation_deadline
            || self.team_formation_deadline > self.submission_deadline
        {
            errors.insert(
                "settings.application_deadline".into(),
                "Set the application, team formation, and submission deadlines in order.".into(),
            );
        }
        if !self.application_schema.is_object() {
            errors.insert(
                "settings.application_schema".into(),
                "Send a JSON object.".into(),
            );
        }
        validate_file_fields(&self.file_fields, &mut errors);
        if !errors.is_empty() {
            return Err(ApiError::Validation(errors));
        }
        Ok(())
    }

    fn apply(self, model: &mut hackathon::ActiveModel) -> Result<(), ApiError> {
        model.name = Set(self.name);
        model.description = Set(self.description);
        model.location = Set(self.location);
        model.contact_email = Set(self.contact_email);
        model.max_team_size = Set(self.max_team_size);
        model.max_participants = Set(self.max_participants);
        model.application_deadline = Set(self.application_deadline);
        model.team_formation_deadline = Set(self.team_formation_deadline);
        model.submission_deadline = Set(self.submission_deadline);
        model.application_schema = Set(self.application_schema);
        model.file_fields = Set(serde_json::to_value(self.file_fields)
            .context("serialize file fields")
            .map_err(ApiError::Internal)?);
        Ok(())
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct CreateHackathon {
    slug: String,
    settings: Settings,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct UpdateHackathon {
    version: i32,
    settings: Option<Settings>,
    is_active: Option<bool>,
}

#[derive(Serialize, ToSchema)]
struct Hackathon {
    slug: String,
    version: i32,
    is_active: bool,
    settings: Settings,
}

impl TryFrom<hackathon::Model> for Hackathon {
    type Error = ApiError;

    fn try_from(model: hackathon::Model) -> Result<Self, ApiError> {
        Ok(Self {
            slug: model.slug,
            version: model.version,
            is_active: model.is_active,
            settings: Settings {
                name: model.name,
                description: model.description,
                location: model.location,
                contact_email: model.contact_email,
                max_team_size: model.max_team_size,
                max_participants: model.max_participants,
                application_deadline: model.application_deadline,
                team_formation_deadline: model.team_formation_deadline,
                submission_deadline: model.submission_deadline,
                application_schema: model.application_schema,
                file_fields: parse_file_fields(model.file_fields)?,
            },
        })
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/hackathons", post(create))
        .route("/hackathons/{slug}", patch(update))
}

pub fn public_routes() -> Router<AppState> {
    Router::new().route("/hackathons/{slug}", get(read))
}

#[utoipa::path(post, path = "/api/hackathons", request_body = CreateHackathon,
    responses((status = 201, body = Hackathon), (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse), (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse), (status = 422, body = ErrorResponse),
        (status = 500, body = ErrorResponse)))]
async fn create(
    auth: Auth<InstanceAdmin>,
    audit: Audit,
    input: Result<Json<CreateHackathon>, JsonRejection>,
) -> Result<(StatusCode, Json<Hackathon>), ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest("Invalid JSON request.".into()))?;
    if input.slug.len() > 80
        || input.slug.split('-').any(|segment| {
            segment.is_empty()
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
    {
        return Err(ApiError::Validation(BTreeMap::from([(
            "slug".into(),
            "Use up to 80 lowercase letters, numbers, and single hyphens.".into(),
        )])));
    }
    input.settings.validate()?;
    let id = Uuid::new_v4();
    let user_id = auth.data;
    let saved = audited!(audit, Scope::NewHackathon(id), |db| {
        let mut model = hackathon::ActiveModel {
            id: Set(id),
            slug: Set(input.slug),
            is_active: Set(true),
            data: Set(json!({})),
            ..Default::default()
        };
        input.settings.apply(&mut model)?;
        let saved = model
            .insert(db)
            .await
            .map_err(|error| match error.sql_err() {
                Some(SqlErr::UniqueConstraintViolation(_)) => {
                    ApiError::Conflict("This slug is already in use.".into())
                }
                _ => error.into(),
            })?;
        user_hackathon_role::ActiveModel {
            user_id: Set(user_id),
            hackathon_id: Set(id),
            role: Set(HackathonRole::Organizer),
            is_active: Set(true),
            ..Default::default()
        }
        .insert(db)
        .await?;
        organizer::ActiveModel {
            user_id: Set(user_id),
            hackathon_id: Set(id),
            capabilities: Set(vec![Capability::HackathonAdmin]),
            is_active: Set(true),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(saved)
    })
    .await?;
    Ok((StatusCode::CREATED, Json(saved.try_into()?)))
}

#[utoipa::path(get, path = "/api/hackathons/{slug}", params(("slug" = String, Path)),
    responses((status = 200, body = Hackathon), (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)))]
async fn read(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Hackathon>, ApiError> {
    Ok(Json(active_hackathon(&state.db, &slug).await?.try_into()?))
}

#[utoipa::path(patch, path = "/api/hackathons/{slug}", params(("slug" = String, Path)),
    request_body = UpdateHackathon,
    responses((status = 200, body = Hackathon), (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse), (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse), (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse), (status = 500, body = ErrorResponse)))]
async fn update(
    auth: Auth<ManageOrganizers>,
    user: CurrentUser,
    audit: Audit,
    input: Result<Json<UpdateHackathon>, JsonRejection>,
) -> Result<Json<Hackathon>, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest("Invalid JSON request.".into()))?;
    if input.is_active == Some(true) || (input.is_active.is_none() && input.settings.is_none()) {
        return Err(ApiError::BadRequest(
            "Send settings or set is_active to false.".into(),
        ));
    }
    if let Some(settings) = &input.settings {
        settings.validate()?;
    }
    let id = auth.data.id;
    let user_id = user.user().id;
    let saved = audited!(audit, Scope::HackathonExclusive(id), |db| {
        ensure_manager(db, user_id, id).await?;
        let current = hackathon::Entity::find_by_id(id)
            .filter(hackathon::Column::IsActive.eq(true))
            .one(db)
            .await?
            .ok_or(ApiError::NotFound)?;
        if current.version != input.version {
            return Err(ApiError::Conflict(
                "The settings changed. Reload before saving.".into(),
            ));
        }
        let mut model: hackathon::ActiveModel = current.into();
        if let Some(settings) = input.settings {
            let oversized_team = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT 1 FROM team_member m JOIN team t ON t.id=m.team_id
                 WHERE t.hackathon_id=$1 AND m.status='active'
                 GROUP BY t.id HAVING count(*) > $2 LIMIT 1",
                    [id.into(), settings.max_team_size.into()],
                ))
                .await?;
            if oversized_team.is_some() {
                return Err(ApiError::Conflict(
                    "Resolve oversized teams before reducing team size.".into(),
                ));
            }
            settings.apply(&mut model)?;
        }
        if input.is_active == Some(false) {
            model.is_active = Set(false);
        }
        Ok(model.update(db).await?)
    })
    .await?;
    Ok(Json(saved.try_into()?))
}
