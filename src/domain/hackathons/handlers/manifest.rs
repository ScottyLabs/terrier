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
        vec![ManifestIcon {
            src: icon_url.clone(),
            sizes: "512x512".to_string(),
            icon_type: guess_icon_type(icon_url),
        }]
    } else {
        // Fall back to default icons (do not exist yet)
        vec![]
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

/// Serve manifest.json at root, extracting hackathon slug from Referer header
pub async fn get_root_manifest(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Try to extract hackathon slug from Referer header
    let slug = headers
        .get("referer")
        .and_then(|r| r.to_str().ok())
        .and_then(|referer| {
            // Parse URL like "https://example.com/h/th26/dashboard"
            // Extract the slug after "/h/"
            if let Some(start) = referer.find("/h/") {
                let rest = &referer[start + 3..];
                // Take until next "/" or end
                let slug = rest.split('/').next()?;
                if !slug.is_empty() {
                    return Some(slug.to_string());
                }
            }
            None
        });

    let Some(slug) = slug else {
        // No hackathon context, return a default/generic manifest
        let manifest = WebManifest {
            name: "Terrier".to_string(),
            short_name: "Terrier".to_string(),
            description: "Hackathon management platform".to_string(),
            start_url: "/".to_string(),
            scope: "/".to_string(),
            display: "fullscreen".to_string(),
            background_color: "#F4F2F3".to_string(),
            theme_color: "#F4F2F3".to_string(),
            orientation: "portrait".to_string(),
            icons: vec![],
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/manifest+json"),
        );
        return (headers, Json(manifest)).into_response();
    };

    // Find the hackathon
    let hackathon = match hackathons::Entity::find()
        .filter(hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
    {
        Ok(Some(h)) => h,
        Ok(None) => {
            // Fallback to generic manifest
            let manifest = WebManifest {
                name: "Terrier".to_string(),
                short_name: "Terrier".to_string(),
                description: "Hackathon management platform".to_string(),
                start_url: "/".to_string(),
                scope: "/".to_string(),
                display: "fullscreen".to_string(),
                background_color: "#F4F2F3".to_string(),
                theme_color: "#F4F2F3".to_string(),
                orientation: "portrait".to_string(),
                icons: vec![],
            };

            let mut headers = HeaderMap::new();
            headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/manifest+json"),
            );
            return (headers, Json(manifest)).into_response();
        }
        Err(e) => {
            tracing::error!("Failed to fetch hackathon: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    let scope = format!("/h/{}/", slug);
    let start_url = format!("/h/{}/", slug);

    let theme_color = hackathon
        .theme_color
        .clone()
        .unwrap_or_else(|| "#F4F2F3".to_string());
    let background_color = hackathon
        .background_color
        .clone()
        .unwrap_or_else(|| "#F4F2F3".to_string());

    let icons = if let Some(ref icon_url) = hackathon.app_icon_url {
        vec![ManifestIcon {
            src: icon_url.clone(),
            sizes: "512x512".to_string(),
            icon_type: guess_icon_type(icon_url),
        }]
    } else {
        vec![]
    };

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

    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/manifest+json"),
    );

    (headers, Json(manifest)).into_response()
}
