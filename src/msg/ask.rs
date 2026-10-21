//! The messages of `approve` and the lines of its summary.

use super::*;

// --- ask.rs -----------------------------------------------------------------

pub fn missing_answer_set(rel: &str, dependent: &str, target: &str) -> Said {
    said(
        &rule::DEPENDS_ENTRY_HAS_ANSWER_SET,
        format!(
            "{}: \"{}\" depends on \"{}\" without an answer set",
            rel, dependent, target
        ),
        Next::agent(Act::Writes(
            "an answer set (=A, =A|B or =*) on that depends entry".into(),
        ))
        .and_owner_runs(command(&["adr", "approve", rel])),
    )
}

/// A `depends:` entry that names what an earlier entry of the same line
/// already names, written another way.
pub fn depends_names_twice(rel: &str, dependent: &str, entry: &str) -> Said {
    said(
        &rule::DEPENDS_ENTRY_ONCE,
        format!(
            "{}: \"{}\": the depends entry \"{}\" names what an earlier entry of the line names",
            rel, dependent, entry
        ),
        Next::agent(Act::Removes("the entry".into())),
    )
}

/// A supersedes line naming one decision twice.
pub fn supersedes_names_twice(rel: &str, heading: &str, entry: &str) -> Said {
    said(
        &rule::SUPERSEDES_ENTRY_ONCE,
        format!(
            "{}: \"{}\": the supersedes entry \"{}\" names what an earlier entry of the line names",
            rel, heading, entry
        ),
        Next::agent(Act::Removes("the entry".into())),
    )
}

pub fn set_on_term(rel: &str, dependent: &str) -> Said {
    said(
        &rule::ANSWER_SET_NOT_ON_TERM,
        format!(
            "{}: \"{}\" depends on a term under an answer set",
            rel, dependent
        ),
        Next::agent(Act::Removes("the answer set from that entry".into()))
            .and_owner_runs(command(&["adr", "approve", rel])),
    )
}

pub fn set_on_answered_question(rel: &str, dependent: &str, target: &str) -> Said {
    said(
        &rule::ANSWER_SET_ON_OPEN_QUESTION_ONLY,
        format!(
            "{}: \"{}\" depends on \"{}\", which is answered, under an answer set",
            rel, dependent, target
        ),
        Next::agent(Act::Removes("the answer set from that entry".into()))
            .and_owner_runs(command(&["adr", "approve", rel])),
    )
}

pub fn question_without_body(rel: &str, heading: &str) -> Said {
    said(
        &rule::QUESTION_HAS_BODY,
        format!(
            "{}: \"{}\" is [{}]",
            rel,
            heading,
            crate::status::QuestionStatus::NoBody.name()
        ),
        Next::agent(Act::Writes(format!("the body of \"{}\"", heading))),
    )
}

/// An `unrelated:` entry names no current decision, no term, no other
/// question of the file and no dropped question.
pub fn unrelated_names_nothing(rel: &str, heading: &str, entry: &str) -> Said {
    said(
        &rule::UNRELATED_ENTRY_NAMES_SOMETHING,
        format!(
            "{}: \"{}\": the unrelated entry \"{}\" names no current decision, term or question of this file",
            rel, heading, entry
        ),
        Next::agent(Act::Rewrites("the entry".into())),
    )
}

/// An `unrelated:` entry is the heading of current decisions in several files.
pub fn unrelated_ambiguous(rel: &str, heading: &str, entry: &str, files: &[String]) -> Said {
    said(
        &rule::UNRELATED_ENTRY_NAMES_ONE_DECISION,
        format!(
            "{}: \"{}\": the unrelated entry \"{}\" is the heading of current decisions in {}",
            rel,
            heading,
            entry,
            files.join(", ")
        ),
        ambiguous_heading_next(),
    )
}
