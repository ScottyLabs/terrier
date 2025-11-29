use axum::{Json, extract::State, http::StatusCode};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use dioxus::prelude::*;

use serde::Serialize;

use crate::{
    AppState,
    auth::extractors::{HackathonRole, RequireGlobalAdmin},
    entities::{hackathons, prelude::*},
    types::HackathonInfo,
};

// #[get("/hackathons", requireGlobalAdmin: RequireGlobalAdmin)]
// pub async fn list_public_hackathons() -> Result<Vec<HackathonInfo>, ServerFnError> {
//     use dioxus::fullstack::extract;

//     // Extract the Extension from the request
//     let axum::Extension(state): axum::Extension<AppState> = extract().await?;

//     let hackathons = Hackathons::find()
//         .all(&state.db)
//         .await
// .map_err(|e| ServerFnError::new(e.to_string()))?;
//     Ok(
//         hackathons
//             .into_iter()
//             .map(|h| HackathonInfo {
//                 id: h.id,
//                 name: h.name,
//                 slug: h.slug,
//                 description: h.description,
//                 start_date: h.start_date,
//                 end_date: h.end_date,
//                 is_active: h.is_active,
//             })
//             .collect(),
//     )
// }


