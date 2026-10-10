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
    /// A topic's heading line in a listing of every topic:
    /// `# <topic> (<path>)`. It reads as a `Record` at depth 0 does.
    Topic { text: String },
    /// A thing in the records and its status; `mark` is the side of a
    /// change beyond its latest approval the thing is on as a whole: a thing
    /// new since that approval (`Added`) or struck from it (`Removed`), the
    /// marks of a diff (per "Does show print what a file holds beyond its
    /// latest approval?", bceceda8 A).
    Status { subject: String, status: Status, mark: Option<Side> },
    /// The reader's word on a neighbour of a question, one line under the
    /// question (per "What does show print of the reader's word on a
    /// question?", 332ff0c7 A). A view: no move stands on it. The reader's
    /// words are not statuses (a closed list), so the line is no `Status`.
    Word { word: String, name: String },
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
        Line::Topic { text } => text.clone(),
        Line::Status { subject, status, mark } => format!("{}[{}] {}", mark_of(*mark), status.name(), subject),
        Line::Word { word, name } => format!("{}[{}] {}", INDENT, word, name),
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
        Line::Topic { text } => format!("<p class=\"record depth-0\">{}</p>", esc(text)),
        Line::Status { subject, status, mark } => format!(
            "<p class=\"status\">{}<span class=\"status-name s-{}\">[{}]</span> {}</p>",
            esc(mark_of(*mark)),
            status_class(status),
            esc(&status.name()),
            esc(subject)
        ),
        Line::Word { word, name } => {
            let class: String = word.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
            format!("<p class=\"word word-{}\"><span class=\"word-name\">[{}]</span> {}</p>", class, esc(word), esc(name))
        }
        Line::Finding(f) => format!("<p class=\"finding\">{}</p>", esc(&f.text())),
        Line::Stop { command, text } => {
            format!("<p class=\"finding stop\"><span class=\"rule\">{}:</span> {}</p>", esc(command.as_str()), esc(text))
        }
        Line::Move(m) => format!("<p class=\"move {}\">{}</p>", mover_class(m), esc(&m.text())),
        Line::Event(e) => format!("<p class=\"event\">{}</p>", esc(&e.text())),
    }
}

/// The first word of a status name, as the page's class: `answered`, `asked`.
fn status_class(status: &Status) -> String {
    let name = status.name();
    name.split(' ').next().unwrap_or("").chars().filter(|c| c.is_ascii_alphanumeric()).collect()
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

/// How a terminal listing is dressed: `colour` paints each kind of line (per
/// "Does the tool colour what it prints to a terminal?", b80e29dc A) and
/// `shape` groups the lines, a blank line between groups and a header per
/// group (per "What does the tool add to the shape of what it prints to a
/// terminal?", f3c9e736 A). Both off is the piped output: one plain line per
/// item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub colour: bool,
    pub shape: bool,
}

impl Style {
    pub const PLAIN: Style = Style { colour: false, shape: false };
}

/// Whether `NO_COLOR` (set and not empty) or `TERM=dumb` forbids colour.
fn colour_forbidden() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) || std::env::var("TERM").is_ok_and(|t| t == "dumb")
}

/// The style of a stream: shaped when it is a terminal, coloured when it is
/// a terminal and colour is not forbidden.
fn style_for(is_terminal: bool) -> Style {
    Style { colour: is_terminal && !colour_forbidden(), shape: is_terminal }
}

/// The style of standard output, decided once per process.
fn stdout_style() -> Style {
    use std::io::IsTerminal;
    static STYLE: std::sync::OnceLock<Style> = std::sync::OnceLock::new();
    *STYLE.get_or_init(|| style_for(std::io::stdout().is_terminal()))
}

/// The style of standard error: dressed as standard output is (a refusal is
/// a group of findings like any other, so on a terminal it opens with the
/// findings header).
fn stderr_style() -> Style {
    use std::io::IsTerminal;
    static STYLE: std::sync::OnceLock<Style> = std::sync::OnceLock::new();
    *STYLE.get_or_init(|| style_for(std::io::stderr().is_terminal()))
}

/// The groups of a listing: a maximal run of one kind of line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Questions,
    Findings,
    Moves,
    Events,
}

