// gurthang:generated:modules:start
pub mod auth;
pub mod dashboard;
pub mod welcome;
// gurthang:generated:modules:end

use axum::{Router, middleware, routing::get};
use axum_login::AuthManagerLayerBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};
use tower_sessions_sqlx_store::PostgresStore;

use crate::{app::AppState, services::auth::AuthBackend, web::assets};

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

    let router = Router::new();
    // gurthang:generated:mounts:start
    let router = welcome::mount(router);
    let router = auth::mount(router);
    let router = dashboard::mount(router);
    // gurthang:generated:mounts:end
    let app = if is_development {
        router
            .nest_service("/assets", ServeDir::new("assets"))
            .nest_service("/build", ServeDir::new("dist"))
    } else {
        router
            .route("/assets/{*path}", get(assets::serve_public))
            .route("/build/{*path}", get(assets::serve_build))
    };
    app.layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            state.config.session_secure,
            gurthang_http::protect,
        ))
        .layer(auth_layer)
        .with_state(state)
}
