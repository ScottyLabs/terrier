use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use axum_oidc::OidcClaims;
use axum_oidc::EmptyAdditionalClaims;

use crate::{
    AppState,
    entities::{messages, prelude::*},
};

/// Message response DTO
#[derive(Serialize, ToSchema)]
pub struct MessageResponse {
    pub id: i32,
    pub sender_user_id: i32,
    pub recipient_user_ids: Vec<i32>,
    pub subject: String,
    pub content: String,
    pub sent_time: String,
    pub parent_message_id: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateMessageRequest {
    pub recipient_user_ids: Vec<i32>,
    pub subject: Option<String>,
    pub content: String,
    pub parent_message_id: Option<i32>,
}

/// Create a new message
#[utoipa::path(
    post,
    path = "/api/messages",
    request_body = CreateMessageRequest,
    responses(
        (status = 201, description = "Message created", body = MessageResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Not authenticated")
    ),
    tag = "Messages"
)]
pub async fn create_message(
    _claims: OidcClaims<EmptyAdditionalClaims>,
    State(_state): State<AppState>,
    Json(_req): Json<CreateMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), StatusCode> {
    // TODO: Implement create logic using SeaORM ActiveModel
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Get a single message by id
#[utoipa::path(
    get,
    path = "/api/messages/{id}",
    responses(
        (status = 200, description = "Message", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Not found")
    ),
    tag = "Messages"
)]
pub async fn get_message(
    _claims: OidcClaims<EmptyAdditionalClaims>,
    State(_state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> Result<Json<MessageResponse>, StatusCode> {
    // TODO: Fetch from DB
    Err(StatusCode::NOT_FOUND)
}

#[derive(Deserialize, ToSchema)]
pub struct ListMessagesQuery {
    pub page_size: Option<u32>,
    pub cursor: Option<String>,
}

/// List messages for a conversation — here simplified as list of messages
#[utoipa::path(
    get,
    path = "/api/messages",
    params(("page_size" = Option<u32>, Query, description = "Page size")),
    responses(
        (status = 200, description = "List of messages", body = Vec<MessageResponse>),
    ),
    tag = "Messages"
)]
pub async fn list_messages(
    _claims: OidcClaims<EmptyAdditionalClaims>,
    State(_state): State<AppState>,
    axum::extract::Query(_query): axum::extract::Query<ListMessagesQuery>,
) -> Result<Json<Vec<MessageResponse>>, StatusCode> {
    // TODO: Query DB for messages
    Ok(Json(vec![]))
}
