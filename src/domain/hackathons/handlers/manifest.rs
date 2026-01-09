#![cfg(feature = "server")]

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;

use crate::{AppState, entities::hackathons};

#[derive(Serialize)]
pub struct ManifestIcon {
    src: String,
    sizes: String,
    #[serde(rename = "type")]
    icon_type: String,
}

#[derive(Serialize)]
pub struct WebManifest {
    name: String,
    short_name: String,
    description: String,
    start_url: String,
    scope: String,
    display: String,
    background_color: String,
    theme_color: String,
    orientation: String,
    icons: Vec<ManifestIcon>,
}

/// Serve a hackathon-specific web manifest
pub async fn get_manifest(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    // Find the hackathon
    let hackathon = match hackathons::Entity::find()
        .filter(hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
    {
        Ok(Some(h)) => h,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Hackathon not found").into_response();
        }
        Err(e) => {
            tracing::error!("Failed to fetch hackathon: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    let scope = format!("/h/{}/", slug);
    let start_url = format!("/h/{}/", slug);

    // Use hackathon-specific colors or defaults
    let theme_color = hackathon
        .theme_color
        .clone()
        .unwrap_or_else(|| "#F4F2F3".to_string());
    let background_color = hackathon
        .background_color
        .clone()
        .unwrap_or_else(|| "#F4F2F3".to_string());

    // Build icons - use custom app icon if available, otherwise fall back to defaults
    let icons = if let Some(ref icon_url) = hackathon.app_icon_url {
        // When a custom icon is uploaded, use it for all sizes
        // The browser will scale it appropriately
        vec![
            ManifestIcon {
                src: icon_url.clone(),
                sizes: "512x512".to_string(),
                icon_type: guess_icon_type(icon_url),
            },
            ManifestIcon {
                src: icon_url.clone(),
                sizes: "192x192".to_string(),
                icon_type: guess_icon_type(icon_url),
            },
            ManifestIcon {
                src: icon_url.clone(),
                sizes: "144x144".to_string(),
                icon_type: guess_icon_type(icon_url),
            },
            ManifestIcon {
                src: icon_url.clone(),
                sizes: "96x96".to_string(),
                icon_type: guess_icon_type(icon_url),
            },
        ]
    } else {
        // Fall back to default icons
        vec![
            ManifestIcon {
                src: "/th26_icons/android/android-launchericon-512-512.png".to_string(),
                sizes: "512x512".to_string(),
                icon_type: "image/png".to_string(),
            },
            ManifestIcon {
                src: "/th26_icons/android/android-launchericon-192-192.png".to_string(),
                sizes: "192x192".to_string(),
                icon_type: "image/png".to_string(),
            },
            ManifestIcon {
                src: "/th26_icons/android/android-launchericon-144-144.png".to_string(),
                sizes: "144x144".to_string(),
                icon_type: "image/png".to_string(),
            },
            ManifestIcon {
                src: "/th26_icons/android/android-launchericon-96-96.png".to_string(),
                sizes: "96x96".to_string(),
                icon_type: "image/png".to_string(),
            },
        ]
    };

    // Build manifest with hackathon-specific data
    let manifest = WebManifest {
        name: hackathon.name.clone(),
        short_name: hackathon.name.clone(),
        description: hackathon
            .description
            .unwrap_or_else(|| format!("The app for {}!", hackathon.name)),
        start_url,
        scope,
        display: "fullscreen".to_string(),
        background_color,
        theme_color,
        orientation: "portrait".to_string(),
        icons,
    };

    // Set proper content type for web manifest
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/manifest+json"),
    );

    (headers, Json(manifest)).into_response()
}

/// Guess the MIME type based on file extension
fn guess_icon_type(url: &str) -> String {
    if url.ends_with(".png") {
        "image/png".to_string()
    } else if url.ends_with(".jpg") || url.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if url.ends_with(".webp") {
        "image/webp".to_string()
    } else if url.ends_with(".gif") {
        "image/gif".to_string()
    } else {
        "image/png".to_string()
    }
}
