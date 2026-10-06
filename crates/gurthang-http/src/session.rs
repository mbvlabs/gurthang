use std::time::SystemTime;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use tower_sessions::{
    SessionStore,
    session::{Id, Record},
    session_store,
};

#[derive(Clone, Debug)]
pub struct PostgresSessionStore {
    pool: PgPool,
}

impl PostgresSessionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn id_exists(&self, conn: &mut PgConnection, id: &Id) -> session_store::Result<bool> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tower_sessions WHERE id = $1)")
            .bind(id.to_string())
            .fetch_one(conn)
            .await
            .map_err(backend)
    }

    async fn save_with_conn(
        &self,
        conn: &mut PgConnection,
        record: &Record,
    ) -> session_store::Result<()> {
        let data = rmp_serde::to_vec(record)
            .map_err(|error| session_store::Error::Encode(error.to_string()))?;
        sqlx::query(
            "INSERT INTO tower_sessions (id, data, expiry_date) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (id) DO UPDATE \
             SET data = excluded.data, expiry_date = excluded.expiry_date",
        )
        .bind(record.id.to_string())
        .bind(data)
        .bind(expiry_as_utc(record.expiry_date))
        .execute(conn)
        .await
        .map_err(backend)?;
        Ok(())
    }
}

#[async_trait]
impl SessionStore for PostgresSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        while self.id_exists(&mut tx, &record.id).await? {
            record.id = Id::default();
        }
        self.save_with_conn(&mut tx, record).await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let mut conn = self.pool.acquire().await.map_err(backend)?;
        self.save_with_conn(&mut conn, record).await
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let data: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT data FROM tower_sessions WHERE id = $1 AND expiry_date > $2")
                .bind(session_id.to_string())
                .bind(Utc::now())
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?;
        data.map(|(bytes,)| {
            rmp_serde::from_slice(&bytes)
                .map_err(|error| session_store::Error::Decode(error.to_string()))
        })
        .transpose()
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        sqlx::query("DELETE FROM tower_sessions WHERE id = $1")
            .bind(session_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}

fn expiry_as_utc(expiry: time::OffsetDateTime) -> DateTime<Utc> {
    DateTime::<Utc>::from(SystemTime::from(expiry))
}

fn backend(error: sqlx::Error) -> session_store::Error {
    session_store::Error::Backend(error.to_string())
}
