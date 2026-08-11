use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "gurthang",
    version,
    about = "Create a small Rust web application"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new Gurthang application.
    New(NewArgs),
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
