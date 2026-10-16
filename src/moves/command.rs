//! The command line a move names, quoted for a shell, and the move of a removed file.
use super::RemovalHold;
use crate::msg::{Act, Next};
use crate::status::QuestionStatus;

/// A command line a move names, as the one who runs it pastes it into a
/// shell. Built from its words by `command`, which quotes every word that a
/// shell would read otherwise, so a path, an identifier or a term name
/// taken from the records runs as the word it is, whatever it holds (per
/// "Does a refusal name the command that clears it?", b0e865b4 A).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cmd(String);

impl Cmd {
    /// A command none of whose words come from the records: written once,
    /// in the code, as the shell reads it.
    pub fn fixed(line: &'static str) -> Cmd {
        Cmd(line.to_string())
    }

    /// The command with `placeholder` after it, a word the one who runs it
    /// writes in (`<letter>`), printed as it stands.
    pub fn then(mut self, placeholder: &'static str) -> Cmd {
        self.0.push(' ');
        self.0.push_str(placeholder);
        self
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The command line of `words`, each quoted as `shell_word` quotes it.
pub fn command(words: &[&str]) -> Cmd {
    Cmd(words
        .iter()
        .map(|w| shell_word(w))
        .collect::<Vec<_>>()
        .join(" "))
}

/// `word` as a POSIX shell reads it back: unchanged when it holds only
/// letters, digits and `_ . / @ : = , + -` (and is not empty), otherwise in
/// single quotes, a quote inside written `'\''`. Nothing inside single
/// quotes is expanded, so `$(..)`, a backtick, a space or a newline stay
/// part of the one word.
pub fn shell_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '_' | '.' | '/' | '@' | ':' | '=' | ',' | '+' | '-')
        });
    if plain {
        return word.to_string();
    }
    // A control character (a newline, an escape) would end or redraw the
    // printed line inside plain quotes, and a zero-width or bidirectional
    // one would be written `\u{..}` by the printer inside them: such a word
    // is written `$'..'`, each of those characters and every quote escaped,
    // so the move stays one line and runs as printed.
    if word.chars().any(|c| c == '\t' || crate::out::is_hidden(c)) {
        let mut s = String::from("$'");
        for c in word.chars() {
            match c {
                '\\' => s.push_str("\\\\"),
                '\'' => s.push_str("\\'"),
                '\n' => s.push_str("\\n"),
                '\t' => s.push_str("\\t"),
                '\r' => s.push_str("\\r"),
                c if (c as u32) < 0x80 && c.is_control() => {
                    s.push_str(&format!("\\x{:02x}", c as u32))
                }
                c if crate::out::is_hidden(c) => s.push_str(&format!("\\U{:08x}", c as u32)),
                c => s.push(c),
            }
        }
        s.push('\'');
        return s;
    }
    format!("'{}'", word.replace('\'', "'\\''"))
}

/// The words of a hold in a refusal: what the question is that keeps the
/// file's removal from being approved.
pub fn hold_words(status: QuestionStatus) -> &'static str {
    match status {
        QuestionStatus::Answered => "a current decision",
        QuestionStatus::Asked => "asked and not answered",
        _ => "approved and not yet asked",
    }
}

/// The clauses that clear a removal held by `holds`, or the owner's
/// approval when nothing holds it. The agent restores the file, then
/// settles each hold by a plan change the owner approves: a current
/// decision is superseded, any other question struck from the plan.
pub fn removal_next(rel: &str, holds: &[RemovalHold]) -> Next {
    // Nothing holds the removal (check's ids rule names a removed file whose
    // questions are refused, say): the owner approves it away.
    if holds.is_empty() {
        return Next::owner_runs(command(&["adr", "approve", rel]));
    }
    let steps: Vec<Act> = holds
        .iter()
        .map(|h| {
            let q = crate::msg::named(&h.id, Some(&h.heading));
            if h.status == QuestionStatus::Answered {
                Act::Writes(format!("a question that supersedes {}", q))
            } else {
                Act::Writes(format!("the plan change that strikes {}", q))
            }
        })
        .collect();
    steps
        .into_iter()
        .fold(Next::agent(Act::Restores(rel.to_string())), |n, a| {
            n.then(a)
        })
        .and_owner_runs(command(&["adr", "approve", rel]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::{Move, MoveKind};

    // adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
    //
    // A word taken from the records runs as the one word it is: nothing in
    // it is expanded by the shell the move is pasted into, and the move
    // stays one line.
    #[test]
    fn a_command_quotes_every_word_a_shell_would_read_otherwise() {
        assert_eq!(
            command(&["adr", "approve", "adrdom/decisions/x-y.md"]).to_string(),
            "adr approve adrdom/decisions/x-y.md"
        );
        assert_eq!(
            command(&["adr", "approve", "adrdom/blame/a$(touch PWNED).md"]).to_string(),
            "adr approve 'adrdom/blame/a$(touch PWNED).md'"
        );
        assert_eq!(shell_word("it's"), "'it'\\''s'");
        assert_eq!(shell_word(""), "''");
        assert_eq!(shell_word("a\nowner runs: x"), "$'a\\nowner runs: x'");
        assert_eq!(shell_word("`id`"), "'`id`'");
        let m = Move::new(MoveKind::Answer {
            id: "x$(touch P)".into(),
            version: "abcd".into(),
            heading: "H?".into(),
        });
        assert_eq!(
            m.clause_text(),
            "owner runs: adr answer 'x$(touch P)@abcd' <letter>"
        );
    }

    // adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
    //
    // A word holding a zero-width, bidirectional or control character is
    // quoted so the terminal line shows it escaped and the shell, reading the
    // move as printed, gets the word back as the file holds it.
    #[test]
    fn a_word_with_a_hidden_character_runs_as_printed() {
        for word in [
            "adrdom/decisions/z\u{200b}z.md",
            "a\u{202e}b.md",
            "x\u{85}y",
            "p\u{1b}[2Kq",
            "it's\u{feff}",
        ] {
            let line = crate::out::text(&crate::out::Line::Move(Move::new(MoveKind::Approve {
                path: word.into(),
            })));
            assert!(!line.contains("\\u{"), "{:?} printed as {}", word, line);
            let printed = line.strip_prefix("owner runs: adr approve ").unwrap();
            let out = std::process::Command::new("bash")
                .arg("-c")
                .arg(format!("printf %s {}", printed))
                .output()
                .unwrap();
            assert_eq!(String::from_utf8(out.stdout).unwrap(), word, "{}", line);
        }
    }

    #[test]
    fn a_hold_is_superseded_when_current_and_struck_otherwise() {
        let holds = vec![
            RemovalHold {
                id: "id-1".into(),
                heading: "A?".into(),
                status: QuestionStatus::Answered,
            },
            RemovalHold {
                id: "id-2".into(),
                heading: "B?".into(),
                status: QuestionStatus::Asked,
            },
        ];
        let text = removal_next("x.md", &holds).render();
        assert!(
            text.contains("supersedes") && text.contains("strikes"),
            "{}",
            text
        );
        assert!(text.ends_with("owner runs: adr approve x.md"), "{}", text);
    }
}
