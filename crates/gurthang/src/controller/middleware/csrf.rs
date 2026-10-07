use axum::{Router, middleware};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer},
    error::Result,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Csrf {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(skip)]
    pub secure: bool,
}

impl Default for Csrf {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("empty CSRF config")
    }
}

fn default_true() -> bool {
    true
}

impl MiddlewareLayer for Csrf {
    fn name(&self) -> &'static str {
        "csrf"
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
        Ok(app.layer(middleware::from_fn_with_state(
            self.secure,
            gurthang_http::protect,
        )))
    }
}
