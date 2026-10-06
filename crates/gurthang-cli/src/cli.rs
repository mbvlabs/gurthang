use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "gurthang",
    version,
    about = "Create and develop a small Rust web application"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new Gurthang application.
    New(NewArgs),
    /// Start the development server with live reload.
    #[command(visible_alias = "r")]
    Run,
    /// Write application code from the database or a name.
    Generate(GenerateArgs),
    /// Refresh generated regions and frontend contracts.
    Sync(SyncArgs),
    /// Create, migrate, and seed the application database.
    Db(DbArgs),
    /// Build frontend assets and a release binary.
    Build,
}

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Cargo package name for the application.
    pub name: String,
    /// Destination directory. Defaults to ./<name>.
    #[arg(long, value_name = "DIRECTORY")]
    pub path: Option<PathBuf>,
    /// Print the target and file manifest without writing files.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct GenerateArgs {
    #[command(subcommand)]
    pub command: GenerateCommand,
}

#[derive(Debug, Subcommand)]
pub enum GenerateCommand {
    /// Write the next SQLx migration file.
    Migration {
        name: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Write a model from the live database schema.
    Model {
        name: String,
        #[arg(long)]
        table: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Write a controller, routes, views, and React pages.
    Controller {
        name: String,
        actions: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Write a model and its controller scaffold.
    Scaffold {
        name: String,
        #[arg(long)]
        table: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Add a typed background job variant.
    Job {
        name: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub command: SyncCommand,
}

#[derive(Debug, Subcommand)]
pub enum SyncCommand {
    /// Rewrite a model's generated region from the live schema.
    Model {
        name: String,
        #[arg(long)]
        check: bool,
    },
    /// Write resources/js/routes.ts from src/routes.
    Routes {
        #[arg(long)]
        check: bool,
    },
    /// Export ts-rs page contracts into resources/js/generated.
    Payloads {
        #[arg(long)]
        check: bool,
    },
    /// Rewrite one factory from the live schema.
    Factory {
        name: String,
        #[arg(long)]
        check: bool,
    },
    /// Check or rewrite every factory.
    Factories {
        #[arg(long)]
        check: bool,
        #[arg(long)]
        sync: bool,
    },
}

#[derive(Debug, Args)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: DbCommand,
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    /// Create the database named in DATABASE_URL.
    Create,
    /// Drop the database named in DATABASE_URL.
    Drop {
        #[arg(long)]
        force: bool,
    },
    /// Drop and create the database.
    Nuke {
        #[arg(long)]
        force: bool,
    },
    /// Drop, create, migrate, and seed the database.
    Rebuild {
        #[arg(long)]
        force: bool,
    },
    /// Apply or inspect SQLx migrations.
    Migrate {
        #[command(subcommand)]
        command: MigrateCommand,
    },
    /// Run `cargo run --bin seed`.
    Seed {
        name: Option<String>,
        #[arg(long)]
        list: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum MigrateCommand {
    /// Apply pending SQLx migrations.
    Up,
    /// Print applied migrations.
    Status,
}
