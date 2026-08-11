use axum::{
    Router, middleware,
    routing::{delete, get},
};
use axum_login::AuthManagerLayerBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};
use tower_sessions_sqlx_store::PostgresStore;

use crate::{
    app::AppState,
    controllers::{auth, dashboard, pages},
    services::auth::AuthBackend,
    web::{assets, csrf, datastar},
};

pub fn router(state: AppState) -> Router {
    let is_development = state.config.is_development();
    let session_store = PostgresStore::new(state.database.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name("gurthang.sid")
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(state.config.session_secure)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));
    let auth_layer =
        AuthManagerLayerBuilder::new(AuthBackend::new(state.database.clone()), session_layer)
            .build();

    let app = Router::new()
        .route("/", get(pages::home))
        .route("/register", get(auth::new_register).post(auth::register))
        .route("/login", get(auth::new_login).post(auth::login))
        .route("/logout", delete(auth::logout))
        .route("/dashboard", get(dashboard::show))
        .route("/demo/counter", get(datastar::counter));
    let app = if is_development {
        app.nest_service("/assets", ServeDir::new("assets"))
            .nest_service("/build", ServeDir::new("dist"))
    } else {
        app.route("/assets/{*path}", get(assets::serve_public))
            .route("/build/{*path}", get(assets::serve_build))
    };
    app.layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(state.clone(), csrf::protect))
        .layer(auth_layer)
        .with_state(state)
}
