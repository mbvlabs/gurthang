use std::{env, path::Path};

use crate::Error;

pub fn load_env(root: &Path) {
    let env_path = root.join(".env");
    if env_path.is_file() {
        let _ = dotenvy::from_path(env_path);
    }
}

pub fn database_url(root: &Path) -> Result<String, Error> {
    load_env(root);
    env::var("DATABASE_URL").map_err(|_| {
        Error::Message(
            "DATABASE_URL is not set; run gurthang db migrate up after copying .env.example".into(),
        )
    })
}

pub fn runtime() -> Result<tokio::runtime::Runtime, Error> {
    tokio::runtime::Runtime::new().map_err(|error| Error::Message(error.to_string()))
}
