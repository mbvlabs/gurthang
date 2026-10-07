use axum::{Router, extract::Request, middleware::Next, response::Response};
use serde::{Deserialize, Serialize};
use tracing::Instrument;

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer, request_id::RequestIdValue},
    error::Result,
};

/// Configuration for the tracing-span middleware.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_span_name")]
    pub span_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable: true,
            span_name: default_span_name(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_span_name() -> String {
    "http.request".into()
}

/// Wrapping middleware that opens a span covering the rest of the pipeline.
pub struct Telemetry {
    config: Config,
}

pub fn new(config: &Option<Config>) -> Telemetry {
    Telemetry {
        config: config.clone().unwrap_or_default(),
    }
}

impl MiddlewareLayer for Telemetry {
    fn name(&self) -> &'static str {
        "telemetry"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Wrap
    }

    fn is_enabled(&self) -> bool {
        self.config.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(&self.config)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        let config = self.config.clone();
        Ok(app.layer(axum::middleware::from_fn(
            move |request: Request, next: Next| {
                let config = config.clone();
                async move { span_pipeline(config, request, next).await }
            },
        )))
    }
}

async fn span_pipeline(config: Config, request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let request_id = request
        .extensions()
        .get::<RequestIdValue>()
        .map(|value| value.0.clone());

    let span = tracing::info_span!(
        "http.request",
        otel.name = %config.span_name,
        http.method = %method,
        http.path = %path,
        request_id = request_id.as_deref().unwrap_or("-"),
    );

    next.run(request).instrument(span).await
}
