pub mod cli;
pub mod error;
pub mod new;
pub mod project_name;
pub mod renderer;
pub mod run;

use std::io::Write;

use cli::{Cli, Command};
use error::Result;

pub fn run(cli: Cli, out: &mut impl Write) -> Result<()> {
    match cli.command {
        Command::New(args) => new::execute(args, out),
        Command::Run => run::execute(out),
    }
}
