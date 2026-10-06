use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{WAKE_CHANNEL, error::{Error, Result}};

const DEFAULT_MAX_ATTEMPTS: i32 = 5;

#[derive(Clone, Debug)]
pub struct JobQueue {
    database: PgPool,
}

impl JobQueue {
    pub fn new(database: PgPool) -> Self {
        Self { database }
    }

    pub async fn enqueue<J: Serialize>(&self, job: J) -> Result<Uuid> {
        self.enqueue_at(job, Utc::now()).await
    }

    pub async fn enqueue_at<J: Serialize>(
        &self,
        job: J,
        available_at: DateTime<Utc>,
    ) -> Result<Uuid> {
        let mut connection = self.database.acquire().await?;
        Self::enqueue_in(&mut connection, job, available_at).await
    }

    pub async fn enqueue_in<J: Serialize>(
        connection: &mut PgConnection,
        job: J,
        available_at: DateTime<Utc>,
    ) -> Result<Uuid> {
        let payload = serde_json::to_string(&job)?;
        let now = Utc::now();
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO background_jobs \
             (id, payload, max_attempts, available_at, created_at, updated_at) \
             VALUES ($1, $2::jsonb, $3, $4, $5, $5)",
        )
        .bind(id)
        .bind(payload)
        .bind(DEFAULT_MAX_ATTEMPTS)
        .bind(available_at)
        .bind(now)
        .execute(&mut *connection)
        .await?;
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(WAKE_CHANNEL)
            .bind(id.to_string())
            .execute(connection)
            .await?;
        Ok(id)
    }
}
