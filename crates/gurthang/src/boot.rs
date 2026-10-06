use std::{env, fs, sync::Arc};

use axum::Router;
use gurthang_inertia::InertiaRenderer;
use gurthang_jobs::JobQueue;
use sqlx::postgres::PgPoolOptions;
use tokio::{net::TcpListener, sync::watch};
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

use crate::{
    app::{Context, Hooks},
    config::{Config, Environment},
    controller::middleware::{self, apply_stack},
    error::{Error, Result},
    mailer::EmailSender,
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
        Some("middleware") => print_middleware(),
        other => {
            let config = Config::load()?;
            init_tracing(&config);
            let mode = StartMode::parse(other)?;
            let boot = H::boot(mode, config).await?;
            let context = boot.context.clone();
            let result = serve(boot).await;
            H::on_shutdown(&context).await;
            result
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
    let mailer = EmailSender::from_config(&config.mailer)?;
    if let Some(sender) = mailer.clone() {
        sender.install();
    }
    let inertia = InertiaRenderer::new("", H::app_name(), "");
    let mut ctx = Context::new(
        environment,
        db,
        Arc::new(config),
        inertia,
        jobs,
        mailer,
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
        let mut router = H::before_routes(&ctx).await?;
        router = router.merge(H::routes(&ctx).collect());
        router = apply_stack(router, H::middlewares(&ctx))?;
        router = H::after_routes(router, &ctx).await?;
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
    router.nest_service("/assets", ServeDir::new("assets"))
}

fn print_middleware() -> Result<()> {
    let environment = Environment::from_env();
    let path = std::path::Path::new("config").join(format!("{}.yaml", environment.as_str()));
    let yaml = fs::read_to_string(&path)
        .map_err(|error| Error::Config(format!("could not read {}: {error}", path.display())))?;
    let mut config = Config::from_yaml(&yaml)?;
    config.environment = environment;
    middleware::print_stack(
        &middleware::stack_from_config(&config),
        &mut std::io::stdout(),
    )
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
