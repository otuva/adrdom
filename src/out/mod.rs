//! The one printer: every line a command prints is a `Line`, and a line has
//! exactly two renderings, one for the terminal (`text`) and one for the page
//! (`html`), so the page shows what the terminal prints (per "Does the ui
//! show what the terminal prints?", 7857d633 A). A command that writes
//! nothing prints what it read, statuses, findings and moves (per "What does
//! a command print when it writes nothing?", 1af76f91 A); a command that writes
//! prints one event per row or file it wrote, or the finding that stopped it
//! (per "What does a command that writes print?", 069a6b4a A).
mod kinds;
mod rendering;
mod style;

pub use kinds::{Event, Status};
pub(crate) use rendering::word_is_marked;
pub use rendering::{esc, html, is_hidden, text, visible};
pub use style::{groups, present, render, render_lines, Group, Style};

use crate::approval::Side;
use crate::moves::Move;
use crate::rules::{Command, Finding};
use std::io::Write;
use style::{render_stream, stderr_style, stdout_style, LAST_STDERR, LAST_STDOUT};

/// One line a command prints.
#[derive(Debug, Clone)]
pub enum Line {
    /// Text the command read from the records, at a depth under what holds
    /// it (a decision's heading at 1, its picked option at 2).
    Record {
        depth: usize,
        text: String,
    },
    /// A topic's heading line in a listing of every topic:
    /// `# <topic> (<path>)`. It reads as a `Record` at depth 0 does.
    Topic {
        text: String,
    },
    /// A thing in the records and its status; `mark` is the side of a
    /// change beyond its latest approval the thing is on as a whole: a thing
    /// new since that approval (`Added`) or struck from it (`Removed`), the
    /// marks of a diff (per "Does show print what a file holds past its latest
    /// approval?", 91f9652a A).
    Status {
        subject: String,
        status: Status,
        mark: Option<Side>,
    },
    /// The reader's word on a neighbour of a question, one line under the
    /// question (per "What does show print of a reader's word on a
    /// question?", 6e72f34e A). A view: no move stands on it. The reader's
    /// words are not statuses (a closed list), so the line is no `Status`.
    Word {
        word: String,
        name: String,
    },
    /// A rule one thing in the records breaks: found and the move.
    Finding(Finding),
    /// What stopped a command on its own input, files, environment or gate:
    /// `<command>: <what>; <move>`.
    Stop {
        command: Command,
        text: String,
    },
    Move(Move),
    Event(Event),
}

const INDENT: &str = "  ";

/// A report as lines: the builds of what is answered first, as the moves
/// are taken (what is answered is built before anything new is written),
/// then each finding, then the other moves.
pub fn report(r: &crate::moves::Report) -> Vec<Line> {
    let is_build = |m: &&Move| matches!(m.kind, crate::moves::MoveKind::Build { .. });
    r.moves
        .iter()
        .filter(is_build)
        .cloned()
        .map(Line::Move)
        .chain(r.stops.iter().cloned())
        .chain(r.findings.iter().cloned().map(Line::Finding))
        .chain(
            r.moves
                .iter()
                .filter(|m| !is_build(m))
                .cloned()
                .map(Line::Move),
        )
        .collect()
}

/// What stopped the command `cmd`, whole: the command's own name leads the
/// line once, whether or not `text` already starts with it.
pub fn refusal(cmd: Command, text: &str) -> Line {
    let prefix = format!("{}: ", cmd);
    let text = text.strip_prefix(&prefix).unwrap_or(text);
    Line::Stop {
        command: cmd,
        text: text.to_string(),
    }
}

/// What stopped the command `cmd`, one line per line of `stop`, the
/// command's name leading each once: what the terminal prints on standard
/// error and the page shows in its banner, the same lines (per "Does the ui
/// show what the terminal prints?", 7857d633 A).
pub fn stop(cmd: Command, stop: &str) -> Vec<Line> {
    stop.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| refusal(cmd, l))
        .collect()
}

/// The exit code of a command whose reader went away, as a shell reports a
/// process ended by a closed pipe (128 + 13).
const READER_GONE: i32 = 141;

/// Prints `lines` to standard output, one terminal line each, dressed as a
/// terminal gets them (`stdout_style`). The reader
/// gone (`adr show | head`) is no fault of the tool: the process ends
/// without a word, with `READER_GONE`, so a pipeline that watches exit codes
/// never reads it as a pass.
///
/// Any other write error leaves the output incomplete: the command says so
/// on standard error, once, and ends with 1, so a run whose lines did not
/// all reach their reader never passes.
pub fn print(lines: &[Line]) {
    let mut out = std::io::stdout().lock();
    for l in render_stream(lines, stdout_style(), &LAST_STDOUT) {
        if let Err(e) = writeln!(out, "{}", l) {
            unwritable(e);
        }
    }
    if let Err(e) = out.flush() {
        unwritable(e);
    }
}

/// Ends the process on a failed write to standard output.
fn unwritable(e: std::io::Error) -> ! {
    if e.kind() == std::io::ErrorKind::BrokenPipe {
        std::process::exit(READER_GONE);
    }
    // The command as it was run, by whoever runs it: the move runs as
    // printed once standard output can be written.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let owner = args
        .first()
        .is_some_and(|a| crate::commands::OWNER_COMMANDS.contains(&a.as_str()));
    let mut words = vec!["adr"];
    words.extend(args.iter().map(String::as_str));
    let actor = if owner {
        crate::msg::Actor::Owner
    } else {
        crate::msg::Actor::Agent
    };
    let _ = writeln!(
        std::io::stderr().lock(),
        "{}",
        crate::msg::stdout_write_failed(&e, actor, crate::moves::command(&words))
    );
    std::process::exit(1);
}

/// Prints `lines` to standard error: the findings that stopped a command,
/// dressed on a terminal as standard output is (`stderr_style`).
pub fn eprint(lines: &[Line]) {
    let mut err = std::io::stderr().lock();
    for l in render_stream(lines, stderr_style(), &LAST_STDERR) {
        let _ = writeln!(err, "{}", l);
    }
}

/// Prints the events a writing command wrote, one line each (per "What
/// does a command that writes print?", 069a6b4a A): nothing when it wrote
/// nothing.
pub fn print_events(events: &[Event]) {
    let lines: Vec<Line> = events.iter().cloned().map(Line::Event).collect();
    print(&lines);
}

/// Prints to standard error what stopped the command `cmd`: `out::stop` of
/// the refusal text, so every line of a refusal of several lines leads with
/// the command's name once.
pub fn eprint_stop(cmd: Command, text: &str) {
    eprint(&stop(cmd, text));
}
