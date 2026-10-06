mod error;
mod queue;
mod worker;

pub use error::{Error, Result};
pub use queue::JobQueue;
pub use worker::{JobWorker, PerformJob, WorkerConfig};

pub const WAKE_CHANNEL: &str = "gurthang_jobs";
