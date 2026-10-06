use axum::Router;
use axum_login::{AuthManagerLayerBuilder, AuthnBackend};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};

use crate::{
    app::Context, controller::middleware::MiddlewareLayer, error::Result,
    http::PostgresSessionStore,
};

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

impl MiddlewareLayer for SessionAuth {
    fn name(&self) -> &'static str {
        "session_auth"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app)
    }
}

pub fn layer<B>(router: Router<Context>, ctx: &Context, backend: B) -> Router<Context>
where
    B: AuthnBackend + Clone + Send + Sync + 'static,
    B::User: Clone + Send + Sync + 'static,
    B::Credentials: Send + Sync + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let enabled = ctx
        .config
        .server
        .middlewares
        .session_auth
        .as_ref()
        .map(|layer| layer.enable)
        .unwrap_or(true);
    if !enabled {
        return router;
    }
    let session_store = PostgresSessionStore::new(ctx.db.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name(ctx.config.session.cookie.clone())
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(ctx.config.session.secure)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();
    router.layer(auth_layer)
}
