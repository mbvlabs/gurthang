mod controller;
mod env;
mod factory;
mod job;
mod migration;
mod model;
mod naming;
mod prepare;
mod region;
mod registration;
mod schema;
mod sync_payloads;
mod sync_routes;
mod tmpl;
mod wiring;

use std::io::Write;

pub use controller::ControllerOptions;
pub use model::ModelOptions;

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

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<askama::Error> for Error {
    fn from(error: askama::Error) -> Self {
        Self::Message(error.to_string())
    }
}

#[derive(Clone, Debug, Default)]
pub struct GenerateOptions {
    pub dry_run: bool,
}

pub fn generate_migration(
    name: &str,
    options: GenerateOptions,
    out: &mut impl Write,
) -> Result<()> {
    migration::generate(name, options, out)
}

pub fn generate_model(name: &str, options: ModelOptions, out: &mut impl Write) -> Result<()> {
    model::generate(name, options.clone(), out)?;
    if !options.dry_run {
        prepare::run(false, out)?;
    }
    Ok(())
}

pub fn sync_model(name: &str, check: bool, out: &mut impl Write) -> Result<()> {
    model::sync(name, check, out)?;
    prepare::run(check, out)
}

pub fn generate_controller(
    name: &str,
    options: ControllerOptions,
    out: &mut impl Write,
) -> Result<()> {
    controller::generate(name, options, out)
}

pub fn generate_scaffold(
    name: &str,
    model: ModelOptions,
    controller: ControllerOptions,
    out: &mut impl Write,
) -> Result<()> {
    let dry_run = model.dry_run || controller.dry_run;
    model::generate(name, model, out)?;
    controller::generate(name, controller, out)?;
    if !dry_run {
        prepare::run(false, out)?;
    }
    Ok(())
}

pub fn generate_job(name: &str, options: GenerateOptions, out: &mut impl Write) -> Result<()> {
    job::generate(name, options.clone(), out)?;
    if !options.dry_run {
        prepare::run(false, out)?;
    }
    Ok(())
}

pub fn sync_factory(name: &str, check: bool, out: &mut impl Write) -> Result<()> {
    factory::sync_one(name, check, out)
}

pub fn sync_factories(check: bool, sync: bool, out: &mut impl Write) -> Result<()> {
    factory::sync_all(check, sync, out)
}

pub fn sync_routes(check: bool, out: &mut impl Write) -> Result<()> {
    sync_routes::sync(check, out)
}

pub fn print_routes(out: &mut impl Write) -> Result<()> {
    sync_routes::print(out)
}

pub fn sync_payloads(check: bool, out: &mut impl Write) -> Result<()> {
    prepare::run(check, out)?;
    sync_payloads::sync(check, out)
}
