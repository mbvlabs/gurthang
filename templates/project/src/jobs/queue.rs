use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{Job, JobError};

const DEFAULT_MAX_ATTEMPTS: i32 = 5;
const WAKE_CHANNEL: &str = "gurthang_jobs";

#[derive(Clone, Debug)]
pub struct JobQueue {
    database: PgPool,
}

impl JobQueue {
    pub fn new(database: PgPool) -> Self {
        Self { database }
    }

    pub async fn enqueue(&self, job: Job) -> Result<Uuid, JobError> {
        self.enqueue_at(job, Utc::now()).await
    }

    pub async fn enqueue_at(
        &self,
        job: Job,
        available_at: DateTime<Utc>,
    ) -> Result<Uuid, JobError> {
        let mut connection = self.database.acquire().await?;
        Self::enqueue_in(&mut connection, job, available_at).await
    }

    /// Enqueues a job on an existing connection or transaction.
    ///
    /// Pass `&mut transaction` to commit the application change and its job
    /// atomically. PostgreSQL delivers the wake-up notification only if that
    /// transaction commits; polling remains the queue's source of correctness.
    pub async fn enqueue_in(
        connection: &mut PgConnection,
        job: Job,
        available_at: DateTime<Utc>,
    ) -> Result<Uuid, JobError> {
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
