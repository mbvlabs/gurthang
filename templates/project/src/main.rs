use std::{io, sync::Arc};

use __GURTHANG_CRATE_NAME__::{
    app::AppState,
    config::Config,
    jobs::{JobWorker, WorkerConfig},
    routes,
    web::{
        assets::AssetResolver, development::DevelopmentWatcher, inertia::InertiaRenderer,
        inertia::InertiaSsr, tera::TeraEngine,
    },
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::{net::TcpListener, sync::watch};
use tower_livereload::LiveReloadLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone, Copy)]
enum Mode {
    All,
    Web,
    Worker,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
                JobWorker::new(database.clone(), worker_config).run(shutdown_receiver),
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
    JobWorker::new(database, worker_config)
        .run(shutdown_receiver)
        .await;
    Ok(())
}

async fn run_web(
    database: PgPool,
    config: Arc<Config>,
    worker_shutdown: Option<watch::Sender<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
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
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            if let Some(sender) = worker_shutdown {
                let _ = sender.send(true);
            }
        })
        .await?;
    drop(development_watcher);
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("could not install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
