use adr::commands;
use adr::version::VERSION;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use std::path::PathBuf;

/// adr: a control layer between AI coding agents and a repository.
#[derive(Parser)]
#[command(
    name = "adr",
    about = "A control layer between AI coding agents and a repository"
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
    /// Record `asked` for every unasked/changed question in an approved topic file.
    Ask { topic_file: PathBuf },
    /// Owner-only: approve a topic file's plan/context, or the terms file.
    Approve { file: PathBuf },
    /// Owner-only: answer a question, identified by `<id>@<version>`, with an option letter or `refuse`.
    Answer { id: String, letter: String },
    /// Print the current decisions and what is next; with a topic file or --all, every question.
    Show {
        /// Print every question of this topic file, with its state and what it stands on.
        topic_file: Option<PathBuf>,
        /// Print every question of every topic file, and the blame files.
        #[arg(long)]
        all: bool,
    },
    /// Read lines from stdin and ban them (and their tokens) from the repo.
    Ban {
        /// Carry an existing terms.md term's uuid alongside each new hash,
        /// matched case-insensitively by name or alias.
        #[arg(long, conflicts_with = "rebuild")]
        term: Option<String>,
        /// Rewrite the ban list from the ledger alone, ignoring whatever it
        /// currently holds; reads no stdin and writes no ledger row.
        #[arg(long)]
        rebuild: bool,
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
        /// Judge the files as they are staged for the commit, not as they are on disk.
        #[arg(long, conflicts_with = "push")]
        staged: bool,
        /// Judge the staged files and look for banned words in this commit message file.
        #[arg(long, conflicts_with = "push")]
        message: Option<PathBuf>,
        /// Print what failed and nothing else.
        #[arg(long)]
        quiet: bool,
        /// Also require every ledger row of <commit>'s own ledger to be present in the one being judged.
        #[arg(long, conflicts_with = "push")]
        since: Option<String>,
        /// Read git's pre-push input from stdin and judge only that: no ref may drop a ledger row the remote already has.
        #[arg(long)]
        push: bool,
    },
}

/// The exit code of a command whose reader went away, as a shell reports a
/// process ended by a closed pipe (128 + 13).
const READER_GONE: i32 = 141;

/// `adr show | head` closes the pipe while the tool still prints, and a
/// print to a closed pipe panics. That is no fault of the tool: the process
/// ends without a word, with `READER_GONE`, so a pipeline that watches exit
/// codes never reads it as a pass. Every other panic is printed as before.
fn end_quietly_when_the_reader_is_gone() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let text = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("");
        if text.starts_with("failed printing to std") && text.contains("Broken pipe") {
            std::process::exit(READER_GONE);
        }
        default_hook(info);
    }));
}

fn main() {
    end_quietly_when_the_reader_is_gone();
    let shown: &'static str = Box::leak(
        adr::msg::version_shown(VERSION, cfg!(feature = "owner-bypass")).into_boxed_str(),
    );
    let matches = Cli::command().version(shown).get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    let code = match cli.command {
        Command::Install { claude, codex } => commands::install::run(claude, codex),
        Command::Uninstall => commands::uninstall::run(),
        Command::Ask { topic_file } => commands::ask::run(topic_file),
        Command::Approve { file } => commands::approve::run(file),
        Command::Answer { id, letter } => commands::answer::run(id, letter),
        Command::Show { topic_file, all } => commands::show::run(topic_file, all),
        Command::Ban { term, rebuild } => commands::ban::run(term, rebuild),
        Command::Unban => commands::unban::run(),
        Command::Serve { port } => commands::serve::run(port),
        Command::Check { staged, message, quiet, since, push } => {
            commands::check::run(staged, message, since, push, quiet)
        }
    };
    std::process::exit(code);
}
