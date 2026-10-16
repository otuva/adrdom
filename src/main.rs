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
        /// Also write CLAUDE.md and the deny entries in .claude/settings.json
        #[arg(long)]
        claude: bool,
        /// Also write .codex/rules/adrdom.rules against the owner's commands
        #[arg(long)]
        codex: bool,
        /// Also fetch the embedding model and the onnxruntime library into the user cache
        #[arg(long)]
        planning: bool,
    },
    /// Owner: remove what install wrote outside adrdom/
    Uninstall,
    /// Ask the owner the unasked questions of a topic file
    Ask {
        /// The topic file, adrdom/decisions/<topic>.md
        topic_file: PathBuf,
    },
    /// Plan a topic file and print the next move
    Plan {
        /// The topic file, adrdom/decisions/<topic>.md
        topic_file: PathBuf,
    },
    /// Compare every current decision with its neighbours
    Read,
    /// Owner: approve a topic file, a terms entry or a blame file
    Approve {
        /// A topic file, adrdom/terms.md, or a blame file under adrdom/blame
        file: PathBuf,
        /// For adrdom/terms.md: the one entry to approve, as `adr show` names it
        entry: Option<String>,
    },
    /// Owner: answer a question with a letter or refuse
    Answer {
        /// The question, as `adr show` prints it: <id>@<version>
        id: String,
        /// The letter of the option picked, or the word refuse
        letter: String,
    },
    /// Print the current decisions and the moves
    #[command(group(clap::ArgGroup::new("whole").args(["topic_file", "all"]).multiple(true)))]
    Show {
        /// A topic file, adrdom/terms.md, or a blame file under adrdom/blame
        topic_file: Option<PathBuf>,
        /// Every file approve takes, as with each named
        #[arg(long, conflicts_with = "topic_file")]
        all: bool,
        /// Each unanswered question's neighbours, with the agent's line and the reader's word
        #[arg(long, requires = "whole")]
        neighbours: bool,
        /// What depends on each question or term entry
        #[arg(long, requires = "whole")]
        dependents: bool,
        /// Each superseded question, with what replaced it
        #[arg(long, requires = "whole")]
        superseded: bool,
        /// Each question's unrelated line, marked when changed
        #[arg(long, requires = "whole")]
        unrelated: bool,
        /// For a blame file: how many rows each question names, and how many none does
        #[arg(long, requires = "whole")]
        questions: bool,
        /// Every word the reader gave on each neighbour, with its reason
        #[arg(long, requires = "whole")]
        reader: bool,
    },
    /// Ban words read from stdin
    Ban {
        /// The term the banned words belong to, by name or alias, any case
        #[arg(long, conflicts_with = "rebuild")]
        term: Option<String>,
        /// Rewrite the ban list from the ledger; reads nothing, bans nothing new
        #[arg(long)]
        rebuild: bool,
    },
    /// Owner: lift a ban
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
        /// For the commit-msg hook: look for banned words in this message file
        #[arg(long, conflicts_with_all = ["push", "staged", "since"])]
        message: Option<PathBuf>,
        /// Also fail when a ledger line present at <commit> is gone
        #[arg(long, conflicts_with = "push")]
        since: Option<String>,
        /// For the pre-push hook: fail when the push would drop a ledger line
        #[arg(long)]
        push: bool,
    },
}

fn main() {
    // fastembed lets `HF_HOME` override the cache directory it is given; ours
    // is the only place the model lives, so the variable is dropped before
    // anything spawns a thread (removing it later is not thread-safe).
    std::env::remove_var("HF_HOME");
    let shown: &'static str = Box::leak(
        adr::msg::version_shown(VERSION, cfg!(feature = "owner-bypass")).into_boxed_str(),
    );
    let matches = Cli::command().version(shown).get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    let code = match cli.command {
        Command::Install { claude, codex, planning } => commands::install::run(claude, codex, planning),
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
