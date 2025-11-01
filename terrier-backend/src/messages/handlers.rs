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
    pub created_at: DateTime,
    pub parent_message_id: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateMessageRequest {
    pub sender_user_id: i32,
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
    // Create message
    let message = messages::ActiveModel {
        sender_user_id: Set(req.sender_user_id),
        recipient_user_ids: Set(req.recipient_user_ids),
        subject: Set(req.subject),
        content: Set(req.content),
        created_at: Set(chrono::Utc::now().naive_utc()),
        parent_message_id: Set(parent_message_id),
        ..Default::default()
    };

    let result = message
        .insert(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(MessageResponse {
            id: result.id,
            sender_user_id: result.sender_user_id,
            recipient_user_ids: result.recipient_user_ids,
            subject: result.subject,
            content: result.content,
            created_at: result.created_at,
            parent_message_id: result.parent_message_id,
        }),
    ))
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

    let message = Messages::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match message {
        Some(msg) => Ok(Json(MessageResponse {
            id: msg.id,
            sender_user_id: msg.sender_user_id,
            recipient_user_ids: msg.recipient_user_ids,
            subject: msg.subject,
            content: msg.content,
            created_at: msg.created_at,
            parent_message_id: msg.parent_message_id,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
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
