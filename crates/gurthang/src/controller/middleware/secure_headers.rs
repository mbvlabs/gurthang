use std::{
    collections::{BTreeMap, HashMap},
    sync::OnceLock,
};

use axum::{
    Router,
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer},
    error::{Error, Result},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecureHeader {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_preset")]
    pub preset: String,
    #[serde(default)]
    pub overrides: Option<BTreeMap<String, String>>,
}

impl Default for SecureHeader {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("empty secure header config")
    }
}

fn default_preset() -> String {
    "github".into()
}

fn presets() -> &'static HashMap<String, BTreeMap<String, String>> {
    static PRESETS: OnceLock<HashMap<String, BTreeMap<String, String>>> = OnceLock::new();
    PRESETS.get_or_init(|| {
        serde_json::from_str(include_str!("secure_headers.json")).expect("secure header presets")
    })
}

impl SecureHeader {
    fn headers(&self) -> Result<Vec<(HeaderName, HeaderValue)>> {
        let mut values = presets().get(&self.preset).cloned().ok_or_else(|| {
            Error::Message(format!("unknown secure header preset {}", self.preset))
        })?;
        if let Some(overrides) = &self.overrides {
            for (name, value) in overrides {
                if value.is_empty() {
                    values.remove(name);
                } else {
                    values.insert(name.clone(), value.clone());
                }
            }
        }
        let mut headers = Vec::new();
        for (name, value) in values {
            headers.push((
                HeaderName::from_bytes(name.as_bytes())
                    .map_err(|error| Error::Message(format!("secure header {name}: {error}")))?,
                HeaderValue::from_str(&value)
                    .map_err(|error| Error::Message(format!("secure header {name}: {error}")))?,
            ));
        }
        Ok(headers)
    }
}

impl MiddlewareLayer for SecureHeader {
    fn name(&self) -> &'static str {
        "secure_headers"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Post
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app.layer(axum::middleware::from_fn_with_state(
            self.headers()?,
            attach_secure_headers,
        )))
    }
}

async fn attach_secure_headers(
    axum::extract::State(headers): axum::extract::State<Vec<(HeaderName, HeaderValue)>>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in headers {
        response.headers_mut().insert(name, value);
    }
    response
}
