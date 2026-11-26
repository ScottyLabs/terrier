use std::{cell::Cell, collections::BTreeMap, time::Duration};

use anyhow::Context;
use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{
        DefaultBodyLimit, Multipart, Query, State,
        multipart::{MultipartError, MultipartRejection},
        rejection::QueryRejection,
    },
    http::{
        StatusCode,
        header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use lopdf::{LoadOptions, Object, ObjectId};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, QuerySelect, prelude::DateTimeWithTimeZone,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Semaphore;
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use crate::{
    applications::{applications_in, ensure_editable, open_hackathon, statement},
    audit::{Audit, Scope},
    audited, audited_automatic,
    auth::policy::{Applicant, ApplicationAccess, Auth, OwnApplication, ReadApplicationFile},
    entities::{application, hackathon},
    error::{ApiError, ErrorResponse},
    state::AppState,
    storage::Storage,
};

/// Largest file any hackathon may accept.
pub const MAX_FILE_BYTES: u64 = 25 * 1024 * 1024;
const DEFAULT_MAX_BYTES: u64 = 5 * 1024 * 1024;
const ANY_TYPE: &str = "*/*";
const MAX_UNSUBMITTED_UPLOADS: usize = 10;
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(120);
static UPLOAD_SLOTS: Semaphore = Semaphore::const_new(8);
const ACTIVE_PDF_NAMES: [&[u8]; 9] = [
    b"JS",
    b"JavaScript",
    b"Launch",
    b"SubmitForm",
    b"ImportData",
    b"GoToE",
    b"XFA",
    b"RichMedia",
    b"RichMediaExecute",
];

const ACTIVE_PDF: &str = "Choose a PDF without scripts, forms or password protection.";

thread_local! {
    static ACTIVE_PDF_OBJECT: Cell<bool> = const { Cell::new(false) };
}

/// Upload rules for one application file field.
#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FileField {
    /// Accepted MIME types, checked against the file contents. `*/*` accepts any file.
    #[serde(default = "pdf_only")]
    pub types: Vec<String>,
    /// Largest accepted file size in bytes.
    #[serde(default = "default_max_bytes")]
    pub max_bytes: u64,
}

fn pdf_only() -> Vec<String> {
    vec!["application/pdf".into()]
}

fn default_max_bytes() -> u64 {
    DEFAULT_MAX_BYTES
}

/// Upload rules keyed by the application answer that holds each file ID.
pub type FileFields = BTreeMap<String, FileField>;

pub fn default_file_fields() -> FileFields {
    FileFields::from([(
        "resume".into(),
        FileField {
            types: pdf_only(),
            max_bytes: DEFAULT_MAX_BYTES,
        },
    )])
}

pub fn parse_file_fields(value: Value) -> Result<FileFields, ApiError> {
    serde_json::from_value(value)
        .context("stored file fields are invalid")
        .map_err(ApiError::Internal)
}

pub fn validate_file_fields(fields: &FileFields, errors: &mut BTreeMap<String, String>) {
    for (key, field) in fields {
        let path = format!("settings.file_fields.{key}");
        let valid_key = !key.is_empty()
            && key.len() <= 64
            && key
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if !valid_key {
            errors.insert(
                path,
                "Use up to 64 lowercase letters, numbers, and underscores.".into(),
            );
            continue;
        }
        if field.types.is_empty()
            || field
                .types
                .iter()
                .any(|kind| kind != ANY_TYPE && !infer::is_mime_supported(kind))
        {
            errors.insert(
                format!("{path}.types"),
                "List file types that can be recognized from their contents, or */* for any file."
                    .into(),
            );
        }
        if field.max_bytes == 0 || field.max_bytes > MAX_FILE_BYTES {
            errors.insert(
                format!("{path}.max_bytes"),
                format!("Use 1 to {MAX_FILE_BYTES} bytes."),
            );
        }
    }
}

/// An upload recorded on its application. Objects are never overwritten.
#[derive(Deserialize, Serialize)]
pub struct Attachment {
    id: Uuid,
    field: String,
    key: String,
    filename: String,
    content_type: String,
    size: u64,
    uploaded_at: String,
    /// Set once a submitted revision references the file.
    submitted: bool,
}

