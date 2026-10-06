use gurthang_jobs::{PerformJob, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub use gurthang_jobs::{JobQueue, JobWorker, WorkerConfig};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Job {
    // gurthang:generated:variants:start
    PurgeExpiredSessions,
    // gurthang:generated:variants:end
}

impl PerformJob for Job {
    fn name(&self) -> &'static str {
        match self {
            // gurthang:generated:names:start
            Self::PurgeExpiredSessions => "purge_expired_sessions",
            // gurthang:generated:names:end
        }
    }

    async fn perform(self, database: &PgPool) -> Result<()> {
        match self {
            // gurthang:generated:handlers:start
            Self::PurgeExpiredSessions => {
                crate::models::sessions::purge_expired(database).await?;
                Ok(())
            }
            // gurthang:generated:handlers:end
        }
    }
}
