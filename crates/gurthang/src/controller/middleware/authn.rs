use std::marker::PhantomData;

use axum::{
    Router,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_login::{AuthSession, AuthnBackend};
use serde::{Deserialize, Serialize};

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer},
    error::Result,
};

/// Configuration for the authentication pre middleware.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub enable: bool,
    /// Paths that bypass the authentication check. Exact match, plus prefix
    /// match for any entry other than `/`.
    #[serde(default = "default_public_paths")]
    pub public_paths: Vec<String>,
    /// Where unauthenticated requests are redirected. When `None`, the request
    /// is rejected with `status` instead.
    #[serde(default)]
    pub redirect: Option<String>,
    #[serde(default = "default_status")]
    pub status: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable: false,
            public_paths: default_public_paths(),
            redirect: None,
            status: default_status(),
        }
    }
}

fn default_public_paths() -> Vec<String> {
    vec!["/".into(), "/login".into(), "/register".into(), "/assets".into()]
}

fn default_status() -> u16 {
    401
}

impl Config {
    pub fn is_public(&self, path: &str) -> bool {
        self.public_paths.iter().any(|prefix| {
            if prefix == "/" {
                path == "/"
            } else {
                path == prefix || path.starts_with(&format!("{prefix}/"))
            }
        })
    }
}

/// Pre-processing middleware that rejects unauthenticated requests before the
/// handler runs. Requires an `AuthSession<B>` (installed by the `session_auth`
/// wrap) to be present.
pub struct RequireAuth<B> {
    config: Config,
    marker: PhantomData<fn() -> B>,
}

impl<B> RequireAuth<B> {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            marker: PhantomData,
        }
    }
}

impl<B> MiddlewareLayer for RequireAuth<B>
where
    B: AuthnBackend + Clone + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        "authn"
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
        Ok(app.layer(axum::middleware::from_fn(
            move |auth: AuthSession<B>, request: Request, next: Next| {
                let config = config.clone();
                async move { require_auth(config, auth, request, next).await }
            },
        )))
    }
}

async fn require_auth<B>(
    config: Config,
    auth: AuthSession<B>,
    request: Request,
    next: Next,
) -> Response
where
    B: AuthnBackend + Clone + Send + Sync + 'static,
{
    if auth.user.is_some() || config.is_public(request.uri().path()) {
        return next.run(request).await;
    }

    match config.redirect.as_deref() {
        Some(location) => gurthang_inertia::mutation_redirect(location)
            .unwrap_or_else(|_| StatusCode::UNAUTHORIZED.into_response()),
        None => StatusCode::from_u16(config.status)
            .unwrap_or(StatusCode::UNAUTHORIZED)
            .into_response(),
    }
}

/// Placeholder used by the framework default stack. The application replaces it
/// with [`RequireAuth`] because the framework cannot know the auth backend type.
pub struct AuthnPlaceholder {
    enable: bool,
}

pub fn placeholder(config: &Option<Config>) -> AuthnPlaceholder {
    AuthnPlaceholder {
        enable: config.as_ref().map(|config| config.enable).unwrap_or(false),
    }
}

impl MiddlewareLayer for AuthnPlaceholder {
    fn name(&self) -> &'static str {
        "authn"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Pre
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(Config {
            enable: self.enable,
            ..Default::default()
        })
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app)
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn root_is_exact_match_only() {
        let config = Config::default();
        assert!(config.is_public("/"));
        assert!(!config.is_public("/dashboard"));
    }

    #[test]
    fn prefixes_are_matched() {
        let config = Config::default();
        assert!(config.is_public("/assets/dist/app.js"));
        assert!(config.is_public("/login"));
        assert!(!config.is_public("/logout"));
    }
}
