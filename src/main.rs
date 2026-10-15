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
    /// Put the hooks and the brief into this repository
    Install {
        /// Also write CLAUDE.md when missing and merge the deny entries into .claude/settings.json
        #[arg(long)]
        claude: bool,
        /// Also write .codex/rules/adrdom.rules, forbidding the owner's commands under Codex
        #[arg(long)]
        codex: bool,
    },
    /// Owner: remove what install wrote outside adrdom/
    Uninstall,
    /// Owner: ask the unasked questions of a topic file
    ///
    /// Asks every question that has a body and is not asked yet; prints every refusal instead when there is one.
    Ask {
        /// The topic file, adrdom/decisions/<topic>.md
        topic_file: PathBuf,
    },
    /// Plan a topic file and print the next move
    ///
    /// Brings the neighbours of its headings and questions, names the lines still to write, and prints the move that follows.
    Plan {
        /// The topic file, adrdom/decisions/<topic>.md
        topic_file: PathBuf,
    },
    /// Compare every current decision with its neighbours
    ///
    /// Reads each decision against its ten nearest and prints each pair the reader marks same or inconsistent.
    Read,
    /// Owner: approve a topic file, a terms entry or a blame file
    ///
    /// Approves the headings and prose of a topic file, one named entry of adrdom/terms.md, or a blame file.
    Approve {
        /// A topic file, adrdom/terms.md, or a blame file under adrdom/blame
        file: PathBuf,
        /// For adrdom/terms.md: the one entry to approve, as `adr show` names it
        entry: Option<String>,
    },
    /// Owner: answer a question with a letter or refuse
    ///
    /// The question is named as `adr show` prints it, `<id>@<version>`.
    Answer {
        /// The question, as `adr show` prints it: <id>@<version>
        id: String,
        /// The letter of the option picked, or the word refuse
        letter: String,
    },
    /// Print the current decisions and the moves
    ///
    /// With a file approve takes, or --all: every question or entry with its status.
    #[command(group(clap::ArgGroup::new("whole").args(["topic_file", "all"]).multiple(true)))]
    Show {
        /// A topic file, adrdom/terms.md, or a blame file under adrdom/blame
        ///
        /// Prints each question or entry with its status, its depends and supersedes lines, the body of an asked question, what is beyond its latest approval marked, and its moves. A blame file: its origin and rows.
        topic_file: Option<PathBuf>,
        /// Print every file approve takes, each as with a file named
        #[arg(long, conflicts_with = "topic_file")]
        all: bool,
        /// With a file or --all: each neighbour of an unanswered question, with the agent's line and the reader's word
        #[arg(long, requires = "whole")]
        neighbours: bool,
        /// With a file or --all: what depends on each question or term entry
        #[arg(long, requires = "whole")]
        dependents: bool,
        /// With a file or --all: each superseded question, with what replaced it
        #[arg(long, requires = "whole")]
        superseded: bool,
        /// With a file or --all: the unrelated line of each question, marked when changed
        #[arg(long, requires = "whole")]
        unrelated: bool,
        /// With a file or --all, for a blame file: how many decision rows each question names, and how many none names
        #[arg(long, requires = "whole")]
        questions: bool,
        /// With a file or --all: every word the reader gave on each neighbour, with its reason
        #[arg(long, requires = "whole")]
        reader: bool,
    },
    /// Ban words read from stdin
    ///
    /// Each line read, and each of its tokens, may then appear nowhere in the repository.
    Ban {
        /// The term the banned words belong to, by name or alias, any case
        #[arg(long, conflicts_with = "rebuild")]
        term: Option<String>,
        /// Rewrite the ban list from the ledger; reads nothing, bans nothing new
        #[arg(long)]
        rebuild: bool,
    },
    /// Owner: lift a ban
    ///
    /// Reads lines from stdin and removes them, and their tokens, from the ban list.
    Unban,
    /// Owner: serve the owner's page on 127.0.0.1
    Serve {
        #[arg(long, default_value_t = 7357)]
        port: u16,
    },
    /// Run the checks the hooks run
    Check {
        /// Judge the files as staged for the commit, not as on disk
        #[arg(long, conflicts_with = "push")]
        staged: bool,
        /// What the commit-msg hook runs: look for banned words in this message file
        ///
        /// At a merge, judge the staged files too. Takes neither --staged nor --since.
        #[arg(long, conflicts_with_all = ["push", "staged", "since"])]
        message: Option<PathBuf>,
        /// Also fail when a ledger line from <commit> is missing now
        #[arg(long, conflicts_with = "push")]
        since: Option<String>,
        /// What the pre-push hook runs: fail when a push would drop a ledger line the remote has
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
        Command::Approve { file, entry } => commands::approve::run(file, entry),
        Command::Answer { id, letter } => commands::answer::run(id, letter),
        Command::Show { topic_file, all, neighbours, dependents, unrelated, questions, reader, superseded } => commands::show::run(
            topic_file,
            all,
            commands::show::Views { neighbours, dependents, unrelated, questions, reader, superseded },
        ),
        Command::Ban { term, rebuild } => commands::ban::run(term, rebuild),
        Command::Unban => commands::unban::run(),
        Command::Serve { port } => commands::serve::run(port),
        Command::Check { staged, message, since, push } => commands::check::run(staged, message, since, push),
    };
    std::process::exit(code);
}
