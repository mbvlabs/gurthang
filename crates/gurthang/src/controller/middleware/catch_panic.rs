use axum::{Router, response::IntoResponse};
use serde::{Deserialize, Serialize};
use tower_http::catch_panic::CatchPanicLayer;

use crate::{app::Context, controller::middleware::MiddlewareLayer, error::Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatchPanic {
    #[serde(default)]
    pub enable: bool,
}

fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> axum::response::Response {
    let message = err.downcast_ref::<String>().map_or_else(
        || {
            err.downcast_ref::<&str>()
                .copied()
                .unwrap_or("no error details")
        },
        String::as_str,
    );
    tracing::error!(panic = message, "server panic");
    axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

impl MiddlewareLayer for CatchPanic {
    fn name(&self) -> &'static str {
        "catch_panic"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app.layer(CatchPanicLayer::custom(handle_panic)))
    }
}
