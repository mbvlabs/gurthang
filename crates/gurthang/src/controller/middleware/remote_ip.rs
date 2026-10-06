use std::net::IpAddr;

use axum::{Router, extract::Request, middleware::Next, response::Response};
use serde::{Deserialize, Serialize};

use crate::{app::Context, controller::middleware::MiddlewareLayer, error::Result};

#[derive(Clone, Debug)]
pub struct RemoteIp(pub Option<IpAddr>);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoteIpMiddleware {
    #[serde(default)]
    pub enable: bool,
}

impl MiddlewareLayer for RemoteIpMiddleware {
    fn name(&self) -> &'static str {
        "remote_ip"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app.layer(axum::middleware::from_fn(attach_remote_ip)))
    }
}

async fn attach_remote_ip(mut request: Request, next: Next) -> Response {
    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .split(',')
                .next_back()
                .and_then(|part| part.trim().parse().ok())
        });
    request.extensions_mut().insert(RemoteIp(ip));
    next.run(request).await
}