impl Attachment {
    pub fn list(model: &application::Model) -> Result<Vec<Self>, ApiError> {
        serde_json::from_value(model.attachments.clone())
            .context("stored attachments are invalid")
            .map_err(ApiError::Internal)
    }

    fn visible_to(&self, access: ApplicationAccess) -> bool {
        match access {
            ApplicationAccess::Owner => true,
            ApplicationAccess::Manager => self.submitted,
            ApplicationAccess::Organizer => false,
        }
    }
}

/// An uploaded application file.
#[derive(Serialize, ToSchema)]
pub struct FileInfo {
    id: String,
    /// The file field this upload belongs to.
    field: String,
    filename: String,
    content_type: String,
    size: u64,
    uploaded_at: String,
    /// Whether a submitted revision uses this file.
    submitted: bool,
}

impl From<Attachment> for FileInfo {
    fn from(attachment: Attachment) -> Self {
        Self {
            id: attachment.id.to_string(),
            field: attachment.field,
            filename: attachment.filename,
            content_type: attachment.content_type,
            size: attachment.size,
            uploaded_at: attachment.uploaded_at,
            submitted: attachment.submitted,
        }
    }
}

pub fn visible_files(
    model: &application::Model,
    access: ApplicationAccess,
) -> Result<Vec<FileInfo>, ApiError> {
    Ok(Attachment::list(model)?
        .into_iter()
        .filter(|attachment| attachment.visible_to(access))
        .map(FileInfo::from)
        .collect())
}

/// Returns the uploads referenced by file fields in `data`.
/// Each reference must name an upload made for that field on this application.
pub fn referenced_files(
    fields: &FileFields,
    attachments: &[Attachment],
    data: &Value,
) -> Result<Vec<Uuid>, ApiError> {
    let mut referenced = Vec::new();
    let mut errors = BTreeMap::new();
    for key in fields.keys() {
        let Some(value) = data.get(key).filter(|value| !value.is_null()) else {
            continue;
        };
        let id = value
            .as_str()
            .and_then(|id| id.parse::<Uuid>().ok())
            .filter(|id| {
                attachments
                    .iter()
                    .any(|attachment| attachment.id == *id && attachment.field == *key)
            });
        match id {
            Some(id) => referenced.push(id),
            None => {
                errors.insert(
                    format!("data.{key}"),
                    "Upload a file for this field, then use the returned file ID.".into(),
                );
            }
        }
    }
    if !errors.is_empty() {
        return Err(ApiError::Validation(errors));
    }
    Ok(referenced)
}

/// Marks the referenced uploads as used by a submitted revision.
pub fn mark_submitted(attachments: &mut [Attachment], referenced: &[Uuid]) {
    for attachment in attachments {
        if referenced.contains(&attachment.id) {
            attachment.submitted = true;
        }
    }
}

pub async fn remove_unused_uploads_hourly(db: DatabaseConnection, storage: Storage) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        if let Err(error) = remove_unused_uploads(&db, &storage).await {
            tracing::error!(?error, "failed to find unused application uploads");
        }
    }
}

/// Deletes uploads older than a day that no answer or submitted revision uses
pub async fn remove_unused_uploads(
    db: &DatabaseConnection,
    storage: &Storage,
) -> Result<usize, ApiError> {
    let rows = db
        .query_all_raw(statement(
            "SELECT a.id, a.hackathon_id, clock_timestamp() - interval '1 day' AS cutoff
                FROM application a JOIN hackathon h ON h.id=a.hackathon_id AND h.is_active
                WHERE EXISTS (SELECT 1 FROM jsonb_array_elements(a.attachments) f
                    WHERE NOT (f->>'submitted')::boolean
                    AND (f->>'uploaded_at')::timestamptz < clock_timestamp() - interval '1 day')",
            [],
        ))
        .await?;
    let mut removed = 0;
    for row in rows {
        let application_id: Uuid = row.try_get("", "id")?;
        let hackathon_id: Uuid = row.try_get("", "hackathon_id")?;
        let cutoff: DateTimeWithTimeZone = row.try_get("", "cutoff")?;
        match remove_unused_from(db, storage, application_id, hackathon_id, cutoff).await {
            Ok(count) => removed += count,
            Err(error) => {
                tracing::error!(%application_id, ?error, "failed to remove unused uploads");
            }
        }
    }
    Ok(removed)
}

