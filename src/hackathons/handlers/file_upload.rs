use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::{AppState, auth::middleware::SyncedUser};
#[cfg(feature = "server")]
use std::io::Cursor;
#[cfg(feature = "server")]
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct FileUploadResponse {
    pub url: String,
    pub field_name: String,
}

/// Upload a file for an application form field
#[cfg_attr(feature = "server", utoipa::path(
    post,
    path = "/api/hackathons/{slug}/application/upload/{field_name}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("field_name" = String, Path, description = "Form field name")
    ),
    responses(
        (status = 200, description = "File uploaded successfully", body = FileUploadResponse),
        (status = 400, description = "Invalid file or field"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[post("/api/hackathons/:slug/application/upload/:field_name", user: SyncedUser)]
pub async fn upload_application_file(
    slug: String,
    field_name: String,
    file_data: Vec<u8>,
    file_name: String,
) -> Result<FileUploadResponse, ServerFnError> {
    use dioxus::fullstack::{FullstackContext, extract::State};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // Extract state from context
    let State(state) = FullstackContext::extract::<State<AppState>, _>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract state: {}", e)))?;

    // Fetch hackathon by slug
    let hackathon = crate::entities::prelude::Hackathons::find()
        .filter(crate::entities::hackathons::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Hackathon not found"))?;

    // Get form config to find the file_path template
    let form_config: crate::schemas::FormSchema = hackathon
        .form_config
        .as_ref()
        .and_then(|config| serde_json::from_value(config.clone()).ok())
        .ok_or_else(|| ServerFnError::new("No form configuration found"))?;

    // Find the field with this name
    let field = form_config
        .fields
        .iter()
        .find(|f| f.name == field_name)
        .ok_or_else(|| ServerFnError::new(format!("Field '{}' not found in form", field_name)))?;

    // Check if field is a file type
    if !matches!(field.field_type, crate::schemas::application_form::FieldType::File) {
        return Err(ServerFnError::new(format!(
            "Field '{}' is not a file upload field",
            field_name
        )));
    }

    // Get file_path template
    let file_path_template = field
        .file_path
        .as_ref()
        .ok_or_else(|| ServerFnError::new(format!("Field '{}' has no file_path configured", field_name)))?;

    // SECURITY: Verify users can only upload for themselves
    if !file_path_template.contains("{user_id}") {
        return Err(ServerFnError::new(
            "File path template must include {user_id} for security",
        ));
    }

    // Extract file extension from filename
    let file_extension = PathBuf::from(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_string());

    // Validate file size if specified
    if let Some(validation) = &field.validation {
        if let crate::schemas::application_form::FieldValidation::File(file_val) = validation {
            if let Some(max_size) = file_val.max_size {
                if file_data.len() > max_size as usize {
                    return Err(ServerFnError::new(format!(
                        "File size {} exceeds maximum {}",
                        file_data.len(),
                        max_size
                    )));
                }
            }
        }
    }

    // Build file path by substituting template variables
    let mut file_path = file_path_template.clone();
    file_path = file_path.replace("{hackathon_id}", &hackathon.id.to_string());
    file_path = file_path.replace("{user_id}", &user.0.id.to_string());
    file_path = file_path.replace("{field_name}", &field_name);

    // Add file extension if present
    if let Some(ext) = file_extension {
        file_path = format!("{}.{}", file_path, ext);
    }

    // Upload to MinIO
    let file_size = file_data.len();
    let mut cursor = Cursor::new(file_data);
    let mut put_args = minio::s3::args::PutObjectArgs::new(
        &state.config.minio_bucket,
        &file_path,
        &mut cursor,
        Some(file_size),
        None,
    )
    .map_err(|e| ServerFnError::new(format!("Invalid upload arguments: {}", e)))?;

    state
        .s3
        .put_object(&mut put_args)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to upload file: {}", e)))?;

    // Generate public URL
    let file_url = format!(
        "{}/{}/{}",
        state.config.minio_endpoint, state.config.minio_bucket, file_path
    );

    Ok(FileUploadResponse {
        url: file_url,
        field_name,
    })
}
