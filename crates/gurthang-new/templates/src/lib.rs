pub mod config;
pub mod controllers;
pub mod error;
pub mod jobs;
pub mod models;
pub mod routes;
pub mod services;
pub mod web;

use std::{io, sync::Arc};

use axum::{Router, extract::FromRef, middleware, routing::get};
use axum_login::AuthManagerLayerBuilder;
use gurthang_http::{AssetResolver, PostgresSessionStore};
use gurthang_inertia::{InertiaRenderer, InertiaSsr, SsrOptions};
use gurthang_jobs::{JobQueue, JobWorker, WorkerConfig};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::{net::TcpListener, sync::watch};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};
use tracing_subscriber::EnvFilter;

use crate::{
    config::Config,
    controllers::{auth::Auth, dashboard::Dashboard, welcome::Welcome},
    jobs::Job,
    services::auth::AuthBackend,
    web::assets,
};

pub const fn application_name() -> &'static str {
    "{{ project_name }}"
}

#[derive(Clone)]
pub struct App {
    pub welcome: Welcome,
    pub auth: Auth,
    pub dashboard: Dashboard,
    pub database: PgPool,
    pub config: Arc<Config>,
    pub jobs: JobQueue,
}

impl App {
    pub fn new(database: PgPool, config: Arc<Config>, inertia: InertiaRenderer) -> Self {
        Self {
            welcome: Welcome {
                inertia: inertia.clone(),
            },
            auth: Auth {
                inertia: inertia.clone(),
            },
            dashboard: Dashboard {
                inertia: inertia.clone(),
            },
            jobs: JobQueue::new(database.clone()),
            database,
            config,
        }
    }
}

impl FromRef<App> for PgPool {
    fn from_ref(app: &App) -> Self {
        app.database.clone()
    }
}

impl FromRef<App> for Arc<Config> {
    fn from_ref(app: &App) -> Self {
        Arc::clone(&app.config)
    }
}

impl FromRef<App> for JobQueue {
    fn from_ref(app: &App) -> Self {
        app.jobs.clone()
    }
}

fn build_router(app: App) -> Router {
    let is_development = app.config.is_development();
    let session_secure = app.config.session_secure;
    let session_store = PostgresSessionStore::new(app.database.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name("gurthang.sid")
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(session_secure)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));
    let auth_layer =
        AuthManagerLayerBuilder::new(AuthBackend::new(app.database.clone()), session_layer).build();

    let router = gurthang_http::mount!(app, {
        routes::welcome::WELCOME => get(app.welcome, Welcome::show),
        routes::auth::REGISTER => get(app.auth, Auth::new_register),
        routes::auth::REGISTER_CREATE => post(app.auth, Auth::register_user),
        routes::auth::LOGIN => get(app.auth, Auth::new_login),
        routes::auth::LOGIN_CREATE => post(app.auth, Auth::login),
        routes::auth::LOGOUT => delete(app.auth, Auth::logout),
        routes::dashboard::DASHBOARD => get(app.dashboard, Dashboard::show),
    });
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
            session_secure,
            gurthang_http::protect,
        ))
        .layer(auth_layer)
        .with_state(app)
}

pub fn export_payloads() -> Result<(), Box<dyn std::error::Error>> {
    use ts_rs::TS;

    use crate::controllers::{
        auth::{LoginProps, RegisterProps},
        dashboard::DashboardProps,
        shared::{AuthProps, FlashProps, SafeUser, SharedProps},
        welcome::WelcomeProps,
    };

    AuthProps::export()?;
    DashboardProps::export()?;
    FlashProps::export()?;
    LoginProps::export()?;
    RegisterProps::export()?;
    SafeUser::export()?;
    SharedProps::export()?;
    WelcomeProps::export()?;
    Ok(())
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
    let app = App::new(database, Arc::clone(&config), inertia);
    let listener = TcpListener::bind(config.socket_addr()?).await?;
    let router = build_router(app);

    tracing::info!(address = %listener.local_addr()?, environment = %config.app_env, "server started");
    axum::serve(listener, router)
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
