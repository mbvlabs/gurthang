use std::{env, sync::Arc};

use axum::{Router, middleware};
use axum_login::{AuthManagerLayerBuilder, AuthnBackend};
use gurthang_http::PostgresSessionStore;
use gurthang_inertia::InertiaRenderer;
use gurthang_jobs::JobQueue;
use sqlx::postgres::PgPoolOptions;
use tokio::{net::TcpListener, sync::watch};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};
use tracing_subscriber::EnvFilter;

use crate::{
    app::{Context, Hooks},
    config::Config,
    error::{Error, Result},
    task::Tasks,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartMode {
    All,
    Web,
    Worker,
}

impl StartMode {
    pub fn parse(value: Option<&str>) -> Result<Self> {
        match value {
            None | Some("all") => Ok(Self::All),
            Some("web") => Ok(Self::Web),
            Some("worker") => Ok(Self::Worker),
            Some(other) => Err(Error::Message(format!(
                "unknown process mode {other:?}; expected all, web, or worker"
            ))),
        }
    }

    pub fn includes_web(self) -> bool {
        matches!(self, Self::All | Self::Web)
    }

    pub fn includes_worker(self) -> bool {
        matches!(self, Self::All | Self::Worker)
    }
}

pub struct AppRoutes {
    routers: Vec<Router<Context>>,
}

impl AppRoutes {
    pub fn new() -> Self {
        Self {
            routers: Vec::new(),
        }
    }

    pub fn add_route(mut self, router: Router<Context>) -> Self {
        self.routers.push(router);
        self
    }

    pub fn collect(self) -> Router<Context> {
        self.routers.into_iter().fold(Router::new(), Router::merge)
    }

    pub fn len(&self) -> usize {
        self.routers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routers.is_empty()
    }
}

impl Default for AppRoutes {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BootResult {
    pub context: Context,
    pub router: Option<Router>,
    pub mode: StartMode,
    #[allow(dead_code)]
    shutdown_sender: watch::Sender<bool>,
}

pub async fn start<H: Hooks>() -> Result<()> {
    dotenvy::dotenv().ok();

    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("task") => run_tasks::<H>(args.collect()).await,
        other => {
            let config = Config::load()?;
            init_tracing(&config);
            let mode = StartMode::parse(other)?;
            let boot = H::boot(mode, config).await?;
            serve(boot).await
        }
    }
}

pub async fn create_app<H: Hooks>(mode: StartMode, config: Config) -> Result<BootResult> {
    let (shutdown_sender, shutdown_receiver) = watch::channel(false);
    let environment = config.environment.clone();
    let max_connections = config.pool_max_connections();
    let db = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&config.database_url)
        .await?;
    let jobs = JobQueue::new(db.clone());
    let inertia = InertiaRenderer::new("", H::app_name(), "");
    let mut ctx = Context::new(
        environment,
        db,
        Arc::new(config),
        inertia,
        jobs,
        mode,
        shutdown_receiver,
    );

    let signal_sender = shutdown_sender.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = signal_sender.send(true);
    });

    let initializers = H::initializers(&ctx).await?;
    for initializer in &initializers {
        initializer.before_run(&mut ctx).await?;
    }

    let router = if mode.includes_web() {
        let mut router = H::routes(&ctx).collect();
        for initializer in &initializers {
            router = initializer.after_routes(router, &ctx).await?;
        }
        Some(router.with_state(ctx.clone()))
    } else {
        None
    };

    if mode.includes_worker() {
        H::connect_workers(&ctx).await?;
    }

    Ok(BootResult {
        context: ctx,
        router,
        mode,
        shutdown_sender,
    })
}

pub fn serve_dev_assets(router: Router<Context>) -> Router<Context> {
    router
        .nest_service("/assets", ServeDir::new("assets"))
        .nest_service("/build", ServeDir::new("dist"))
}

pub fn apply_http_layers<B>(router: Router<Context>, ctx: &Context, backend: B) -> Router<Context>
where
    B: AuthnBackend + Clone + Send + Sync + 'static,
    B::User: Clone + Send + Sync + 'static,
    B::Credentials: Send + Sync + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let session_secure = ctx.config.session.secure;
    let session_store = PostgresSessionStore::new(ctx.db.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name(ctx.config.session.cookie.clone())
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(session_secure)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();
    router
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            session_secure,
            gurthang_http::protect,
        ))
        .layer(auth_layer)
}

async fn serve(boot: BootResult) -> Result<()> {
    match boot.mode {
        StartMode::Worker => Ok(()),
        StartMode::Web | StartMode::All => {
            let addr = boot.context.config.socket_addr()?;
            let listener = TcpListener::bind(addr).await?;
            let router = boot
                .router
                .ok_or_else(|| Error::Message("web start mode is missing a router".into()))?;
            tracing::info!(
                address = %listener.local_addr()?,
                environment = %boot.context.environment,
                "server started"
            );
            let mut shutdown = boot.context.shutdown();
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown.changed().await;
                })
                .await?;
            Ok(())
        }
    }
}

async fn run_tasks<H: Hooks>(args: Vec<String>) -> Result<()> {
    let list = args.is_empty() || args.iter().any(|arg| arg == "--list" || arg == "-l");
    let name = args.into_iter().find(|arg| !arg.starts_with('-'));
    let mut tasks = Tasks::new();
    H::register_tasks(&mut tasks);
    if list || name.is_none() {
        if tasks.is_empty() {
            println!("No tasks registered.");
            return Ok(());
        }
        for line in tasks.list_lines() {
            println!("{line}");
        }
        return Ok(());
    }
    let name = name.expect("task name");
    let config = Config::load()?;
    init_tracing(&config);
    let boot = create_app::<H>(StartMode::Web, config).await?;
    tasks.run(&name, &boot.context, &Default::default()).await
}

fn init_tracing(config: &Config) {
    let default = if config.logger.level.is_empty() {
        "info".to_owned()
    } else {
        config.logger.level.clone()
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| default.into()))
        .init();
}

async fn shutdown_signal() {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("could not install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_mode_parses_known_values() {
        assert_eq!(StartMode::parse(None).unwrap(), StartMode::All);
        assert_eq!(StartMode::parse(Some("all")).unwrap(), StartMode::All);
        assert_eq!(StartMode::parse(Some("web")).unwrap(), StartMode::Web);
        assert_eq!(StartMode::parse(Some("worker")).unwrap(), StartMode::Worker);
        assert!(StartMode::parse(Some("sidekiq")).is_err());
        assert!(StartMode::All.includes_web() && StartMode::All.includes_worker());
        assert!(StartMode::Web.includes_web() && !StartMode::Web.includes_worker());
        assert!(!StartMode::Worker.includes_web() && StartMode::Worker.includes_worker());
    }

    #[test]
    fn app_routes_collects_controller_routers() {
        let routes = AppRoutes::new()
            .add_route(Router::<Context>::new())
            .add_route(Router::<Context>::new());
        assert_eq!(routes.len(), 2);
        let _router = routes.collect();
    }
}