async fn remove_unused_from(
    db: &DatabaseConnection,
    storage: &Storage,
    application_id: Uuid,
    hackathon_id: Uuid,
    cutoff: DateTimeWithTimeZone,
) -> Result<usize, ApiError> {
    let storage = storage.clone();
    audited_automatic!(
        db,
        Scope::Hackathon(hackathon_id),
        "Remove unused application uploads",
        |db| {
            let Some(current) = application::Entity::find_by_id(application_id)
                .lock_exclusive()
                .one(db)
                .await?
            else {
                return Ok(0);
            };
            let mut kept = Vec::new();
            let mut removed = 0;
            for attachment in Attachment::list(&current)? {
                let uploaded_at = DateTimeWithTimeZone::parse_from_rfc3339(&attachment.uploaded_at)
                    .context("stored upload time is invalid")
                    .map_err(ApiError::Internal)?;
                let answered = |data: &Value| {
                    data.get(&attachment.field)
                        .and_then(Value::as_str)
                        .and_then(|id| id.parse().ok())
                        == Some(attachment.id)
                };
                if attachment.submitted
                    || uploaded_at >= cutoff
                    || answered(&current.data)
                    || answered(&current.draft_data)
                {
                    kept.push(attachment);
                } else {
                    storage
                        .delete(&attachment.key)
                        .await
                        .map_err(ApiError::Internal)?;
                    removed += 1;
                }
            }
            if removed > 0 {
                let kept = serde_json::to_value(&kept)
                    .context("serialize attachments")
                    .map_err(ApiError::Internal)?;
                db.execute_raw(statement(
                    "UPDATE application SET attachments=$2 WHERE id=$1",
                    [application_id.into(), kept.into()],
                ))
                .await?;
            }
            Ok(removed)
        }
    )
    .await
}

#[derive(OpenApi)]
#[openapi(paths(upload_file, download_file))]
pub struct FilesApiDoc;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/hackathons/{slug}/applications/{user_id}/files",
            post(upload_file).layer(DefaultBodyLimit::max(MAX_FILE_BYTES as usize + 64 * 1024)),
        )
        .route(
            "/hackathons/{slug}/applications/{user_id}/files/{file_id}",
            get(download_file),
        )
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct UploadTarget {
    /// A key from the hackathon's `file_fields` settings.
    field: String,
}

/// A multipart form with one `file` part.
#[derive(ToSchema)]
#[allow(dead_code)]
struct Upload {
    #[schema(value_type = String, format = Binary)]
    file: Vec<u8>,
}

/// Stores a file for the caller's application. Put the returned ID in the field's answer to use it.
#[utoipa::path(post, path = "/api/hackathons/{slug}/applications/{user_id}/files",
    tag = "applications",
    params(("slug" = String, Path, description = "Hackathon slug"),
        ("user_id" = String, Path, description = "The signed-in user's Terrier UUID"),
        UploadTarget),
    request_body(content = Upload, content_type = "multipart/form-data"),
    responses((status = 201, body = FileInfo),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
        (status = 500, body = ErrorResponse)))]
