use std::collections::BTreeMap;

use axum::{Json, Router, extract::rejection::JsonRejection, routing::get};
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

use crate::{
    audit::{Audit, Scope},
    audited,
    auth::policy::{Auth, OwnProfile},
    entities::user,
    error::ApiError,
    state::AppState,
};

const MAX_DISPLAY_NAME: usize = 100;

#[derive(OpenApi)]
#[openapi(paths(profile, update_profile))]
pub struct ProfileApiDoc;

#[derive(Serialize, ToSchema)]
struct Profile {
    id: String,
    email: String,
    display_name: String,
}

impl From<&user::Model> for Profile {
    fn from(user: &user::Model) -> Self {
        Self {
            id: user.id.to_string(),
            email: user.email.clone(),
            display_name: user.display_name.clone(),
        }
    }
}

/// Omitted fields are left unchanged. Email comes from the identity provider.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct UpdateProfile {
    display_name: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/users/{user_id}", get(profile).patch(update_profile))
}

#[utoipa::path(get, path = "/api/users/{user_id}", tag = "users",
    params(("user_id" = String, Path, description = "Terrier user UUID")),
    responses((status = 200, body = Profile),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn profile(auth: Auth<OwnProfile>) -> Json<Profile> {
    Json(auth.data.user().into())
}

#[utoipa::path(patch, path = "/api/users/{user_id}", tag = "users",
    params(("user_id" = String, Path, description = "Terrier user UUID")),
    request_body = UpdateProfile,
    responses((status = 200, body = Profile),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 422, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn update_profile(
    auth: Auth<OwnProfile>,
    audit: Audit,
    input: Result<Json<UpdateProfile>, JsonRejection>,
) -> Result<Json<Profile>, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest("Invalid JSON request.".into()))?;
    let current = auth.data.user();
    let Some(name) = input.display_name else {
        return Ok(Json(current.into()));
    };
    let name = name.trim().to_owned();
    if name.is_empty() || name.chars().count() > MAX_DISPLAY_NAME {
        return Err(ApiError::Validation(BTreeMap::from([(
            "display_name".into(),
            format!("Use 1 to {MAX_DISPLAY_NAME} characters."),
        )])));
    }
    if name == current.display_name {
        return Ok(Json(current.into()));
    }
    let changes = user::ActiveModel {
        id: Set(current.id),
        display_name: Set(name),
        ..Default::default()
    };
    let saved = audited!(audit, Scope::Global, |db| { Ok(changes.update(db).await?) }).await?;
    Ok(Json((&saved).into()))
}
