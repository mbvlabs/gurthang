pub mod app;
pub mod config;
pub mod controllers;
pub mod error;
pub mod jobs;
pub mod models;
pub mod routes;
pub mod services;
pub mod views;
pub mod web;

use std::{io, sync::Arc};

use gurthang_http::AssetResolver;
use gurthang_inertia::{InertiaRenderer, InertiaSsr, SsrOptions};
use gurthang_jobs::{JobWorker, WorkerConfig};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::{net::TcpListener, sync::watch};
use tracing_subscriber::EnvFilter;

use crate::{app::AppState, config::Config, jobs::Job};

pub const fn application_name() -> &'static str {
    "__GURTHANG_PROJECT_NAME__"
}

#[derive(Clone, Copy)]
enum Mode {
    All,
    Web,
    Worker,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let mode = mode()?;
    let worker_config = WorkerConfig::from_env()?;
    let max_connections = u32::try_from(worker_config.concurrency)
        .unwrap_or(u32::MAX)
        .saturating_add(10);
    let config = Arc::new(Config::from_env()?);
    let database = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&config.database_url)
        .await?;

    match mode {
        Mode::Worker => run_worker(database, worker_config).await,
        Mode::Web => run_web(database, config, None).await,
        Mode::All => {
            let (shutdown_sender, shutdown_receiver) = watch::channel(false);
            let worker = tokio::spawn(
                JobWorker::<Job>::new(database.clone(), worker_config).run(shutdown_receiver),
            );
            let result = run_web(database, config, Some(shutdown_sender.clone())).await;
            let _ = shutdown_sender.send(true);
            if let Err(error) = worker.await {
                tracing::error!(%error, "background job worker task stopped unexpectedly");
            }
            result
        }
    }
}

fn mode() -> Result<Mode, io::Error> {
    match std::env::args().nth(1).as_deref() {
        None | Some("all") => Ok(Mode::All),
        Some("web") => Ok(Mode::Web),
        Some("worker") => Ok(Mode::Worker),
        Some(other) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown process mode {other:?}; expected all, web, or worker"),
        )),
    }
}

async fn run_worker(
    database: PgPool,
    worker_config: WorkerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let (shutdown_sender, shutdown_receiver) = watch::channel(false);
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_sender.send(true);
    });
    JobWorker::<Job>::new(database, worker_config)
        .run(shutdown_receiver)
        .await;
    Ok(())
}

async fn run_web(
    database: PgPool,
    config: Arc<Config>,
    worker_shutdown: Option<watch::Sender<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let assets = if config.is_development() {
        let server_url = config.vite_dev_server_url.clone().ok_or_else(|| {
            crate::error::AppError::Asset("VITE_DEV_SERVER_URL is required in development".into())
        })?;
        AssetResolver::development(server_url)
    } else {
        AssetResolver::from_manifest_bytes(crate::web::assets::manifest_bytes())
            .map_err(|error| crate::error::AppError::Asset(error.to_string()))?
    };
    let ssr = InertiaSsr::from_options(SsrOptions {
        runtime: config.inertia_ssr_runtime.clone(),
        timeout_ms: config.inertia_ssr_timeout_ms,
        development_bundle: std::path::PathBuf::from("dist-ssr/ssr.mjs"),
        embedded_bundle: include_bytes!(concat!(env!("OUT_DIR"), "/inertia-ssr.mjs")),
        is_development: config.is_development(),
    });
    let inertia = InertiaRenderer::new(
        assets.head_tags(),
        application_name(),
        env!("CARGO_PKG_VERSION"),
    )
    .with_ssr(ssr);
    let state = AppState::new(database, Arc::clone(&config), inertia);
    let listener = TcpListener::bind(config.socket_addr()?).await?;
    let app = routes::router(state);

    tracing::info!(address = %listener.local_addr()?, environment = %config.app_env, "server started");
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            if let Some(sender) = worker_shutdown {
                let _ = sender.send(true);
            }
        })
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("could not install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}
