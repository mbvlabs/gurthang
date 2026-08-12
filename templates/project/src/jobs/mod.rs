mod queue;
mod worker;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub use queue::JobQueue;
pub use worker::{JobWorker, WorkerConfig};

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("background job database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("background job serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("background job configuration error: {0}")]
    Config(String),
}

/// Every durable background operation supported by this application.
///
/// Add a variant and its handler here instead of introducing a dynamic job
/// registry. Renaming or removing a variant requires migrating jobs already
/// stored in the database.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Job {
    PurgeExpiredSessions,
}

impl Job {
    pub fn name(&self) -> &'static str {
        match self {
            Self::PurgeExpiredSessions => "purge_expired_sessions",
        }
    }

    async fn perform(self, database: &PgPool) -> Result<(), JobError> {
        match self {
            Self::PurgeExpiredSessions => {
                sqlx::query("DELETE FROM tower_sessions WHERE expiry_date < NOW()")
                    .execute(database)
                    .await?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_payloads_are_stably_tagged() {
        let payload = serde_json::to_string(&Job::PurgeExpiredSessions).unwrap();
        assert_eq!(payload, r#"{"type":"purge_expired_sessions"}"#);
        let decoded: Job = serde_json::from_str(&payload).unwrap();
        assert_eq!(decoded.name(), "purge_expired_sessions");
    }
}
