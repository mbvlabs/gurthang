pub mod catch_panic;
pub mod compression;
pub mod cors;
pub mod csrf;
pub mod etag;
pub mod fallback;
pub mod limit_payload;
pub mod logger;
pub mod powered_by;
pub mod remote_ip;
pub mod request_id;
pub mod secure_headers;
pub mod session_auth;
pub mod static_assets;
pub mod timeout;

use std::io::Write;

use axum::Router;
use serde::{Deserialize, Serialize};

use crate::{
    app::Context,
    config::{Config as AppConfig, Environment},
    error::{Error, Result},
};

pub trait MiddlewareLayer: Send + Sync {
    fn name(&self) -> &'static str;

    fn is_enabled(&self) -> bool {
        true
    }

    fn config(&self) -> serde_json::Result<serde_json::Value>;

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>>;
}

pub fn default_middleware_stack(ctx: &Context) -> Vec<Box<dyn MiddlewareLayer>> {
    stack_from_config(ctx.config.as_ref())
}

pub fn stack_from_config(config: &AppConfig) -> Vec<Box<dyn MiddlewareLayer>> {
    let middlewares = &config.server.middlewares;
    let mut static_assets = middlewares.static_assets.clone().unwrap_or_default();
    static_assets.development = config.environment.is_development();
    let mut csrf = middlewares.csrf.clone().unwrap_or_default();
    csrf.secure = config.session.secure;
    vec![
        Box::new(middlewares.limit_payload.clone().unwrap_or_default()),
        Box::new(middlewares.cors.clone().unwrap_or_else(|| cors::Cors {
            enable: false,
            ..Default::default()
        })),
        Box::new(
            middlewares
                .catch_panic
                .clone()
                .unwrap_or(catch_panic::CatchPanic { enable: true }),
        ),
        Box::new(
            middlewares
                .etag
                .clone()
                .unwrap_or(etag::Etag { enable: true }),
        ),
        Box::new(
            middlewares
                .remote_ip
                .clone()
                .unwrap_or(remote_ip::RemoteIpMiddleware { enable: false }),
        ),
        Box::new(
            middlewares
                .compression
                .clone()
                .unwrap_or(compression::Compression { enable: false }),
        ),
        Box::new(
            middlewares
                .timeout_request
                .clone()
                .unwrap_or_else(|| timeout::TimeOut {
                    enable: false,
                    ..Default::default()
                }),
        ),
        Box::new(static_assets),
        Box::new(middlewares.secure_headers.clone().unwrap_or_else(|| {
            secure_headers::SecureHeader {
                enable: false,
                ..Default::default()
            }
        })),
        Box::new(logger::new(
            &middlewares
                .logger
                .clone()
                .unwrap_or(logger::Config { enable: true }),
            &config.environment,
        )),
        Box::new(
            middlewares
                .request_id
                .clone()
                .unwrap_or(request_id::RequestId { enable: true }),
        ),
        Box::new(
            middlewares
                .fallback
                .clone()
                .unwrap_or_else(|| fallback::Fallback {
                    enable: config.environment != Environment::Production,
                    ..Default::default()
                }),
        ),
        Box::new(powered_by::new(config.server.ident.as_deref())),
        Box::new(csrf),
        Box::new(middlewares.session_auth.clone().unwrap_or_default()),
    ]
}

pub fn apply_stack(
    router: Router<Context>,
    stack: Vec<Box<dyn MiddlewareLayer>>,
) -> Result<Router<Context>> {
    let mut router = router;
    for layer in stack {
        if layer.is_enabled() {
            router = layer.apply(router)?;
        }
    }
    Ok(router)
}

pub fn print_stack(stack: &[Box<dyn MiddlewareLayer>], out: &mut impl Write) -> Result<()> {
    for layer in stack {
        let status = if layer.is_enabled() {
            "enabled"
        } else {
            "disabled"
        };
        writeln!(out, "{:<22} {}", layer.name(), status).map_err(Error::from)?;
    }
    Ok(())
}

pub trait MiddlewareStackExt {
    fn insert_before(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self;
    fn insert_after(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self;
    fn replace(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self;
    fn delete(&mut self, name: &str) -> &mut Self;
}

impl MiddlewareStackExt for Vec<Box<dyn MiddlewareLayer>> {
    fn insert_before(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self {
        if let Some(idx) = self.iter().position(|layer| layer.name() == name) {
            self.insert(idx, middleware);
        } else {
            tracing::warn!(
                middleware = name,
                "insert_before: no middleware named `{name}`; appending"
            );
            self.push(middleware);
        }
        self
    }

    fn insert_after(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self {
        if let Some(idx) = self.iter().position(|layer| layer.name() == name) {
            self.insert(idx + 1, middleware);
        } else {
            tracing::warn!(
                middleware = name,
                "insert_after: no middleware named `{name}`; appending"
            );
            self.push(middleware);
        }
        self
    }

    fn replace(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self {
        if let Some(idx) = self.iter().position(|layer| layer.name() == name) {
            self[idx] = middleware;
        } else {
            tracing::warn!(
                middleware = name,
                "replace: no middleware named `{name}`; leaving stack unchanged"
            );
        }
        self
    }

    fn delete(&mut self, name: &str) -> &mut Self {
        if let Some(idx) = self.iter().position(|layer| layer.name() == name) {
            self.remove(idx);
        } else {
            tracing::warn!(
                middleware = name,
                "delete: no middleware named `{name}`; leaving stack unchanged"
            );
        }
        self
    }
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
}

#[cfg(test)]
mod tests {
    use super::{MiddlewareLayer, MiddlewareStackExt, stack_from_config};
    use crate::config::Config;

    struct Dummy(&'static str);

    impl MiddlewareLayer for Dummy {
        fn name(&self) -> &'static str {
            self.0
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

    fn names(stack: &[Box<dyn MiddlewareLayer>]) -> Vec<&'static str> {
        stack.iter().map(|layer| layer.name()).collect()
    }

    #[test]
    fn stack_ext_inserts_replaces_and_deletes() {
        let mut stack: Vec<Box<dyn MiddlewareLayer>> = ["a", "b", "c"]
            .into_iter()
            .map(|name| Box::new(Dummy(name)) as Box<dyn MiddlewareLayer>)
            .collect();
        stack
            .insert_before("a", Box::new(Dummy("x")))
            .insert_after("c", Box::new(Dummy("y")))
            .replace("b", Box::new(Dummy("z")))
            .delete("a");
        assert_eq!(names(&stack), vec!["x", "z", "c", "y"]);
    }

    #[test]
    fn default_stack_order_puts_csrf_before_session_auth() {
        let yaml = r#"
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
        let config = Config::from_yaml(yaml).unwrap();
        let stack = stack_from_config(&config);
        let names = names(&stack);
        let csrf = names.iter().position(|name| *name == "csrf").unwrap();
        let session = names
            .iter()
            .position(|name| *name == "session_auth")
            .unwrap();
        assert!(csrf < session, "session_auth must wrap CSRF: {names:?}");
        assert!(
            stack
                .iter()
                .find(|layer| layer.name() == "csrf")
                .unwrap()
                .is_enabled()
        );
        assert!(
            stack
                .iter()
                .find(|layer| layer.name() == "session_auth")
                .unwrap()
                .is_enabled()
        );
        assert!(
            stack
                .iter()
                .find(|layer| layer.name() == "logger")
                .unwrap()
                .is_enabled()
        );
        assert!(
            !stack
                .iter()
                .find(|layer| layer.name() == "cors")
                .unwrap()
                .is_enabled()
        );
    }
}
