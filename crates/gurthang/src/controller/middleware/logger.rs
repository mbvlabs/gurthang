use std::time::Instant;

use axum::{Router, extract::Request, middleware::Next, response::Response};
use serde::{Deserialize, Serialize};

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer, request_id::RequestIdValue},
    error::Result,
};

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

pub struct Logger {
    config: Config,
}

pub fn new(config: &Config) -> Logger {
    Logger {
        config: config.clone(),
    }
}

impl MiddlewareLayer for Logger {
    fn name(&self) -> &'static str {
        "logger"
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
        Ok(app.layer(axum::middleware::from_fn(
            move |request: Request, next: Next| {
                let config = config.clone();
                async move { log_request(config, request, next).await }
            },
        )))
    }
}

async fn log_request(config: Config, request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let request_id = request
        .extensions()
        .get::<RequestIdValue>()
        .map(|value| value.0.clone());
    let start = Instant::now();

    let response = next.run(request).await;

    if !config.skips(&path) {
        let duration = start.elapsed();
        tracing::info!(
            http.method = %method,
            http.path = %path,
            http.status = response.status().as_u16(),
            duration_ms = duration.as_secs_f64() * 1000.0,
            request_id = request_id.as_deref().unwrap_or("-"),
            "request completed",
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn skip_paths_match_prefixes() {
        let config = Config::default();
        assert!(config.skips("/assets/dist/app.js"));
        assert!(config.skips("/favicon.ico"));
        assert!(!config.skips("/dashboard"));
    }
}
