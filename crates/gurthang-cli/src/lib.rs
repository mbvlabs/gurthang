pub mod cli;
pub mod error;

use std::{io::Write, path::PathBuf};

use cli::{
    Cli, Command, DbCommand, GenerateCommand, MigrateCommand, SyncCommand,
};
use error::{Error, Result};

pub fn run(cli: Cli, out: &mut impl Write) -> Result<()> {
    match cli.command {
        Command::New(args) => gurthang_new::execute(
            &args.name,
            args.path,
            args.dry_run,
            &source_root(),
            out,
        )?,
        Command::Run => gurthang_run::execute(out)?,
        Command::Generate(args) => generate(args.command, out)?,
        Command::Sync(args) => sync(args.command, out)?,
        Command::Db(args) => db(args.command, out)?,
        Command::Build => gurthang_build::execute(out)?,
    }
    Ok(())
}

fn generate(command: GenerateCommand, out: &mut impl Write) -> Result<()> {
    match command {
        GenerateCommand::Migration { name, dry_run } => {
            gurthang_generate::generate_migration(
                &name,
                gurthang_generate::GenerateOptions { dry_run },
                out,
            )?;
        }
        GenerateCommand::Model {
            name,
            table,
            dry_run,
        } => {
            gurthang_generate::generate_model(
                &name,
                gurthang_generate::ModelOptions { table, dry_run },
                out,
            )?;
        }
        GenerateCommand::Controller {
            name,
            actions,
            dry_run,
        } => {
            gurthang_generate::generate_controller(
                &name,
                gurthang_generate::ControllerOptions { actions, dry_run },
                out,
            )?;
        }
        GenerateCommand::Scaffold {
            name,
            table,
            dry_run,
        } => {
            gurthang_generate::generate_scaffold(
                &name,
                gurthang_generate::ModelOptions {
                    table,
                    dry_run,
                },
                gurthang_generate::ControllerOptions {
                    actions: Vec::new(),
                    dry_run,
                },
                out,
            )?;
        }
        GenerateCommand::Job { name, dry_run } => {
            gurthang_generate::generate_job(
                &name,
                gurthang_generate::GenerateOptions { dry_run },
                out,
            )?;
        }
    }
    Ok(())
}

fn sync(command: SyncCommand, out: &mut impl Write) -> Result<()> {
    match command {
        SyncCommand::Model { name, check } => {
            gurthang_generate::sync_model(&name, check, out)?;
        }
        SyncCommand::Routes { check } => gurthang_generate::sync_routes(check, out)?,
        SyncCommand::Payloads { check } => gurthang_generate::sync_payloads(check, out)?,
        SyncCommand::Factory { name, check } => {
            gurthang_generate::sync_factory(&name, check, out)?;
        }
        SyncCommand::Factories { check, sync } => {
            gurthang_generate::sync_factories(check, sync, out)?;
        }
    }
    Ok(())
}

fn db(command: DbCommand, out: &mut impl Write) -> Result<()> {
    match command {
        DbCommand::Create => gurthang_db::create(out)?,
        DbCommand::Drop { force } => gurthang_db::drop(gurthang_db::DbOptions { force }, out)?,
        DbCommand::Nuke { force } => gurthang_db::nuke(gurthang_db::DbOptions { force }, out)?,
        DbCommand::Rebuild { force } => {
            gurthang_db::rebuild(gurthang_db::DbOptions { force }, out)?;
        }
        DbCommand::Migrate { command } => match command {
            MigrateCommand::Up => gurthang_db::migrate_up(out)?,
            MigrateCommand::Status => gurthang_db::migrate_status(out)?,
        },
        DbCommand::Seed { name, list } => {
            gurthang_db::seed(name.as_deref(), list, out)?;
        }
    }
    Ok(())
}

fn source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

impl From<gurthang_new::Error> for Error {
    fn from(error: gurthang_new::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<gurthang_run::Error> for Error {
    fn from(error: gurthang_run::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<gurthang_generate::Error> for Error {
    fn from(error: gurthang_generate::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<gurthang_db::Error> for Error {
    fn from(error: gurthang_db::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<gurthang_build::Error> for Error {
    fn from(error: gurthang_build::Error) -> Self {
        Self::Message(error.to_string())
    }
}
