use std::any;
use crate::auth;

use dioxus::fullstack::{Form, SetCookie, SetHeader};

#[cfg(feature = "server")]
use crate::{
    entities::{prelude::*, users},
};
#[cfg(feature = "server")]
use axum_oidc::{EmptyAdditionalClaims, OidcClaims, OidcRpInitiatedLogout};


#[cfg(feature = "server")]
use chrono::NaiveDateTime;
#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use dioxus::fullstack::extract::State;
use dioxus::prelude::*;

use serde::Serialize;

#[cfg(feature = "server")]
use crate::{
    AppState,
    auth::extractors::{HackathonRole, RequireGlobalAdmin},
    entities::{hackathons, prelude::*},
};

use crate::dioxus_fullstack::ServerFnError;
use crate::types::HackathonInfo;

// Hackathon handlers

#[get("/hackathons/public", state: State<AppState>)]
pub async fn list_public_hackathons() -> Result<Vec<HackathonInfo>> {
    let hackathons = Hackathons::find()
        .all(&state.db)
        .await
        .map_err(|_| ServerFnError::MissingArg(("Error".to_string())))?;

    Ok(hackathons
        .into_iter()
        .map(|h| HackathonInfo {
            id: h.id,
            name: h.name,
            slug: h.slug,
            description: h.description,
            start_date: h.start_date,
            end_date: h.end_date,
            is_active: h.is_active,
        })
        .collect())
}

#[derive(Deserialize, Serialize)]
pub struct CreateHackathonForm {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub start_time: String,
    pub end_time: String,
}

impl CreateHackathonForm {
    pub fn from_vectors(keys: Vec<String>, values: Vec<String>) -> Result<Self, ServerFnError> {
        let mut map = std::collections::HashMap::new();
        for (k, v) in keys.into_iter().zip(values.into_iter()) {
            map.insert(k, v);
        }

        let name = map
            .remove("name")
            .ok_or_else(|| ServerFnError::new("Missing name"))?;
        let slug = map
            .remove("slug")
            .ok_or_else(|| ServerFnError::new("Missing slug"))?;
        let description = map.remove("description");
        let start_date = map
            .remove("start_date")
            .ok_or_else(|| ServerFnError::new("Missing start_date"))?;
        let end_date = map
            .remove("end_date")
            .ok_or_else(|| ServerFnError::new("Missing end_date"))?;
        let start_time = map
            .remove("start_time")
            .ok_or_else(|| ServerFnError::new("Missing start_time"))?;
        let end_time = map
            .remove("end_time")
            .ok_or_else(|| ServerFnError::new("Missing end_time"))?;

        Ok(CreateHackathonForm {
            name,
            slug,
            description,
            start_date,
            end_date,
            start_time,
            end_time
        })
    }
}

#[post("/hackathons", state: State<AppState>, requireGlobalAdmin: RequireGlobalAdmin)]
pub async fn create_hackathon(form: Form<CreateHackathonForm>) -> Result<HackathonInfo> {
    // Check if slug already exists
    let existing = Hackathons::find()
        .filter(hackathons::Column::Slug.eq(&form.0.slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new("Slug exists"))?;

    if existing.is_some() {
return Err(ServerFnError::new("Error: Hackathon slug already exists").into());    }

    // Create hackathon
    let hackathon = hackathons::ActiveModel {
        name: Set(form.0.name),
        slug: Set(form.0.slug),
        description: Set(form.0.description),
        start_date: Set(
            NaiveDateTime::parse_from_str(&format!("{}T{}", form.0.start_date, form.0.start_time), "%Y-%m-%dT%H:%M")
                // .or_else(|_| NaiveDateTime::parse_from_str(&form.0.start_date, "%Y-%m-%dT%H:%M:%S"))
                .map_err(|_| ServerFnError::new(("Can't parse1".to_string())))?,
        ),
        end_date: Set(
            NaiveDateTime::parse_from_str(&format!("{}T{}", form.0.end_date, form.0.end_time), "%Y-%m-%dT%H:%M")
                // .or_else(|_| NaiveDateTime::parse_from_str(&form.0.end_date, "%Y-%m-%dT%H:%M:%S"))
                .map_err(|_| ServerFnError::new(("Can't parse2".to_string())))?,
        ),
        is_active: Set(false),
        ..Default::default()
    };

    let result = hackathon
        .insert(&state.db)
        .await
        .map_err(|e| ServerFnError::new("e2"))?;
    Ok(HackathonInfo {
        id: result.id,
        name: result.name,
        slug: result.slug,
        description: result.description,
        start_date: result.start_date,
        end_date: result.end_date,
        is_active: result.is_active,
    })
}

/// Get current user information
#[get("/auth/status", state: State<AppState>, claims: Option<OidcClaims<EmptyAdditionalClaims>>)]
pub async fn user_status() -> Result<auth::UserInfo, ServerFnError> {
    match claims {
        Some(claims) => {
            let email = claims
                .email()
                .map(|s| s.to_string())
                .ok_or_else(|| ServerFnError::new("No email in claims"))?;

            let is_admin = state.config.admin_emails.contains(&email.to_lowercase());

            let oidc_sub = claims.subject().to_string();
            let user = Users::find()
                .filter(users::Column::OidcSub.eq(&oidc_sub))
                .one(&state.db)
                .await
                .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
                .ok_or_else(|| ServerFnError::new("User not found"))?;

            Ok(auth::UserInfo {
                id: user.id.to_string(),
                email: user.email,
                name: user.name,
                picture: user.picture,
                is_admin,
            })
        }
        None => Err(ServerFnError::new("Not authenticated")),
    }
}
