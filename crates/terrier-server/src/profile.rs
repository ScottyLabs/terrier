use axum::{Json, Router, routing::get};
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

use crate::{
    auth::policy::{Auth, OwnProfile},
    state::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(profile))]
pub struct ProfileApiDoc;

#[derive(Serialize, ToSchema)]
struct Profile {
    id: String,
    email: String,
    display_name: String,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/users/{user_id}", get(profile))
}

#[utoipa::path(get, path = "/api/users/{user_id}", tag = "users",
    params(("user_id" = String, Path, description = "Terrier user UUID")),
    responses((status = 200, body = Profile),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
async fn profile(auth: Auth<OwnProfile>) -> Json<Profile> {
    let user = auth.data.user();
    Json(Profile {
        id: user.id.to_string(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
    })
}
