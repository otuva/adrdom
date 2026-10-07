//! The one printer: every line a command prints is a `Line`, and a line has
//! exactly two renderings, one for the terminal (`text`) and one for the page
//! (`html`), so the page shows what the terminal prints (per "Does the ui
//! show what the terminal prints?", 7857d633 A). A command that writes
//! nothing prints what it read, statuses, findings and moves (per "What does
//! a command that writes nothing print?", 848e1b76 A); a command that writes
//! prints one event per row or file it wrote, or the finding that stopped it
//! (per "What does a command that writes print?", 069a6b4a A).
use crate::approval::Side;
use crate::ledger::Row;
use crate::moves::Move;
use crate::rules::Finding;
use crate::status::{FileStatus, QuestionStatus, TermStatus};
use std::collections::HashMap;
use std::io::Write;

/// A status from one of the closed lists of `status`, with what the name
/// carries beside it: the picked letter of an answered question, the
/// version of an asked one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Question { status: QuestionStatus, letter: Option<char>, version: Option<String> },
    File(FileStatus),
    Term(TermStatus),
}

impl Status {
    /// The status's name: the one `name()` of its list, with the letter of
    /// an answered question and the version of an asked one.
    pub fn name(&self) -> String {
        match self {
            Status::Question { status, letter, version } => match version {
                Some(v) => format!("{} @{}", status.name_with(*letter), v),
                None => status.name_with(*letter),
            },
            Status::File(s) => s.name().to_string(),
            Status::Term(s) => s.name().to_string(),
        }
    }
}

/// One event a writing command wrote: a ledger row, or a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The plan of a topic file approved (its file row).
    PlanApproved { path: String },
    /// One question's text approved (its own row).
    QuestionApproved { heading: String },
    /// The prose of a topic file approved.
    ProseApproved { path: String },
    TermsApproved,
    TermApproved { name: String },
    TermRemoved { name: String },
    BlameApproved { path: String },
    Asked { heading: String },
    /// A question asked while one standing on it carries an answer set.
    Assumes { dependent: String, target: String },
    /// `sentence` is the picked option's sentence when the caller knows it
    /// (`answer` does; a row alone does not carry it).
    Answered { heading: String, letter: String, sentence: String },
    Refused { heading: String },
    Superseded { heading: String },
    Withdrawn { heading: String },
    Dropped { heading: String },
    Banned { term: Option<String> },
    Unbanned,
    FileWritten { path: String },
    FileRemoved { path: String },
}

impl Event {
    /// The event a ledger row records. `names` holds the heading of every
    /// question and the name of every term by identifier; an identifier it
    /// does not hold is printed as it is.
    pub fn from_row(row: &Row, names: &HashMap<String, String>) -> Option<Event> {
        let name = |id: &str| names.get(id).cloned().unwrap_or_else(|| id.to_string());
        Some(match row.event.as_str() {
            "approved" if row.id == "-" => Event::PlanApproved { path: row.detail.clone() },
            "approved" => Event::QuestionApproved { heading: name(&row.id) },
            "context" => Event::ProseApproved { path: row.detail.clone() },
            "terms" => Event::TermsApproved,
            "term" if row.hash == crate::state::TERM_REMOVED => Event::TermRemoved { name: row.detail.clone() },
            "term" => Event::TermApproved { name: row.detail.clone() },
            "blame" => Event::BlameApproved { path: row.detail.clone() },
            "asked" => Event::Asked { heading: row.detail.clone() },
            "assumes" => Event::Assumes { dependent: name(&row.detail), target: name(&row.id) },
            "answered" => {
                Event::Answered { heading: name(&row.id), letter: row.detail.clone(), sentence: String::new() }
            }
            "refused" => Event::Refused { heading: row.detail.clone() },
            "superseded" => Event::Superseded { heading: name(&row.id) },
            "withdrawn" => Event::Withdrawn { heading: row.detail.clone() },
            "dropped" => Event::Dropped { heading: name(&row.id) },
            "banned" => Event::Banned { term: (!row.detail.is_empty()).then(|| name(&row.detail)) },
            "unbanned" => Event::Unbanned,
            _ => return None,
        })
    }

