//! The messages of `answer` and `refuse` (`actions.rs`).

use super::*;

// --- actions.rs: answer/refuse --------------------------------------------

pub fn id_not_found(id: &str) -> String {
    refuse(
        format!(
            "id {} not found in any topic file; show prints the id of every question that waits for an answer",
            id
        ),
        Next::owner_runs(Cmd::fixed("adr show")),
    )
}

/// The id was in no topic file that could be read, and `unread` (paths,
/// never empty) could not be read at all: the question may be in one of
/// them.
pub fn id_not_found_some_unread(id: &str, unread: &[String]) -> String {
    refuse(
        format!(
            "id {} not found in the topic files that could be read; cannot read {}",
            id,
            unread.join(", ")
        ),
        Next::agent(Act::Rewrites("each file that cannot be read".into())),
    )
}

/// The question `answer` was given is closed: its latest ledger state is
/// `st`, an answer, a refusal, a supersede, a withdrawal or a drop.
pub fn already_closed(heading: &str, st: crate::status::QuestionStatus) -> String {
    refuse(
        format!(
            "\"{}\" is already {}; show prints every question that waits for an answer",
            heading,
            st.name()
        ),
        Next::owner_runs(Cmd::fixed("adr show")),
    )
}

/// `letters` are the letters of the question's options; `id` and `version`
/// are the ones the owner gave, both good when this is printed.
pub fn letter_not_an_option(
    letter: char,
    heading: &str,
    letters: &[char],
    id: &str,
    version: &str,
) -> String {
    let has: Vec<String> = letters.iter().map(|c| c.to_string()).collect();
    refuse(
        format!(
            "letter {} does not name an option in \"{}\"; its options are {}",
            letter,
            heading,
            has.join(", ")
        ),
        crate::moves::clause(MoveKind::Answer {
            id: id.to_string(),
            version: version.to_string(),
            heading: heading.to_string(),
        }),
    )
}

/// `report`: what the rules find in the question's file and its moves
/// (`moves::of_unasked_question_in`).
pub fn not_in_asked_state(heading: &str, report: &crate::moves::Report) -> String {
    format!(
        "\"{}\" is edited since it was asked; {}",
        heading,
        report
            .moves_tail(Next::agent(Act::Restores("the question".into())))
            .render()
    )
}

/// A question with a body that no `ask` has put to the owner.
/// `report`: what the rules find in the question's file and its moves
/// (`moves::of_unasked_question_in`).
pub fn not_asked_yet(heading: &str, rel: &str, report: &crate::moves::Report) -> String {
    format!(
        "\"{}\" is {}; {}",
        heading,
        crate::status::QuestionStatus::NotAsked.name(),
        report
            .moves_tail(Next::agent_runs(command(&["adr", "ask", rel])))
            .render()
    )
}

/// An answer that would remove a question with no identifier.
pub fn drop_without_id(heading: &str, rel: &str) -> Said {
    said(
        &crate::rule::DROP_NAMES_AN_IDENTIFIER,
        format!(
            "answering would remove \"{}\", which has no identifier",
            heading
        ),
        Next::owner_runs(command(&["adr", "approve", rel])),
    )
}

/// An answer that would remove a question that holds a decision or a
/// refusal (per "Does a decision change after it is answered?").
pub fn drop_of_closed(heading: &str, refused: bool, rel: &str) -> Said {
    said(
        &crate::rule::ANSWER_NEVER_REMOVES_CLOSED_QUESTION,
        format!(
            "answering would remove \"{}\", which is {}",
            heading,
            if refused { "refused" } else { "answered" }
        ),
        Next::agent(Act::Writes(format!(
            "a question that supersedes \"{}\"",
            heading
        )))
        .and_owner_runs(command(&["adr", "approve", rel])),
    )
}

/// An answer that would remove a question whose text matches no approval row.
pub fn drop_not_approved(heading: &str, rel: &str) -> Said {
    said(
        &crate::rule::DROP_HAS_APPROVED_TEXT,
        format!(
            "answering would remove \"{}\", whose text matches no approved row of {}",
            heading, rel
        ),
        Next::owner_runs(command(&["adr", "approve", rel])),
    )
}

