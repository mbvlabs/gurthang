use std::sync::Arc;

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
    controller::middleware::{MiddlewareKind, MiddlewareLayer},
    error::Result,
};

/// Application-defined authorization decision. Returns `true` when the request
/// is allowed.
pub trait Authorizer: Send + Sync + 'static {
    fn is_authorized(&self, request: &Request) -> bool;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub enable: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { enable: false }
    }
}

/// Pre-processing middleware that rejects authenticated-but-forbidden requests
/// before the handler runs. Off by default; the app installs an [`Authorizer`].
pub struct RequireAuthz {
    config: Config,
    authorizer: Option<Arc<dyn Authorizer>>,
}

pub fn new() -> RequireAuthz {
    RequireAuthz {
        config: Config::default(),
        authorizer: None,
    }
}

/// Placeholder so the default stack can list `authz` without denying every
/// request. Replace with [`RequireAuthz`] plus an [`Authorizer`] to enforce.
pub struct AuthzPlaceholder;

pub fn placeholder() -> AuthzPlaceholder {
    AuthzPlaceholder
}

impl MiddlewareLayer for AuthzPlaceholder {
    fn name(&self) -> &'static str {
        "authz"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Pre
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(Config::default())
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app)
    }
}

impl RequireAuthz {
    pub fn with_authorizer(mut self, authorizer: Arc<dyn Authorizer>) -> Self {
        self.authorizer = Some(authorizer);
        self
    }
}

impl MiddlewareLayer for RequireAuthz {
    fn name(&self) -> &'static str {
        "authz"
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
        let authorizer = self.authorizer.clone();
        Ok(app.layer(axum::middleware::from_fn(
            move |request: Request, next: Next| {
                let config = config.clone();
                let authorizer = authorizer.clone();
                async move { authorize(config, authorizer, request, next).await }
            },
        )))
    }
}

async fn authorize(
    config: Config,
    authorizer: Option<Arc<dyn Authorizer>>,
    request: Request,
    next: Next,
) -> Response {
    let _ = &config;
    let allowed = match authorizer {
        Some(authorizer) => authorizer.is_authorized(&request),
        None => {
            tracing::error!("authz enabled without an authorizer; denying request");
            false
        }
    };

    if allowed {
        next.run(request).await
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}
