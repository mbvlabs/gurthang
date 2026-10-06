use axum::{Router, extract::Request, http::HeaderValue, middleware::Next, response::Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer},
    error::Result,
};

const X_REQUEST_ID: &str = "x-request-id";

#[derive(Clone, Debug)]
pub struct RequestIdValue(pub String);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestId {
    #[serde(default)]
    pub enable: bool,
}

impl MiddlewareLayer for RequestId {
    fn name(&self) -> &'static str {
        "request_id"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Wrap
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app.layer(axum::middleware::from_fn(attach_request_id)))
    }
}

async fn attach_request_id(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(X_REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .map(sanitize)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    request
        .extensions_mut()
        .insert(RequestIdValue(request_id.clone()));
    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(X_REQUEST_ID, value);
    }
    response
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '@'))
        .take(255)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn strips_disallowed_request_id_characters() {
        assert_eq!(sanitize("foo-bar=baz"), "foo-barbaz");
        assert!(sanitize("==========").is_empty());
    }
}
