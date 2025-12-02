use axum::{
    RequestPartsExt,
    extract::{FromRequestParts, Path},
    http::{StatusCode, request::Parts},
};
use axum_oidc::{EmptyAdditionalClaims, OidcClaims};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, JoinType, QueryFilter,
    QuerySelect, RelationTrait,
};
use std::collections::HashMap;

use crate::{
    AppState,
    entities::{hackathons, prelude::*, user_hackathon_roles, users},
};

pub struct RequireGlobalAdmin {
    pub email: String,
}

impl FromRequestParts<AppState> for RequireGlobalAdmin {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = OidcClaims::<EmptyAdditionalClaims>::from_request_parts(parts, &state)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let email = claims
            .0
            .email()
            .map(|e| e.to_string())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if state.config.admin_emails.contains(&email.to_lowercase()) {
            Ok(RequireGlobalAdmin { email })
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}

pub struct HackathonRole {
    pub user_id: i32,
    pub hackathon_id: i32,
    pub role: String,
    pub slug: String,
}

impl HackathonRole {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    pub fn is_organizer(&self) -> bool {
        // Everywhere accessible to organizers should also be accessible to admins
        matches!(self.role.as_str(), "admin" | "organizer")
    }

    pub fn is_judge(&self) -> bool {
        matches!(self.role.as_str(), "admin" | "organizer" | "judge")
    }

    pub fn is_sponsor(&self) -> bool {
        matches!(self.role.as_str(), "admin" | "organizer" | "sponsor")
    }

    pub fn is_participant(&self) -> bool {
        matches!(self.role.as_str(), "admin" | "participant")
    }

    pub fn is_applicant(&self) -> bool {
        matches!(self.role.as_str(), "admin" | "applicant")
    }
}

impl FromRequestParts<AppState> for HackathonRole {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract hackathon slug from path - use HashMap to handle routes with multiple params
        let Path(params) = parts
            .extract::<Path<HashMap<String, String>>>()
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let slug = params.get("slug").cloned().ok_or(StatusCode::BAD_REQUEST)?;

        let claims = OidcClaims::<EmptyAdditionalClaims>::from_request_parts(parts, state)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let email = claims
            .0
            .email()
            .map(|e| e.to_string())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // Global admins have admin role in all hackathons
        if state.config.admin_emails.contains(&email.to_lowercase()) {
            let hackathon = Hackathons::find()
                .filter(hackathons::Column::Slug.eq(&slug))
                .one(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::NOT_FOUND)?;

            let user = Users::find()
                .filter(users::Column::OidcSub.eq(claims.0.subject().to_string()))
                .one(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::UNAUTHORIZED)?;

            return Ok(HackathonRole {
                user_id: user.id,
                hackathon_id: hackathon.id,
                role: "admin".to_string(),
                slug,
            });
        }

        // Look up role in database
        let result = UserHackathonRoles::find()
            .join(
                JoinType::InnerJoin,
                user_hackathon_roles::Relation::Users.def(),
            )
            .join(
                JoinType::InnerJoin,
                user_hackathon_roles::Relation::Hackathons.def(),
            )
            .filter(users::Column::OidcSub.eq(claims.0.subject().to_string()))
            .filter(hackathons::Column::Slug.eq(&slug))
            .find_also_related(Hackathons)
            .one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::FORBIDDEN)?;

        Ok(HackathonRole {
            user_id: result.0.user_id,
            hackathon_id: result.0.hackathon_id,
            role: result.0.role,
            slug,
        })
    }
}

/// An extractor that auto-creates an "applicant" role for authenticated users
/// who don't yet have a role in the hackathon. Use this for application endpoints.
pub struct ApplicantRole {
    pub user_id: i32,
    pub hackathon_id: i32,
    pub role: String,
    pub slug: String,
}

impl ApplicantRole {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    pub fn is_organizer(&self) -> bool {
        matches!(self.role.as_str(), "admin" | "organizer")
    }
}

impl FromRequestParts<AppState> for ApplicantRole {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        tracing::info!("ApplicantRole extractor called for path: {:?}", parts.uri.path());
        
        // Extract hackathon slug from path - use HashMap to handle routes with multiple params
        let Path(params) = parts
            .extract::<Path<HashMap<String, String>>>()
            .await
            .map_err(|e| {
                tracing::error!("Failed to extract path params: {:?}", e);
                StatusCode::BAD_REQUEST
            })?;

        tracing::info!("Extracted params: {:?}", params);

        let slug = params.get("slug").cloned().ok_or_else(|| {
            tracing::error!("No 'slug' in params: {:?}", params);
            StatusCode::BAD_REQUEST
        })?;

        tracing::info!("Got slug: {}", slug);

        let claims = OidcClaims::<EmptyAdditionalClaims>::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get OIDC claims: {:?}", e);
                StatusCode::UNAUTHORIZED
            })?;

        let email = claims
            .0
            .email()
            .map(|e| e.to_string())
            .ok_or_else(|| {
                tracing::error!("No email in claims");
                StatusCode::UNAUTHORIZED
            })?;

        tracing::info!("Got email: {}", email);

        // Get the hackathon
        let hackathon = Hackathons::find()
            .filter(hackathons::Column::Slug.eq(&slug))
            .one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("DB error finding hackathon: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or_else(|| {
                tracing::error!("Hackathon not found for slug: {}", slug);
                StatusCode::NOT_FOUND
            })?;

        tracing::info!("Found hackathon: {}", hackathon.id);

        // Get the user
        let user = Users::find()
            .filter(users::Column::OidcSub.eq(claims.0.subject().to_string()))
            .one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("DB error finding user: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or_else(|| {
                tracing::error!("User not found for sub: {:?}", claims.0.subject());
                StatusCode::UNAUTHORIZED
            })?;

        tracing::info!("Found user: {}", user.id);

        // Global admins have admin role in all hackathons
        if state.config.admin_emails.contains(&email.to_lowercase()) {
            return Ok(ApplicantRole {
                user_id: user.id,
                hackathon_id: hackathon.id,
                role: "admin".to_string(),
                slug,
            });
        }

        // Check if user already has a role
        let existing_role = UserHackathonRoles::find()
            .filter(user_hackathon_roles::Column::UserId.eq(user.id))
            .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon.id))
            .one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(role) = existing_role {
            return Ok(ApplicantRole {
                user_id: user.id,
                hackathon_id: hackathon.id,
                role: role.role,
                slug,
            });
        }

        // Auto-create applicant role for new users
        let new_role = user_hackathon_roles::ActiveModel {
            user_id: Set(user.id),
            hackathon_id: Set(hackathon.id),
            role: Set("applicant".to_string()),
            ..Default::default()
        };

        new_role
            .insert(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        tracing::info!(
            "Auto-created applicant role for user {} in hackathon {}",
            user.id,
            hackathon.id
        );

        Ok(ApplicantRole {
            user_id: user.id,
            hackathon_id: hackathon.id,
            role: "applicant".to_string(),
            slug,
        })
    }
}
