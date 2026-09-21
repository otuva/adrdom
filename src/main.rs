mod actions;
mod banned;
mod commands;
mod ledger;
mod record;
mod repo;
mod state;
mod terms;
mod view;

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
    Init {
        /// Also merge the deny entries into .claude/settings.json.
        #[arg(long)]
        claude: bool,
    },
    /// Assign ids and record `asked` for every unasked/changed question in a record.
    Ask { record: PathBuf },
    /// Owner-only: approve a record's plan/context, or the terms file.
    Approve { file: PathBuf },
    /// Owner-only: answer a question by id with an option letter.
    Lock { id: String, letter: String },
    /// Print every decision currently in force.
    Show { record: Option<PathBuf> },
    /// Read lines from stdin and ban them (and their tokens) from the repo.
    Ban {
        /// Carry an existing terms.md entry's uuid alongside each new hash,
        /// matched case-insensitively by name or alias.
        #[arg(long)]
        term: Option<String>,
    },
    /// Owner-only: read lines from stdin and remove them (and their tokens)
    /// from the ban list.
    Unban,
    /// Owner-only: serve the owner's page on 127.0.0.1.
    Serve {
        #[arg(long, default_value_t = 7357)]
        port: u16,
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
        Command::Init { claude } => commands::init::run(claude),
        Command::Ask { record } => commands::ask::run(record),
        Command::Approve { file } => commands::approve::run(file),
        Command::Lock { id, letter } => commands::lock::run(id, letter),
        Command::Show { record } => commands::show::run(record),
        Command::Ban { term } => commands::ban::run(term),
        Command::Unban => commands::unban::run(),
        Command::Serve { port } => commands::serve::run(port),
        Command::Check { staged, message } => commands::check::run(staged, message),
    };
    std::process::exit(code);
}
