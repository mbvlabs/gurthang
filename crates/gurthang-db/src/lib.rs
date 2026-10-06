use std::{
    env,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process::Command,
};

use gurthang_project::find_root;
use sqlx::{
    Connection, PgConnection, postgres::PgConnectOptions,
    migrate::Migrator,
};

#[derive(Debug)]
pub enum Error {
    Message(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<gurthang_project::Error> for Error {
    fn from(error: gurthang_project::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Message(error.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct DbOptions {
    pub force: bool,
}

pub fn create(out: &mut impl Write) -> Result<()> {
    let root = project_root()?;
    runtime()?.block_on(create_async(&root, out))
}

pub fn drop(options: DbOptions, out: &mut impl Write) -> Result<()> {
    let root = project_root()?;
    confirm_destructive(&options, "drop the database")?;
    runtime()?.block_on(drop_async(&root, out))
}

pub fn nuke(options: DbOptions, out: &mut impl Write) -> Result<()> {
    let root = project_root()?;
    confirm_destructive(&options, "drop and recreate the database")?;
    runtime()?.block_on(async {
        drop_async(&root, out).await?;
        create_async(&root, out).await
    })
}

pub fn rebuild(options: DbOptions, out: &mut impl Write) -> Result<()> {
    let root = project_root()?;
    confirm_destructive(&options, "rebuild the database")?;
    runtime()?.block_on(async {
        drop_async(&root, out).await?;
        create_async(&root, out).await?;
        migrate_up_async(&root, out).await
    })?;
    seed(Some("development"), false, out)
}

pub fn migrate_up(out: &mut impl Write) -> Result<()> {
    let root = project_root()?;
    runtime()?.block_on(migrate_up_async(&root, out))
}

pub fn migrate_status(out: &mut impl Write) -> Result<()> {
    let root = project_root()?;
    runtime()?.block_on(migrate_status_async(&root, out))
}

pub fn seed(name: Option<&str>, list: bool, out: &mut impl Write) -> Result<()> {
    let root = project_root()?;
    load_env(&root);
    let mut command = Command::new("cargo");
    command.arg("run").arg("--bin").arg("seed").arg("--");
    if list {
        command.arg("--list");
    } else {
        command.arg(name.unwrap_or("development"));
    }
    command.current_dir(&root);
    let status = command.status()?;
    if !status.success() {
        return Err(Error::Message(format!("seed failed ({status})")));
    }
    if list {
        writeln!(out, "Listed seeds.")?;
    } else {
        writeln!(out, "Seed finished.")?;
    }
    Ok(())
}

fn project_root() -> Result<PathBuf> {
    Ok(find_root()?)
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Runtime::new().map_err(|error| Error::Message(error.to_string()))
}

fn load_env(root: &Path) {
    let env_path = root.join(".env");
    if env_path.is_file() {
        let _ = dotenvy::from_path(env_path);
    }
}

fn database_url(root: &Path) -> Result<String> {
    load_env(root);
    env::var("DATABASE_URL").map_err(|_| {
        Error::Message("DATABASE_URL is not set; copy .env.example to .env".into())
    })
}

fn confirm_destructive(options: &DbOptions, action: &str) -> Result<()> {
    if options.force {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        return Err(Error::Message(format!(
            "refusing to {action} without --force in a non-interactive session"
        )));
    }
    eprint!("This will {action}. Type yes to continue: ");
    let _ = io::stderr().flush();
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    if line.trim() != "yes" {
        return Err(Error::Message("aborted".into()));
    }
    Ok(())
}

async fn create_async(root: &Path, out: &mut impl Write) -> Result<()> {
    let url = database_url(root)?;
    let (admin, name) = admin_url(&url)?;
    let mut connection = PgConnection::connect(&admin).await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
        .bind(&name)
        .fetch_one(&mut connection)
        .await?;
    if exists {
        writeln!(out, "Database {name} already exists.")?;
        return Ok(());
    }
    let ident = quote_ident(&name)?;
    sqlx::query(&format!("CREATE DATABASE {ident}"))
        .execute(&mut connection)
        .await?;
    writeln!(out, "Created database {name}.")?;
    Ok(())
}

async fn drop_async(root: &Path, out: &mut impl Write) -> Result<()> {
    let url = database_url(root)?;
    let (admin, name) = admin_url(&url)?;
    let mut connection = PgConnection::connect(&admin).await?;
    let ident = quote_ident(&name)?;
    sqlx::query(&format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{name}' AND pid <> pg_backend_pid()"
    ))
    .execute(&mut connection)
    .await
    .ok();
    sqlx::query(&format!("DROP DATABASE IF EXISTS {ident}"))
        .execute(&mut connection)
        .await?;
    writeln!(out, "Dropped database {name}.")?;
    Ok(())
}

async fn migrate_up_async(root: &Path, out: &mut impl Write) -> Result<()> {
    let url = database_url(root)?;
    let mut connection = PgConnection::connect(&url).await?;
    let migrator = Migrator::new(root.join("migrations"))
        .await
        .map_err(|error| Error::Message(error.to_string()))?;
    migrator
        .run(&mut connection)
        .await
        .map_err(|error| Error::Message(error.to_string()))?;
    writeln!(out, "Migrations applied.")?;
    sqlx_prepare(&root, out)?;
    Ok(())
}

fn sqlx_prepare(root: &Path, out: &mut impl Write) -> Result<()> {
    let mut command = Command::new("cargo");
    command
        .args(["sqlx", "prepare", "--workspace"])
        .env_remove("SQLX_OFFLINE")
        .current_dir(root);
    writeln!(out, "+ cargo sqlx prepare --workspace")?;
    let status = command.status()?;
    if !status.success() {
        return Err(Error::Message(format!(
            "cargo sqlx prepare failed ({status})"
        )));
    }
    writeln!(out, "Updated SQLx offline query data")?;
    Ok(())
}

async fn migrate_status_async(root: &Path, out: &mut impl Write) -> Result<()> {
    let url = database_url(root)?;
    let mut connection = PgConnection::connect(&url).await?;
    let applied: Vec<(i64, String)> = sqlx::query_as(
        "SELECT version, description FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap_or_default();
    if applied.is_empty() {
        writeln!(out, "No applied migrations.")?;
        return Ok(());
    }
    for (version, description) in applied {
        writeln!(out, "{version}\t{description}")?;
    }
    Ok(())
}

fn admin_url(database_url: &str) -> Result<(String, String)> {
    let options: PgConnectOptions = database_url
        .parse()
        .map_err(|error| Error::Message(format!("invalid DATABASE_URL: {error}")))?;
    let name = options
        .get_database()
        .unwrap_or("postgres")
        .to_owned();
    if name == "postgres" {
        return Err(Error::Message(
            "refusing to manage the postgres admin database".into(),
        ));
    }
    let admin = strip_database_name(database_url)?;
    Ok((admin, name))
}

fn strip_database_name(database_url: &str) -> Result<String> {
    let Some((prefix, rest)) = database_url.rsplit_once('/') else {
        return Err(Error::Message("DATABASE_URL has no database name".into()));
    };
    let name = rest.split('?').next().unwrap_or(rest);
    if name.is_empty() {
        return Err(Error::Message("DATABASE_URL has no database name".into()));
    }
    let suffix = rest.strip_prefix(name).unwrap_or("");
    Ok(format!("{prefix}/postgres{suffix}"))
}

fn quote_ident(name: &str) -> Result<String> {
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        || name.is_empty()
    {
        return Err(Error::Message(format!(
            "refusing to use unsafe database name {name:?}"
        )));
    }
    Ok(format!("\"{name}\""))
}
