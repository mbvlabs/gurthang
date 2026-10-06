use std::{fmt, io, path::PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidProjectName(String),
    NotAProject,
    MissingToml(PathBuf),
    Toml(String),
    Io { context: String, source: io::Error },
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
            Self::NotAProject => write!(
                formatter,
                "not inside a Gurthang application (gurthang.toml and Cargo.toml not found)"
            ),
            Self::MissingToml(path) => {
                write!(formatter, "could not read {}: file is missing", path.display())
            }
            Self::Toml(message) => write!(formatter, "invalid gurthang.toml: {message}"),
            Self::Io { context, source } => write!(formatter, "{context}: {source}"),
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
