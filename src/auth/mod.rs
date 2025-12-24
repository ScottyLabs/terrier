#[cfg(feature = "server")]
pub mod extractors;
pub mod hooks;
#[cfg(feature = "server")]
pub mod middleware;

use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct HackathonRole {
    pub user_id: i32,
    pub hackathon_id: i32,
    pub role: String,
    pub slug: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub enum HackathonRoleType {
    Admin,
    Organizer,
    Judge,
    Sponsor,
    Participant,
    Applicant,
}

impl HackathonRoleType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Self::Admin),
            "organizer" => Some(Self::Organizer),
            "judge" => Some(Self::Judge),
            "sponsor" => Some(Self::Sponsor),
            "participant" => Some(Self::Participant),
            "applicant" => Some(Self::Applicant),
            _ => None,
        }
    }
}

impl HackathonRole {
    pub fn role_type(&self) -> Option<HackathonRoleType> {
        HackathonRoleType::from_str(&self.role)
    }
}

pub fn has_access(role: &HackathonRole, allowed: &[HackathonRoleType]) -> bool {
    if let Some(rt) = role.role_type() {
        allowed.contains(&rt)
    } else {
        false
    }
}

/// Check if a role string matches any of the allowed role types
pub fn has_role(role: &str, allowed: &[HackathonRoleType]) -> bool {
    if let Some(rt) = HackathonRoleType::from_str(role) {
        allowed.contains(&rt)
    } else {
        false
    }
}

// Centralized role definitions for hackathon pages
pub const DASHBOARD_ROLES: &[HackathonRoleType] = &[
    HackathonRoleType::Participant,
    HackathonRoleType::Judge,
    HackathonRoleType::Sponsor,
    HackathonRoleType::Organizer,
    HackathonRoleType::Admin,
];

pub const APPLICANTS_ROLES: &[HackathonRoleType] = &[HackathonRoleType::Admin];

pub const PEOPLE_ROLES: &[HackathonRoleType] =
    &[HackathonRoleType::Admin, HackathonRoleType::Organizer];

pub const TEAM_ROLES: &[HackathonRoleType] = &[
    HackathonRoleType::Participant,
    HackathonRoleType::Applicant,
    HackathonRoleType::Admin,
];

pub const SCHEDULE_ROLES: &[HackathonRoleType] = &[
    HackathonRoleType::Participant,
    HackathonRoleType::Judge,
    HackathonRoleType::Sponsor,
    HackathonRoleType::Organizer,
    HackathonRoleType::Admin,
];

pub const SUBMISSION_ROLES: &[HackathonRoleType] = &[HackathonRoleType::Participant];

pub const CHECKIN_ROLES: &[HackathonRoleType] = &[
    HackathonRoleType::Participant,
    HackathonRoleType::Sponsor,
    HackathonRoleType::Organizer,
    HackathonRoleType::Admin,
];

pub const SETTINGS_ROLES: &[HackathonRoleType] = &[HackathonRoleType::Admin];

pub const APPLY_ROLES: &[HackathonRoleType] = &[
    HackathonRoleType::Applicant,
    HackathonRoleType::Participant,
    HackathonRoleType::Organizer,
    HackathonRoleType::Admin,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct LoginQuery {
    pub redirect_uri: Option<String>,
}