async fn upload_file(
    auth: Auth<OwnApplication>,
    audit: Audit,
    State(state): State<AppState>,
    Extension(storage): Extension<Storage>,
    target: Result<Query<UploadTarget>, QueryRejection>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<(StatusCode, Json<FileInfo>), ApiError> {
    let Query(target) = target.map_err(|_| ApiError::BadRequest("Choose a file field.".into()))?;
    let multipart = multipart
        .map_err(|_| ApiError::BadRequest("Send the file as multipart form data.".into()))?;
    let Applicant {
        user_id,
        hackathon_id,
        has_role,
    } = auth.data;
    if !has_role {
        return Err(no_application());
    }
    let hackathon = hackathon::Entity::find_by_id(hackathon_id)
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    let rule = parse_file_fields(hackathon.file_fields)?
        .remove(&target.field)
        .ok_or_else(|| {
            field_error(
                "field",
                "This hackathon does not accept files for this field.",
            )
        })?;
    let current = application::Entity::find()
        .filter(application::Column::UserId.eq(user_id))
        .filter(application::Column::HackathonId.eq(hackathon_id))
        .one(&state.db)
        .await?
        .ok_or_else(no_application)?;
    ensure_upload_room(&current)?;
    let _slot = UPLOAD_SLOTS
        .acquire()
        .await
        .context("upload slots closed")
        .map_err(ApiError::Internal)?;
    let (filename, body) =
        tokio::time::timeout(UPLOAD_TIMEOUT, read_file(multipart, rule.max_bytes))
            .await
            .map_err(|_| ApiError::BadRequest("The upload took too long. Try again.".into()))??;
    let content_type = detect_type(&rule, &body)?;
    if content_type == "application/pdf" {
        let pdf = body.clone();
        if let Some(problem) = tokio::task::spawn_blocking(move || pdf_problem(&pdf))
            .await
            .context("inspect PDF")
            .map_err(ApiError::Internal)?
        {
            return Err(field_error("file", problem));
        }
    }
    let id = Uuid::new_v4();
    let key = format!("applications/{hackathon_id}/{user_id}/{id}");
    let attachment = json!({
        "id": id,
        "field": target.field,
        "key": key,
        "filename": filename,
        "content_type": content_type,
        "size": body.len(),
        "submitted": false,
    });
    let saved = audited!(audit, Scope::Hackathon(hackathon_id), |db| {
        let current = application::Entity::find()
            .filter(application::Column::UserId.eq(user_id))
            .filter(application::Column::HackathonId.eq(hackathon_id))
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or_else(no_application)?;
        ensure_editable(&current)?;
        open_hackathon(db, hackathon_id).await?;
        ensure_upload_room(&current)?;
        storage
            .put(&key, body, &content_type)
            .await
            .map_err(ApiError::Internal)?;
        record_upload(db, current.id, hackathon_id, attachment).await
    })
    .await?;
    Ok((StatusCode::CREATED, Json(saved.into())))
}

/// Appends an upload to its application, timestamped by the database.
async fn record_upload(
    db: &DatabaseTransaction,
    application_id: Uuid,
    hackathon_id: Uuid,
    attachment: Value,
) -> Result<Attachment, ApiError> {
    let row = db
        .query_one_raw(statement(
            "UPDATE application SET attachments = attachments
                || jsonb_build_array($3::jsonb || jsonb_build_object('uploaded_at', clock_timestamp()))
                WHERE id=$1 AND hackathon_id=$2 RETURNING attachments->-1 AS attachment",
            [application_id.into(), hackathon_id.into(), attachment.into()],
        ))
        .await?
    .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("uploaded application is missing")))?;
    serde_json::from_value(row.try_get::<Value>("", "attachment")?)
        .context("recorded attachment is invalid")
        .map_err(ApiError::Internal)
}

/// Sends a stored application file to its owner or, once submitted, to application managers.
#[utoipa::path(get, path = "/api/hackathons/{slug}/applications/{user_id}/files/{file_id}",
    tag = "applications",
    params(("slug" = String, Path, description = "Hackathon slug"),
        ("user_id" = String, Path, description = "Applicant's Terrier user UUID"),
        ("file_id" = String, Path, description = "File ID returned by the upload")),
    responses((status = 200, description = "The file contents", content_type = "application/octet-stream"),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)))]
