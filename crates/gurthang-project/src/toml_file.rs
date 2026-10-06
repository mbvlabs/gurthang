use std::{fs, path::Path};

use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Clone, Debug, Deserialize)]
pub struct GurthangToml {
    pub schema_version: u32,
    pub project: ProjectConfig,
    pub database: DatabaseConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub inertia: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    pub engine: String,
}

impl GurthangToml {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("gurthang.toml");
        let text = fs::read_to_string(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Error::MissingToml(path)
            } else {
                Error::io(format!("could not read {}", path.display()), error)
            }
        })?;
        let parsed: Self = toml::from_str(&text).map_err(|error| Error::Toml(error.to_string()))?;
        if parsed.schema_version != 1 {
            return Err(Error::Toml(format!(
                "unsupported schema_version {}",
                parsed.schema_version
            )));
        }
        if parsed.database.engine != "postgres" {
            return Err(Error::Toml(format!(
                "unsupported database engine {}",
                parsed.database.engine
            )));
        }
        Ok(parsed)
    }
}
