pub mod cli;
pub mod error;

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use cli::{Cli, Command, DbCommand, GenerateCommand, MigrateCommand, SyncCommand, ToolsCommand};
use error::{Error, Result};

pub fn run(cli: Cli, out: &mut impl Write) -> Result<()> {
    match cli.command {
        Command::New(args) => {
            gurthang_new::execute(&args.name, args.path, args.dry_run, &source()?, out)?
        }
        Command::Run => gurthang_run::execute(out)?,
        Command::Generate(args) => generate(args.command, out)?,
        Command::Sync(args) => sync(args.command, out)?,
        Command::Routes => gurthang_generate::print_routes(out)?,
        Command::Task { name } => task(name.as_deref(), out)?,
        Command::Db(args) => db(args.command, out)?,
        Command::Build => gurthang_build::execute(out)?,
        Command::Tools(args) => match args.command {
            None | Some(ToolsCommand::Check) => gurthang_tools::check(out)?,
            Some(ToolsCommand::Sync) => gurthang_tools::sync(out)?,
        },
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
                gurthang_generate::ModelOptions { table, dry_run },
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

fn task(name: Option<&str>, out: &mut impl Write) -> Result<()> {
    let current = std::env::current_dir().map_err(|error| {
        Error::Message(format!("could not determine current directory: {error}"))
    })?;
    let root = gurthang_project::find_root_from(&current)?;
    let bin = gurthang_project::GurthangToml::load(&root)?.project.name;
    let mut command = std::process::Command::new("cargo");
    command
        .arg("run")
        .arg("--bin")
        .arg(&bin)
        .arg("--")
        .arg("task")
        .current_dir(&root);
    if let Some(name) = name {
        command.arg(name);
    } else {
        command.arg("--list");
    }
    let status = command
        .status()
        .map_err(|error| Error::Message(format!("could not run cargo: {error}")))?;
    if !status.success() {
        return Err(Error::Message(format!("task exited with {status}")));
    }
    let _ = out;
    Ok(())
}

fn source() -> Result<gurthang_new::Source> {
    resolve_source(
        std::env::var_os("GURTHANG_ROOT").map(PathBuf::from),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        cargo_home(),
        env!("GURTHANG_GIT_URL"),
        option_env_rev(env!("GURTHANG_GIT_REV")),
    )
}

fn resolve_source(
    gurthang_root: Option<PathBuf>,
    compiled_root: PathBuf,
    cargo_home: Option<PathBuf>,
    git_url: &str,
    git_rev: Option<&str>,
) -> Result<gurthang_new::Source> {
    if let Some(root) = gurthang_root {
        if is_checkout(&root) {
            return Ok(gurthang_new::Source::Path(root));
        }
        return Err(Error::Message(format!(
            "GURTHANG_ROOT is not a gurthang checkout: {}",
            root.display()
        )));
    }

    if is_developer_checkout(&compiled_root, cargo_home.as_deref()) {
        return Ok(gurthang_new::Source::Path(compiled_root));
    }

    Ok(gurthang_new::Source::Git {
        url: git_url.to_string(),
        rev: git_rev.map(str::to_string),
    })
}

fn is_developer_checkout(root: &Path, cargo_home: Option<&Path>) -> bool {
    if !is_checkout(root) {
        return false;
    }
    match cargo_home {
        Some(cargo_home) if root.starts_with(cargo_home) => false,
        _ => true,
    }
}

fn is_checkout(root: &Path) -> bool {
    root.join("crates/gurthang/Cargo.toml").is_file()
}

fn cargo_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("CARGO_HOME") {
        return Some(PathBuf::from(home));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo"))
}

fn option_env_rev(rev: &str) -> Option<&str> {
    if rev.is_empty() { None } else { Some(rev) }
}

impl From<gurthang_project::Error> for Error {
    fn from(error: gurthang_project::Error) -> Self {
        Self::Message(error.to_string())
    }
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

impl From<gurthang_tools::Error> for Error {
    fn from(error: gurthang_tools::Error) -> Self {
        Self::Message(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fake_checkout(root: &Path) {
        let crate_dir = root.join("crates/gurthang");
        fs::create_dir_all(&crate_dir).unwrap();
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"gurthang\"\n",
        )
        .unwrap();
    }

    #[test]
    fn gurthang_root_uses_the_checkout() {
        let temp = tempfile::tempdir().unwrap();
        fake_checkout(temp.path());
        let source = resolve_source(
            Some(temp.path().to_path_buf()),
            PathBuf::from("/missing"),
            None,
            "https://example.com/gurthang",
            Some("abc"),
        )
        .unwrap();
        assert_eq!(
            source,
            gurthang_new::Source::Path(temp.path().to_path_buf())
        );
    }

    #[test]
    fn invalid_gurthang_root_errors() {
        let temp = tempfile::tempdir().unwrap();
        let error = resolve_source(
            Some(temp.path().to_path_buf()),
            PathBuf::from("/missing"),
            None,
            "https://example.com/gurthang",
            Some("abc"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("GURTHANG_ROOT"));
    }

    #[test]
    fn compiled_checkout_is_used_when_present() {
        let temp = tempfile::tempdir().unwrap();
        fake_checkout(temp.path());
        let source = resolve_source(
            None,
            temp.path().to_path_buf(),
            None,
            "https://example.com/gurthang",
            Some("abc"),
        )
        .unwrap();
        assert_eq!(
            source,
            gurthang_new::Source::Path(temp.path().to_path_buf())
        );
    }

    #[test]
    fn cargo_install_git_checkout_uses_git() {
        let cargo_home = tempfile::tempdir().unwrap();
        let root = cargo_home
            .path()
            .join("git/checkouts/gurthang-deadbeef/abc123");
        fake_checkout(&root);
        let source = resolve_source(
            None,
            root,
            Some(cargo_home.path().to_path_buf()),
            "https://example.com/gurthang",
            Some("abc"),
        )
        .unwrap();
        assert_eq!(
            source,
            gurthang_new::Source::Git {
                url: "https://example.com/gurthang".into(),
                rev: Some("abc".into()),
            }
        );
    }

    #[test]
    fn missing_compiled_checkout_uses_git() {
        let source = resolve_source(
            None,
            PathBuf::from("/missing/gurthang"),
            None,
            "https://example.com/gurthang",
            Some("abc"),
        )
        .unwrap();
        assert_eq!(
            source,
            gurthang_new::Source::Git {
                url: "https://example.com/gurthang".into(),
                rev: Some("abc".into()),
            }
        );
    }
}