/// A question with no body yet: nothing to ask or answer.
pub fn not_written(heading: &str, report: &crate::moves::Report) -> String {
    format!(
        "\"{}\" is [{}]; {}",
        heading,
        crate::status::QuestionStatus::NoBody.name(),
        report
            .moves_tail(Next::agent(Act::Writes(format!(
                "the body of \"{}\"",
                heading
            ))))
            .render()
    )
}

/// `adr answer <id> <letter>` with no `@<version>` (per "Does an answer
/// carry the version of the question the owner read?"): a bare id is
/// refused before it is ever looked up, since there is nothing yet to
/// check it against.
/// `asked` is the heading and the version of the question `id` names, when
/// it names one that is asked: the command is then printed whole.
pub fn answer_missing_version(id: &str, asked: Option<(&str, &str)>) -> String {
    match asked {
        Some((heading, version)) => refuse(
            format!("{}: no version given", named(id, Some(heading))),
            crate::moves::clause(MoveKind::Answer {
                id: id.to_string(),
                version: version.to_string(),
                heading: heading.to_string(),
            }),
        ),
        None => refuse(
            format!("{}: no version given", id),
            Next::owner_runs(Cmd::fixed("adr show")),
        ),
    }
}

/// The version on an answer (from the terminal's `<id>@<version>` or the
/// page's hidden field) no longer matches the question's current text (per
/// "Does an answer carry the version of the question the owner read?"):
/// the question moved -- usually a re-ask after an edit -- since that
/// version was read.
///
/// The refusal never prints the current version: the version on an answer
/// is the owner's receipt of having read the text, so the owner is sent to
/// read the file as it is now.
pub fn stale_version(heading: &str, rel: &str) -> String {
    refuse(
        format!(
            "\"{}\" has moved since that version was read; show prints it as it is now",
            heading
        ),
        Next::owner_runs(command(&["adr", "show", rel])),
    )
}

/// The version on `<id>@<version>` is not exactly `record::VERSION_LEN`
/// hex characters once lowercased (per "Does an answer carry the version
/// of the question the owner read?"): a malformed argument, never a
/// question that moved after being read, so it gets its own message
/// instead of `stale_version`.
pub fn invalid_version_format(version: &str) -> String {
    refuse(
        format!(
            "\"{}\" is not a valid version; show prints the id and version of every question that waits for an answer",
            version
        ),
        Next::owner_runs(Cmd::fixed("adr show")),
    )
}

/// The move that writes an asked question's terms line again: `ask` of its
/// file, which rewrites the line in place.
pub(super) fn ask_again(rel: &str) -> Next {
    Next::agent_runs(command(&["adr", "ask", rel]))
}

pub fn no_terms_line(rel: &str, id: &str, heading: &str) -> Said {
    said(
        &crate::rule::ASKED_QUESTION_HAS_TERMS_LINE,
        format!("{}: no terms line", named(id, Some(heading))),
        ask_again(rel),
    )
}

pub fn malformed_terms_line(rel: &str, id: &str, heading: &str) -> Said {
    said(
        &crate::rule::ASKED_TERMS_LINE_PARSES,
        format!("{}: malformed terms line", named(id, Some(heading))),
        ask_again(rel),
    )
}

/// `word` is the word of the question the gone term was named by.
pub fn terms_line_names_gone_term(
    rel: &str,
    id: &str,
    heading: &str,
    uuid: &str,
    word: &str,
) -> Said {
    said(
        &crate::rule::ANSWERED_TERMS_LINE_TERM_EXISTS,
        format!(
            "{}: terms line names term {} that no longer exists",
            named(id, Some(heading)),
            named(uuid, Some(word))
        ),
        ask_again(rel),
    )
}

pub fn terms_line_names_retired_term(rel: &str, id: &str, heading: &str, name: &str) -> Said {
    said(
        &crate::rule::ANSWERED_TERMS_LINE_TERM_IS_LIVE,
        format!(
            "{}: terms line names retired term \"{}\"",
            named(id, Some(heading)),
            name
        ),
        ask_again(rel),
    )
}

/// An identifier with the name the tool can put to it (a heading, a term)
/// right beside it, `<id> ("<name>")`, or the identifier alone when the
/// tool holds no name for it.
pub fn named(id: &str, name: Option<&str>) -> String {
    match name {
        Some(n) => format!("{} (\"{}\")", id, n),
        None => id.to_string(),
    }
}
