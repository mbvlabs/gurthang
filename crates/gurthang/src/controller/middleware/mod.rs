//! Pavex-style middleware for Gurthang.
//!
//! Middleware is grouped into three kinds, matching Pavex's mental model:
//!
//! | Kind   | Starts           | Completes        | Typical use |
//! |--------|------------------|------------------|-------------|
//! | `pre`  | before handler   | before handler   | early response, reject unauthenticated requests |
//! | `post` | after handler    | after handler    | logging, headers, metrics |
//! | `wrap` | before handler   | after handler    | timeouts, tracing spans, sessions |
//!
//! Registration order only matters *within* a kind. The pipeline is always
//! composed as `wrap -> post -> pre -> handler`, so pre-processing middleware
//! can short-circuit the handler and post-processing middleware still observe
//! the final response (including an early return from a pre middleware). See
//! `docs/middleware.md` for the full ordering rules and the mapping to Pavex.

pub mod authn;
pub mod authz;
pub mod catch_panic;
pub mod compression;
pub mod cors;
pub mod csrf;
pub mod etag;
pub mod fallback;
pub mod limit_payload;
pub mod logger;
pub mod metrics;
pub mod powered_by;
pub mod rate_limit;
pub mod remote_ip;
pub mod request_id;
pub mod secure_headers;
pub mod session_auth;
pub mod static_assets;
pub mod telemetry;
pub mod timeout;

use std::{
    collections::HashSet,
    io::Write,
};

use axum::Router;
use serde::{Deserialize, Serialize};

use crate::{
    app::Context,
    config::{Config as AppConfig, Environment},
    error::{Error, Result},
};

/// The three first-class middleware kinds.
///
/// The composition rule is fixed by kind, so registration order across kinds
/// never silently mis-wires the pipeline: `wrap` is outermost, then `post`,
/// then `pre`, then the handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiddlewareKind {
    Pre,
    Post,
    Wrap,
}

impl MiddlewareKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pre => "pre",
            Self::Post => "post",
            Self::Wrap => "wrap",
        }
    }
}

impl std::fmt::Display for MiddlewareKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A single middleware in the stack.
///
/// `apply` is the escape hatch onto Axum/Tower: it receives the router and
/// returns a router with the layer installed. `kind` decides where the layer
/// sits in the composed pipeline.
pub trait MiddlewareLayer: Send + Sync {
    fn name(&self) -> &'static str;

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Wrap
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn config(&self) -> serde_json::Result<serde_json::Value>;

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>>;
}

/// An ordered, kind-aware collection of middleware.
///
/// Layers are stored in registration order. `composed` folds them into the
/// concrete pipeline order, and `validate` rejects mis-wiring (duplicate names
/// and operations against unknown names) before the app starts serving.
#[derive(Default)]
pub struct MiddlewareStack {
    layers: Vec<Box<dyn MiddlewareLayer>>,
    errors: Vec<String>,
}

