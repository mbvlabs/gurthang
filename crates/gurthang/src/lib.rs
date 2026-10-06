pub mod app;
pub mod boot;
pub mod config;
pub mod controller;
pub mod error;
pub mod mailer;
pub mod prelude;
pub mod task;

pub use gurthang_http as http;
pub use gurthang_inertia as inertia;
pub use gurthang_jobs as jobs;

pub use app::{Context, Hooks, Initializer};
pub use boot::{AppRoutes, BootResult, StartMode, create_app, serve_dev_assets, start};
pub use config::{Config, Environment};
pub use controller::middleware::{
    MiddlewareLayer, MiddlewareStackExt, default_middleware_stack, session_auth,
};
pub use error::{Error, Result};
pub use gurthang_http::{AssetResolver, PostgresSessionStore, Route, mount, on, protect};
pub use gurthang_inertia::{
    InertiaRenderer, InertiaRequest, InertiaSsr, SsrOptions, mutation_redirect,
};
pub use gurthang_jobs::{JobQueue, JobWorker, PerformJob, WorkerConfig};
pub use mailer::{Email, EmailSender, Mailer, MailerWorker};
pub use task::{Task, TaskInfo, Tasks};