impl Group {
    fn of(line: &Line) -> Group {
        match line {
            Line::Record { .. } | Line::Topic { .. } | Line::Status { .. } | Line::Word { .. } => Group::Questions,
            Line::Finding(_) | Line::Stop { .. } => Group::Findings,
            Line::Move(_) => Group::Moves,
            Line::Event(_) => Group::Events,
        }
    }

    /// The class of a group's element on the page.
    pub fn class(self) -> &'static str {
        match self {
            Group::Questions => "questions",
            Group::Findings => "findings",
            Group::Moves => "moves",
            Group::Events => "events",
        }
    }

    /// The header word of a group; the questions group is headed by the
    /// topic or file line it opens with, and events get none.
    pub fn header(self) -> Option<String> {
        match self {
            Group::Findings => Some(crate::msg::group_findings()),
            Group::Moves => Some(crate::msg::group_moves()),
            Group::Questions | Group::Events => None,
        }
    }
}

/// `lines` in maximal runs of one group, in order.
pub fn groups<'a>(lines: &[&'a Line]) -> Vec<(Group, Vec<&'a Line>)> {
    let mut out: Vec<(Group, Vec<&'a Line>)> = Vec::new();
    for l in lines {
        let g = Group::of(l);
        match out.last_mut() {
            Some((last, run)) if *last == g => run.push(l),
            _ => out.push((g, vec![l])),
        }
    }
    out
}

const RESET: &str = "\u{1b}[0m";
const BOLD: &str = "1";
const DIM: &str = "2";
const RED: &str = "31";
const GREEN: &str = "32";
const YELLOW: &str = "33";

fn sgr(code: &str, s: &str) -> String {
    format!("\u{1b}[{}m{}{}", code, s, RESET)
}

/// The colour of a status name: settled green, refused or removed red, open
/// or changed yellow, gone dim, the rest default.
fn status_colour(status: &Status) -> Option<&'static str> {
    match status {
        Status::Question { status, .. } => match status {
            QuestionStatus::Answered => Some(GREEN),
            QuestionStatus::Refused => Some(RED),
            QuestionStatus::Asked | QuestionStatus::ChangedSinceAsked => Some(YELLOW),
            QuestionStatus::Superseded | QuestionStatus::Withdrawn | QuestionStatus::Dropped => Some(DIM),
            QuestionStatus::NoBody | QuestionStatus::NotAsked => None,
        },
        Status::File(s) => match s {
            FileStatus::Approved => Some(GREEN),
            FileStatus::Removed => Some(RED),
            FileStatus::New | FileStatus::Changed => Some(YELLOW),
        },
        Status::Term(s) => match s {
            TermStatus::Approved => Some(GREEN),
            TermStatus::Retired | TermStatus::Removed => Some(RED),
            TermStatus::New | TermStatus::Changed => Some(YELLOW),
        },
    }
}

/// `line` for a terminal with colour: `text(line)` (already free of escapes)
/// with the escape codes of its kind added around it.
fn painted(line: &Line) -> String {
    let t = text(line);
    match line {
        Line::Topic { .. } => sgr(BOLD, &t),
        Line::Record { .. } => t,
        Line::Status { status, mark, .. } => {
            let Some(code) = status_colour(status) else { return t };
            let start = mark_of(*mark).len();
            match t[start..].find(']') {
                Some(i) => {
                    let end = start + i + 1;
                    format!("{}{}{}", &t[..start], sgr(code, &t[start..end]), &t[end..])
                }
                None => t,
            }
        }
        Line::Word { word, .. } => match word.as_str() {
            "inconsistent" => sgr("1;31", &t),
            "same" => sgr("1;33", &t),
            _ => sgr(DIM, &t),
        },
        Line::Finding(_) => sgr(YELLOW, &t),
        Line::Stop { .. } => sgr(RED, &t),
        Line::Move(m) => match m.actor() {
            crate::moves::Mover::Owner => sgr(BOLD, &t),
            crate::moves::Mover::NotYet => sgr(DIM, &t),
            crate::moves::Mover::Agent | crate::moves::Mover::Builder => t,
        },
        Line::Event(_) => sgr(GREEN, &t),
    }
}

/// The lines as `style` writes them, one string per line of output (a blank
/// line is the empty string). With `Style::PLAIN` these are exactly the
/// `text` of each presented line.
pub fn render_lines(lines: &[Line], style: Style) -> Vec<String> {
    render_after(lines, style, &mut None)
}

