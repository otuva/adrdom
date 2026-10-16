//! The notes `check` prints, which are never a failure.

use super::*;

// --- check notes ------------------------------------------------------------

/// An untested current decision, a note and never a failure (per "What does
/// check do with an untested current decision?"), with its move (per "Does
/// check print a line with no move on it?", bd83aa22 A): `mv` is the move's
/// text, `moves::of_untested`.
pub fn untested(rel: &str, heading: &str, mv: Next) -> Said {
    said(
        &crate::rule::DECISION_HAS_TEST_TIE,
        format!(
            "{}: {} is answered and no tracked file holds its tie",
            rel, heading
        ),
        mv,
    )
}

/// A topic file `check` cannot read: a `parse` finding.
pub fn topic_file_unreadable(path: &str, e: impl Display) -> String {
    file_unreadable(path, e)
}

/// A file of the records that exists and cannot be read as text: a topic
/// file, or adrdom/terms.md.
pub fn file_unreadable(path: &str, e: impl Display) -> String {
    refuse(
        format!(
            "{}: cannot be read as text (e.g. its encoding): {}",
            path, e
        ),
        Next::agent(Act::Rewrites(path.to_string())),
    )
}

/// The owner's approve of one entry that waits, `adr approve adrdom/terms.md
/// <name>` (per "What does approve take for a terms file holding several
/// entries that wait?", da0dc8b1 A).
pub fn approve_entry_cmd(name: &str) -> Cmd {
    command(&["adr", "approve", crate::repo::TERMS_PATH, name])
}

/// The owner's approve of one entry of those the finding names: the owner
/// writes the name in place of `<name>`, as `<letter>` in an answer.
fn approve_one_of() -> Next {
    Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md").then("<name>"))
}

fn quoted_list(names: &[String]) -> String {
    names
        .iter()
        .map(|n| format!("\"{}\"", n))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One entry of `adrdom/terms.md` waits for the owner's approve (per "Does
/// the tool stop while the terms file waits for approval?", aa475aa3 A, and
/// "What does approve take for a terms file holding several entries that
/// wait?", da0dc8b1 A): one finding per entry, each with its own approve.
/// `rel` is the topic file a gated command named, as `terms_not_approved`.
pub fn terms_entry_waits(rel: Option<&str>, name: &str, status: crate::status::TermStatus) -> Said {
    let at = rel.map(|r| format!("{}: ", r)).unwrap_or_default();
    said(
        &crate::rule::TERMS_FILE_APPROVED,
        format!("{}adrdom/terms.md: \"{}\" is {}", at, name, status.name()),
        Next::owner_runs(approve_entry_cmd(name)),
    )
}

/// `adr approve adrdom/terms.md` with no name while entries wait: the
/// owner names one (per da0dc8b1 A).
pub fn approve_terms_names_no_entry(waiting: &[String]) -> Said {
    said(
        &crate::rule::APPROVE_NAMES_ONE_TERM_ENTRY,
        format!(
            "adrdom/terms.md: {} wait, and approve names none of them",
            quoted_list(waiting)
        ),
        approve_one_of(),
    )
}

/// `adr approve adrdom/terms.md <name>` for a name no entry that waits
/// holds; with none waiting, the file is approved as a whole.
pub fn approve_terms_entry_not_waiting(name: &str, waiting: &[String]) -> Said {
    let (found, next) = if waiting.is_empty() {
        (
            format!(
                "adrdom/terms.md: \"{}\" does not wait, and no entry does",
                name
            ),
            Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
        )
    } else {
        (
            format!(
                "adrdom/terms.md: \"{}\" does not wait; {} wait",
                name,
                quoted_list(waiting)
            ),
            approve_one_of(),
        )
    };
    said(&crate::rule::APPROVE_NAMES_ONE_TERM_ENTRY, found, next)
}

/// An entry name given to approve of a file other than the terms file.
pub fn approve_entry_on_other_file(rel: &str) -> Said {
    said(
        &crate::rule::APPROVE_NAMES_ONE_TERM_ENTRY,
        format!(
            "{}: approve takes an entry name for adrdom/terms.md only",
            rel
        ),
        Next::owner_runs(command(&["adr", "approve", rel])),
    )
}

/// The finding on unban's input when `n` (never 0) of the words given were
/// not banned. A count and never the words: a word given here may be a
/// misspelling of a banned one.
pub fn unban_not_banned(n: usize) -> String {
    refuse(
        format!("unban: {} of the words given were not banned", n),
        Next::owner_runs(Cmd::fixed("adr unban")),
    )
}

/// `check --message <file>` could not read the commit message file.
pub fn message_file_unreadable(path: &str, e: impl Display) -> String {
    refuse(
        format!("cannot read the commit message file {}: {}", path, e),
        Next::agent(Act::Rewrites(format!("the commit message file {}", path))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untested_names_the_decision_its_place_and_its_move() {
        let line = untested("x.md", "heading", Next::agent(Act::Builds("it".into()))).text();
        assert_eq!(
            line,
            "x.md: heading is answered and no tracked file holds its tie; agent builds it"
        );
    }

    // adr: 50b31713-4fea-4aa3-842a-79c2750660ba
    //
    // A move that has two legitimate acts names both, and the one for a
}
