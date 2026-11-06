use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use slac::Policy;
use uuid::Uuid;

use super::current_user::CurrentUser;
use crate::{error::ApiError, state::AppState};

pub type Auth<P> = slac::Authorized<P, AppState>;

pub struct OwnProfile;

impl Policy<AppState> for OwnProfile {
    type Output = CurrentUser;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path(id) = Path::<Uuid>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid user ID.".into()))?;
        if user.user().id != id {
            return Err(ApiError::Forbidden);
        }
        Ok(user)
    }
}
