use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("response serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("background job operation failed")]
    Jobs(#[from] gurthang::jobs::Error),
    #[error("inertia rendering failed")]
    Inertia(#[from] gurthang::inertia::Error),
    #[error("asset configuration error: {0}")]
    Asset(String),
    #[error("session operation failed: {0}")]
    Session(String),
    #[error("authentication operation failed: {0}")]
    Authentication(String),
    #[error("CSRF verification failed")]
    Csrf,
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error("internal server error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, public_message) = match &self {
            Self::NotFound => (StatusCode::NOT_FOUND, "Not found"),
            Self::BadRequest(message) => {
                return (StatusCode::BAD_REQUEST, message.clone()).into_response();
            }
            Self::Csrf => {
                return (StatusCode::FORBIDDEN, "CSRF verification failed").into_response();
            }
            Self::Config(_)
            | Self::Database(_)
            | Self::Serialization(_)
            | Self::Jobs(_)
            | Self::Inertia(_)
            | Self::Asset(_)
            | Self::Session(_)
            | Self::Authentication(_)
            | Self::Internal => {
                tracing::error!(error = %self, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };
        (status, public_message).into_response()
    }
}
