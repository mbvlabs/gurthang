use std::{
    env,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use gurthang_project::{GurthangToml, find_root};

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

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Message(error.to_string())
    }
}

pub fn execute(out: &mut impl Write) -> Result<()> {
    let root = find_root()?;
    let toml = GurthangToml::load(&root)?;
    load_env(&root);

    if !root.join("node_modules").is_dir() {
        run(&root, "npm", &["install"], out)?;
    }
    run(&root, "npm", &["run", "build"], out)?;
    run(&root, "npm", &["run", "build:ssr"], out)?;

    if env::var("DATABASE_URL").is_ok() {
        let _ = run(&root, "cargo", &["sqlx", "prepare"], out);
    } else {
        writeln!(
            out,
            "DATABASE_URL is unset; using committed .sqlx offline metadata"
        )?;
    }

    let mut prepare = Command::new("cargo");
    prepare
        .args(["build", "--release"])
        .env("SQLX_OFFLINE", "true")
        .current_dir(&root);
    writeln!(out, "+ SQLX_OFFLINE=true cargo build --release")?;
    let status = prepare.status()?;
    if !status.success() {
        return Err(Error::Message(format!(
            "cargo build --release failed ({status})"
        )));
    }

    let binary = release_binary(&root, &toml.project.name);
    writeln!(out, "Built {}", binary.display())?;
    Ok(())
}

fn load_env(root: &Path) {
    let env_path = root.join(".env");
    if env_path.is_file() {
        let _ = dotenvy::from_path(env_path);
    }
}

fn run(root: &Path, program: &str, args: &[&str], out: &mut impl Write) -> Result<()> {
    writeln!(out, "+ {program} {}", args.join(" "))?;
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()?;
    if !status.success() {
        return Err(Error::Message(format!(
            "{program} {} failed ({status})",
            args.join(" ")
        )));
    }
    Ok(())
}

fn release_binary(root: &Path, name: &str) -> PathBuf {
    root.join("target/release").join(name)
}
