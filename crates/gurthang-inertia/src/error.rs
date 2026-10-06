use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("response serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("{0}")]
    BadRequest(String),
    #[error("internal server error")]
    Internal,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match &self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message.clone()).into_response(),
            Self::Serialization(_) | Self::Internal => {
                tracing::error!(error = %self, "inertia response failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}
