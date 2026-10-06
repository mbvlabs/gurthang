use axum::{
    Router,
    http::header::{HeaderName, HeaderValue},
};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::{app::Context, controller::middleware::MiddlewareLayer, error::Result};

pub struct PoweredBy {
    ident: Option<HeaderValue>,
}

pub fn new(ident: Option<&str>) -> PoweredBy {
    let ident = match ident {
        None => Some(HeaderValue::from_static("gurthang")),
        Some("") => None,
        Some(ident) => Some(
            HeaderValue::from_str(ident).unwrap_or_else(|_| HeaderValue::from_static("gurthang")),
        ),
    };
    PoweredBy { ident }
}

impl MiddlewareLayer for PoweredBy {
    fn name(&self) -> &'static str {
        "powered_by"
    }

    fn is_enabled(&self) -> bool {
        self.ident.is_some()
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        Ok(match &self.ident {
            Some(ident) => serde_json::json!({"ident": ident.to_str().unwrap_or_default()}),
            None => serde_json::json!({}),
        })
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        let ident = self
            .ident
            .clone()
            .unwrap_or_else(|| HeaderValue::from_static("gurthang"));
        Ok(app.layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-powered-by"),
            ident,
        )))
    }
}
