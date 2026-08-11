use std::sync::Arc;

use __GURTHANG_CRATE_NAME__::{
    app::AppState,
    config::Config,
    routes,
    web::{assets::AssetResolver, inertia::InertiaRenderer, tera::TeraEngine},
};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
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
    let templates = TeraEngine::load("templates/**/*.html")?;
    let assets = AssetResolver::from_config(&config)?;
    let inertia = InertiaRenderer::new(templates.clone(), assets, env!("CARGO_PKG_VERSION"));
    let state = AppState::new(database, Arc::clone(&config), templates, inertia);
    let listener = TcpListener::bind(config.socket_addr()?).await?;

    tracing::info!(address = %listener.local_addr()?, environment = %config.app_env, "server started");
    axum::serve(listener, routes::router(state)).await?;
    Ok(())
}
