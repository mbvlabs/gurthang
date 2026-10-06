use sqlx::PgPool;

pub async fn purge_expired(pool: &PgPool) -> sqlx::Result<u64> {
    let result = sqlx::query!("DELETE FROM tower_sessions WHERE expiry_date < NOW()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
