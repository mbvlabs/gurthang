use std::{env, fs, net::SocketAddr, path::Path, time::Duration};

use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Environment {
    #[default]
    Development,
    Test,
    Production,
    Other(String),
}

impl Environment {
    pub fn from_env() -> Self {
        Self::parse(&env::var("APP_ENV").unwrap_or_else(|_| "development".into()))
    }

    pub fn parse(value: &str) -> Self {
        match value.trim() {
            "development" | "dev" => Self::Development,
            "test" => Self::Test,
            "production" | "prod" => Self::Production,
            other => Self::Other(other.to_owned()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Development => "development",
            Self::Test => "test",
            Self::Production => "production",
            Self::Other(value) => value,
        }
    }

    pub fn is_development(&self) -> bool {
        matches!(self, Self::Development)
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub session: SessionConfig,
    pub inertia: InertiaConfig,
    pub workers: WorkersConfig,
    #[serde(default)]
    pub logger: LoggerConfig,
    #[serde(skip)]
    pub database_url: String,
    #[serde(skip)]
    pub environment: Environment,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SessionConfig {
    pub secure: bool,
    #[serde(default = "default_session_cookie")]
    pub cookie: String,
}

fn default_session_cookie() -> String {
    "gurthang.sid".into()
}

#[derive(Clone, Debug, Deserialize)]
pub struct InertiaConfig {
    #[serde(default)]
    pub vite_dev_server_url: Option<String>,
    #[serde(default = "default_ssr_runtime")]
    pub ssr_runtime: String,
    #[serde(default = "default_ssr_timeout_ms")]
    pub ssr_timeout_ms: u64,
}

fn default_ssr_runtime() -> String {
    "node".into()
}

fn default_ssr_timeout_ms() -> u64 {
    5000
}

#[derive(Clone, Debug, Deserialize)]
pub struct WorkersConfig {
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
    #[serde(default = "default_lease_seconds")]
    pub lease_seconds: u64,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

fn default_concurrency() -> usize {
    4
}

fn default_poll_interval_ms() -> u64 {
    1_000
}

fn default_lease_seconds() -> u64 {
    300
}

fn default_timeout_seconds() -> u64 {
    240
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct LoggerConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_log_level() -> String {
    "info".into()
}

impl Config {
    pub fn load() -> Result<Self> {
        let environment = Environment::from_env();
        Self::load_from(Path::new("config").join(format!("{}.yaml", environment.as_str())), environment)
    }

    pub fn load_from(path: impl AsRef<Path>, environment: Environment) -> Result<Self> {
        let path = path.as_ref();
        let yaml = fs::read_to_string(path).map_err(|error| {
            Error::Config(format!("could not read {}: {error}", path.display()))
        })?;
        let mut config = Self::from_yaml(&yaml)?;
        config.environment = environment;
        config.database_url = env::var("DATABASE_URL").map_err(|_| {
            Error::Config("missing required environment variable DATABASE_URL".into())
        })?;
        if config.inertia.ssr_timeout_ms == 0 {
            return Err(Error::Config(
                "inertia.ssr_timeout_ms must be greater than zero".into(),
            ));
        }
        Ok(config)
    }

    pub fn from_yaml(yaml: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(yaml)?)
    }

    pub fn socket_addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.server.host, self.server.port)
            .parse()
            .map_err(|_| {
                Error::Config("server.host and server.port do not form a valid socket address".into())
            })
    }

    pub fn is_development(&self) -> bool {
        self.environment.is_development()
    }

    pub fn worker_config(&self) -> gurthang_jobs::Result<gurthang_jobs::WorkerConfig> {
        gurthang_jobs::WorkerConfig::new(
            self.workers.concurrency,
            Duration::from_millis(self.workers.poll_interval_ms),
            Duration::from_secs(self.workers.lease_seconds),
            Duration::from_secs(self.workers.timeout_seconds),
        )
    }

    pub fn pool_max_connections(&self) -> u32 {
        u32::try_from(self.workers.concurrency)
            .unwrap_or(u32::MAX)
            .saturating_add(10)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const SAMPLE: &str = r#"
server:
  host: 127.0.0.1
  port: 3000
  url: http://127.0.0.1:3000
session:
  secure: false
inertia:
  vite_dev_server_url: http://127.0.0.1:5173
  ssr_runtime: node
  ssr_timeout_ms: 5000
workers:
  concurrency: 4
  poll_interval_ms: 1000
  lease_seconds: 300
  timeout_seconds: 240
logger:
  level: info
"#;

    #[test]
    fn parses_development_yaml() {
        let config = Config::from_yaml(SAMPLE).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert!(!config.session.secure);
        assert_eq!(config.session.cookie, "gurthang.sid");
        assert_eq!(
            config.inertia.vite_dev_server_url.as_deref(),
            Some("http://127.0.0.1:5173")
        );
        assert_eq!(config.workers.concurrency, 4);
        assert!(config.database_url.is_empty());
    }

    #[test]
    fn load_from_temp_yaml_reads_and_requires_database_url() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("test.yaml");
        fs::write(&path, SAMPLE).unwrap();
        let _lock = ENV_LOCK.lock().unwrap();
        let previous = env::var("DATABASE_URL").ok();
        unsafe { env::remove_var("DATABASE_URL") };
        let error = Config::load_from(&path, Environment::Development).unwrap_err();
        assert!(error.to_string().contains("DATABASE_URL"));
        unsafe { env::set_var("DATABASE_URL", "postgres://postgres@localhost/app") };
        let config = Config::load_from(&path, Environment::Test).unwrap();
        match previous {
            Some(previous) => unsafe { env::set_var("DATABASE_URL", previous) },
            None => unsafe { env::remove_var("DATABASE_URL") },
        }
        assert_eq!(config.environment, Environment::Test);
        assert_eq!(config.database_url, "postgres://postgres@localhost/app");
        assert!(!config.is_development());
    }

    #[test]
    fn environment_parses_aliases() {
        assert_eq!(Environment::parse("dev"), Environment::Development);
        assert_eq!(Environment::parse("prod"), Environment::Production);
        assert_eq!(Environment::parse("staging").as_str(), "staging");
    }
}
