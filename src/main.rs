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
    /// Plan a topic file: bring the neighbours of its headings and of its questions with their bodies, name the lines still to write, and print the move that follows: the owner's approve once the plan is ready.
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
    /// Print every current decision and every move; with a file approve takes or --all, every question or entry with its status (a blame file: its origin and rows).
    #[command(group(clap::ArgGroup::new("whole").args(["topic_file", "all"]).multiple(true)))]
    Show {
        /// A topic file, adrdom/terms.md, or a blame file under adrdom/blame: every question or entry with its status (a blame file: its origin and rows), every depends and supersedes line, the body of an asked question, what is beyond its latest approval marked, and its moves.
        topic_file: Option<PathBuf>,
        /// Print every file approve takes, each as with a file named.
        #[arg(long, conflicts_with = "topic_file")]
        all: bool,
        /// With a file or --all: each neighbour of a question not yet answered, with the reader's word and the agent's line.
        #[arg(long, requires = "whole")]
        neighbours: bool,
        /// With a file or --all: what depends on each question or term entry, and what replaced a superseded question.
        #[arg(long, requires = "whole")]
        dependents: bool,
        /// With a file or --all: the unrelated line of each question, marked when changed.
        #[arg(long, requires = "whole")]
        unrelated: bool,
        /// With a file or --all: for a blame file, how many of its decision rows a question names and how many none names.
        #[arg(long, requires = "whole")]
        questions: bool,
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
        /// What the commit-msg hook runs: look for banned words in this commit message file; at a merge, judge the staged files too. Takes neither --staged nor --since.
        #[arg(long, conflicts_with_all = ["push", "staged", "since"])]
        message: Option<PathBuf>,
        /// Also fail when a line of the ledger as it was at <commit> is missing from the ledger now.
        #[arg(long, conflicts_with = "push")]
        since: Option<String>,
        /// What the pre-push hook runs: read git's pre-push input and fail when a push would drop a line of the ledger the remote has.
        #[arg(long)]
        push: bool,
    },
}

fn main() {
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
        Command::Show { topic_file, all, neighbours, dependents, unrelated, questions } => {
            commands::show::run(topic_file, all, commands::show::Views { neighbours, dependents, unrelated, questions })
        }
        Command::Ban { term, rebuild } => commands::ban::run(term, rebuild),
        Command::Unban => commands::unban::run(),
        Command::Serve { port } => commands::serve::run(port),
        Command::Check { staged, message, since, push } => commands::check::run(staged, message, since, push),
    };
    std::process::exit(code);
}
