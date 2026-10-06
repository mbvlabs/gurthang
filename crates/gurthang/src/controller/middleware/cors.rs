use std::time::Duration;

use axum::Router;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::cors::{self, Any};

use crate::{
    app::Context,
    controller::middleware::MiddlewareLayer,
    error::{Error, Result},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cors {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_allow_origins")]
    pub allow_origins: Vec<String>,
    #[serde(default = "default_allow_headers")]
    pub allow_headers: Vec<String>,
    #[serde(default = "default_allow_methods")]
    pub allow_methods: Vec<String>,
    #[serde(default = "default_expose_headers")]
    pub expose_headers: Vec<String>,
    #[serde(default)]
    pub allow_credentials: bool,
    pub max_age: Option<u64>,
    #[serde(default = "default_vary_headers")]
    pub vary: Vec<String>,
}

fn default_allow_origins() -> Vec<String> {
    vec!["*".into()]
}

fn default_allow_headers() -> Vec<String> {
    vec!["*".into()]
}

fn default_allow_methods() -> Vec<String> {
    vec!["*".into()]
}

fn default_expose_headers() -> Vec<String> {
    Vec::new()
}

fn default_vary_headers() -> Vec<String> {
    vec![
        "origin".into(),
        "access-control-request-method".into(),
        "access-control-request-headers".into(),
    ]
}

impl Default for Cors {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("empty CORS config")
    }
}

impl Cors {
    fn layer(&self) -> Result<cors::CorsLayer> {
        let mut cors = cors::CorsLayer::new();
        if self.allow_origins == default_allow_origins() {
            cors = cors.allow_origin(Any);
        } else {
            let mut origins = Vec::new();
            for origin in &self.allow_origins {
                origins.push(parse_header(origin)?);
            }
            if !origins.is_empty() {
                cors = cors.allow_origin(origins);
            }
        }
        if self.allow_headers == default_allow_headers() {
            cors = cors.allow_headers(Any);
        } else {
            let mut headers = Vec::new();
            for header in &self.allow_headers {
                headers.push(parse_name(header)?);
            }
            if !headers.is_empty() {
                cors = cors.allow_headers(headers);
            }
        }
        if self.allow_methods == default_allow_methods() {
            cors = cors.allow_methods(Any);
        } else {
            let mut methods = Vec::new();
            for method in &self.allow_methods {
                methods.push(
                    method
                        .parse()
                        .map_err(|error| Error::Message(format!("CORS method: {error}")))?,
                );
            }
            if !methods.is_empty() {
                cors = cors.allow_methods(methods);
            }
        }
        if self.expose_headers != default_expose_headers() {
            let mut headers = Vec::new();
            for header in &self.expose_headers {
                headers.push(parse_name(header)?);
            }
            if !headers.is_empty() {
                cors = cors.expose_headers(headers);
            }
        }
        let mut vary = Vec::new();
        for header in &self.vary {
            vary.push(parse_name(header)?);
        }
        if !vary.is_empty() {
            cors = cors.vary(vary);
        }
        if let Some(max_age) = self.max_age {
            cors = cors.max_age(Duration::from_secs(max_age));
        }
        if self.allow_credentials {
            let wildcard = |values: &[String]| values.iter().any(|value| value == "*");
            if wildcard(&self.allow_origins)
                || wildcard(&self.allow_headers)
                || wildcard(&self.allow_methods)
                || wildcard(&self.expose_headers)
            {
                return Err(Error::Message(
                    "CORS allow_credentials cannot be combined with a wildcard".into(),
                ));
            }
        }
        Ok(cors.allow_credentials(self.allow_credentials))
    }
}

fn parse_header(value: &str) -> Result<axum::http::HeaderValue> {
    value
        .parse()
        .map_err(|error| Error::Message(format!("CORS header {value:?}: {error}")))
}

fn parse_name(value: &str) -> Result<axum::http::HeaderName> {
    value
        .parse()
        .map_err(|error| Error::Message(format!("CORS header name {value:?}: {error}")))
}

impl MiddlewareLayer for Cors {
    fn name(&self) -> &'static str {
        "cors"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app.layer(self.layer()?))
    }
}
