mod generated;
pub use generated::*;

use axum::{Router, middleware, routing::get};
use axum_login::AuthManagerLayerBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};
use tower_sessions_sqlx_store::PostgresStore;

use crate::{app::App, services::auth::AuthBackend, web::assets};

pub fn router(app: App) -> Router {
    let is_development = app.config.is_development();
    let session_store = PostgresStore::new(app.database.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name("gurthang.sid")
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(app.config.session_secure)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));
    let auth_layer =
        AuthManagerLayerBuilder::new(AuthBackend::new(app.database.clone()), session_layer).build();

    let router = crate::controllers::register(Router::<App>::new());
    let router = if is_development {
        router
            .nest_service("/assets", ServeDir::new("assets"))
            .nest_service("/build", ServeDir::new("dist"))
    } else {
        router
            .route("/assets/{*path}", get(assets::serve_public))
            .route("/build/{*path}", get(assets::serve_build))
    };
    router
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            app.config.session_secure,
            gurthang_http::protect,
        ))
        .layer(auth_layer)
        .with_state(app)
}
