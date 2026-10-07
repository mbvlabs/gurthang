use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer, remote_ip::RemoteIp},
    error::Result,
};

/// Configuration for the IP-based rate limiter.
///
/// The limiter uses a fixed window per client IP. It defaults to enabled with a
/// generous ceiling so the load-shedding battery is on out of the box. Omit
/// it from `Hooks::middlewares` (or `delete` it) to turn it off. The test
/// default stack omits it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_max_requests")]
    pub max_requests: u64,
    #[serde(default = "default_window_ms")]
    pub window_ms: u64,
    #[serde(default = "default_skip_paths")]
    pub skip_paths: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable: true,
            max_requests: default_max_requests(),
            window_ms: default_window_ms(),
            skip_paths: default_skip_paths(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_requests() -> u64 {
    100
}

fn default_window_ms() -> u64 {
    1_000
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

#[derive(Debug, Clone, Copy)]
struct Window {
    start: Instant,
    count: u64,
}

/// Shared, in-process fixed-window counters keyed by client IP.
#[derive(Clone, Default)]
pub struct RateLimiter {
    windows: Arc<Mutex<HashMap<IpAddr, Window>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when the request is allowed, `false` when the caller has
    /// exceeded the window.
    pub fn check(&self, ip: IpAddr, max_requests: u64, window: Duration) -> bool {
        let now = Instant::now();
        let mut windows = self.windows.lock().expect("rate limiter mutex");
        let entry = windows.entry(ip).or_insert(Window {
            start: now,
            count: 0,
        });
        if now.duration_since(entry.start) >= window {
            entry.start = now;
            entry.count = 0;
        }
        entry.count += 1;
        entry.count <= max_requests
    }
}

/// Pre-processing middleware that sheds load before the rest of the pipeline.
pub struct RateLimit {
    config: Config,
    limiter: RateLimiter,
}

pub fn new(config: &Option<Config>) -> RateLimit {
    RateLimit {
        config: config.clone().unwrap_or_default(),
        limiter: RateLimiter::new(),
    }
}

impl MiddlewareLayer for RateLimit {
    fn name(&self) -> &'static str {
        "rate_limit"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Pre
    }

    fn is_enabled(&self) -> bool {
        self.config.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(&self.config)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        let config = self.config.clone();
        let limiter = self.limiter.clone();
        Ok(app.layer(axum::middleware::from_fn(
            move |request: Request, next: Next| {
                let config = config.clone();
                let limiter = limiter.clone();
                async move { limit(config, limiter, request, next).await }
            },
        )))
    }
}

async fn limit(config: Config, limiter: RateLimiter, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let ip = request.extensions().get::<RemoteIp>().and_then(|ip| ip.0);

    if let Some(ip) = ip
        && !config.skips(path)
        && !limiter.check(
            ip,
            config.max_requests,
            Duration::from_millis(config.window_ms),
        )
    {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("retry-after", "1")],
            "Too many requests",
        )
            .into_response();
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use std::{net::IpAddr, time::Duration};

    use super::RateLimiter;

    #[test]
    fn allows_up_to_the_limit_then_rejects() {
        let limiter = RateLimiter::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let window = Duration::from_millis(1_000);
        for _ in 0..3 {
            assert!(limiter.check(ip, 3, window));
        }
        assert!(!limiter.check(ip, 3, window));
    }

    #[test]
    fn windows_are_per_ip() {
        let limiter = RateLimiter::new();
        let first: IpAddr = "127.0.0.1".parse().unwrap();
        let second: IpAddr = "127.0.0.2".parse().unwrap();
        let window = Duration::from_millis(1_000);
        assert!(limiter.check(first, 1, window));
        assert!(!limiter.check(first, 1, window));
        assert!(limiter.check(second, 1, window));
    }
}
