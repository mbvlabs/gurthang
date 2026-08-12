use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    config::Config,
    jobs::JobQueue,
    web::{inertia::InertiaRenderer, tera::TeraEngine},
};

#[derive(Clone)]
pub struct AppState {
    pub database: PgPool,
    pub config: Arc<Config>,
    pub templates: TeraEngine,
    pub inertia: InertiaRenderer,
    pub jobs: JobQueue,
}

impl AppState {
    pub fn new(
        database: PgPool,
        config: Arc<Config>,
        templates: TeraEngine,
        inertia: InertiaRenderer,
    ) -> Self {
        let jobs = JobQueue::new(database.clone());
        Self {
            database,
            config,
            templates,
            inertia,
            jobs,
        }
    }
}