async fn download_file(
    auth: Auth<ReadApplicationFile>,
    State(state): State<AppState>,
    Extension(storage): Extension<Storage>,
) -> Result<Response, ApiError> {
    let (target, file_id) = auth.data;
    if target.access == ApplicationAccess::Organizer {
        return Err(ApiError::Forbidden);
    }
    let model = applications_in(target.hackathon_id, target.access)
        .filter(application::Column::UserId.eq(target.applicant_id))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    let attachment = Attachment::list(&model)?
        .into_iter()
        .find(|attachment| attachment.id == file_id && attachment.visible_to(target.access))
        .ok_or(ApiError::NotFound)?;
    let body = storage
        .get(&attachment.key)
        .await
        .map_err(ApiError::Internal)?;
    Ok((
        [
            (CONTENT_TYPE, attachment.content_type),
            (CONTENT_DISPOSITION, disposition(&attachment.filename)),
            (X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
            (CACHE_CONTROL, "private, no-store".into()),
        ],
        body,
    )
        .into_response())
}

fn ensure_upload_room(current: &application::Model) -> Result<(), ApiError> {
    let unsubmitted = Attachment::list(current)?
        .iter()
        .filter(|attachment| !attachment.submitted)
        .count();
    if unsubmitted >= MAX_UNSUBMITTED_UPLOADS {
        return Err(ApiError::Conflict(
            "You have uploaded too many files. Unused files are removed after 24 hours.".into(),
        ));
    }
    Ok(())
}

fn no_application() -> ApiError {
    ApiError::Conflict("Save your application before uploading files.".into())
}

fn field_error(field: &str, message: &str) -> ApiError {
    ApiError::Validation(BTreeMap::from([(field.into(), message.into())]))
}

fn too_large(max_bytes: u64) -> ApiError {
    field_error(
        "file",
        &format!(
            "Choose a file no larger than {:.1} MB.",
            max_bytes as f64 / 1_048_576.0
        ),
    )
}

fn upload_error(error: MultipartError, max_bytes: u64) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return too_large(max_bytes);
    }
    ApiError::BadRequest("The file upload was incomplete or malformed.".into())
}

async fn read_file(mut multipart: Multipart, max_bytes: u64) -> Result<(String, Bytes), ApiError> {
    while let Some(mut part) = multipart
        .next_field()
        .await
        .map_err(|error| upload_error(error, max_bytes))?
    {
        if part.name() != Some("file") {
            continue;
        }
        let filename = clean_filename(part.file_name().unwrap_or_default());
        let mut body = Vec::new();
        while let Some(chunk) = part
            .chunk()
            .await
            .map_err(|error| upload_error(error, max_bytes))?
        {
            if (body.len() + chunk.len()) as u64 > max_bytes {
                return Err(too_large(max_bytes));
            }
            body.extend_from_slice(&chunk);
        }
        if body.is_empty() {
            return Err(field_error("file", "Choose a file that is not empty."));
        }
        return Ok((filename, body.into()));
    }
    Err(field_error("file", "Choose a file to upload."))
}

fn detect_type(rule: &FileField, body: &[u8]) -> Result<String, ApiError> {
    let detected = infer::get(body).map(|kind| kind.mime_type());
    if rule.types.iter().any(|kind| kind == ANY_TYPE) {
        return Ok(detected.unwrap_or("application/octet-stream").into());
    }
    detected
        .filter(|mime| rule.types.iter().any(|kind| kind == mime))
        .map(Into::into)
        .ok_or_else(|| {
            field_error(
                "file",
                &format!(
                    "Choose a file of an accepted type: {}.",
                    rule.types.join(", ")
                ),
            )
        })
}

fn pdf_problem(body: &[u8]) -> Option<&'static str> {
    if pdf_names(body).any(|name| name == b"Encrypt") {
        return Some(ACTIVE_PDF);
    }
    ACTIVE_PDF_OBJECT.set(false);
    let options = LoadOptions {
        filter: Some(inspect_pdf_object),
        max_decompressed_size: Some(4 * 1024 * 1024),
        ..LoadOptions::default()
    };
    if lopdf::Document::load_mem_with_options(body, options).is_err() {
        return Some("This PDF could not be read. Export it again and upload it.");
    }
    ACTIVE_PDF_OBJECT.get().then_some(ACTIVE_PDF)
}

