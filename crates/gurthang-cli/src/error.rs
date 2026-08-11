use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidProjectName(String),
    DestinationExists(String),
    Io { context: String, source: io::Error },
    UnknownPlaceholder { path: String },
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
            Self::UnknownPlaceholder { path } => {
                write!(formatter, "unresolved scaffold placeholder in {path}")
            }
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
