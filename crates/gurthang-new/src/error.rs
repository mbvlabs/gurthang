use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidProjectName(String),
    DestinationExists(String),
    Io { context: String, source: io::Error },
    Render(String),
}

impl Error {
    pub fn io(context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProjectName(message) => {
                write!(formatter, "invalid project name: {message}")
            }
            Self::DestinationExists(message) => write!(formatter, "{message}"),
            Self::Io { context, source } => write!(formatter, "{context}: {source}"),
            Self::Render(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<gurthang_project::Error> for Error {
    fn from(error: gurthang_project::Error) -> Self {
        match error {
            gurthang_project::Error::InvalidProjectName(message) => {
                Self::InvalidProjectName(message)
            }
            gurthang_project::Error::Io { context, source } => Self::Io { context, source },
            other => Self::DestinationExists(other.to_string()),
        }
    }
}

impl From<askama::Error> for Error {
    fn from(error: askama::Error) -> Self {
        Self::Render(error.to_string())
    }
}
