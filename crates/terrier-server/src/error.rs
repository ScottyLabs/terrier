use std::collections::BTreeMap;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

/// API errors. Messages and field errors are sent to clients.
///
/// Internal errors are logged here; do not log them again upstream.
#[derive(Debug)]
pub enum ApiError {
    /// 400: Malformed input or unknown enum value.
    BadRequest(String),
    /// 401: Missing, expired, or invalid authentication. Use `Internal` for storage failures.
    Unauthenticated,
    /// 403: Authenticated caller lacks permission or eligibility.
    Forbidden,
    /// 404: Resource not found within the requested scope.
    NotFound,
    /// 409: Duplicate resource, capacity limit, stale revision, or workflow conflict.
    Conflict(String),
    /// 422: Well-formed input fails validation. Maps field paths to error messages.
    Validation(BTreeMap<String, String>),
    /// 500: Unexpected failure. Context is logged, not sent to clients; exclude secrets.
    Internal(anyhow::Error),
}

/// Error response body. Omits empty field errors.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut fields = BTreeMap::new();
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Sign in to continue.".into(),
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "You do not have permission to perform this action.".into(),
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "The requested resource was not found.".into(),
            ),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", message),
            Self::Validation(errors) => {
                fields = errors;
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation_failed",
                    "Request data is invalid.".into(),
                )
            }
            Self::Internal(error) => {
                tracing::error!(error = ?error, "request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An unexpected error occurred.".into(),
                )
            }
        };

        (
            status,
            Json(ErrorResponse {
                code,
                message,
                fields,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::CONTENT_TYPE},
        routing::get,
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::ApiError;

    #[tokio::test]
    async fn handler_errors_hide_internal_details() {
        async fn handler() -> Result<(), ApiError> {
            Err(ApiError::Internal(
                anyhow::anyhow!("private database detail").context("failed to load user"),
            ))
        }

        let response = Router::new()
            .route("/", get(handler))
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            body,
            json!({
                "code": "internal_error",
                "message": "An unexpected error occurred.",
            })
        );
    }
}
