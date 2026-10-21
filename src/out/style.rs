//! How a listing is presented and dressed: the order and dedupe of its lines, colour, groups and shape.
use super::rendering::{mark_of, text, word_class};
use super::{Line, Status};
use crate::status::{FileStatus, QuestionStatus, TermStatus};

/// The lines as every output shows them, the terminal's standard output and
/// standard error and the page alike: each run of findings in the one order
/// of `rules::finding_order`, and a finding or stop whose text an earlier
/// line of the call already has left out.
pub fn present(lines: &[Line]) -> Vec<&Line> {
    let mut shown: Vec<&Line> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if matches!(lines[i], Line::Finding(_)) {
            let mut run: Vec<&Line> = lines[i..]
                .iter()
                .take_while(|l| matches!(l, Line::Finding(_)))
                .collect();
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
    pub const PLAIN: Style = Style {
        colour: false,
        shape: false,
    };
}

/// Whether `NO_COLOR` (set and not empty) or `TERM=dumb` forbids colour.
fn colour_forbidden() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
        || std::env::var("TERM").is_ok_and(|t| t == "dumb")
}

/// The style of a stream: shaped when it is a terminal, coloured when it is
/// a terminal and colour is not forbidden.
fn style_for(is_terminal: bool) -> Style {
    Style {
        colour: is_terminal && !colour_forbidden(),
        shape: is_terminal,
    }
}

/// The style of standard output, decided once per process.
pub(super) fn stdout_style() -> Style {
    use std::io::IsTerminal;
    static STYLE: std::sync::OnceLock<Style> = std::sync::OnceLock::new();
    *STYLE.get_or_init(|| style_for(std::io::stdout().is_terminal()))
}

