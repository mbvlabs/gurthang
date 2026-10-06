use std::{env, marker::PhantomData, time::Duration};

use serde::de::DeserializeOwned;
use sqlx::{PgPool, postgres::PgListener};
use tokio::{sync::watch, task::JoinSet, time::timeout};
use uuid::Uuid;

use crate::{
    WAKE_CHANNEL,
    error::{Error, Result},
};

const MAX_RETRY_DELAY_SECONDS: u64 = 60 * 60;
const STORED_ERROR_LIMIT: usize = 2_000;

pub trait PerformJob: DeserializeOwned + Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn perform(
        self,
        database: &PgPool,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

#[derive(Clone, Debug)]
pub struct WorkerConfig {
    pub concurrency: usize,
    pub poll_interval: Duration,
    pub lease: Duration,
    pub job_timeout: Duration,
}

impl WorkerConfig {
    pub fn new(
        concurrency: usize,
        poll_interval: Duration,
        lease: Duration,
        job_timeout: Duration,
    ) -> Result<Self> {
        if concurrency == 0 {
            return Err(Error::Config("worker concurrency must be greater than zero".into()));
        }
        if job_timeout >= lease {
            return Err(Error::Config(
                "job timeout must be less than the job lease".into(),
            ));
        }
        Ok(Self {
            concurrency,
            poll_interval,
            lease,
            job_timeout,
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(
            positive_usize("JOB_WORKERS", 4)?,
            Duration::from_millis(positive_u64("JOB_POLL_INTERVAL_MS", 1_000)?),
            Duration::from_secs(positive_u64("JOB_LEASE_SECONDS", 300)?),
            Duration::from_secs(positive_u64("JOB_TIMEOUT_SECONDS", 240)?),
        )
    }
}

#[derive(Debug)]
pub struct JobWorker<J> {
    database: PgPool,
    config: WorkerConfig,
    worker_id: Uuid,
    _job: PhantomData<J>,
}

impl<J> Clone for JobWorker<J> {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            config: self.config.clone(),
            worker_id: self.worker_id,
            _job: PhantomData,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ClaimedJob {
    id: Uuid,
    payload: String,
    attempts: i32,
    max_attempts: i32,
}

impl<J: PerformJob> JobWorker<J> {
    pub fn new(database: PgPool, config: WorkerConfig) -> Self {
        Self {
            database,
            config,
            worker_id: Uuid::new_v4(),
            _job: PhantomData,
        }
    }

    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        let mut listener = self.listener().await;
        tracing::info!(
            worker_id = %self.worker_id,
            concurrency = self.config.concurrency,
            "background job worker started"
        );
        loop {
            if *shutdown.borrow() {
                break;
            }
            match self.run_once().await {
                Ok(0) => self.wait_for_work(&mut listener, &mut shutdown).await,
                Ok(count) => tracing::debug!(count, "background job batch completed"),
                Err(error) => {
                    tracing::error!(%error, "background job claim failed");
                    wait_or_shutdown(self.config.poll_interval, &mut shutdown).await;
                }
            }
        }
        tracing::info!(worker_id = %self.worker_id, "background job worker stopped");
    }

    pub async fn run_once(&self) -> Result<usize> {
        let jobs = self.claim().await?;
        let count = jobs.len();
        let mut tasks = JoinSet::new();
        for job in jobs {
            let worker = self.clone();
            tasks.spawn(async move { worker.process(job).await });
        }
        while let Some(result) = tasks.join_next().await {
            if let Err(error) = result {
                tracing::error!(%error, "background job task stopped unexpectedly");
            }
        }
        Ok(count)
    }

    async fn listener(&self) -> Option<PgListener> {
        match PgListener::connect_with(&self.database).await {
            Ok(mut listener) => match listener.listen(WAKE_CHANNEL).await {
                Ok(()) => Some(listener),
                Err(error) => {
                    tracing::warn!(%error, "could not listen for job wake-ups; using polling");
                    None
                }
            },
            Err(error) => {
                tracing::warn!(%error, "could not connect job wake-up listener; using polling");
                None
            }
        }
    }

    async fn wait_for_work(
        &self,
        listener: &mut Option<PgListener>,
        shutdown: &mut watch::Receiver<bool>,
    ) {
        if let Some(listener) = listener {
            tokio::select! {
                _ = shutdown.changed() => {}
                _ = tokio::time::sleep(self.config.poll_interval) => {}
                notification = listener.recv() => {
                    if let Err(error) = notification {
                        tracing::warn!(%error, "job wake-up listener failed; polling will continue");
                    }
                }
            }
        } else {
            wait_or_shutdown(self.config.poll_interval, shutdown).await;
        }
    }

    async fn claim(&self) -> Result<Vec<ClaimedJob>> {
        let lease_seconds = i64::try_from(self.config.lease.as_secs()).unwrap_or(i64::MAX);
        let limit = i64::try_from(self.config.concurrency).unwrap_or(i64::MAX);
        sqlx::query(
            "UPDATE background_jobs \
             SET status = 'failed', locked_at = NULL, locked_by = NULL, \
                 last_error = 'job lease expired after final attempt', updated_at = NOW() \
             WHERE status = 'running' AND attempts >= max_attempts \
               AND locked_at < NOW() - ($1 * INTERVAL '1 second')",
        )
        .bind(lease_seconds)
        .execute(&self.database)
        .await?;
        Ok(sqlx::query_as::<_, ClaimedJob>(
            "UPDATE background_jobs AS job \
             SET status = 'running', attempts = attempts + 1, locked_at = NOW(), \
                 locked_by = $1, updated_at = NOW(), last_error = NULL \
             WHERE job.id IN ( \
                 SELECT id FROM background_jobs \
                 WHERE attempts < max_attempts AND ( \
                    (status = 'queued' AND available_at <= NOW()) \
                    OR (status = 'running' AND locked_at < NOW() - ($2 * INTERVAL '1 second')) \
                 ) \
                 ORDER BY available_at, id \
                 FOR UPDATE SKIP LOCKED \
                 LIMIT $3 \
             ) \
             RETURNING job.id, job.payload::text AS payload, job.attempts, job.max_attempts",
        )
        .bind(self.worker_id)
        .bind(lease_seconds)
        .bind(limit)
        .fetch_all(&self.database)
        .await?)
    }

    async fn process(self, claimed: ClaimedJob) {
        let job = match serde_json::from_str::<J>(&claimed.payload) {
            Ok(job) => job,
            Err(error) => {
                self.fail(&claimed, format!("invalid job payload: {error}"))
                    .await;
                return;
            }
        };
        let job_name = job.name();
        tracing::info!(
            job_id = %claimed.id,
            job = job_name,
            attempt = claimed.attempts,
            "background job started"
        );
        let outcome = timeout(self.config.job_timeout, job.perform(&self.database)).await;
        match outcome {
            Ok(Ok(())) => match self.complete(claimed.id).await {
                Ok(true) => tracing::info!(
                    job_id = %claimed.id,
                    job = job_name,
                    "background job completed"
                ),
                Ok(false) => tracing::warn!(
                    job_id = %claimed.id,
                    job = job_name,
                    "background job lease was lost before completion"
                ),
                Err(error) => {
                    tracing::error!(job_id = %claimed.id, job = job_name, %error, "could not complete background job")
                }
            },
            Ok(Err(error)) => self.fail(&claimed, error.to_string()).await,
            Err(_) => self.fail(&claimed, "job execution timed out".into()).await,
        }
    }

    async fn complete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE background_jobs \
             SET status = 'completed', locked_at = NULL, locked_by = NULL, \
                 completed_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND status = 'running' AND locked_by = $2",
        )
        .bind(id)
        .bind(self.worker_id)
        .execute(&self.database)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    async fn fail(&self, claimed: &ClaimedJob, error: String) {
        let final_attempt = claimed.attempts >= claimed.max_attempts;
        let status = if final_attempt { "failed" } else { "queued" };
        let delay = retry_delay(claimed.attempts);
        let stored_error = truncate_error(error);
        match sqlx::query(
            "UPDATE background_jobs \
             SET status = $3, available_at = NOW() + ($4 * INTERVAL '1 second'), \
                 locked_at = NULL, locked_by = NULL, last_error = $5, updated_at = NOW() \
             WHERE id = $1 AND status = 'running' AND locked_by = $2",
        )
        .bind(claimed.id)
        .bind(self.worker_id)
        .bind(status)
        .bind(i64::try_from(delay.as_secs()).unwrap_or(i64::MAX))
        .bind(&stored_error)
        .execute(&self.database)
        .await
        {
            Ok(result) if result.rows_affected() == 1 => tracing::warn!(
                job_id = %claimed.id,
                attempt = claimed.attempts,
                final_attempt,
                error = %stored_error,
                "background job failed"
            ),
            Ok(_) => tracing::warn!(
                job_id = %claimed.id,
                "background job lease was lost after failure"
            ),
            Err(database_error) => {
                tracing::error!(job_id = %claimed.id, %database_error, "could not record background job failure")
            }
        }
    }
}

fn retry_delay(attempt: i32) -> Duration {
    let exponent = u32::try_from(attempt.saturating_sub(1))
        .unwrap_or(0)
        .min(16);
    Duration::from_secs(
        5_u64
            .saturating_mul(2_u64.pow(exponent))
            .min(MAX_RETRY_DELAY_SECONDS),
    )
}

fn truncate_error(mut error: String) -> String {
    if error.len() <= STORED_ERROR_LIMIT {
        return error;
    }
    let mut boundary = STORED_ERROR_LIMIT;
    while !error.is_char_boundary(boundary) {
        boundary -= 1;
    }
    error.truncate(boundary);
    error
}

async fn wait_or_shutdown(duration: Duration, shutdown: &mut watch::Receiver<bool>) {
    tokio::select! {
        _ = shutdown.changed() => {}
        _ = tokio::time::sleep(duration) => {}
    }
}

fn positive_u64(name: &str, default: u64) -> Result<u64> {
    let value = match env::var(name) {
        Ok(value) if !value.trim().is_empty() => value
            .parse()
            .map_err(|_| Error::Config(format!("{name} must be a positive integer")))?,
        _ => default,
    };
    if value == 0 {
        return Err(Error::Config(format!("{name} must be greater than zero")));
    }
    Ok(value)
}

fn positive_usize(name: &str, default: usize) -> Result<usize> {
    usize::try_from(positive_u64(name, default as u64)?)
        .map_err(|_| Error::Config(format!("{name} is too large")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_back_off_and_are_capped() {
        assert_eq!(retry_delay(1), Duration::from_secs(5));
        assert_eq!(retry_delay(2), Duration::from_secs(10));
        assert_eq!(retry_delay(20), Duration::from_secs(3_600));
    }
}
