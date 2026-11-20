mod init;
mod project_root;

use std::env;
use std::fmt;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// A terminal outliner for structured text.
#[derive(Parser)]
#[command(name = "ardent", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Make the working directory an Ardent project by creating `.ardent/` in it
    Init,
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Init => init(),
    }
}

fn init() -> ExitCode {
    let project_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(err) => return fail(format_args!("cannot read the working directory: {err}")),
    };
    match init::run(&project_dir) {
        Ok(ardent_dir) => {
            println!("Created {}", ardent_dir.display());
            ExitCode::SUCCESS
        }
        Err(err) => fail(err),
    }
}

fn fail(message: impl fmt::Display) -> ExitCode {
    eprintln!("ardent: {message}");
    ExitCode::FAILURE
}
