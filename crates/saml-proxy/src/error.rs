use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("invalid SAML request: {0}")]
    InvalidSamlRequest(String),
    #[error("invalid SAML response: {0}")]
    InvalidSamlResponse(String),
    #[error("MDQ fetch failed: {0}")]
    MdqFetchFailed(String),
    #[error("missing university selection")]
    MissingUniversitySelection,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match &self {
            Error::SessionNotFound(_) => StatusCode::NOT_FOUND,
            Error::InvalidSamlRequest(_) | Error::MissingUniversitySelection => {
                StatusCode::BAD_REQUEST
            }
            Error::InvalidSamlResponse(_) => StatusCode::BAD_GATEWAY,
            Error::MdqFetchFailed(_) => StatusCode::BAD_GATEWAY,
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        tracing::error!(error = %self, "request failed");
        (status, self.to_string()).into_response()
    }
}