fn inspect_pdf_object(id: ObjectId, object: &mut Object) -> Option<(ObjectId, Object)> {
    if has_active_name(object) {
        ACTIVE_PDF_OBJECT.set(true);
    }
    let object_stream = matches!(object, Object::Stream(stream) if stream.dict.has_type(b"ObjStm"));
    object_stream.then(|| (id, object.clone()))
}

fn has_active_name(object: &Object) -> bool {
    let dictionary = match object {
        Object::Name(name) => return ACTIVE_PDF_NAMES.contains(&name.as_slice()),
        Object::Array(items) => return items.iter().any(has_active_name),
        Object::Dictionary(dictionary) => dictionary,
        Object::Stream(stream) => &stream.dict,
        _ => return false,
    };
    dictionary
        .iter()
        .any(|(key, value)| ACTIVE_PDF_NAMES.contains(&key.as_slice()) || has_active_name(value))
}

fn pdf_names(body: &[u8]) -> impl Iterator<Item = Vec<u8>> + '_ {
    body.split(|&byte| byte == b'/').skip(1).map(|rest| {
        let end = rest
            .iter()
            .position(|byte| byte.is_ascii_whitespace() || b"/[]<>(){}%".contains(byte))
            .unwrap_or(rest.len());
        let mut name = Vec::new();
        let mut bytes = rest[..end].iter();
        while let Some(&byte) = bytes.next() {
            let escaped = (byte == b'#')
                .then(|| std::str::from_utf8(bytes.as_slice().get(..2)?).ok())
                .flatten()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok());
            match escaped {
                Some(decoded) => {
                    name.push(decoded);
                    bytes.nth(1);
                }
                None => name.push(byte),
            }
        }
        name
    })
}

fn clean_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let clean: String = base.chars().filter(|c| !c.is_control()).take(255).collect();
    match clean.trim() {
        "" => "file".into(),
        trimmed => trimmed.into(),
    }
}

fn disposition(filename: &str) -> String {
    let ascii: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("attachment; filename=\"{ascii}\"")
}

#[cfg(test)]
mod tests {
    use lopdf::{Dictionary, Document, SaveOptions, dictionary};

    use super::*;

    fn attachment(id: Uuid, field: &str) -> Attachment {
        Attachment {
            id,
            field: field.into(),
            key: format!("key/{id}"),
            filename: "resume.pdf".into(),
            content_type: "application/pdf".into(),
            size: 4,
            uploaded_at: "2026-11-21T00:00:00+00:00".into(),
            submitted: false,
        }
    }

    #[test]
    fn file_fields_default_to_pdf() {
        let fields: FileFields = serde_json::from_value(json!({"portfolio": {}})).unwrap();
        assert_eq!(fields["portfolio"].types, ["application/pdf"]);
        assert_eq!(fields["portfolio"].max_bytes, DEFAULT_MAX_BYTES);
    }

    #[test]
    fn file_field_settings_are_validated() {
        let fields: FileFields = serde_json::from_value(json!({
            "Bad Key": {},
            "any": {"types": ["*/*"]},
            "unknown": {"types": ["text/x-unknown"]},
            "huge": {"max_bytes": MAX_FILE_BYTES + 1},
        }))
        .unwrap();
        let mut errors = BTreeMap::new();
        validate_file_fields(&fields, &mut errors);
        assert_eq!(
            errors.keys().collect::<Vec<_>>(),
            [
                "settings.file_fields.Bad Key",
                "settings.file_fields.huge.max_bytes",
                "settings.file_fields.unknown.types",
            ]
        );
    }

    #[test]
    fn references_must_name_uploads_for_the_same_field() {
        let fields: FileFields =
            serde_json::from_value(json!({"resume": {}, "cover": {}})).unwrap();
        let resume = Uuid::new_v4();
        let cover = Uuid::new_v4();
        let attachments = [attachment(resume, "resume"), attachment(cover, "cover")];

        let data = json!({"resume": resume, "cover": null, "name": "Ada"});
        assert_eq!(
            referenced_files(&fields, &attachments, &data).unwrap(),
            [resume]
        );

        for data in [
            json!({"resume": cover}),
            json!({"resume": Uuid::new_v4()}),
            json!({"resume": "not-a-uuid"}),
            json!({"resume": 7}),
        ] {
            let Err(ApiError::Validation(errors)) = referenced_files(&fields, &attachments, &data)
            else {
                panic!("accepted {data}");
            };
            assert!(errors.contains_key("data.resume"));
        }
    }

