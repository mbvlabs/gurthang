use axum::Router;
use serde::{Deserialize, Serialize};
use tower_http::compression::CompressionLayer;

use crate::{app::Context, controller::middleware::MiddlewareLayer, error::Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Compression {
    #[serde(default)]
    pub enable: bool,
}

impl MiddlewareLayer for Compression {
    fn name(&self) -> &'static str {
        "compression"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app.layer(CompressionLayer::new()))
    }
}
