use std::time::Duration;

use axum::{Router, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::timeout::TimeoutLayer;

use crate::{app::Context, controller::middleware::MiddlewareLayer, error::Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeOut {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

impl Default for TimeOut {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("empty timeout config")
    }
}

fn default_timeout() -> u64 {
    5_000
}

impl MiddlewareLayer for TimeOut {
    fn name(&self) -> &'static str {
        "timeout_request"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app.layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_millis(self.timeout),
        )))
    }
}