impl MiddlewareStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, layer: Box<dyn MiddlewareLayer>) -> &mut Self {
        self.layers.push(layer);
        self
    }

    #[must_use]
    pub fn with(mut self, layer: Box<dyn MiddlewareLayer>) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn get(&self, name: &str) -> Option<&dyn MiddlewareLayer> {
        self.layers
            .iter()
            .find(|layer| layer.name() == name)
            .map(Box::as_ref)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.layers.iter().any(|layer| layer.name() == name)
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.layers.iter().map(|layer| layer.name()).collect()
    }

    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    pub fn insert_before(&mut self, name: &str, layer: Box<dyn MiddlewareLayer>) -> &mut Self {
        match self.layers.iter().position(|entry| entry.name() == name) {
            Some(index) => {
                self.layers.insert(index, layer);
            }
            None => self
                .errors
                .push(format!("insert_before: no middleware named `{name}`")),
        }
        self
    }

    pub fn insert_after(&mut self, name: &str, layer: Box<dyn MiddlewareLayer>) -> &mut Self {
        match self.layers.iter().position(|entry| entry.name() == name) {
            Some(index) => {
                self.layers.insert(index + 1, layer);
            }
            None => self
                .errors
                .push(format!("insert_after: no middleware named `{name}`")),
        }
        self
    }

    pub fn replace(&mut self, name: &str, layer: Box<dyn MiddlewareLayer>) -> &mut Self {
        match self.layers.iter().position(|entry| entry.name() == name) {
            Some(index) => {
                self.layers[index] = layer;
            }
            None => self
                .errors
                .push(format!("replace: no middleware named `{name}`")),
        }
        self
    }

    pub fn delete(&mut self, name: &str) -> &mut Self {
        match self.layers.iter().position(|entry| entry.name() == name) {
            Some(index) => {
                self.layers.remove(index);
            }
            None => self
                .errors
                .push(format!("delete: no middleware named `{name}`")),
        }
        self
    }

    /// The concrete pipeline order, outermost first.
    ///
    /// `wrap` layers keep registration order, `post` layers are nested so their
    /// response handling completes in registration order, and `pre` layers keep
    /// registration order. Everything is applied outside the handler.
    pub fn composed(&self) -> Vec<&dyn MiddlewareLayer> {
        let mut ordered: Vec<&dyn MiddlewareLayer> = Vec::with_capacity(self.layers.len());
        ordered.extend(
            self.layers
                .iter()
                .filter(|layer| layer.kind() == MiddlewareKind::Wrap)
                .map(Box::as_ref),
        );
        ordered.extend(
            self.layers
                .iter()
                .filter(|layer| layer.kind() == MiddlewareKind::Post)
                .rev()
                .map(Box::as_ref),
        );
        ordered.extend(
            self.layers
                .iter()
                .filter(|layer| layer.kind() == MiddlewareKind::Pre)
                .map(Box::as_ref),
        );
        ordered
    }

    /// Startup-time validation. Prefer failing fast over silently mis-wiring.
    pub fn validate(&self) -> Result<()> {
        let mut seen = HashSet::new();
        for layer in &self.layers {
            if !seen.insert(layer.name()) {
                return Err(Error::Message(format!(
                    "duplicate middleware named `{}`",
                    layer.name()
                )));
            }
        }
        if let Some(error) = self.errors.first() {
            return Err(Error::Message(error.clone()));
        }
        Ok(())
    }
}

pub fn default_middleware_stack(ctx: &Context) -> MiddlewareStack {
    stack_from_config(ctx.config.as_ref())
}

pub fn stack_from_config(config: &AppConfig) -> MiddlewareStack {
    let middlewares = &config.server.middlewares;
    let mut static_assets = middlewares.static_assets.clone().unwrap_or_default();
    static_assets.development = config.environment.is_development();
    let mut csrf = middlewares.csrf.clone().unwrap_or_default();
    csrf.secure = config.session.secure;

    let mut stack = MiddlewareStack::new();

    // wrap: outermost first
    stack
        .push(Box::new(
            middlewares
                .catch_panic
                .clone()
                .unwrap_or(catch_panic::CatchPanic { enable: true }),
        ))
        .push(Box::new(session_auth::placeholder(
            &middlewares.session_auth,
        )))
        .push(Box::new(csrf))
        .push(Box::new(
            middlewares
                .request_id
                .clone()
                .unwrap_or(request_id::RequestId { enable: true }),
        ))
        .push(Box::new(telemetry::new(&middlewares.telemetry)))
        .push(Box::new(
            middlewares
                .timeout_request
                .clone()
                .unwrap_or_else(|| timeout::TimeOut {
                    enable: true,
                    ..Default::default()
                }),
        ))
        .push(Box::new(middlewares.cors.clone().unwrap_or_else(|| cors::Cors {
            enable: false,
            ..Default::default()
        })))
        .push(Box::new(
            middlewares.limit_payload.clone().unwrap_or_default(),
        ))
        .push(Box::new(static_assets))
        .push(Box::new(
            middlewares
                .fallback
                .clone()
                .unwrap_or_else(|| fallback::Fallback {
                    enable: config.environment != Environment::Production,
                    ..Default::default()
                }),
        ));

    // post
    stack
        .push(Box::new(
            middlewares
                .compression
                .clone()
                .unwrap_or(compression::Compression { enable: false }),
        ))
        .push(Box::new(
            middlewares
                .etag
                .clone()
                .unwrap_or(etag::Etag { enable: true }),
        ))
        .push(Box::new(powered_by::new(config.server.ident.as_deref())))
        .push(Box::new(middlewares.secure_headers.clone().unwrap_or_else(
            || secure_headers::SecureHeader {
                enable: false,
                ..Default::default()
            },
        )))
        .push(Box::new(metrics::new(&middlewares.metrics)))
        .push(Box::new(logger::new(
            &middlewares.logger.clone().unwrap_or_default(),
        )));

    // pre
    stack
        .push(Box::new(
            middlewares
                .remote_ip
                .clone()
                .unwrap_or(remote_ip::RemoteIpMiddleware { enable: false }),
        ))
        .push(Box::new(rate_limit::new(&middlewares.rate_limit)))
        .push(Box::new(authn::placeholder(&middlewares.authn)))
        .push(Box::new(authz::new(&middlewares.authz)));

    stack
}

