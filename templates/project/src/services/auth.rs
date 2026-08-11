use axum_login::{AuthnBackend, UserId};
use sqlx::PgPool;
use tokio::sync::OnceCell;

use crate::models::user::{CreateUserData, User, UserError, normalize_email, validate};

#[derive(Clone)]
pub struct AuthBackend {
    pool: PgPool,
    dummy_hash: std::sync::Arc<OnceCell<String>>,
}

#[derive(Clone)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error(transparent)]
    User(#[from] UserError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

impl AuthBackend {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            dummy_hash: std::sync::Arc::new(OnceCell::new()),
        }
    }

    pub async fn register(&self, data: CreateUserData) -> Result<User, RegistrationError> {
        let errors = validate(&data);
        if !errors.is_empty() {
            return Err(RegistrationError::Validation(errors));
        }
        let email = normalize_email(&data.email);
        let hash = User::hash_password(data.password).await?;
        let mut transaction = self.pool.begin().await?;
        let user = User::create(&mut transaction, email, hash).await?;
        transaction.commit().await?;
        Ok(user)
    }
}

impl AuthnBackend for AuthBackend {
    type User = User;
    type Credentials = Credentials;
    type Error = AuthError;

    async fn authenticate(
        &self,
        credentials: Self::Credentials,
    ) -> Result<Option<User>, AuthError> {
        let user = User::find_by_email(&self.pool, &credentials.email).await?;
        if let Some(user) = user {
            return Ok(user
                .verify_password(credentials.password)
                .await?
                .then_some(user));
        }

        // Preserve roughly comparable Argon2 work for unknown accounts so the
        // invalid-credentials response does not become an account oracle.
        let dummy = self
            .dummy_hash
            .get_or_try_init(|| User::hash_password("gurthang-dummy-password".into()))
            .await?
            .clone();
        tokio::task::spawn_blocking(move || {
            use argon2::{Argon2, PasswordHash, PasswordVerifier};
            if let Ok(hash) = PasswordHash::new(&dummy) {
                let _ = Argon2::default().verify_password(credentials.password.as_bytes(), &hash);
            }
        })
        .await
        .map_err(UserError::PasswordWorker)?;
        Ok(None)
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<User>, AuthError> {
        Ok(User::find_by_id(&self.pool, *user_id).await?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("validation failed")]
    Validation(std::collections::BTreeMap<String, String>),
    #[error(transparent)]
    User(#[from] UserError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub type AuthSession = axum_login::AuthSession<AuthBackend>;
