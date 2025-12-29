use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use utoipa::ToSchema;

/// Public hackathon information that can be shared with clients
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
    pub background_url: Option<String>,
    pub updated_at: NaiveDateTime,
    pub form_config: Option<serde_json::Value>,
}

/// Event schedule item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct ScheduleEvent {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    /// NULL = visible to everyone, otherwise the role required to view
    pub visible_to_role: Option<String>,
    /// Event type for color coding: default, hacking, speaker, sponsor, food
    pub event_type: String,
    /// Whether the event is visible to participants (draft/published)
    pub is_visible: bool,
    /// User IDs of organizers assigned to this event
    pub organizer_ids: Vec<i32>,
}
