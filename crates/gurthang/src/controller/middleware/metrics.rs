use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{Router, extract::Request, middleware::Next, response::Response};
use serde::{Deserialize, Serialize};

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer, request_id::RequestIdValue},
    error::Result,
};

/// A single request observation handed to a [`MetricsRecorder`].
#[derive(Clone, Debug)]
pub struct RequestMetrics {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration: Duration,
    pub request_id: Option<String>,
}

/// Pluggable metrics backend. Gurthang ships [`TracingMetrics`]; apps can
/// replace it with an exporter by building [`Metrics::with_recorder`].
pub trait MetricsRecorder: Send + Sync + 'static {
    fn record(&self, metrics: &RequestMetrics);
}

/// Default recorder: emits a structured `metric` event through `tracing`.
#[derive(Debug, Default)]
pub struct TracingMetrics;

impl MetricsRecorder for TracingMetrics {
    fn record(&self, metrics: &RequestMetrics) {
        tracing::info!(
            metric = "http.server.requests",
            http.method = %metrics.method,
            http.path = %metrics.path,
            http.status = metrics.status,
            duration_ms = metrics.duration.as_secs_f64() * 1000.0,
            request_id = metrics.request_id.as_deref().unwrap_or("-"),
            "metric",
        );
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_skip_paths")]
    pub skip_paths: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable: true,
            skip_paths: default_skip_paths(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_skip_paths() -> Vec<String> {
    vec!["/assets".into(), "/favicon.ico".into()]
}

impl Config {
    fn skips(&self, path: &str) -> bool {
        self.skip_paths.iter().any(|prefix| {
            if prefix == "/" {
                path == "/"
            } else {
                path == prefix || path.starts_with(&format!("{prefix}/"))
            }
        })
    }
}

/// Post-processing middleware that records counters/histograms per request.
pub struct Metrics {
    config: Config,
    recorder: Arc<dyn MetricsRecorder>,
}

pub fn new(config: &Option<Config>) -> Metrics {
    Metrics {
        config: config.clone().unwrap_or_default(),
        recorder: Arc::new(TracingMetrics),
    }
}

impl Metrics {
    pub fn with_recorder(mut self, recorder: Arc<dyn MetricsRecorder>) -> Self {
        self.recorder = recorder;
        self
    }
}

impl MiddlewareLayer for Metrics {
    fn name(&self) -> &'static str {
        "metrics"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Post
    }

    fn is_enabled(&self) -> bool {
        self.config.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(&self.config)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        let config = self.config.clone();
        let recorder = Arc::clone(&self.recorder);
        Ok(app.layer(axum::middleware::from_fn(
            move |request: Request, next: Next| {
                let config = config.clone();
                let recorder = Arc::clone(&recorder);
                async move { record_metrics(config, recorder, request, next).await }
            },
        )))
    }
}

async fn record_metrics(
    config: Config,
    recorder: Arc<dyn MetricsRecorder>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let request_id = request
        .extensions()
        .get::<RequestIdValue>()
        .map(|value| value.0.clone());
    let start = Instant::now();

    let response = next.run(request).await;

    if !config.skips(&path) {
        recorder.record(&RequestMetrics {
            method,
            path,
            status: response.status().as_u16(),
            duration: start.elapsed(),
            request_id,
        });
    }

    response
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{Config, MetricsRecorder, RequestMetrics};

    #[derive(Default)]
    struct Capture(Arc<Mutex<Vec<RequestMetrics>>>);

    impl MetricsRecorder for Capture {
        fn record(&self, metrics: &RequestMetrics) {
            self.0.lock().unwrap().push(metrics.clone());
        }
    }

    #[test]
    fn default_config_skips_assets() {
        let config = Config::default();
        assert!(config.skips("/assets/dist/app.js"));
        assert!(!config.skips("/dashboard"));
    }

    #[test]
    fn capture_records_metrics() {
        let capture = Capture::default();
        capture.record(&RequestMetrics {
            method: "GET".into(),
            path: "/".into(),
            status: 200,
            duration: std::time::Duration::from_millis(3),
            request_id: None,
        });
        assert_eq!(capture.0.lock().unwrap().len(), 1);
    }
}
