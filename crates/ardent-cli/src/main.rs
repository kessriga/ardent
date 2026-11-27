mod browse;
mod init;
mod project_root;
mod release;
mod sync;
mod tracked;
mod update;
mod xdg;

use std::env;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// A terminal outliner for structured text. Run without a command to browse the project.
#[derive(Parser)]
#[command(name = "ardent", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Make the working directory an Ardent project by creating `.ardent/` in it
    Init {
        /// Install these comma-separated format plugins in this project (e.g. markdown,rust). Omit to select in a terminal.
        #[arg(long, value_name = "NAMES")]
        plugins: Option<String>,
    },
    /// Bring the structure of each tracked source file in `.ardent/cache/` up to date
    Sync,
    /// Replace installed release format plugins with the copies bundled with this executable
    Update,
    /// Browse tracked files, their outlines, and source text in a terminal
    Browse,
}

fn main() -> ExitCode {
    let command = Cli::parse().command.unwrap_or(Command::Browse);
    let working_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(err) => return fail(format_args!("cannot read the working directory: {err}")),
    };
    match command {
        Command::Init { plugins } => init(working_dir, plugins.as_deref()),
        Command::Update => match update::run(&working_dir) {
            Ok(report) => {
                let mut out = io::stdout().lock();
                if let Some(names) = report.names() {
                    let _ = writeln!(out, "Updated {names}");
                } else if report.errors.is_empty() {
                    let _ = writeln!(out, "Installed release plugins are up to date.");
                }
                for warning in &report.warnings {
                    let _ = writeln!(io::stderr(), "ardent: warning: {warning}");
                }
                for error in &report.errors {
                    let _ = writeln!(io::stderr(), "ardent: {error}");
                }
                if report.errors.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(err) => fail(err),
        },
        Command::Sync => sync(working_dir),
        Command::Browse => match browse::run(&working_dir) {
            Ok(status) => status,
            Err(err) => fail(err),
        },
    }
}

fn init(project_dir: PathBuf, plugins: Option<&str>) -> ExitCode {
    match init::run(&project_dir, plugins) {
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
    for (path, error) in &report.files {
        if let Some(error) = error {
            let _ = writeln!(stderr, "ardent: {path}: {error}");
        }
    }
    let _ = writeln!(
        io::stdout(),
        "Synced {} tracked files: {} scanned, {} removed",
        report.files.len(),
        report.scanned,
        report.removed
    );
    if report.errors.is_empty() && report.files.iter().all(|(_, err)| err.is_none()) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn fail(message: impl fmt::Display) -> ExitCode {
    let _ = writeln!(io::stderr(), "ardent: {message}");
    ExitCode::FAILURE
}