    #[test]
    fn detected_type_must_be_allowed() {
        let pdf = b"%PDF-1.7\n";
        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
        let rule = FileField {
            types: pdf_only(),
            max_bytes: DEFAULT_MAX_BYTES,
        };
        assert_eq!(detect_type(&rule, pdf).unwrap(), "application/pdf");
        assert!(detect_type(&rule, png).is_err());
        assert!(detect_type(&rule, b"plain text").is_err());

        let any = FileField {
            types: vec![ANY_TYPE.into()],
            max_bytes: DEFAULT_MAX_BYTES,
        };
        assert_eq!(detect_type(&any, png).unwrap(), "image/png");
        assert_eq!(
            detect_type(&any, b"plain text").unwrap(),
            "application/octet-stream"
        );
    }

    const MINIMAL_PDF: &str = "%PDF-1.4\n1 0 obj <</Type/Catalog/Pages 2 0 R{}>> endobj\n2 0 obj <</Type/Pages/Kids[]/Count 0>> endobj\ntrailer <</Root 1 0 R{}>>\n%%EOF\n";

    fn minimal_pdf(catalog: &str, trailer: &str) -> Vec<u8> {
        MINIMAL_PDF
            .replacen("{}", catalog, 1)
            .replacen("{}", trailer, 1)
            .into_bytes()
    }

    fn compressed_pdf(open_action: Option<Dictionary>) -> Vec<u8> {
        let mut document = Document::with_version("1.5");
        let pages =
            document.add_object(dictionary! {"Type" => "Pages", "Kids" => vec![], "Count" => 0});
        let mut catalog = dictionary! {"Type" => "Catalog", "Pages" => pages};
        if let Some(action) = open_action {
            catalog.set("OpenAction", action);
        }
        let catalog = document.add_object(catalog);
        document.trailer.set("Root", catalog);
        let mut body = Vec::new();
        document
            .save_with_options(
                &mut body,
                SaveOptions::builder()
                    .use_object_streams(true)
                    .use_xref_streams(true)
                    .compression_level(6)
                    .build(),
            )
            .unwrap();
        body
    }

    #[test]
    fn pdfs_with_active_content_are_detected() {
        assert_eq!(pdf_problem(&minimal_pdf("", "")), None);
        assert_eq!(pdf_problem(&compressed_pdf(None)), None);

        let script = dictionary! {
            "S" => "JavaScript",
            "JS" => Object::string_literal("app.alert(1)"),
        };
        let hidden = compressed_pdf(Some(script));
        assert!(!pdf_names(&hidden).any(|name| name == b"JavaScript"));
        assert_eq!(pdf_problem(&hidden), Some(ACTIVE_PDF));

        for pdf in [
            minimal_pdf("/OpenAction<</S/Java#53cript/JS(app.alert(1))>>", ""),
            minimal_pdf("/OpenAction<</S/Launch/F(run.exe)>>", ""),
            minimal_pdf("", "/Encr#79pt 3 0 R"),
        ] {
            assert_eq!(
                pdf_problem(&pdf),
                Some(ACTIVE_PDF),
                "{}",
                String::from_utf8_lossy(&pdf)
            );
        }
        assert!(
            pdf_problem(b"%PDF-1.7\nnot a document").is_some_and(|problem| problem != ACTIVE_PDF)
        );
    }

    #[test]
    fn filenames_are_cleaned_for_storage_and_headers() {
        assert_eq!(clean_filename("C:\\Users\\ada\\resume.pdf"), "resume.pdf");
        assert_eq!(clean_filename("../../etc/passwd"), "passwd");
        assert_eq!(clean_filename(" \n "), "file");
        assert_eq!(
            disposition("r\u{e9}sum\u{e9} \"final\".pdf"),
            "attachment; filename=\"r_sum_ _final_.pdf\""
        );
    }
}