    pub fn text(&self) -> String {
        match self {
            Event::PlanApproved { path } => format!("approved: plan of {}", path),
            Event::QuestionApproved { heading } => format!("approved: \"{}\"", heading),
            Event::ProseApproved { path } => format!("approved: prose of {}", path),
            Event::TermsApproved => format!("approved: {}", crate::repo::TERMS_PATH),
            Event::TermApproved { name } => format!("approved: term \"{}\"", name),
            Event::TermRemoved { name } => format!("removed: term \"{}\"", name),
            Event::BlameApproved { path } => format!("approved: {}", path),
            Event::Asked { heading } => format!("asked: \"{}\"", heading),
            Event::Assumes { dependent, target } => {
                format!("assumes: \"{}\" stands on \"{}\" under its answer set", dependent, target)
            }
            Event::Answered { heading, letter, sentence } if sentence.is_empty() => {
                format!("answered: \"{}\" {}", heading, letter)
            }
            Event::Answered { heading, letter, sentence } => {
                format!("answered: \"{}\" {}: {}", heading, letter, sentence)
            }
            Event::Refused { heading } => format!("refused: \"{}\"", heading),
            Event::Superseded { heading } => format!("superseded: \"{}\"", heading),
            Event::Withdrawn { heading } => format!("withdrawn: \"{}\"", heading),
            Event::Dropped { heading } => format!("dropped: \"{}\"", heading),
            Event::Banned { term: Some(t) } => format!("banned: a word of term \"{}\"", t),
            Event::Banned { term: None } => "banned: a word".to_string(),
            Event::Unbanned => "unbanned: a word".to_string(),
            Event::FileWritten { path } => format!("written: {}", path),
            Event::FileRemoved { path } => format!("removed: {}", path),
        }
    }
}

/// One line a command prints.
#[derive(Debug, Clone)]
pub enum Line {
    /// Text the command read from the records, at a depth under what holds
    /// it (a decision's heading at 1, its picked option at 2).
    Record { depth: usize, text: String },
    /// A thing in the records and its status; `mark` is the side of a
    /// change beyond its latest approval the thing is on as a whole: a thing
    /// new since that approval (`Added`) or struck from it (`Removed`), the
    /// marks of a diff (per "Does show print what a file holds beyond its
    /// latest approval?", bceceda8 A).
    Status { subject: String, status: Status, mark: Option<Side> },
    Finding(Finding),
    Move(Move),
    Event(Event),
}

const INDENT: &str = "  ";

/// A report as lines: each finding, then each move.
pub fn report(r: &crate::moves::Report) -> Vec<Line> {
    r.findings.iter().cloned().map(Line::Finding).chain(r.moves.iter().cloned().map(Line::Move)).collect()
}

/// The line for the terminal.
pub fn text(line: &Line) -> String {
    match line {
        Line::Record { depth, text } => format!("{}{}", INDENT.repeat(*depth), text),
        Line::Status { subject, status, mark } => format!("{}[{}] {}", mark_of(*mark), status.name(), subject),
        Line::Finding(f) => f.text(),
        Line::Move(m) => m.text(),
        Line::Event(e) => e.text(),
    }
}

/// The line for the page: the words of `text`, the indent left to the page,
/// each kind in an element of its own class.
pub fn html(line: &Line) -> String {
    match line {
        Line::Record { depth, text } => {
            format!("<p class=\"record depth-{}\">{}</p>", depth, esc(text))
        }
        Line::Status { subject, status, mark } => format!(
            "<p class=\"status\">{}<span class=\"status-name\">[{}]</span> {}</p>",
            esc(mark_of(*mark)),
            esc(&status.name()),
            esc(subject)
        ),
        Line::Finding(f) => format!(
            "<p class=\"finding\"><span class=\"rule\">{}:</span> {}</p>",
            esc(f.rule),
            esc(&f.message)
        ),
        Line::Move(m) => format!("<p class=\"move {}\">{}</p>", mover_class(m), esc(&m.text())),
        Line::Event(e) => format!("<p class=\"event\">{}</p>", esc(&e.text())),
    }
}

/// The mark a status line leads with: `+ ` or `- `, the marks of the change
/// lines under it; nothing for a thing approved as it stands, or changed in
/// part (its lines carry the marks).
fn mark_of(mark: Option<Side>) -> &'static str {
    match mark {
        Some(Side::Added) => "+ ",
        Some(Side::Removed) => "- ",
        Some(Side::Both) | None => "",
    }
}

