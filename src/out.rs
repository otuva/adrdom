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
use crate::rules::{Command, Finding};
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
    /// A key of git's configuration set to `value` (install's
    /// `core.hooksPath`).
    ConfigSet { key: String, value: String },
    /// A key of git's configuration unset.
    ConfigUnset { key: String },
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
            Event::ConfigSet { key, value } => format!("set: {} {}", key, value),
            Event::ConfigUnset { key } => format!("unset: {}", key),
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
    /// A rule one thing in the records breaks: found, what the rule
    /// protects, the move.
    Finding(Finding),
    /// What stopped a command on its own input, files, environment or gate:
    /// `<command>: <what>; <move>`.
    Stop { command: Command, text: String },
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
        .chain(r.moves.iter().filter(|m| !is_build(m)).cloned().map(Line::Move))
        .collect()
}

/// The line for the terminal, every character that would move the cursor,
/// clear the line or turn the text around written visibly (`visible`).
pub fn text(line: &Line) -> String {
    visible(&match line {
        Line::Record { depth, text } => format!("{}{}", INDENT.repeat(*depth), text),
        Line::Status { subject, status, mark } => format!("{}[{}] {}", mark_of(*mark), status.name(), subject),
        Line::Finding(f) => f.text(),
        Line::Stop { command, text } => format!("{}: {}", command, text),
        Line::Move(m) => m.text(),
        Line::Event(e) => e.text(),
    })
}

/// `s` with every control character but the tab, every bidirectional
/// control and every zero-width character written as `\u{..}`: text from
/// the records reaches the terminal and the page as the characters it
/// holds, never as a carriage return, an escape sequence or a reordering
/// that rewrites what is shown.
pub fn visible(s: &str) -> String {
    if !s.chars().any(is_hidden) {
        return s.to_string();
    }
    s.chars().map(|c| if is_hidden(c) { format!("\\u{{{:x}}}", c as u32) } else { c.to_string() }).collect()
}

/// A character the terminal or the page would act on rather than show: a
/// control character but the tab, a bidirectional control, a zero-width
/// character. `visible` writes each as `\u{..}`; `moves::shell_word` quotes
/// a word holding one so the move still runs as printed.
pub fn is_hidden(c: char) -> bool {
    (c.is_control() && c != '\t')
        || matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{061C}' | '\u{FEFF}')
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
        Line::Finding(f) => format!("<p class=\"finding\">{}</p>", esc(&f.text())),
        Line::Stop { command, text } => {
            format!("<p class=\"finding\"><span class=\"rule\">{}:</span> {}</p>", esc(command.as_str()), esc(text))
        }
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
    match m.actor() {
        crate::moves::Mover::Agent => "agent",
        crate::moves::Mover::Builder => "builder",
        crate::moves::Mover::Owner => "owner",
        crate::moves::Mover::NotYet => "not-yet",
    }
}

/// `s` for the page: the characters HTML reads as markup written as
/// entities, every hidden character written visibly (`visible`). The one
/// escaper of the page.
pub fn esc(s: &str) -> String {
    let s = visible(s);
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// What stopped the command `cmd`, whole: the command's own name leads the
/// line once, whether or not `text` already starts with it.
pub fn refusal(cmd: Command, text: &str) -> Line {
    let prefix = format!("{}: ", cmd);
    let text = text.strip_prefix(&prefix).unwrap_or(text);
    Line::Stop { command: cmd, text: text.to_string() }
}

/// What stopped the command `cmd`, one line per line of `stop`, the
/// command's name leading each once: what the terminal prints on standard
/// error and the page shows in its banner, the same lines (per "Does the ui
/// show what the terminal prints?", 7857d633 A).
pub fn stop(cmd: Command, stop: &str) -> Vec<Line> {
    stop.lines().filter(|l| !l.trim().is_empty()).map(|l| refusal(cmd, l)).collect()
}

/// The exit code of a command whose reader went away, as a shell reports a
/// process ended by a closed pipe (128 + 13).
const READER_GONE: i32 = 141;

/// The lines as every output shows them, the terminal's standard output and
/// standard error and the page alike: each run of findings in the one order
/// of `rules::finding_order`, and a finding or stop whose text an earlier
/// line of the call already has left out.
pub fn present(lines: &[Line]) -> Vec<&Line> {
    let mut shown: Vec<&Line> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if matches!(lines[i], Line::Finding(_)) {
            let mut run: Vec<&Line> = lines[i..].iter().take_while(|l| matches!(l, Line::Finding(_))).collect();
            i += run.len();
            run.sort_by(|a, b| match (a, b) {
                (Line::Finding(x), Line::Finding(y)) => crate::rules::finding_order(x, y),
                _ => std::cmp::Ordering::Equal,
            });
            shown.extend(run);
        } else {
            shown.push(&lines[i]);
            i += 1;
        }
    }
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    shown.retain(|l| !matches!(l, Line::Finding(_) | Line::Stop { .. }) || seen.insert(text(l)));
    shown
}

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
    for l in present(lines) {
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
    // The command as it was run, by whoever runs it: the move runs as
    // printed once standard output can be written.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let owner = args.first().is_some_and(|a| crate::commands::OWNER_COMMANDS.contains(&a.as_str()));
    let mut words = vec!["adr"];
    words.extend(args.iter().map(String::as_str));
    let actor = if owner { crate::msg::Actor::Owner } else { crate::msg::Actor::Agent };
    let _ = writeln!(
        std::io::stderr().lock(),
        "{}",
        crate::msg::stdout_write_failed(&e, actor, crate::moves::command(&words))
    );
    std::process::exit(1);
}

/// Prints `lines` to standard error: the findings that stopped a command.
pub fn eprint(lines: &[Line]) {
    let mut err = std::io::stderr().lock();
    for l in present(lines) {
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

/// Prints to standard error what stopped the command `cmd`: `out::stop` of
/// the refusal text, so every line of a refusal of several lines leads with
/// the command's name once.
pub fn eprint_stop(cmd: Command, text: &str) {
    eprint(&stop(cmd, text));
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
            Line::Finding(Finding::new(&crate::rule::TOPIC_FILE_SHAPE, "x.md: a <fault>".to_string(), "agent rewrites the file".to_string())),
            Line::Stop { command: Command::Show, text: "x.md is not a file; agent names one".to_string() },
            Line::Move(Move::new(MoveKind::Approve { path: "x.md".to_string() })),
            Line::Event(Event::Asked { heading: "Does it?".to_string() }),
        ];
        for l in &lines {
            let t = text(l);
            let words = esc(t.trim_start());
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
        out.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
    }

    // adr: 7857d633-2055-47eb-857f-cb70da9fa7ba
    //
    // A carriage return and an escape sequence in a record never reach the
    // terminal or the page as themselves: both show them written out.
    #[test]
    fn a_hidden_character_is_written_visibly_on_the_terminal_and_the_page() {
        let line = Line::Record { depth: 1, text: "A) ok\r\u{1b}[2K- A) other \u{202e}x\u{200b}y\tz".to_string() };
        let t = text(&line);
        assert_eq!(t, "  A) ok\\u{d}\\u{1b}[2K- A) other \\u{202e}x\\u{200b}y\tz");
        let h = html(&line);
        assert!(!h.contains('\r') && !h.contains('\u{1b}') && !h.contains('\u{202e}'), "{:?}", h);
        assert!(h.contains("\\u{1b}"), "{}", h);
    }
}
