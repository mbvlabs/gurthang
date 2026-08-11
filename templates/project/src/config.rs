use std::{env, net::SocketAddr};

use crate::error::{AppError, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub app_env: String,
    pub app_host: String,
    pub app_port: u16,
    pub app_url: String,
    pub database_url: String,
    pub session_secure: bool,
    pub vite_dev_server_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let app_env = value("APP_ENV")?;
        let app_host = value("APP_HOST")?;
        let app_port = value("APP_PORT")?.parse().map_err(|_| {
            AppError::Config("APP_PORT must be an integer between 1 and 65535".into())
        })?;
        let app_url = value("APP_URL")?;
        let database_url = value("DATABASE_URL")?;
        let session_secure = value("SESSION_SECURE")?
            .parse()
            .map_err(|_| AppError::Config("SESSION_SECURE must be true or false".into()))?;
        let vite_dev_server_url = env::var("VITE_DEV_SERVER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty());

        Ok(Self {
            app_env,
            app_host,
            app_port,
            app_url,
            database_url,
            session_secure,
            vite_dev_server_url,
        })
    }

    pub fn socket_addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.app_host, self.app_port)
            .parse()
            .map_err(|_| {
                AppError::Config("APP_HOST and APP_PORT do not form a valid socket address".into())
            })
    }

    pub fn is_development(&self) -> bool {
        self.app_env == "development"
    }
}

fn value(name: &str) -> Result<String> {
    env::var(name)
        .map_err(|_| AppError::Config(format!("missing required environment variable {name}")))
}