/// The style of standard error: dressed as standard output is (a refusal is
/// a group of findings like any other, so on a terminal it opens with the
/// findings header).
pub(super) fn stderr_style() -> Style {
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
            Line::Record { .. } | Line::Topic { .. } | Line::Status { .. } | Line::Word { .. } => {
                Group::Questions
            }
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
            QuestionStatus::Superseded | QuestionStatus::Withdrawn | QuestionStatus::Dropped => {
                Some(DIM)
            }
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
            let Some(code) = status_colour(status) else {
                return t;
            };
            let start = mark_of(*mark).len();
            match t[start..].find(']') {
                Some(i) => {
                    let end = start + i + 1;
                    format!("{}{}{}", &t[..start], sgr(code, &t[start..end]), &t[end..])
                }
                None => t,
            }
        }
        Line::Word { word, .. } => match word_class(word) {
            "inconsistent" | "closes" => sgr("1;31", &t),
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
            let apart = if out.is_empty() {
                continued
            } else {
                !out.last().is_some_and(|s| s.is_empty())
            };
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
pub(super) static LAST_STDOUT: std::sync::Mutex<Option<Group>> = std::sync::Mutex::new(None);
pub(super) static LAST_STDERR: std::sync::Mutex<Option<Group>> = std::sync::Mutex::new(None);

pub(super) fn render_stream(
    lines: &[Line],
    style: Style,
    last: &std::sync::Mutex<Option<Group>>,
) -> Vec<String> {
    let mut g = last.lock().unwrap_or_else(|e| e.into_inner());
    render_after(lines, style, &mut g)
}

/// `render_lines` joined by newlines, each line ended by one.
pub fn render(lines: &[Line], style: Style) -> String {
    render_lines(lines, style)
        .into_iter()
        .map(|l| l + "\n")
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::{Move, MoveKind};
    use crate::out::Event;
    use crate::out::{html, word_is_marked};
    use crate::rules::{Command, Finding};

    #[test]
    fn a_closes_word_prints_with_its_letter_in_red_and_one_page_class() {
        let l = Line::Word {
            word: "closes B".to_string(),
            name: "Does it?".to_string(),
        };
        assert_eq!(text(&l), "  [closes B] Does it?");
        assert_eq!(painted(&l), format!("\x1b[1;31m{}\x1b[0m", text(&l)));
        assert!(
            html(&l).starts_with("<p class=\"word word-closes\">"),
            "{}",
            html(&l)
        );
        assert!(
            word_is_marked("closes B") && word_is_marked("same") && !word_is_marked("unrelated")
        );
    }

    fn sample() -> Vec<Line> {
        vec![
            Line::Topic {
                text: "# Example (x.md)".to_string(),
            },
            Line::Status {
                subject: "Does it?".to_string(),
                status: Status::Question {
                    status: QuestionStatus::Answered,
                    letter: Some('A'),
                    version: None,
                },
                mark: None,
            },
            Line::Word {
                word: "inconsistent".to_string(),
                name: "Does it not?".to_string(),
            },
            Line::Finding(Finding::new(
                &crate::rule::TOPIC_FILE_SHAPE,
                "x.md: a fault".to_string(),
                crate::msg::Next::agent(crate::msg::Act::Rewrites(crate::msg::Part::Line)),
            )),
            Line::Move(Move::new(MoveKind::Approve {
                path: "x.md".to_string(),
            })),
            Line::Topic {
                text: "# Other (y.md)".to_string(),
            },
            Line::Event(Event::Asked {
                heading: "Does it?".to_string(),
            }),
        ]
    }

    // adr: 17059963-8eb3-479c-8f12-1df7b25b19f6
    //
    // With colour on, each kind of line carries the code of its kind; text
    // from the records still comes out with its escape written out; with
    // colour off the lines are exactly the plain text.
    #[test]
    fn colour_paints_each_kind_and_never_lets_a_record_escape() {
        let on = Style {
            colour: true,
            shape: false,
        };
        let lines = render_lines(&sample(), on);
        assert_eq!(lines[0], "\u{1b}[1m# Example (x.md)\u{1b}[0m");
        assert!(
            lines[1].starts_with("\u{1b}[32m[answered A]\u{1b}[0m Does it?"),
            "{:?}",
            lines[1]
        );
        assert!(
            lines[2].starts_with("\u{1b}[1;31m") && lines[2].contains("[inconsistent]"),
            "{:?}",
            lines[2]
        );
        assert!(lines[3].starts_with("\u{1b}[33m"), "{:?}", lines[3]);
        assert!(
            lines[4].starts_with("\u{1b}[1m") && lines[4].contains("owner runs:"),
            "{:?}",
            lines[4]
        );
        assert!(lines[6].starts_with("\u{1b}[32masked"), "{:?}", lines[6]);
        let stop = render_lines(
            &[Line::Stop {
                command: Command::Show,
                text: "no".to_string(),
            }],
            on,
        );
        assert_eq!(stop[0], "\u{1b}[31mshow: no\u{1b}[0m");
        let agent = Line::Move(Move::new(MoveKind::Plan {
            rel: "x.md".to_string(),
        }));
        assert!(!render_lines(&[agent], on)[0].contains('\u{1b}'));
        let hostile = Line::Record {
            depth: 1,
            text: "a\u{1b}[2Kb".to_string(),
        };
        assert_eq!(
            render_lines(std::slice::from_ref(&hostile), on),
            vec!["  a\\u{1b}[2Kb".to_string()]
        );
        let off: Vec<String> = render_lines(&sample(), Style::PLAIN);
        assert_eq!(
            off,
            present(&sample()).into_iter().map(text).collect::<Vec<_>>()
        );
        assert!(off.iter().all(|l| !l.contains('\u{1b}')));
    }

    // adr: a5503032-f7f1-45a2-874a-5580d1211d37
    //
    // A refusal on standard error is a findings group: with shape on it
    // opens with the findings header.
    #[test]
    fn a_shaped_refusal_opens_with_the_findings_header() {
        let stop = [Line::Stop {
            command: Command::Show,
            text: "no".to_string(),
        }];
        assert_eq!(
            render_lines(
                &stop,
                Style {
                    colour: false,
                    shape: true
                }
            ),
            vec!["Findings".to_string(), "show: no".to_string()]
        );
        assert_eq!(
            render_lines(&stop, Style::PLAIN),
            vec!["show: no".to_string()]
        );
    }

    // adr: a5503032-f7f1-45a2-874a-5580d1211d37
    //
    // With shape on, a blank line separates the groups and a header heads
    // the findings and the moves; with shape off the lines are the plain
    // lines, one per item.
    #[test]
    fn shape_separates_the_groups_with_a_blank_line_and_a_header() {
        let shaped = render_lines(
            &sample(),
            Style {
                colour: false,
                shape: true,
            },
        );
        let plain = render_lines(&sample(), Style::PLAIN);
        assert_eq!(plain.len(), 7);
        assert!(plain.iter().all(|l| !l.is_empty()));
        let at = |h: &str| shaped.iter().position(|l| l == h).unwrap();
        assert_eq!(shaped[at("Findings") - 1], "");
        assert_eq!(shaped[at("Moves") - 1], "");
        assert_eq!(shaped[0], "# Example (x.md)");
        let other = shaped
            .iter()
            .position(|l| l.starts_with("# Other"))
            .unwrap();
        assert_eq!(shaped[other - 1], "");
        let events = shaped.iter().position(|l| l.starts_with("asked:")).unwrap();
        assert_eq!(shaped[events - 1], "");
        assert_eq!(
            shaped
                .iter()
                .filter(|l| !l.is_empty())
                .filter(|l| *l != "Findings" && *l != "Moves")
                .cloned()
                .collect::<Vec<_>>(),
            plain
        );
    }

    // adr: a5503032-f7f1-45a2-874a-5580d1211d37
    //
    // A later call on a terminal stream continues the group the stream
    // ended in, and starts another group with a blank line and its header.
    #[test]
    fn a_later_call_continues_or_starts_a_group_on_one_stream() {
        let shaped = Style {
            colour: false,
            shape: true,
        };
        let stop = |t: &str| {
            [Line::Stop {
                command: Command::Show,
                text: t.to_string(),
            }]
        };
        let mut last = None;
        assert_eq!(
            render_after(&stop("a"), shaped, &mut last),
            vec!["Findings", "show: a"]
        );
        assert_eq!(render_after(&stop("b"), shaped, &mut last), vec!["show: b"]);
        let ev = [Line::Event(Event::TermsApproved)];
        let got = render_after(&ev, shaped, &mut last);
        assert_eq!(got[0], "");
        assert_eq!(got.len(), 2);
        assert_eq!(
            render_after(&stop("c"), shaped, &mut last),
            vec!["", "Findings", "show: c"]
        );
        // A topic heading that opens a continuing call is set off by a blank
        // line; one that opens a stream is not.
        let rec = [Line::Record {
            depth: 1,
            text: "x".to_string(),
        }];
        let top = [Line::Topic {
            text: "# T (t.md)".to_string(),
        }];
        let mut q = None;
        assert_eq!(render_after(&top, shaped, &mut q), vec!["# T (t.md)"]);
        assert_eq!(render_after(&rec, shaped, &mut q), vec!["  x"]);
        assert_eq!(render_after(&top, shaped, &mut q), vec!["", "# T (t.md)"]);
        // Piped output ignores the state.
        let mut none = None;
        assert_eq!(
            render_after(&stop("a"), Style::PLAIN, &mut none),
            vec!["show: a"]
        );
        assert_eq!(none, None);
    }
}
