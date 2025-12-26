use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::core::auth::{context::RequestContext, middleware::SyncedUser};
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
    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get form config to find the file_path template
    let form_config: crate::domain::applications::types::FormSchema = hackathon
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

    // Check if field is a file type and extract file_path and validation
    let (file_path_template, file_validation) = match &field.field_type {
        crate::domain::applications::types::FieldType::File {
            file_path,
            validation,
        } => (file_path, validation),
        _ => {
            return Err(ServerFnError::new(format!(
                "Field '{}' is not a file upload field",
                field_name
            )));
        }
    };

    // Verify users can only upload for themselves, for this hackathon
    if !(file_path_template.contains("{user_id}") || file_path_template.contains("{user_oidc_sub}"))
    {
        return Err(ServerFnError::new("Missing user placeholder"));
    }

    if !(file_path_template.contains("{hackathon_id}")
        || file_path_template.contains("{hackathon_slug}"))
    {
        return Err(ServerFnError::new("Missing hackathon placeholder"));
    }

    // Extract file extension from filename
    let file_extension = PathBuf::from(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_string());

    // Validate file size if specified
    if let Some(validation) = file_validation {
        if let Some(max_size) = validation.max_size {
            if file_data.len() > max_size as usize {
                return Err(ServerFnError::new(format!(
                    "File size {} exceeds maximum {}",
                    file_data.len(),
                    max_size
                )));
            }
        }
    }

    // Build file path by substituting template variables
    let mut file_path = file_path_template.clone();
    file_path = file_path.replace("{hackathon_id}", &hackathon.id.to_string());
    file_path = file_path.replace("{hackathon_slug}", &hackathon.slug);
    file_path = file_path.replace("{user_id}", &ctx.user.id.to_string());
    file_path = file_path.replace("{user_oidc_sub}", &ctx.user.oidc_sub.to_string());
    file_path = file_path.replace("{field_name}", &field_name);

    // Add file extension if present
    if let Some(ext) = file_extension {
        file_path = format!("{}.{}", file_path, ext);
    }

    // Upload to MinIO
    let file_size = file_data.len();
    let mut cursor = Cursor::new(file_data);
    let mut put_args = minio::s3::args::PutObjectArgs::new(
        &ctx.state.config.minio_bucket,
        &file_path,
        &mut cursor,
        Some(file_size),
        None,
    )
    .map_err(|e| ServerFnError::new(format!("Invalid upload arguments: {}", e)))?;

    ctx.state
        .s3
        .put_object(&mut put_args)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to upload file: {}", e)))?;

    // Generate public URL
    let file_url = format!(
        "{}/{}/{}",
        ctx.state.config.minio_public_endpoint, ctx.state.config.minio_bucket, file_path
    );

    Ok(FileUploadResponse {
        url: file_url,
        field_name,
    })
}

/// Delete a file from an application form field
#[cfg_attr(feature = "server", utoipa::path(
    delete,
    path = "/api/hackathons/{slug}/application/upload/{field_name}",
    params(
        ("slug" = String, Path, description = "Hackathon slug"),
        ("field_name" = String, Path, description = "Form field name")
    ),
    responses(
        (status = 200, description = "File deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Hackathon or file not found"),
        (status = 500, description = "Server error")
    ),
    tag = "applications"
))]
#[delete("/api/hackathons/:slug/application/upload/:field_name", user: SyncedUser)]
pub async fn delete_application_file(
    slug: String,
    field_name: String,
) -> Result<(), ServerFnError> {
    let ctx = RequestContext::extract(&user)
        .await?
        .with_hackathon(&slug)
        .await?;

    let hackathon = ctx.hackathon()?;

    // Get form config to find the file_path template
    let form_config: crate::domain::applications::types::FormSchema = hackathon
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

    // Check if field is a file type and extract file_path
    let file_path_template = match &field.field_type {
        crate::domain::applications::types::FieldType::File { file_path, .. } => file_path,
        _ => {
            return Err(ServerFnError::new(format!(
                "Field '{}' is not a file upload field",
                field_name
            )));
        }
    };

    // Build file path by substituting template variables (without extension for pattern matching)
    let mut file_path = file_path_template.clone();
    file_path = file_path.replace("{hackathon_id}", &hackathon.id.to_string());
    file_path = file_path.replace("{hackathon_slug}", &hackathon.slug);
    file_path = file_path.replace("{user_id}", &ctx.user.id.to_string());
    file_path = file_path.replace("{user_oidc_sub}", &ctx.user.oidc_sub.to_string());
    file_path = file_path.replace("{field_name}", &field_name);

    // List objects matching the pattern (to handle different extensions)
    let mut list_args = minio::s3::args::ListObjectsV2Args::new(&ctx.state.config.minio_bucket)
        .map_err(|e| ServerFnError::new(format!("Invalid list arguments: {}", e)))?;
    list_args.prefix = Some(&file_path);

    let objects = ctx
        .state
        .s3
        .list_objects_v2(&list_args)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to list files: {}", e)))?;

    // Delete all matching files
    for item in objects.contents {
        let remove_args =
            minio::s3::args::RemoveObjectArgs::new(&ctx.state.config.minio_bucket, &item.name)
                .map_err(|e| ServerFnError::new(format!("Invalid remove arguments: {}", e)))?;

        ctx.state
            .s3
            .remove_object(&remove_args)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete file: {}", e)))?;
    }

    Ok(())
}
