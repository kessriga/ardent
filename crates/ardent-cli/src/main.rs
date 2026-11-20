mod init;
mod project_root;
mod sync;
mod tracked;

use std::env;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
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
    /// Bring the structure of each tracked source file in `.ardent/cache/` up to date
    Sync,
}

fn main() -> ExitCode {
    let command = Cli::parse().command;
    let working_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(err) => return fail(format_args!("cannot read the working directory: {err}")),
    };
    match command {
        Command::Init => init(working_dir),
        Command::Sync => sync(working_dir),
    }
}

fn init(project_dir: PathBuf) -> ExitCode {
    match init::run(&project_dir) {
        Ok(ardent_dir) => {
            // The project exists now, so a closed stdout must not turn this run into a failure.
            let _ = writeln!(io::stdout(), "Created {}", ardent_dir.display());
            ExitCode::SUCCESS
        }
        Err(err) => fail(err),
    }
}

fn sync(working_dir: PathBuf) -> ExitCode {
    let report = match sync::run(&working_dir) {
        Ok(report) => report,
        Err(err) => return fail(err),
    };
    let mut stderr = io::stderr();
    for warning in &report.warnings {
        let _ = writeln!(stderr, "ardent: warning: {warning}");
    }
    for error in &report.errors {
        let _ = writeln!(stderr, "ardent: {error}");
    }
    let _ = writeln!(
        io::stdout(),
        "Synced {} tracked files: {} scanned, {} removed",
        report.tracked,
        report.scanned,
        report.removed
    );
    if report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn fail(message: impl fmt::Display) -> ExitCode {
    let _ = writeln!(io::stderr(), "ardent: {message}");
    ExitCode::FAILURE
}
