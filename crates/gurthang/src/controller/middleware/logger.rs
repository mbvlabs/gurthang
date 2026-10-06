use axum::{Router, http};
use serde::{Deserialize, Serialize};
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};

use crate::{
    app::Context,
    config::Environment,
    controller::middleware::{MiddlewareLayer, request_id::RequestIdValue},
    error::Result,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub enable: bool,
}

pub struct Logger {
    config: Config,
    environment: Environment,
}

pub fn new(config: &Config, environment: &Environment) -> Logger {
    Logger {
        config: config.clone(),
        environment: environment.clone(),
    }
}

impl MiddlewareLayer for Logger {
    fn name(&self) -> &'static str {
        "logger"
    }

    fn is_enabled(&self) -> bool {
        self.config.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(&self.config)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app
            .layer(
                TraceLayer::new_for_http().make_span_with(|request: &http::Request<_>| {
                    let request_id = request
                        .extensions()
                        .get::<RequestIdValue>()
                        .map(|id| id.0.clone())
                        .unwrap_or_else(|| "req-id-none".into());
                    let user_agent = request
                        .headers()
                        .get(http::header::USER_AGENT)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("")
                        .to_owned();
                    let environment = request
                        .extensions()
                        .get::<Environment>()
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    tracing::info_span!(
                        "http-request",
                        "http.method" = %request.method(),
                        "http.uri" = %request.uri(),
                        "http.user_agent" = user_agent,
                        environment = environment.as_str(),
                        request_id = request_id.as_str(),
                    )
                }),
            )
            .layer(AddExtensionLayer::new(self.environment.clone())))
    }
}
