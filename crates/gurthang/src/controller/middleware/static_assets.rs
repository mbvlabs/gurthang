use std::path::PathBuf;

use axum::Router;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::services::ServeDir;

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer},
    error::{Error, Result},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticAssets {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default)]
    pub must_exist: bool,
    #[serde(default = "default_folder")]
    pub folder: FolderConfig,
    #[serde(skip)]
    pub development: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FolderConfig {
    pub uri: String,
    pub path: PathBuf,
}

impl Default for StaticAssets {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("empty static assets config")
    }
}

fn default_true() -> bool {
    true
}

fn default_folder() -> FolderConfig {
    FolderConfig {
        uri: "/assets".into(),
        path: PathBuf::from("assets"),
    }
}

impl MiddlewareLayer for StaticAssets {
    fn name(&self) -> &'static str {
        "static"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Wrap
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        if !self.development {
            return Ok(app);
        }
        if self.must_exist && !self.folder.path.exists() {
            return Err(Error::Message(format!(
                "static assets folder {} does not exist",
                self.folder.path.display()
            )));
        }
        Ok(app.nest_service(&self.folder.uri, ServeDir::new(&self.folder.path)))
    }
}
