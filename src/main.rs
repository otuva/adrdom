mod banned;
mod commands;
mod ledger;
mod record;
mod repo;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// adr: a control layer between AI coding agents and a repository.
#[derive(Parser)]
#[command(
    name = "adr",
    about = "A control layer between AI coding agents and a repository",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Set up adrdom/ in the current repository.
    Init,
    /// Assign ids and record `asked` for every unasked/changed question in a record.
    Ask { record: PathBuf },
    /// Owner-only: answer a question by id with an option letter.
    Lock { id: String, letter: String },
    /// Print every decision currently in force.
    Show { record: Option<PathBuf> },
    /// Read lines from stdin and ban them (and their tokens) from the repo.
    Ban,
    /// Owner-only: record that a check's failure is accepted for now.
    Override {
        check: String,
        id: String,
        #[arg(long)]
        reason: String,
    },
    /// Run all governance checks.
    Check {
        #[arg(long)]
        staged: bool,
        #[arg(long)]
        message: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Init => commands::init::run(),
        Command::Ask { record } => commands::ask::run(record),
        Command::Lock { id, letter } => commands::lock::run(id, letter),
        Command::Show { record } => commands::show::run(record),
        Command::Ban => commands::ban::run(),
        Command::Override { check, id, reason } => commands::override_cmd::run(check, id, reason),
        Command::Check { staged, message } => commands::check::run(staged, message),
    };
    std::process::exit(code);
}
