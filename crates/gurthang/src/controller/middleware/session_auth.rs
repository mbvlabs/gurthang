use std::marker::PhantomData;

use axum::Router;
use axum_login::{AuthManagerLayerBuilder, AuthnBackend};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer},
    error::Result,
    http::PostgresSessionStore,
};

/// Config for the session/auth middleware. Kept as `session_auth` in
/// `config.server.middlewares`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionAuth {
    #[serde(default = "default_true")]
    pub enable: bool,
}

impl Default for SessionAuth {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("empty session auth config")
    }
}

fn default_true() -> bool {
    true
}

/// Wrapping middleware that loads the session before the handler and persists
/// it after, and installs the `AuthSession<B>` extractor.
///
/// The framework cannot build this layer on its own: the backend type is owned
/// by the application. The app replaces the `session_auth` placeholder in
/// `Hooks::middlewares` with `SessionAuthLayer::new(backend, ctx)`.
pub struct SessionAuthLayer<B> {
    backend: B,
    store: PostgresSessionStore,
    cookie: String,
    secure: bool,
    enable: bool,
    marker: PhantomData<fn() -> B>,
}

impl<B> SessionAuthLayer<B>
where
    B: AuthnBackend + Clone + Send + Sync + 'static,
{
    pub fn new(backend: B, ctx: &Context) -> Self {
        let config = ctx
            .config
            .server
            .middlewares
            .session_auth
            .clone()
            .unwrap_or_default();
        Self {
            backend,
            store: PostgresSessionStore::new(ctx.db.clone()),
            cookie: ctx.config.session.cookie.clone(),
            secure: ctx.config.session.secure,
            enable: config.enable,
            marker: PhantomData,
        }
    }
}

impl<B> MiddlewareLayer for SessionAuthLayer<B>
where
    B: AuthnBackend + Clone + Send + Sync + 'static,
    B::User: Clone + Send + Sync + 'static,
    B::Credentials: Send + Sync + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        "session_auth"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Wrap
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(SessionAuth {
            enable: self.enable,
        })
    }

    fn apply(&self, router: Router<Context>) -> Result<Router<Context>> {
        let session_layer = SessionManagerLayer::new(self.store.clone())
            .with_name(self.cookie.clone())
            .with_http_only(true)
            .with_same_site(SameSite::Lax)
            .with_secure(self.secure)
            .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));
        let auth_layer = AuthManagerLayerBuilder::new(self.backend.clone(), session_layer).build();
        Ok(router.layer(auth_layer))
    }
}

/// Placeholder used by the framework default stack. It carries the enabled flag
/// so `gurthang middleware` lists it, but installs nothing; the app swaps it for
/// [`SessionAuthLayer`].
pub struct SessionAuthPlaceholder {
    enable: bool,
}

pub fn placeholder(config: &Option<SessionAuth>) -> SessionAuthPlaceholder {
    SessionAuthPlaceholder {
        enable: config.as_ref().map(|layer| layer.enable).unwrap_or(true),
    }
}

impl MiddlewareLayer for SessionAuthPlaceholder {
    fn name(&self) -> &'static str {
        "session_auth"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Wrap
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(SessionAuth {
            enable: self.enable,
        })
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app)
    }
}

/// Convenience helper kept for callers that install the session layer directly.
pub fn layer<B>(router: Router<Context>, ctx: &Context, backend: B) -> Router<Context>
where
    B: AuthnBackend + Clone + Send + Sync + 'static,
    B::User: Clone + Send + Sync + 'static,
    B::Credentials: Send + Sync + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let layer = SessionAuthLayer::new(backend, ctx);
    if !layer.enable {
        return router;
    }
    let session_layer = SessionManagerLayer::new(layer.store.clone())
        .with_name(layer.cookie.clone())
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(layer.secure)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));
    let auth_layer = AuthManagerLayerBuilder::new(layer.backend.clone(), session_layer).build();
    router.layer(auth_layer)
}
