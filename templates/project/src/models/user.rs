use std::fmt;

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum_login::AuthUser;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

#[derive(Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl fmt::Debug for User {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("User")
            .field("id", &self.id)
            .field("email", &self.email)
            .field("password_hash", &"[REDACTED]")
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl AuthUser for User {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

pub struct CreateUserData {
    pub email: String,
    pub password: String,
}

impl fmt::Debug for CreateUserData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreateUserData")
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("email has already been registered")]
    DuplicateEmail,
    #[error("password operation failed")]
    Password,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("password worker failed")]
    PasswordWorker(#[from] tokio::task::JoinError),
}

impl User {
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT id, email, password_hash, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT id, email, password_hash, created_at, updated_at FROM users WHERE email = $1",
        )
        .bind(normalize_email(email))
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        connection: &mut PgConnection,
        email: String,
        password_hash: String,
    ) -> Result<Self, UserError> {
        let now = Utc::now();
        sqlx::query_as::<_, Self>(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $4) \
             RETURNING id, email, password_hash, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(email)
        .bind(password_hash)
        .bind(now)
        .fetch_one(connection)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .is_some_and(|database| database.is_unique_violation())
            {
                UserError::DuplicateEmail
            } else {
                UserError::Database(error)
            }
        })
    }

    pub async fn hash_password(password: String) -> Result<String, UserError> {
        tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|_| UserError::Password)
        })
        .await?
    }

    pub async fn verify_password(&self, password: String) -> Result<bool, UserError> {
        let password_hash = self.password_hash.clone();
        tokio::task::spawn_blocking(move || {
            let parsed = PasswordHash::new(&password_hash).map_err(|_| UserError::Password)?;
            Ok(Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok())
        })
        .await?
    }
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn validate(data: &CreateUserData) -> std::collections::BTreeMap<String, String> {
    let mut errors = std::collections::BTreeMap::new();
    let email = normalize_email(&data.email);
    if email.len() > 254 || !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        errors.insert("email".into(), "Enter a valid email address.".into());
    }
    if data.password.len() < 12 {
        errors.insert(
            "password".into(),
            "Password must be at least 12 characters.".into(),
        );
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_is_trimmed_and_lowercased() {
        assert_eq!(
            normalize_email("  Person@Example.COM "),
            "person@example.com"
        );
    }

    #[tokio::test]
    async fn argon2id_hashes_verify_without_exposing_plaintext() {
        let hash = User::hash_password("correct horse battery staple".into())
            .await
            .unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(!hash.contains("correct horse"));
        let user = User {
            id: Uuid::new_v4(),
            email: "person@example.com".into(),
            password_hash: hash,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(
            user.verify_password("correct horse battery staple".into())
                .await
                .unwrap()
        );
        assert!(
            !user
                .verify_password("incorrect password".into())
                .await
                .unwrap()
        );
        assert!(!format!("{user:?}").contains("$argon2"));
    }
}
