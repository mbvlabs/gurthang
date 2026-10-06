use std::sync::Arc;

use axum::extract::FromRef;
use gurthang_inertia::InertiaRenderer;
use gurthang_jobs::JobQueue;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct App {
    pub database: PgPool,
    pub config: Arc<Config>,
    pub inertia: InertiaRenderer,
    pub jobs: JobQueue,
}

impl App {
    pub fn new(database: PgPool, config: Arc<Config>, inertia: InertiaRenderer) -> Self {
        let jobs = JobQueue::new(database.clone());
        Self {
            database,
            config,
            inertia,
            jobs,
        }
    }
}

impl FromRef<App> for PgPool {
    fn from_ref(app: &App) -> Self {
        app.database.clone()
    }
}

impl FromRef<App> for InertiaRenderer {
    fn from_ref(app: &App) -> Self {
        app.inertia.clone()
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
