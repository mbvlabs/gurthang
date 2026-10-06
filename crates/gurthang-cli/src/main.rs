use std::process::ExitCode;

use clap::Parser;
use gurthang_cli::cli::Cli;

#[cfg(not(target_os = "linux"))]
compile_error!("Gurthang supports Linux only");

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut stdout = std::io::stdout().lock();

    match gurthang_cli::run(cli, &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
