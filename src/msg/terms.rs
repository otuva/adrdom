//! The messages about `adrdom/terms.md`, shared by `actions` and `check/terms.rs`.

use super::*;
use crate::status::FileStatus;

// --- adrdom/terms.md: shared by actions, check/terms.rs ------------------

pub fn terms_missing() -> Said {
    said(
        &crate::rule::TERMS_FILE_EXISTS,
        "adrdom/terms.md missing",
        Next::agent(Act::Writes("adrdom/terms.md".into()))
            .and_owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// A read/parse failure on `adrdom/terms.md` itself (an I/O error, not one
/// of its shape rules) -- rare enough that the caller's own context decides
/// who fixes it.
pub fn terms_load_error(e: &str, next: Next) -> String {
    refuse(
        format!("adrdom/terms.md: {}", e),
        next.on("adrdom/terms.md"),
    )
}

/// One of `adrdom/terms.md`'s shape errors (from `terms::parse`): always
/// the agent's fix, regardless of which command found it.
pub fn terms_shape_error(e: &str) -> Said {
    let fault = e.split_once(": ").map(|(_, f)| f).unwrap_or(e);
    let found = format!("adrdom/terms.md: {}", e);
    if fault.contains("used in both") {
        said(
            &crate::rule::WORD_NAMES_ONE_TERM,
            found,
            Next::agent(Act::Removes("the word from one of the two entries".into())),
        )
    } else if fault.starts_with("missing h1")
        || fault.starts_with("duplicate h1")
        || fault == "unknown line"
    {
        said(
            &crate::rule::TERMS_FILE_SHAPE,
            found,
            Next::agent(Act::Rewrites("the line".into())),
        )
    } else if fault.starts_with("duplicate id ") {
        said(
            &crate::rule::TERM_IDENTIFIER_IS_UNIQUE,
            found,
            Next::agent(Act::Rewrites("the entry".into())),
        )
    } else {
        said(
            &crate::rule::TERM_ENTRY_FIELDS,
            found,
            Next::agent(Act::Rewrites("the entry".into())),
        )
    }
}

/// `adrdom/terms.md` exists and parses, but isn't approved at its current
/// hash -- named from a dependent topic file's own path (`rel`), the way
/// `approve`/`answer` all report it.
pub fn terms_not_approved(rel: &str, status: FileStatus) -> Said {
    said(
        &crate::rule::TERMS_FILE_APPROVED,
        format!("{}: adrdom/terms.md is {}", rel, status.name()),
        Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// `adrdom/terms.md` exists and parses, but isn't approved at its current
/// hash (per "Does the tool stop while the terms file waits for approval?",
/// aa475aa3 A): `check`'s own report of the same fact `terms_not_approved`
/// names from a dependent topic file -- this one names the file itself,
/// since `check` judges it on its own, not on behalf of one dependent.
pub fn terms_file_not_approved(status: FileStatus) -> Said {
    said(
        &crate::rule::TERMS_FILE_APPROVED,
        format!("adrdom/terms.md is {}", status.name()),
        Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// `check`: no terms brief where the terms file is approved (per "Does
/// check hold the terms brief to the terms file?", ec3f875c A). `approve`
/// is the one writer (per "Does approve write the terms brief?", fcab4bf9
/// A), so the move is the owner's.
pub fn terms_brief_missing() -> Said {
    said(
        &crate::rule::TERMS_BRIEF_MATCHES_TERMS,
        format!("{} missing", crate::repo::TERMS_BRIEF_PATH),
        Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// `check`: the terms brief does not hold what the terms file gives.
pub fn terms_brief_differs() -> Said {
    said(
        &crate::rule::TERMS_BRIEF_MATCHES_TERMS,
        format!(
            "{} differs from adrdom/terms.md",
            crate::repo::TERMS_BRIEF_PATH
        ),
        Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// `check` at a commit: the working tree holds the right terms brief and
/// the staged files do not.
pub fn terms_brief_not_staged() -> Said {
    said(
        &crate::rule::STAGED_FILES_ARE_JUDGED_FILES,
        format!("{} not staged", crate::repo::TERMS_BRIEF_PATH),
        Next::agent_runs(command(&["git", "add", crate::repo::TERMS_BRIEF_PATH])),
    )
}

/// A cycle's names, in traversal order, closed by repeating the first at
/// the end: "a -> b -> a" -- what `circular_definition` names, and what the
/// page's own display of the same cycle names too.
pub fn cycle_chain(names: &[String]) -> String {
    let mut chain = names.to_vec();
    if let Some(first) = names.first() {
        chain.push(first.clone());
    }
    chain.join(" -> ")
}

pub fn circular_definition(names: &[String]) -> Said {
    let mut all = names.to_vec();
    if let Some(first) = names.first() {
        all.push(first.clone());
    }
    let last = all.pop().unwrap_or_default();
    let run = if all.is_empty() {
        last
    } else {
        format!("{} and {}", all.join(", "), last)
    };
    said(
        &crate::rule::DEFINITION_NOT_CIRCULAR,
        format!(
            "adrdom/terms.md: circular definition: a cycle runs through {}",
            run
        ),
        Next::agent(Act::Rewrites("one of these definitions".into())),
    )
}

/// A dependent (a decision's or a term's own `terms:` line) still names a
/// uuid that no longer exists in `adrdom/terms.md` at all.
/// `word` is the word `source` names the term by, when it names it by a
/// word (its terms line); a depends line names it by the identifier alone.
/// `decision` is the dependent's id and heading when it is a current
/// decision. Marking the term retired is not offered: a retire is held while
/// the dependent stands (per "What holds a retire of a term?"), so the move
/// is restoring the term as it was, then superseding the decision, named
/// by its identifier and heading (`named`).
pub fn term_removed_but_carried(
    uuid: &str,
    word: Option<&str>,
    source: &str,
    decision: Option<(&str, &str)>,
) -> Said {
    let next = match decision {
        Some((id, heading)) => {
            Next::agent(Act::Restores("the term in adrdom/terms.md".into())).then(Act::Writes(
                format!("a question that supersedes {}", named(id, Some(heading))),
            ))
        }
        None => Next::agent(Act::Restores("the term in adrdom/terms.md".into()))
            .and_owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    };
    said(
        &crate::rule::TERM_STAYS_WHILE_USED,
        format!(
            "adrdom/terms.md: term {} removed but used by {}",
            named(uuid, word),
            source
        ),
        next,
    )
}

/// A dependent still names a term that is now retired. `next` differs by
/// dependent kind: a decision supersedes itself with a new question
/// (built by the caller, which knows the decision's id); a term's own
/// definition is reworded by the agent and re-approved by the owner --
/// terms are never superseded (per "What holds a retire of a term?"; the fix for a term-dependent is
/// always this same pair, so it's fixed here).
pub fn term_retired_but_carried(name: &str, source: &str, user: RetiredUser) -> Said {
    let found = format!(
        "adrdom/terms.md: term \"{}\" retired but used by {}",
        name, source
    );
    match user {
        RetiredUser::Decision(id, heading) => said(
            &crate::rule::TERM_NOT_RETIRED_WHILE_NAMED,
            found,
            Next::agent(Act::Rewrites(format!("the entry of \"{}\"", name))).then(Act::Writes(
                format!("a question that supersedes {}", named(id, Some(heading))),
            )),
        ),
        RetiredUser::Asked(rel) => said(
            &crate::rule::TERM_NOT_RETIRED_WHILE_NAMED,
            found,
            Next::agent(Act::Rewrites("the question".into()))
                .and_owner_runs(command(&["adr", "approve", rel])),
        ),
        RetiredUser::PlanHeading(rel) => said(
            &crate::rule::TERM_NOT_RETIRED_WHILE_NAMED,
            found,
            Next::agent(Act::Rewrites("the heading".into()))
                .and_owner_runs(command(&["adr", "approve", rel])),
        ),
        RetiredUser::Definition => said(
            &crate::rule::DEFINITION_USES_LIVE_TERMS,
            found,
            Next::agent(Act::Rewrites("the definition".into()))
                .and_owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
        ),
    }
}

/// What still names a retired term, with the part of it a move needs.
pub enum RetiredUser<'a> {
    /// A current decision, by its identifier and heading.
    Decision(&'a str, &'a str),
    /// An asked question, by its topic file.
    Asked(&'a str),
    /// A heading of an approved plan, by its topic file.
    PlanHeading(&'a str),
    /// Another term's definition.
    Definition,
}

/// A dependent's pairing (`word=uuid`) no longer resolves that word to
/// that term at all -- the wording moved, without the term being retired
/// or removed. `next` mirrors `term_retired_but_carried`'s split.
pub fn word_moved_off_term(
    word: &str,
    term_name: &str,
    source: &str,
    decision_id: Option<&str>,
) -> Said {
    let next = match decision_id {
        // A current decision is never re-asked: the wording in the terms
        // file returns.
        Some(_) => Next::agent(Act::Rewrites(format!(
            "the entry of \"{}\" so \"{}\" names it",
            term_name, word
        ))),
        None => Next::agent(Act::Rewrites("the definition".into()))
            .and_owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    };
    said(
        &crate::rule::WORD_STAYS_ON_TERM,
        format!(
            "adrdom/terms.md: word \"{}\" no longer names term \"{}\", used by {}",
            word, term_name, source
        ),
        next,
    )
}

/// A term's definition runs past two sentences (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76:
/// a definition says what a thing is "in one or two sentences").
pub fn definition_too_long(name: &str, count: usize) -> Said {
    said(
        &crate::rule::DEFINITION_SENTENCE_LIMIT,
        format!(
            "adrdom/terms.md: term \"{}\" definition has {} sentences, the most is 2",
            name, count
        ),
        Next::agent(Act::Rewrites("the definition".into())),
    )
}

pub fn terms_banned_word(line: usize, suffix: &str) -> Said {
    said(
        &crate::rule::BANNED_WORD_ABSENT_FROM_RECORDS,
        format!("adrdom/terms.md:{}: banned word{}", line, suffix),
        Next::agent(Act::Rewrites("the line".into())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_duplicate_term_identifier_is_judged_by_its_own_rule() {
        let said = terms_shape_error("line 9: duplicate id \"u\" (also used at line 3)");
        assert_eq!(said.rule.name, "term_identifier_is_unique");
        assert_eq!(
            terms_shape_error("line 4: duplicate field \"id\"")
                .rule
                .name,
            "term_entry_fields"
        );
    }
}
