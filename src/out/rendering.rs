//! The two renderings of a line, the terminal text and the page html, and what keeps hidden characters out of both.
use super::{Line, Status, INDENT};
use crate::approval::Side;
use crate::moves::Move;

/// The line for the terminal, every character that would move the cursor,
/// clear the line or turn the text around written visibly (`visible`).
pub fn text(line: &Line) -> String {
    visible(&match line {
        Line::Record { depth, text } => format!("{}{}", INDENT.repeat(*depth), text),
        Line::Topic { text } => text.clone(),
        Line::Status {
            subject,
            status,
            mark,
        } => format!("{}[{}] {}", mark_of(*mark), status.name(), subject),
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
    s.chars()
        .map(|c| {
            if is_hidden(c) {
                format!("\\u{{{:x}}}", c as u32)
            } else {
                c.to_string()
            }
        })
        .collect()
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
        Line::Status {
            subject,
            status,
            mark,
        } => format!(
            "<p class=\"status\">{}<span class=\"status-name s-{}\">[{}]</span> {}</p>",
            esc(mark_of(*mark)),
            status_class(status),
            esc(&status.name()),
            esc(subject)
        ),
        Line::Word { word, name } => {
            let class: String = word_class(word)
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            format!(
                "<p class=\"word word-{}\"><span class=\"word-name\">[{}]</span> {}</p>",
                class,
                esc(word),
                esc(name)
            )
        }
        Line::Finding(f) => format!("<p class=\"finding\">{}</p>", esc(&f.text())),
        Line::Stop { command, text } => {
            format!(
                "<p class=\"finding stop\"><span class=\"rule\">{}:</span> {}</p>",
                esc(command.as_str()),
                esc(text)
            )
        }
        Line::Move(m) => format!(
            "<p class=\"move {}\">{}</p>",
            mover_class(m),
            esc(&m.text())
        ),
        Line::Event(e) => format!("<p class=\"event\">{}</p>", esc(&e.text())),
    }
}

/// The page class of a reader's word: the word itself, but a `closes` word
/// with its letter (`closes B`) is the one class `closes`.
pub(super) fn word_class(word: &str) -> &str {
    if word.starts_with("closes ") {
        "closes"
    } else {
        word
    }
}

/// Whether the plain listing of `show` marks a neighbour with this word:
/// `inconsistent`, `same`, or `closes` with its letter.
pub(crate) fn word_is_marked(word: &str) -> bool {
    matches!(word_class(word), "inconsistent" | "same" | "closes")
}

/// The first word of a status name, as the page's class: `answered`, `asked`.
fn status_class(status: &Status) -> String {
    let name = status.name();
    name.split(' ')
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// The mark a status line leads with: `+ ` or `- `, the marks of the change
/// lines under it; nothing for a thing approved as it stands, or changed in
/// part (its lines carry the marks).
pub(super) fn mark_of(mark: Option<Side>) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::{Move, MoveKind};
    use crate::out::Event;
    use crate::rules::{Command, Finding};
    use crate::status::QuestionStatus;

    #[test]
    fn a_line_reads_the_same_words_on_the_terminal_and_the_page() {
        let lines = vec![
            Line::Record {
                depth: 1,
                text: "Does it?".to_string(),
            },
            Line::Status {
                subject: "Does it?".to_string(),
                status: Status::Question {
                    status: QuestionStatus::Asked,
                    letter: None,
                    version: Some("ab12".into()),
                },
                mark: Some(Side::Added),
            },
            Line::Word {
                word: "inconsistent".to_string(),
                name: "Does <it>?".to_string(),
            },
            Line::Finding(Finding::new(
                &crate::rule::TOPIC_FILE_SHAPE,
                "x.md: a <fault>".to_string(),
                crate::msg::Next::agent(crate::msg::Act::Rewrites("the file".into())),
            )),
            Line::Stop {
                command: Command::Show,
                text: "x.md is not a file; agent names one".to_string(),
            },
            Line::Move(Move::new(MoveKind::Approve {
                path: "x.md".to_string(),
            })),
            Line::Event(Event::Asked {
                heading: "Does it?".to_string(),
            }),
        ];
        for l in &lines {
            let t = text(l);
            let words = esc(t.trim_start());
            let page = html(l);
            for w in words
                .split(' ')
                .filter(|w| !w.starts_with('[') && !w.ends_with(']') && !w.ends_with(':'))
            {
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
        out.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&")
    }

    // adr: a9e9e370-7768-4a79-b67b-7fe0926b5670
    //
    // A carriage return and an escape sequence in a record never reach the
    // terminal or the page as themselves: both show them written out.
    #[test]
    fn a_hidden_character_is_written_visibly_on_the_terminal_and_the_page() {
        let line = Line::Record {
            depth: 1,
            text: "A) ok\r\u{1b}[2K- A) other \u{202e}x\u{200b}y\tz".to_string(),
        };
        let t = text(&line);
        assert_eq!(
            t,
            "  A) ok\\u{d}\\u{1b}[2K- A) other \\u{202e}x\\u{200b}y\tz"
        );
        let h = html(&line);
        assert!(
            !h.contains('\r') && !h.contains('\u{1b}') && !h.contains('\u{202e}'),
            "{:?}",
            h
        );
        assert!(h.contains("\\u{1b}"), "{}", h);
    }
}
