pub mod handlers;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct HackathonInfo {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub is_active: bool,
    pub max_team_size: i32,
    pub banner_url: Option<String>,
    pub updated_at: NaiveDateTime,
    pub form_config: Option<serde_json::Value>,
}
