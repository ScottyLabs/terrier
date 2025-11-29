use crate::auth;
use std::any;

use dioxus::fullstack::{Form, SetCookie, SetHeader};

#[cfg(feature = "server")]
use crate::entities::{prelude::*, users};
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
            end_time,
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
        return Err(ServerFnError::new("Error: Hackathon slug already exists").into());
    }

    // Create hackathon
    let hackathon = hackathons::ActiveModel {
        name: Set(form.0.name),
        slug: Set(form.0.slug),
        description: Set(form.0.description),
        start_date: Set(NaiveDateTime::parse_from_str(
            &format!("{}T{}", form.0.start_date, form.0.start_time),
            "%Y-%m-%dT%H:%M",
        )
        // .or_else(|_| NaiveDateTime::parse_from_str(&form.0.start_date, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| ServerFnError::new(("Can't parse1".to_string())))?),
        end_date: Set(NaiveDateTime::parse_from_str(
            &format!("{}T{}", form.0.end_date, form.0.end_time),
            "%Y-%m-%dT%H:%M",
        )
        // .or_else(|_| NaiveDateTime::parse_from_str(&form.0.end_date, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| ServerFnError::new(("Can't parse2".to_string())))?),
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

// hackathon
#[get("/hackathons/:slug/form", state: State<AppState>)]
pub async fn get_hackathon_form(slug: String) -> Result<serde_json::Value, ServerFnError> {
    let form = serde_json::json!({
        "personal": [
            {"id": "display_name", "type": "single-line-text", "question": "Display Name", "description": null, "maxLength": 100, "required": true},
            {"id": "first_name", "type": "single-line-text", "question": "First Name", "description": null, "maxLength": 100, "required": true},
            {"id": "middle_names", "type": "single-line-text", "question": "Middle Names", "description": null, "maxLength": 100, "required": false},
            {"id": "last_name", "type": "single-line-text", "question": "Last Name", "description": null, "maxLength": 100, "required": true},
            {"id": "gender", "type": "dropdown", "question": "Gender", "description": null, "options": ["Man", "Woman", "Non-binary", "Prefer not to say", "Other (please specify)"], "required": true},
            {"id": "gender_other", "type": "single-line-text", "question": "Please specify your gender", "description": null, "maxLength": 100, "required": true, "condition": {"id": "gender", "value": "Other (please specify)"}},
            {"id": "ethnicity", "type": "multi-checkbox", "question": "Ethnicity", "description": "Select all that apply.", "options": ["Native American", "Asian", "Black", "Pacific Islander", "White", "Hispanic", "Other", "Prefer not to say"], "required": true},
            {"id": "ethnicity_other", "type": "single-line-text", "question": "Please specify your ethnicity", "description": null, "maxLength": 100, "required": true, "condition": {"id": "ethnicity", "value": "Other"}},
            {"id": "age", "type": "single-line-text", "question": "Age", "description": null, "maxLength": 3, "required": true},
            {"id": "city", "type": "single-line-text", "question": "City", "description": null, "maxLength": 100, "required": false},
            {"id": "country", "type": "dropdown", "question": "Country", "description": null, "options": ["United States", "Canada", "Mexico", "United Kingdom", "China", "India", "Other"], "required": true}
        ],
        "school": [
            {"id": "school_select", "type": "dropdown", "question": "School", "description": null, "options": ["Carnegie Mellon University", "University of Pittsburgh", "Penn State", "Other"], "required": true},
            {"id": "school_manual", "type": "single-line-text", "question": "School Name", "description": "Please enter the name of your school.", "maxLength": 200, "required": true, "condition": {"id": "school_select", "value": "Other"}},
            {"id": "cmu_college", "type": "dropdown", "question": "College (CMU)", "description": null, "options": ["SCS", "CIT", "CFA", "Dietrich", "MCS", "Tepper", "Heinz"], "required": true, "condition": {"id": "school_select", "value": "Carnegie Mellon University"}},
            {"id": "academic_program", "type": "dropdown", "question": "Academic Program", "description": null, "options": ["Undergraduate", "Masters", "Doctorate", "Other"], "required": true},
            {"id": "graduation_year", "type": "dropdown", "question": "Graduation Year", "description": null, "options": ["2026", "2027", "2028", "2029", "2030"], "required": true},
            {"id": "major", "type": "single-line-text", "question": "Major", "description": null, "maxLength": 100, "required": true}
        ],
        "experience": [
            {"id": "hackathon_experience", "type": "dropdown", "question": "Years of Hackathon Experience", "description": null, "options": ["0", "1-3", "4+"], "required": true}
        ],
        "sponsor": [
            {"id": "work_auth", "type": "dropdown", "question": "US Work Authorization", "description": null, "options": ["I am a US citizen", "I will need employer sponsorship at some point in the future", "I will NOT need employer sponsorship at some point in the future"], "required": false},
            {"id": "work_location", "type": "single-line-text", "question": "Work Location Preferences", "description": null, "maxLength": 200, "required": false}
        ],
        "portfolio": [
            {"id": "resume", "type": "single-line-text", "question": "Resume URL", "description": "Please provide a link to your resume (Google Drive, Dropbox, etc).", "maxLength": 500, "required": true},
            {"id": "github", "type": "single-line-text", "question": "Github Username", "description": null, "maxLength": 100, "required": true},
            {"id": "linkedin", "type": "single-line-text", "question": "LinkedIn Profile URL", "description": null, "maxLength": 200, "required": false},
            {"id": "website", "type": "single-line-text", "question": "Personal Website", "description": null, "maxLength": 200, "required": false},
            {"id": "design_portfolio", "type": "single-line-text", "question": "Design Portfolio", "description": null, "maxLength": 200, "required": false}
        ],
        "travel": [
            {"id": "travel_reimbursement", "type": "checkbox", "question": "Would you like to apply for travel reimbursement?", "description": null, "required": false},
            {"id": "travel_details", "type": "long-response", "question": "Travel Details", "description": "Please provide details regarding your travel plans.", "maxLength": 500, "required": false, "condition": {"id": "travel_reimbursement", "value": "true"}}
        ],
        "diversity": [
            {"id": "diversity_statement", "type": "long-response", "question": "Diversity Statement", "description": "Optional statement regarding diversity.", "maxLength": 1000, "required": false}
        ],
        "logistics": [
            {"id": "phone_number", "type": "single-line-text", "question": "Phone Number", "description": null, "maxLength": 20, "required": true},
            {"id": "dietary_restrictions", "type": "single-line-text", "question": "Dietary Considerations", "description": null, "maxLength": 200, "required": false},
            {"id": "shirt_size", "type": "dropdown", "question": "Shirt Size", "description": "Unisex sizes.", "options": ["XS", "S", "M", "L", "XL", "XXL"], "required": true},
            {"id": "hardware_needs", "type": "checkbox", "question": "Will you use hardware?", "description": null, "required": false},
            {"id": "additional_notes", "type": "long-response", "question": "Additional Notes", "description": null, "maxLength": 500, "required": false}
        ],
        "consent": [
            {"id": "coc_consent", "type": "checkbox", "question": "I agree to the THX Code of Conduct", "description": null, "required": true},
            {"id": "media_release", "type": "checkbox", "question": "Media Release Consent", "description": "I authorize the use of my image/media.", "required": true},
            {"id": "sponsor_consent", "type": "checkbox", "question": "I authorize sharing my info with sponsors", "description": null, "required": false},
            {"id": "signature", "type": "signature", "question": "Signature", "description": "By signing, you agree to the terms above.", "required": true}
        ]
    });

    Ok(form)
}
