use std::sync::Arc;

use gurthang_inertia::InertiaRenderer;
use gurthang_jobs::JobQueue;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub database: PgPool,
    pub config: Arc<Config>,
    pub inertia: InertiaRenderer,
    pub jobs: JobQueue,
}

impl AppState {
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