/// Like `render_lines` for a call that follows earlier calls on the same
/// stream: `last` is the group the stream's latest line was in. A first group
/// equal to it continues it, with no blank line and no header; any other
/// first group starts as a later group does. `last` is left at the group of
/// the final line rendered. Not used without shape.
fn render_after(lines: &[Line], style: Style, last: &mut Option<Group>) -> Vec<String> {
    let shown = present(lines);
    let paint = |l: &Line| if style.colour { painted(l) } else { text(l) };
    if !style.shape {
        return shown.into_iter().map(paint).collect();
    }
    let mut out: Vec<String> = Vec::new();
    for (group, run) in groups(&shown) {
        let continued = out.is_empty() && *last == Some(group);
        if !continued && (!out.is_empty() || last.is_some()) {
            out.push(String::new());
        }
        if !continued {
            if let Some(h) = group.header() {
                out.push(if style.colour { sgr(BOLD, &h) } else { h });
            }
        }
        for l in run {
            // A topic heading is set off by a blank line, also when it opens
            // a call that continues the stream's group.
            let apart = if out.is_empty() { continued } else { !out.last().is_some_and(|s| s.is_empty()) };
            if matches!(l, Line::Topic { .. }) && apart {
                out.push(String::new());
            }
            out.push(paint(l));
        }
        *last = Some(group);
    }
    out
}

/// The group last written to each stream by this process, for a terminal.
static LAST_STDOUT: std::sync::Mutex<Option<Group>> = std::sync::Mutex::new(None);
static LAST_STDERR: std::sync::Mutex<Option<Group>> = std::sync::Mutex::new(None);

fn render_stream(lines: &[Line], style: Style, last: &std::sync::Mutex<Option<Group>>) -> Vec<String> {
    let mut g = last.lock().unwrap_or_else(|e| e.into_inner());
    render_after(lines, style, &mut g)
}

