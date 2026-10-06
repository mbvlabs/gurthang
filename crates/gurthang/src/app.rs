use std::sync::Arc;

use async_trait::async_trait;
use axum::{Router, extract::FromRef};
use gurthang_inertia::InertiaRenderer;
use gurthang_jobs::JobQueue;
use sqlx::PgPool;
use tokio::sync::watch;

use crate::{
    boot::{AppRoutes, BootResult, StartMode},
    config::{Config, Environment},
    error::Result,
    task::Tasks,
};

#[derive(Clone)]
pub struct Context {
    pub environment: Environment,
    pub db: PgPool,
    pub config: Arc<Config>,
    pub inertia: InertiaRenderer,
    pub jobs: JobQueue,
    pub start_mode: StartMode,
    shutdown: watch::Receiver<bool>,
}

impl Context {
    pub fn new(
        environment: Environment,
        db: PgPool,
        config: Arc<Config>,
        inertia: InertiaRenderer,
        jobs: JobQueue,
        start_mode: StartMode,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            environment,
            db,
            config,
            inertia,
            jobs,
            start_mode,
            shutdown,
        }
    }

    pub fn shutdown(&self) -> watch::Receiver<bool> {
        self.shutdown.clone()
    }
}

impl FromRef<Context> for PgPool {
    fn from_ref(ctx: &Context) -> Self {
        ctx.db.clone()
    }
}

impl FromRef<Context> for Arc<Config> {
    fn from_ref(ctx: &Context) -> Self {
        Arc::clone(&ctx.config)
    }
}

impl FromRef<Context> for JobQueue {
    fn from_ref(ctx: &Context) -> Self {
        ctx.jobs.clone()
    }
}

impl FromRef<Context> for InertiaRenderer {
    fn from_ref(ctx: &Context) -> Self {
        ctx.inertia.clone()
    }
}

#[async_trait]
pub trait Initializer: Send + Sync {
    fn name(&self) -> &'static str;

    async fn before_run(&self, ctx: &mut Context) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    async fn after_routes(&self, router: Router<Context>, ctx: &Context) -> Result<Router<Context>> {
        let _ = ctx;
        Ok(router)
    }
}

#[async_trait]
pub trait Hooks: Sized {
    fn app_name() -> &'static str;

    async fn boot(mode: StartMode, config: Config) -> Result<BootResult>;

    fn routes(ctx: &Context) -> AppRoutes;

    async fn connect_workers(ctx: &Context) -> Result<()>;

    fn register_tasks(tasks: &mut Tasks);

    async fn initializers(ctx: &Context) -> Result<Vec<Box<dyn Initializer>>>;

    fn export_payloads() -> Result<()>;
}
