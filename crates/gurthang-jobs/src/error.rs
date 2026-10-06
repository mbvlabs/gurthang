pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("background job database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("background job serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("background job configuration error: {0}")]
    Config(String),
}