/// `render_lines` joined by newlines, each line ended by one.
pub fn render(lines: &[Line], style: Style) -> String {
    render_lines(lines, style).into_iter().map(|l| l + "\n").collect()
}

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
            Line::Word { word: "inconsistent".to_string(), name: "Does <it>?".to_string() },
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

    fn sample() -> Vec<Line> {
        vec![
            Line::Topic { text: "# Example (x.md)".to_string() },
            Line::Status {
                subject: "Does it?".to_string(),
                status: Status::Question { status: QuestionStatus::Answered, letter: Some('A'), version: None },
                mark: None,
            },
            Line::Word { word: "inconsistent".to_string(), name: "Does it not?".to_string() },
            Line::Finding(Finding::new(&crate::rule::TOPIC_FILE_SHAPE, "x.md: a fault".to_string(), "agent rewrites the file".to_string())),
            Line::Move(Move::new(MoveKind::Approve { path: "x.md".to_string() })),
            Line::Topic { text: "# Other (y.md)".to_string() },
            Line::Event(Event::Asked { heading: "Does it?".to_string() }),
        ]
    }

    // adr: b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051
    //
    // With colour on, each kind of line carries the code of its kind; text
    // from the records still comes out with its escape written out; with
    // colour off the lines are exactly the plain text.
    #[test]
    fn colour_paints_each_kind_and_never_lets_a_record_escape() {
        let on = Style { colour: true, shape: false };
        let lines = render_lines(&sample(), on);
        assert_eq!(lines[0], "\u{1b}[1m# Example (x.md)\u{1b}[0m");
        assert!(lines[1].starts_with("\u{1b}[32m[answered A]\u{1b}[0m Does it?"), "{:?}", lines[1]);
        assert!(lines[2].starts_with("\u{1b}[1;31m") && lines[2].contains("[inconsistent]"), "{:?}", lines[2]);
        assert!(lines[3].starts_with("\u{1b}[33m"), "{:?}", lines[3]);
        assert!(lines[4].starts_with("\u{1b}[1m") && lines[4].contains("owner runs:"), "{:?}", lines[4]);
        assert!(lines[6].starts_with("\u{1b}[32masked"), "{:?}", lines[6]);
        let stop = render_lines(&[Line::Stop { command: Command::Show, text: "no".to_string() }], on);
        assert_eq!(stop[0], "\u{1b}[31mshow: no\u{1b}[0m");
        let agent = Line::Move(Move::new(MoveKind::Ask { rel: "x.md".to_string(), headings: vec![] }));
        assert!(!render_lines(&[agent], on)[0].contains('\u{1b}'));
        let hostile = Line::Record { depth: 1, text: "a\u{1b}[2Kb".to_string() };
        assert_eq!(render_lines(&[hostile.clone()], on), vec!["  a\\u{1b}[2Kb".to_string()]);
        let off: Vec<String> = render_lines(&sample(), Style::PLAIN);
        assert_eq!(off, present(&sample()).into_iter().map(text).collect::<Vec<_>>());
        assert!(off.iter().all(|l| !l.contains('\u{1b}')));
    }

    // adr: f3c9e736-157c-4197-b49e-19e3674e4e0c
    //
    // A refusal on standard error is a findings group: with shape on it
    // opens with the findings header.
    #[test]
    fn a_shaped_refusal_opens_with_the_findings_header() {
        let stop = [Line::Stop { command: Command::Show, text: "no".to_string() }];
        assert_eq!(render_lines(&stop, Style { colour: false, shape: true }), vec!["Findings".to_string(), "show: no".to_string()]);
        assert_eq!(render_lines(&stop, Style::PLAIN), vec!["show: no".to_string()]);
    }

    // adr: f3c9e736-157c-4197-b49e-19e3674e4e0c
    //
    // With shape on, a blank line separates the groups and a header heads
    // the findings and the moves; with shape off the lines are the plain
    // lines, one per item.
    #[test]
    fn shape_separates_the_groups_with_a_blank_line_and_a_header() {
        let shaped = render_lines(&sample(), Style { colour: false, shape: true });
        let plain = render_lines(&sample(), Style::PLAIN);
        assert_eq!(plain.len(), 7);
        assert!(plain.iter().all(|l| !l.is_empty()));
        let at = |h: &str| shaped.iter().position(|l| l == h).unwrap();
        assert_eq!(shaped[at("Findings") - 1], "");
        assert_eq!(shaped[at("Moves") - 1], "");
        assert_eq!(shaped[0], "# Example (x.md)");
        let other = shaped.iter().position(|l| l.starts_with("# Other")).unwrap();
        assert_eq!(shaped[other - 1], "");
        let events = shaped.iter().position(|l| l.starts_with("asked:")).unwrap();
        assert_eq!(shaped[events - 1], "");
        assert_eq!(shaped.iter().filter(|l| !l.is_empty()).filter(|l| *l != "Findings" && *l != "Moves").cloned().collect::<Vec<_>>(), plain);
    }

    // adr: f3c9e736-157c-4197-b49e-19e3674e4e0c
    //
    // A later call on a terminal stream continues the group the stream
    // ended in, and starts another group with a blank line and its header.
    #[test]
    fn a_later_call_continues_or_starts_a_group_on_one_stream() {
        let shaped = Style { colour: false, shape: true };
        let stop = |t: &str| [Line::Stop { command: Command::Show, text: t.to_string() }];
        let mut last = None;
        assert_eq!(render_after(&stop("a"), shaped, &mut last), vec!["Findings", "show: a"]);
        assert_eq!(render_after(&stop("b"), shaped, &mut last), vec!["show: b"]);
        let ev = [Line::Event(Event::TermsApproved)];
        let got = render_after(&ev, shaped, &mut last);
        assert_eq!(got[0], "");
        assert_eq!(got.len(), 2);
        assert_eq!(render_after(&stop("c"), shaped, &mut last), vec!["", "Findings", "show: c"]);
        // A topic heading that opens a continuing call is set off by a blank
        // line; one that opens a stream is not.
        let rec = [Line::Record { depth: 1, text: "x".to_string() }];
        let top = [Line::Topic { text: "# T (t.md)".to_string() }];
        let mut q = None;
        assert_eq!(render_after(&top, shaped, &mut q), vec!["# T (t.md)"]);
        assert_eq!(render_after(&rec, shaped, &mut q), vec!["  x"]);
        assert_eq!(render_after(&top, shaped, &mut q), vec!["", "# T (t.md)"]);
        // Piped output ignores the state.
        let mut none = None;
        assert_eq!(render_after(&stop("a"), Style::PLAIN, &mut none), vec!["show: a"]);
        assert_eq!(none, None);
    }
}
