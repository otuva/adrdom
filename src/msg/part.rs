//! What a move acts on: a closed set of the parts of the records and of the
//! repository the tool reads (per "What does a finding line hold?", 50b31713
//! A). Every act of a move names one `Part`, and `Part::render` is the one
//! place its words are written; a message holds no words of its own for it.

/// A line of a question's section, or of the records, by what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Id,
    Depends,
    Supersedes,
    Blame,
    Terms,
    Answer,
    Topic,
    Origin,
    FromComment,
    TieComment,
}

impl LineKind {
    fn name(self) -> &'static str {
        match self {
            LineKind::Id => "id line",
            LineKind::Depends => "depends line",
            LineKind::Supersedes => "supersedes line",
            LineKind::Blame => "blame line",
            LineKind::Terms => "terms line",
            LineKind::Answer => "answer line",
            LineKind::Topic => "topic line",
            LineKind::Origin => "origin line",
            LineKind::FromComment => "from-comment",
            LineKind::TieComment => "tie comment",
        }
    }
}

/// Several parts of one kind, one of which the move acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Many {
    IdLines,
    Sections,
    Headings,
    Definitions,
}

impl Many {
    fn name(self) -> &'static str {
        match self {
            Many::IdLines => "id lines",
            Many::Sections => "sections",
            Many::Headings => "headings",
            Many::Definitions => "definitions",
        }
    }
}

/// One thing a move acts on. A variant that carries data carries what the
/// records or the repository hold (a path, a heading, a name, a line
/// number), never words of a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    /// A git repository.
    Repository,
    /// A file or directory, by its path.
    File(String),
    /// The path a command was given.
    Path,
    /// The ledger.
    Ledger,
    /// The rows of a file that HEAD or another commit holds and this one
    /// does not.
    MissingRows,
    /// A row of a file.
    Row,
    /// The row at this line of its file.
    RowAt(usize),
    /// The line a finding names.
    Line,
    /// A line by its kind.
    LineOf(LineKind),
    /// A line by its kind, under this heading.
    LineUnder(LineKind, String),
    /// One of several parts of a kind.
    OneOf(Many),
    /// A line by its kind, with these identifiers in it.
    Retie(LineKind, Vec<String>),
    /// The entry a finding names, of a depends, supersedes or terms line.
    Entry,
    /// The entry of this term in the terms file.
    EntryOf(String),
    /// An answer set on a depends entry.
    AnswerSet,
    /// A term's definition.
    Definition,
    /// A term in the terms file.
    Term,
    /// A banned word.
    Word,
    /// The question a finding names.
    Question,
    /// The section of the question a finding names.
    Section,
    /// The heading a finding names.
    Heading,
    /// The option a finding names.
    Option,
    /// The bodies of these headings.
    Body(Vec<String>),
    /// The plan of a topic file.
    Plan,
    /// The decision of this heading, tied to a test by this identifier.
    Decision(String),
    /// A program the tool calls and does not ship.
    Program(String),
    /// What was piped to the command.
    Input,
    /// The part a failure names, filled in by `Next::on`.
    Pending,
}

/// `"a"`, `"a", "b"`: headings as a move names them.
fn quoted(items: &[String]) -> String {
    items
        .iter()
        .map(|h| format!("\"{}\"", h))
        .collect::<Vec<_>>()
        .join(", ")
}

impl Part {
    /// The words of the part, as a move prints them after its act.
    pub fn render(&self) -> String {
        match self {
            Part::File(p) => p.clone(),
            Part::Repository => "a git repository".into(),
            Part::Path => "the path".into(),
            Part::Ledger => "the ledger".into(),
            Part::MissingRows => "the missing rows".into(),
            Part::Row => "the row".into(),
            Part::RowAt(n) => format!("the row at line {}", n),
            Part::Line => "the line".into(),
            Part::LineOf(k) => format!("the {}", k.name()),
            Part::LineUnder(k, h) => format!("the {} of \"{}\"", k.name(), h),
            Part::OneOf(m) => format!("one of the {}", m.name()),
            Part::Retie(k, ids) => format!("the {} to {}", k.name(), ids.join(" and ")),
            Part::Entry => "the entry".into(),
            Part::EntryOf(name) => format!("the entry of \"{}\"", name),
            Part::AnswerSet => "the answer set".into(),
            Part::Definition => "the definition".into(),
            Part::Term => "the term".into(),
            Part::Word => "the word".into(),
            Part::Question => "the question".into(),
            Part::Section => "the section".into(),
            Part::Heading => "the heading".into(),
            Part::Option => "the option".into(),
            Part::Body(hs) => format!("the body of {}", quoted(hs)),
            Part::Plan => "the plan".into(),
            Part::Decision(heading) => format!("\"{}\"", heading),
            Part::Program(name) => format!("the {} program", name),
            Part::Input => "the input".into(),
            Part::Pending => "it".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every part a move can name, one of each, with its words: a new part
    /// does not compile until it is added here, so the set changes in one
    /// place a review reads whole.
    fn every_part() -> Vec<(Part, &'static str)> {
        let h = || "h".to_string();
        let all = vec![
            (Part::Repository, "a git repository"),
            (Part::File("x.md".into()), "x.md"),
            (Part::Path, "the path"),
            (Part::Ledger, "the ledger"),
            (Part::MissingRows, "the missing rows"),
            (Part::Row, "the row"),
            (Part::RowAt(3), "the row at line 3"),
            (Part::Line, "the line"),
            (Part::LineOf(LineKind::Depends), "the depends line"),
            (Part::LineUnder(LineKind::Id, h()), "the id line of \"h\""),
            (Part::OneOf(Many::IdLines), "one of the id lines"),
            (
                Part::Retie(LineKind::TieComment, vec!["a".into(), "b".into()]),
                "the tie comment to a and b",
            ),
            (Part::Entry, "the entry"),
            (Part::EntryOf(h()), "the entry of \"h\""),
            (Part::AnswerSet, "the answer set"),
            (Part::Definition, "the definition"),
            (Part::Term, "the term"),
            (Part::Word, "the word"),
            (Part::Question, "the question"),
            (Part::Section, "the section"),
            (Part::Heading, "the heading"),
            (Part::Option, "the option"),
            (Part::Body(vec![h()]), "the body of \"h\""),
            (Part::Plan, "the plan"),
            (Part::Decision(h()), "\"h\""),
            (Part::Program("codex".into()), "the codex program"),
            (Part::Input, "the input"),
            (Part::Pending, "it"),
        ];
        for (p, _) in &all {
            // No arm may be `_`: a new variant fails here first.
            match p {
                Part::Repository
                | Part::File(_)
                | Part::Path
                | Part::Ledger
                | Part::MissingRows
                | Part::Row
                | Part::RowAt(_)
                | Part::Line
                | Part::LineOf(_)
                | Part::LineUnder(..)
                | Part::OneOf(_)
                | Part::Retie(..)
                | Part::Entry
                | Part::EntryOf(_)
                | Part::AnswerSet
                | Part::Definition
                | Part::Term
                | Part::Word
                | Part::Question
                | Part::Section
                | Part::Heading
                | Part::Option
                | Part::Body(_)
                | Part::Plan
                | Part::Decision(_)
                | Part::Program(_)
                | Part::Input
                | Part::Pending => {}
            }
        }
        all
    }

    #[test]
    fn every_part_renders_as_pinned() {
        for (p, words) in every_part() {
            assert_eq!(p.render(), words, "{:?}", p);
        }
    }
}
