use gurthang::jobs::{PerformJob, Result};
use gurthang::Context;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PurgeExpiredSessions;

impl PerformJob for PurgeExpiredSessions {
    fn name(&self) -> &'static str {
        "purge_expired_sessions"
    }

    async fn perform(self, database: &PgPool) -> Result<()> {
        crate::models::sessions::purge_expired(database).await?;
        Ok(())
    }
}

pub fn register(_ctx: &Context) {}
