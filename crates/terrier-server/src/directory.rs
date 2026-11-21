use axum::{Json, Router, extract::State, routing::get};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use serde_json::{Map, Value};
use utoipa::{OpenApi, ToSchema};

use crate::{
    auth::policy::{Auth, BrowseDirectory},
    entities::{sea_orm_active_enums::HackathonRole, user, user_hackathon_role},
    error::ApiError,
    state::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(directory))]
pub struct DirectoryApiDoc;

pub fn routes() -> Router<AppState> {
    Router::new().route("/hackathons/{slug}/users", get(directory))
}

#[derive(Serialize, ToSchema)]
struct DirectoryEntry {
    user_id: String,
    display_name: String,
    /// The participant's answers for the hackathon's published directory fields only.
    #[schema(value_type = Object)]
    profile: Value,
}

#[derive(Serialize, ToSchema)]
struct Directory {
    users: Vec<DirectoryEntry>,
}

fn published(profile: &Value, fields: &Value) -> Value {
    let mut out = Map::new();
    if let (Some(profile), Some(fields)) = (profile.as_object(), fields.as_array()) {
        for field in fields.iter().filter_map(Value::as_str) {
            if let Some(value) = profile.get(field) {
                out.insert(field.to_owned(), value.clone());
            }
        }
    }
    Value::Object(out)
}

#[utoipa::path(get, path = "/api/hackathons/{slug}/users", tag = "hackathons",
    params(("slug" = String, Path, description = "Hackathon slug")),
    responses((status = 200, body = Directory),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn directory(
    auth: Auth<BrowseDirectory>,
    State(state): State<AppState>,
) -> Result<Json<Directory>, ApiError> {
    let hackathon = auth.data;
    let users = user_hackathon_role::Entity::find()
        .filter(user_hackathon_role::Column::HackathonId.eq(hackathon.id))
        .filter(user_hackathon_role::Column::IsActive.eq(true))
        .filter(user_hackathon_role::Column::DirectoryOptIn.eq(true))
        .filter(user_hackathon_role::Column::Role.eq(HackathonRole::Hacker))
        .find_also_related(user::Entity)
        .order_by_asc(user_hackathon_role::Column::UserId)
        .all(&state.db)
        .await?
        .into_iter()
        .filter_map(|(role, user)| {
            let user = user?;
            Some(DirectoryEntry {
                user_id: user.id.to_string(),
                display_name: user.display_name,
                profile: published(&role.profile_data, &hackathon.directory_fields),
            })
        })
        .collect();
    Ok(Json(Directory { users }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::published;

    #[test]
    fn only_published_fields_are_returned() {
        let profile = json!({"school": "CMU", "phone": "555-0100", "major": "Math"});
        assert_eq!(
            published(&profile, &json!(["school", "major", "missing"])),
            json!({"school": "CMU", "major": "Math"})
        );
        assert_eq!(published(&profile, &json!({"school": true})), json!({}));
        assert_eq!(published(&json!([]), &json!(["school"])), json!({}));
    }
}