fn mover_class(m: &Move) -> &'static str {
    match m.actor {
        crate::moves::Mover::Agent => "agent",
        crate::moves::Mover::Owner => "owner",
        crate::moves::Mover::NotYet => "not-yet",
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The refusal a command `cmd` stopped on, whole, as a finding of the rule
/// named after the command: the command's own name leads the line once,
/// whether or not `text` already starts with it.
pub fn refusal(cmd: &'static str, text: &str) -> Line {
    let prefix = format!("{}: ", cmd);
    let message = text.strip_prefix(&prefix).unwrap_or(text);
    Line::Finding(Finding::new(cmd, message.to_string()))
}

/// What stopped the command `cmd`, one finding per line of `stop`, the
/// command's name leading each once: what the terminal prints on standard
/// error and the page shows in its banner, the same lines (per "Does the ui
/// show what the terminal prints?", 7857d633 A).
pub fn stop(cmd: &'static str, stop: &str) -> Vec<Line> {
    stop.lines().filter(|l| !l.trim().is_empty()).map(|l| refusal(cmd, l)).collect()
}

/// The exit code of a command whose reader went away, as a shell reports a
/// process ended by a closed pipe (128 + 13).
const READER_GONE: i32 = 141;

/// Prints `lines` to standard output, one terminal line each. The reader
/// gone (`adr show | head`) is no fault of the tool: the process ends
/// without a word, with `READER_GONE`, so a pipeline that watches exit codes
/// never reads it as a pass.
///
/// Any other write error leaves the output incomplete: the command says so
/// on standard error, once, and ends with 1, so a run whose lines did not
/// all reach their reader never passes.
pub fn print(lines: &[Line]) {
    let mut out = std::io::stdout().lock();
    for l in lines {
        if let Err(e) = writeln!(out, "{}", text(l)) {
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
    let _ = writeln!(std::io::stderr().lock(), "{}", crate::msg::stdout_write_failed(&e));
    std::process::exit(1);
}

/// Prints `lines` to standard error: the findings that stopped a command.
pub fn eprint(lines: &[Line]) {
    let mut err = std::io::stderr().lock();
    for l in lines {
        let _ = writeln!(err, "{}", text(l));
    }
}

/// Prints the events a writing command wrote, one line each (per "What
/// does a command that writes print?", 069a6b4a A): nothing when it wrote
/// nothing.
pub fn print_events(events: &[Event]) {
    let lines: Vec<Line> = events.iter().cloned().map(Line::Event).collect();
    print(&lines);
}

/// Prints to standard error what stopped a command, as the refusal text
/// built for it: one finding per line, each naming its move.
pub fn eprint_stop(stop: &str) {
    let mut err = std::io::stderr().lock();
    for l in stop.lines() {
        let _ = writeln!(err, "{}", l);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::MoveKind;

    fn row(event: &str, id: &str, hash: &str, detail: &str) -> Row {
        Row {
            time: String::new(),
            event: event.to_string(),
            id: id.to_string(),
            hash: hash.to_string(),
            detail: detail.to_string(),
            version: String::new(),
        }
    }

    #[test]
    fn every_ledger_event_a_command_writes_reads_as_an_event() {
        let names: HashMap<String, String> = [("id-1".to_string(), "Does it?".to_string())].into();
        for event in crate::ledger::EVENTS {
            let r = row(event, "id-1", "h", "d");
            assert!(Event::from_row(&r, &names).is_some(), "no event for a {} row", event);
        }
        let answered = Event::from_row(&row("answered", "id-1", "h", "A"), &names).unwrap();
        assert_eq!(answered.text(), "answered: \"Does it?\" A");
        let plan = Event::from_row(&row("approved", "-", "h", "adrdom/decisions/x.md"), &names).unwrap();
        assert_eq!(plan.text(), "approved: plan of adrdom/decisions/x.md");
    }

    #[test]
    fn a_line_reads_the_same_words_on_the_terminal_and_the_page() {
        let lines = vec![
            Line::Record { depth: 1, text: "Does it?".to_string() },
            Line::Status {
                subject: "Does it?".to_string(),
                status: Status::Question { status: QuestionStatus::Asked, letter: None, version: Some("ab12".into()) },
                mark: Some(Side::Added),
            },
            Line::Finding(Finding::new("plan", "x.md: a <fault>; agent fixes it".to_string())),
            Line::Move(Move::new(MoveKind::Approve { path: "x.md".to_string() })),
            Line::Event(Event::Asked { heading: "Does it?".to_string() }),
        ];
        for l in &lines {
            let t = text(l);
            let words = t.trim_start().replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
            let page = html(l);
            for w in words.split(' ').filter(|w| !w.starts_with('[') && !w.ends_with(']') && !w.ends_with(':')) {
                assert!(page.contains(w), "{:?} lacks {:?}", page, w);
            }
        }
        assert_eq!(text(&lines[1]), "+ [asked @ab12] Does it?");
        // The page's text of each line is the terminal's line, its indent
        // left to the page.
        for l in &lines {
            assert_eq!(page_text(&html(l)), text(l).trim_start(), "{:?}", l);
        }
    }

    /// The text of a rendered line: tags dropped, entities read back.
    fn page_text(html: &str) -> String {
        let mut out = String::new();
        let mut in_tag = false;
        for c in html.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => out.push(c),
                _ => {}
            }
        }
        out.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&amp;", "&")
    }
}
