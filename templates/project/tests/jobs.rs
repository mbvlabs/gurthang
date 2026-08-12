use std::time::Duration;

use chrono::Utc;
use sqlx::PgPool;

use __GURTHANG_CRATE_NAME__::jobs::{Job, JobQueue, JobWorker, WorkerConfig};

async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    sqlx::migrate!().run(&pool).await.ok()?;
    Some(pool)
}

fn worker(pool: PgPool) -> JobWorker {
    JobWorker::new(
        pool,
        WorkerConfig {
            concurrency: 10,
            poll_interval: Duration::from_millis(10),
            lease: Duration::from_secs(10),
            job_timeout: Duration::from_secs(5),
        },
    )
}

#[tokio::test]
async fn postgres_jobs_are_transactional_durable_and_retryable() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping PostgreSQL job integration test; set TEST_DATABASE_URL");
        return;
    };
    let queue = JobQueue::new(pool.clone());

    let mut transaction = pool.begin().await.unwrap();
    let rolled_back_id =
        JobQueue::enqueue_in(&mut transaction, Job::PurgeExpiredSessions, Utc::now())
            .await
            .unwrap();
    transaction.rollback().await.unwrap();
    let rolled_back_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM background_jobs WHERE id = $1)")
            .bind(rolled_back_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!rolled_back_exists);

    let completed_id = queue.enqueue(Job::PurgeExpiredSessions).await.unwrap();
    let invalid_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO background_jobs \
         (id, payload, max_attempts, available_at, created_at, updated_at) \
         VALUES ($1, '{\"type\":\"removed_job\"}'::jsonb, 1, NOW(), NOW(), NOW())",
    )
    .bind(invalid_id)
    .execute(&pool)
    .await
    .unwrap();

    worker(pool.clone()).run_once().await.unwrap();

    let completed: (String, i32) =
        sqlx::query_as("SELECT status, attempts FROM background_jobs WHERE id = $1")
            .bind(completed_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(completed, ("completed".into(), 1));
    let failed: (String, i32, Option<String>) =
        sqlx::query_as("SELECT status, attempts, last_error FROM background_jobs WHERE id = $1")
            .bind(invalid_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(failed.0, "failed");
    assert_eq!(failed.1, 1);
    assert!(failed.2.unwrap().contains("invalid job payload"));

    sqlx::query("DELETE FROM background_jobs WHERE id = ANY($1)")
        .bind(&[completed_id, invalid_id][..])
        .execute(&pool)
        .await
        .unwrap();
}
