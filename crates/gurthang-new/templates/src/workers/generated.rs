use gurthang::jobs::{PerformJob, Result};
use gurthang::mailer::Email;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Job {
    PurgeExpiredSessions,
    Mailer(Email),
}

impl PerformJob for Job {
    fn name(&self) -> &'static str {
        match self {
            Self::PurgeExpiredSessions => "purge_expired_sessions",
            Self::Mailer(_) => "mailer",
        }
    }

    async fn perform(self, database: &PgPool) -> Result<()> {
        match self {
            Self::PurgeExpiredSessions => {
                purge_expired_sessions::PurgeExpiredSessions
                    .perform(database)
                    .await
            }
            Self::Mailer(email) => gurthang::mailer::perform_job(email, database).await,
        }
    }
}
