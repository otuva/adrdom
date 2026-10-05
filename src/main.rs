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
    /// Ask the owner every question of a topic file that has a body and is not asked yet, or print every refusal.
    Ask {
        /// The topic file, adrdom/decisions/<topic>.md.
        topic_file: PathBuf,
    },
    /// Plan a topic file: bring the neighbours of its headings and of its questions with their bodies, name the lines still to write, or say it is ready for approval.
    Plan {
        /// The topic file, adrdom/decisions/<topic>.md.
        topic_file: PathBuf,
    },
    /// Read every current decision against its ten nearest current decisions and print each pair the reader marks same, inconsistent or depends.
    Read,
    /// Owner-only: approve the headings and prose of a topic file, the terms file, or a blame file.
    Approve {
        /// A topic file, adrdom/terms.md, or a blame file under adrdom/blame.
        file: PathBuf,
    },
    /// Owner-only: answer a question, identified by `<id>@<version>`, with an option letter or `refuse`.
    Answer {
        /// The question, as `adr show` prints it: <id>@<version>.
        id: String,
        /// The letter of the option picked, or the word refuse.
        letter: String,
    },
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
        /// Name the term the banned words belong to, by its name or an
        /// alias; the case of the letters does not count.
        #[arg(long, conflicts_with = "rebuild")]
        term: Option<String>,
        /// Write the ban list again from what the ledger says is banned;
        /// reads no input and bans nothing new. Run it when a refusal
        /// names it.
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
        /// Also fail when a line of the ledger as it was at <commit> is missing from the ledger now.
        #[arg(long, conflicts_with = "push")]
        since: Option<String>,
        /// What the pre-push hook runs: read git's pre-push input and fail when a push would drop a line of the ledger the remote has.
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
        Command::Plan { topic_file } => commands::plan::run(topic_file),
        Command::Read => commands::read::run(),
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