pub fn apply_stack(router: Router<Context>, stack: MiddlewareStack) -> Result<Router<Context>> {
    stack.validate()?;
    let mut router = router;
    for layer in stack.composed().into_iter().rev() {
        if layer.is_enabled() {
            router = layer.apply(router)?;
        }
    }
    Ok(router)
}

pub fn print_stack(stack: &MiddlewareStack, out: &mut impl Write) -> Result<()> {
    writeln!(out, "{:<6} {:<22} {}", "kind", "name", "status").map_err(Error::from)?;
    for layer in stack.composed() {
        let status = if layer.is_enabled() {
            "enabled"
        } else {
            "disabled"
        };
        writeln!(out, "{:<6} {:<22} {}", layer.kind(), layer.name(), status).map_err(Error::from)?;
    }
    Ok(())
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct MiddlewareConfig {
    pub compression: Option<compression::Compression>,
    pub etag: Option<etag::Etag>,
    pub limit_payload: Option<limit_payload::LimitPayload>,
    pub logger: Option<logger::Config>,
    pub catch_panic: Option<catch_panic::CatchPanic>,
    pub timeout_request: Option<timeout::TimeOut>,
    pub cors: Option<cors::Cors>,
    #[serde(rename = "static")]
    pub static_assets: Option<static_assets::StaticAssets>,
    pub secure_headers: Option<secure_headers::SecureHeader>,
    pub remote_ip: Option<remote_ip::RemoteIpMiddleware>,
    pub fallback: Option<fallback::Fallback>,
    pub request_id: Option<request_id::RequestId>,
    pub csrf: Option<csrf::Csrf>,
    pub session_auth: Option<session_auth::SessionAuth>,
    pub telemetry: Option<telemetry::Config>,
    pub metrics: Option<metrics::Config>,
    pub rate_limit: Option<rate_limit::Config>,
    pub authn: Option<authn::Config>,
    pub authz: Option<authz::Config>,
}

#[cfg(test)]
mod tests {
    use super::{MiddlewareKind, MiddlewareLayer, MiddlewareStack, stack_from_config};
    use crate::config::Config;

    struct Dummy(&'static str, MiddlewareKind);

    impl MiddlewareLayer for Dummy {
        fn name(&self) -> &'static str {
            self.0
        }

        fn kind(&self) -> MiddlewareKind {
            self.1
        }

        fn config(&self) -> serde_json::Result<serde_json::Value> {
            Ok(serde_json::Value::Null)
        }

        fn apply(
            &self,
            app: axum::Router<crate::app::Context>,
        ) -> crate::error::Result<axum::Router<crate::app::Context>> {
            Ok(app)
        }
    }

    fn dummy(name: &'static str, kind: MiddlewareKind) -> Box<dyn MiddlewareLayer> {
        Box::new(Dummy(name, kind))
    }

    fn names(stack: &MiddlewareStack) -> Vec<&'static str> {
        stack.composed().iter().map(|layer| layer.name()).collect()
    }

    const SAMPLE: &str = r#"
server:
  host: 127.0.0.1
  port: 3000
  url: http://127.0.0.1:3000
session:
  secure: false
inertia:
  ssr_runtime: node
  ssr_timeout_ms: 5000
workers:
  concurrency: 1
  poll_interval_ms: 1000
  lease_seconds: 10
  timeout_seconds: 5
logger:
  level: info
"#;

    #[test]
    fn stack_ext_inserts_replaces_and_deletes() {
        let mut stack = MiddlewareStack::new();
        for name in ["a", "b", "c"] {
            stack.push(dummy(name, MiddlewareKind::Wrap));
        }
        stack
            .insert_before("a", dummy("x", MiddlewareKind::Wrap))
            .insert_after("c", dummy("y", MiddlewareKind::Wrap))
            .replace("b", dummy("z", MiddlewareKind::Wrap))
            .delete("a");
        assert_eq!(names(&stack), vec!["x", "z", "c", "y"]);
    }

    #[test]
    fn unknown_names_fail_validation() {
        let mut stack = MiddlewareStack::new();
        stack.push(dummy("a", MiddlewareKind::Wrap));
        stack.delete("missing");
        assert!(stack.validate().is_err());
    }

    #[test]
    fn duplicate_names_fail_validation() {
        let mut stack = MiddlewareStack::new();
        stack.push(dummy("a", MiddlewareKind::Wrap));
        stack.push(dummy("a", MiddlewareKind::Wrap));
        assert!(stack.validate().is_err());
    }

    #[test]
    fn composition_groups_kinds_and_reverses_posts() {
        let mut stack = MiddlewareStack::new();
        stack
            .push(dummy("pre_1", MiddlewareKind::Pre))
            .push(dummy("post_1", MiddlewareKind::Post))
            .push(dummy("wrap_1", MiddlewareKind::Wrap))
            .push(dummy("pre_2", MiddlewareKind::Pre))
            .push(dummy("post_2", MiddlewareKind::Post))
            .push(dummy("wrap_2", MiddlewareKind::Wrap));
        assert_eq!(
            names(&stack),
            vec![
                "wrap_1", "wrap_2", "post_2", "post_1", "pre_1", "pre_2"
            ]
        );
    }

    #[test]
    fn default_stack_orders_kinds_and_keeps_session_outermost() {
        let config = Config::from_yaml(SAMPLE).unwrap();
        let stack = stack_from_config(&config);
        let names = names(&stack);
        let session = names
            .iter()
            .position(|name| *name == "session_auth")
            .unwrap();
        let csrf = names.iter().position(|name| *name == "csrf").unwrap();
        let logger = names.iter().position(|name| *name == "logger").unwrap();
        let request_id = names
            .iter()
            .position(|name| *name == "request_id")
            .unwrap();
        let rate_limit = names
            .iter()
            .position(|name| *name == "rate_limit")
            .unwrap();

        // session_auth wraps CSRF, which wraps the request id.
        assert!(session < csrf, "session must wrap csrf: {names:?}");
        assert!(csrf < request_id, "csrf must wrap request_id: {names:?}");
        // request_id is a wrap, so it runs before the post logger.
        assert!(request_id < logger, "request_id must wrap logger: {names:?}");
        // pre middleware runs closest to the handler.
        assert!(logger < rate_limit, "pre must run after post: {names:?}");
    }

    #[test]
    fn default_stack_enables_telemetry_and_timeout() {
        let config = Config::from_yaml(SAMPLE).unwrap();
        let stack = stack_from_config(&config);
        for name in ["telemetry", "timeout_request", "logger", "metrics"] {
            let layer = stack.get(name).unwrap();
            assert!(layer.is_enabled(), "{name} should be enabled by default");
        }
    }

    #[test]
    fn print_stack_lists_kind_name_and_status() {
        let config = Config::from_yaml(SAMPLE).unwrap();
        let stack = stack_from_config(&config);
        let mut out = Vec::new();
        super::print_stack(&stack, &mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("kind"));
        assert!(out.contains("wrap"));
        assert!(out.contains("post"));
        assert!(out.contains("pre"));
        assert!(out.contains("session_auth"));
        assert!(out.contains("enabled"));
    }
}
