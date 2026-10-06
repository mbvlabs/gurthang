use axum::Router;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer},
    error::Result,
};

#[derive(Debug, Clone, Copy)]
pub enum BodyLimit {
    Disable,
    Limit(usize),
}

impl Serialize for BodyLimit {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Disable => serializer.serialize_str("disable"),
            Self::Limit(bytes) => serializer.serialize_u64(*bytes as u64),
        }
    }
}

impl<'de> Deserialize<'de> for BodyLimit {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = BodyLimit;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("\"disable\" or a byte limit")
            }

            fn visit_str<E: serde::de::Error>(
                self,
                value: &str,
            ) -> std::result::Result<Self::Value, E> {
                if value.eq_ignore_ascii_case("disable") {
                    return Ok(BodyLimit::Disable);
                }
                value
                    .parse()
                    .map(BodyLimit::Limit)
                    .map_err(serde::de::Error::custom)
            }

            fn visit_u64<E: serde::de::Error>(
                self,
                value: u64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(BodyLimit::Limit(
                    usize::try_from(value).map_err(serde::de::Error::custom)?,
                ))
            }

            fn visit_i64<E: serde::de::Error>(
                self,
                value: i64,
            ) -> std::result::Result<Self::Value, E> {
                if value < 0 {
                    return Err(serde::de::Error::custom("body limit must be positive"));
                }
                self.visit_u64(value as u64)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitPayload {
    #[serde(default = "default_body_limit")]
    pub body_limit: BodyLimit,
}

impl Default for LimitPayload {
    fn default() -> Self {
        Self {
            body_limit: default_body_limit(),
        }
    }
}

fn default_body_limit() -> BodyLimit {
    BodyLimit::Limit(2_000_000)
}

impl MiddlewareLayer for LimitPayload {
    fn name(&self) -> &'static str {
        "limit_payload"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Wrap
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        let layer = match self.body_limit {
            BodyLimit::Disable => axum::extract::DefaultBodyLimit::disable(),
            BodyLimit::Limit(limit) => axum::extract::DefaultBodyLimit::max(limit),
        };
        Ok(app.layer(layer))
    }
}
