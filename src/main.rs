mod actions;
mod banned;
mod commands;
mod depends;
mod ledger;
mod msg;
mod record;
mod repo;
mod state;
mod tells;
mod terms;
mod version;
mod view;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use version::VERSION;

/// adr: a control layer between AI coding agents and a repository.
#[derive(Parser)]
#[command(
    name = "adr",
    about = "A control layer between AI coding agents and a repository",
    version = VERSION
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Set up adrdom/ and AGENTS.md in the current repository.
    Install {
        /// Also create CLAUDE.md with the AGENTS.md import when it is missing, and merge the deny entries into .claude/settings.json.
        #[arg(long)]
        claude: bool,
        /// Also write .codex/rules/adrdom.rules, forbidding the owner's commands under Codex's project rules.
        #[arg(long)]
        codex: bool,
    },
    /// Owner-only: remove what `adr install` wrote outside adrdom/.
    Uninstall,
    /// Assign ids and record `asked` for every unasked/changed question in a topic file.
    Ask { topic_file: PathBuf },
    /// Owner-only: approve a topic file's plan/context, or the terms file.
    Approve { file: PathBuf },
    /// Owner-only: answer a question by id with an option letter.
    Answer { id: String, letter: String },
    /// Print every current decision.
    Show { topic_file: Option<PathBuf> },
    /// Read lines from stdin and ban them (and their tokens) from the repo.
    Ban {
        /// Carry an existing terms.md term's uuid alongside each new hash,
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
        Command::Install { claude, codex } => commands::install::run(claude, codex),
        Command::Uninstall => commands::uninstall::run(),
        Command::Ask { topic_file } => commands::ask::run(topic_file),
        Command::Approve { file } => commands::approve::run(file),
        Command::Answer { id, letter } => commands::answer::run(id, letter),
        Command::Show { topic_file } => commands::show::run(topic_file),
        Command::Ban { term } => commands::ban::run(term),
        Command::Unban => commands::unban::run(),
        Command::Serve { port } => commands::serve::run(port),
        Command::Check { staged, message } => commands::check::run(staged, message),
    };
    std::process::exit(code);
}
