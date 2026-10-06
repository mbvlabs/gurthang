use axum::{Router, http::StatusCode, response::Html};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::json;
use tower_http::services::ServeFile;

use crate::{app::Context, controller::middleware::MiddlewareLayer, error::Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Fallback {
    #[serde(default)]
    pub enable: bool,
    #[serde(
        default = "default_status_code",
        serialize_with = "serialize_status_code",
        deserialize_with = "deserialize_status_code"
    )]
    pub code: StatusCode,
    pub file: Option<String>,
    pub not_found: Option<String>,
}

impl Default for Fallback {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("empty fallback config")
    }
}

fn default_status_code() -> StatusCode {
    StatusCode::NOT_FOUND
}

fn deserialize_status_code<'de, D>(deserializer: D) -> std::result::Result<StatusCode, D::Error>
where
    D: Deserializer<'de>,
{
    let code = u16::deserialize(deserializer)?;
    StatusCode::from_u16(code).map_err(|_| {
        serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(u64::from(code)),
            &"an HTTP status code",
        )
    })
}

fn serialize_status_code<S>(
    status: &StatusCode,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u16(status.as_u16())
}

impl MiddlewareLayer for Fallback {
    fn name(&self) -> &'static str {
        "fallback"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        if let Some(path) = &self.file {
            return Ok(app.fallback_service(ServeFile::new(path)));
        }
        if let Some(message) = &self.not_found {
            let message = message.clone();
            let status = self.code;
            return Ok(app.fallback(move || async move { (status, message) }));
        }
        let content = include_str!("fallback.html");
        let status = self.code;
        Ok(app.fallback(move || async move { (status, Html(content)) }))
    }
}
