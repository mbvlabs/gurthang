pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Jobs(#[from] gurthang_jobs::Error),
    #[error(transparent)]
    Inertia(#[from] gurthang_inertia::Error),
    #[error("asset configuration error: {0}")]
    Asset(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<gurthang_http::AssetError> for Error {
    fn from(error: gurthang_http::AssetError) -> Self {
        Self::Asset(error.to_string())
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(error: serde_yaml::Error) -> Self {
        Self::Config(error.to_string())
    }
}
