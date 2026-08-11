use std::sync::Arc;

use __GURTHANG_CRATE_NAME__::{
    app::AppState,
    config::Config,
    routes,
    web::{
        assets::AssetResolver, development::DevelopmentWatcher, inertia::InertiaRenderer,
        inertia::InertiaSsr, tera::TeraEngine,
    },
};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_livereload::LiveReloadLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Arc::new(Config::from_env()?);
    let database = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    let templates = if config.is_development() {
        TeraEngine::load("templates/**/*.html")?
    } else {
        TeraEngine::embedded()?
    };
    let assets = AssetResolver::from_config(&config)?;
    let ssr = InertiaSsr::from_config(&config);
    let inertia =
        InertiaRenderer::new(templates.clone(), assets, env!("CARGO_PKG_VERSION")).with_ssr(ssr);
    let state = AppState::new(database, Arc::clone(&config), templates.clone(), inertia);
    let listener = TcpListener::bind(config.socket_addr()?).await?;
    let app = routes::router(state);
    let development_watcher;
    let app = if config.is_development() {
        let live_reload = LiveReloadLayer::new();
        development_watcher = Some(DevelopmentWatcher::start(
            templates,
            live_reload.reloader(),
        )?);
        app.layer(live_reload)
    } else {
        development_watcher = None;
        app
    };

    tracing::info!(address = %listener.local_addr()?, environment = %config.app_env, "server started");
    axum::serve(listener, app).await?;
    drop(development_watcher);
    Ok(())
}
