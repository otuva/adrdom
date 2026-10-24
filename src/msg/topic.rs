//! The refusals about the shape, tells, depends lines and tiers of a topic file.

use super::*;

// --- a topic file's own shape/tells/depends/tier refusals ------------------
// Shared by `rules::record_refusals` (approve + check) and, for the
// derive-error variant, by `approve` as well.

pub fn shape_error(rel: &str, e: &str) -> Said {
    if e.starts_with("tier heading") {
        return said(
            &rule::TIER_HEADING_ABSENT,
            format!("{}: {}", rel, e),
            Next::agent(Act::Removes(Part::Heading))
                .then(Act::Writes(Part::LineOf(LineKind::Depends))),
        );
    }
    said(
        &rule::TOPIC_FILE_SHAPE,
        format!("{}: {}", rel, e),
        Next::agent(Act::Rewrites(Part::File(rel.to_string()))),
    )
}

/// A `### ` line that does not end in `?` (per "What does a topic file
/// hold?": a heading is a question).
pub fn heading_not_a_question(rel: &str, line: usize) -> Said {
    said(
        &rule::QUESTION_HEADING_ENDS_IN_MARK,
        format!(
            "{}: line {}: a heading is not a question ending in ?",
            rel, line
        ),
        Next::agent(Act::Rewrites(Part::Line)),
    )
}

pub fn section_shape_error(rel: &str, heading: &str, e: &str) -> Said {
    let at = format!("{}: \"{}\": ", rel, heading);
    if e.starts_with("depends: duplicate entry") {
        return said(
            &rule::DEPENDS_ENTRY_ONCE,
            format!("{}{}", at, e),
            Next::agent(Act::Removes(Part::Entry)),
        );
    }
    if e.starts_with("second answer line") {
        return said(
            &rule::ANSWER_LINE_IS_LAST,
            format!("{}{}", at, e),
            Next::agent(Act::Removes(Part::LineOf(LineKind::Answer))),
        );
    }
    if e.starts_with("answer line not last") {
        return said(
            &rule::ANSWER_LINE_IS_LAST,
            format!("{}{}", at, e),
            Next::agent(Act::Removes(Part::Line)),
        );
    }
    for kind in ["depends", "supersedes", "blame", "terms"] {
        let Some(rest) = e.strip_prefix(&format!("{}: ", kind)) else {
            continue;
        };
        if rest.starts_with("second ") {
            let n = e.rsplit(' ').next().unwrap_or("");
            return said(
                &rule::REFERENCE_LINE_ONCE,
                format!("{}{} second line, line {}", at, kind, n),
                Next::agent(Act::Rewrites(Part::LineOf(line_kind(kind)))),
            );
        }
        return said(
            &rule::REFERENCE_LINE_ONCE,
            format!("{}{}", at, e),
            Next::agent(Act::Rewrites(Part::LineOf(line_kind(kind)))),
        );
    }
    said(
        &rule::QUESTION_SECTION_SHAPE,
        format!("{}{}", at, e),
        Next::agent(Act::Rewrites(Part::Question)),
    )
}

/// One provenance/code-state/date/loaded-heading tell (per "Does the tool
/// stop loaded phrases in a question?").
pub fn tell(
    rel: &str,
    line: usize,
    label: &str,
    phrase: &str,
    kind: crate::tells::TellKind,
) -> Said {
    use crate::tells::TellKind;
    let at = format!("{}:{}: \"{}\": ", rel, line + 1, label);
    let (r, found, mv) = match kind {
        TellKind::Provenance => (
            &rule::TELL_NAMES_SPEAKER,
            format!("\"{}\" names who said it", phrase),
            Part::Line,
        ),
        TellKind::CodeState => (
            &rule::TELL_STATES_CODE_NOW,
            format!("\"{}\" states what the code does", phrase),
            Part::Line,
        ),
        TellKind::Date => (
            &rule::TELL_CARRIES_DATE,
            format!("\"{}\" is a date", phrase),
            Part::Line,
        ),
        TellKind::LoadedOpening => (
            &rule::TELL_ASSUMES_ANSWER,
            format!("the heading opens with \"{}\"", phrase),
            Part::Heading,
        ),
        TellKind::Join => (
            &rule::TELL_JOINS_TWO_QUESTIONS,
            format!("the heading holds \"{}\"", phrase),
            Part::Heading,
        ),
    };
    said(
        r,
        format!("{}{}", at, found),
        Next::agent(Act::Rewrites(mv)),
    )
}

/// An option, Why line or Costs line over the length rule (per "Does ask
/// refuse an option over a word limit?"); `words` is the line's word count.
pub fn too_long(
    rel: &str,
    line: usize,
    label: &str,
    kind: crate::tells::LengthKind,
    words: usize,
) -> Said {
    use crate::tells::LengthKind;
    let (r, thing, most, part) = match kind {
        LengthKind::Why => (
            &rule::WHY_WORD_LIMIT,
            "Why line",
            crate::tells::WORD_LIMIT,
            Part::Line,
        ),
        LengthKind::Costs => (
            &rule::COSTS_WORD_LIMIT,
            "Costs line",
            crate::tells::WORD_LIMIT,
            Part::Line,
        ),
        LengthKind::OneWordOption => (&rule::DOES_OPTION_IS_ONE_WORD, "option", 1, Part::Option),
        LengthKind::Option => (
            &rule::OPTION_WORD_LIMIT,
            "option",
            crate::tells::WORD_LIMIT,
            Part::Option,
        ),
    };
    said(
        r,
        format!(
            "{}:{}: \"{}\": the {} has {} words, the most is {}",
            rel,
            line + 1,
            label,
            thing,
            words,
            most
        ),
        Next::agent(Act::Rewrites(part)),
    )
}

pub fn no_depends_line(rel: &str, heading: &str) -> Said {
    said(
        &rule::QUESTION_HAS_DEPENDS_LINE,
        format!("{}: \"{}\" has no depends line", rel, heading),
        Next::agent(Act::Writes(Part::LineUnder(
            LineKind::Depends,
            heading.to_string(),
        ))),
    )
}

/// Same as `no_depends_line`, without naming the file: `approve`'s own
/// defensive fallback in `insert_missing_ids`, unreachable in practice
/// since `record_refusals` already refuses on this first.
pub fn no_depends_line_bare(heading: &str) -> Said {
    said(
        &rule::QUESTION_HAS_DEPENDS_LINE,
        format!("\"{}\" has no depends line", heading),
        Next::agent(Act::Writes(Part::LineUnder(
            LineKind::Depends,
            heading.to_string(),
        ))),
    )
}

/// A closed question's depends line names `entry` by text: `st` is its
/// status (answered, refused or superseded), named as the listing names it.
pub fn frozen_depends_text(
    rel: &str,
    heading: &str,
    st: crate::status::QuestionStatus,
    entry: &str,
) -> Said {
    said(
        &rule::ANSWERED_DEPENDS_LINE_FROZEN,
        format!(
            "{}: \"{}\" is [{}] and its depends line names \"{}\" by text",
            rel,
            heading,
            st.name(),
            entry
        ),
        Next::agent(Act::Restores(Part::Line)),
    )
}

pub fn dep_not_current_decision(rel: &str, heading: &str, id: &str, name: Option<&str>) -> Said {
    said(
        &rule::DEPENDS_ENTRY_RESOLVES,
        format!(
            "{}: \"{}\" depends on {} which names no term and no question of this file",
            rel,
            heading,
            named(id, name)
        ),
        Next::agent(Act::Rewrites(Part::Entry)),
    )
}

/// A live section's depends line names, by identifier, a term that is
/// retired.
pub fn dep_retired_term_id(rel: &str, heading: &str, id: &str, name: Option<&str>) -> Said {
    said(
        &rule::DEPENDS_TERM_IS_LIVE,
        format!(
            "{}: \"{}\" depends on the term {}, which is retired",
            rel,
            heading,
            name.map_or_else(|| id.to_string(), |n| format!("\"{}\"", n))
        ),
        Next::agent(Act::Rewrites(Part::Entry)),
    )
}

pub fn dep_no_term(rel: &str, heading: &str, text: &str) -> Said {
    said(
        &rule::DEPENDS_TERM_EXISTS,
        format!(
            "{}: \"{}\" depends on \"{}\" which is no term",
            rel, heading, text
        ),
        Next::agent(Act::Writes(Part::Term)),
    )
}

/// A `depends:` entry, on a question being judged, names a current decision
/// of another topic file by its identifier.
pub fn dep_names_decision_of_other_file(
    rel: &str,
    heading: &str,
    id: &str,
    name: Option<&str>,
) -> Said {
    dep_names_other_file(rel, heading, &named(id, name))
}

/// The same, written as the heading of that decision.
pub fn dep_names_heading_of_other_file(rel: &str, heading: &str, text: &str) -> Said {
    dep_names_other_file(rel, heading, &format!("\"{}\"", text))
}

fn dep_names_other_file(rel: &str, heading: &str, target: &str) -> Said {
    said(
        &rule::DEPENDS_ENTRY_NAMES_TERM_OR_FILE_QUESTION,
        format!(
            "{}: \"{}\" depends on {}, a decision of another topic file",
            rel, heading, target
        ),
        Next::agent(Act::Rewrites(Part::LineUnder(
            LineKind::Depends,
            heading.to_string(),
        ))),
    )
}

/// The depends line of a question being judged names more questions than the
/// cap: `names` are the questions its entries name, in line order.
pub fn depends_line_over_limit(rel: &str, heading: &str, names: &[String], cap: usize) -> Said {
    let list = names
        .iter()
        .map(|n| format!("\"{}\"", n))
        .collect::<Vec<_>>()
        .join(", ");
    said(
        &rule::DEPENDS_LINE_QUESTION_LIMIT,
        format!(
            "{}: \"{}\" depends on {} questions, the most is {}: {}",
            rel,
            heading,
            names.len(),
            cap,
            list
        ),
        Next::agent(Act::Rewrites(Part::LineUnder(
            LineKind::Depends,
            heading.to_string(),
        ))),
    )
}

/// `show <file>` on a file that is not one of the topic files.
/// `show` names a path approve does not take (per "Does show print what a file
/// holds past its latest approval?": show takes every file approve
/// takes).
/// `approve` names a path that is no topic file: it asks the questions of topic
/// files alone.
pub fn plan_not_a_topic_file(rel: &str) -> String {
    refuse(
        format!("{} is not a topic file", rel),
        Next::agent(Act::Rewrites(Part::Path)),
    )
}

/// `adr approve` on a path outside the repository or outside what approve
/// takes: the owner names one it takes.
pub fn approve_not_a_file_approve_takes(path: &str) -> String {
    refuse(
        format!("{} is not a file approve takes", path),
        Next::owner(Act::Rewrites(Part::Path)),
    )
}

pub fn show_not_a_file_approve_takes(rel: &str) -> String {
    refuse(
        format!("{} is not a file approve takes", rel),
        Next::agent(Act::Rewrites(Part::Path)),
    )
}

pub fn dep_retired_term(rel: &str, heading: &str, text: &str) -> Said {
    said(
        &rule::DEPENDS_TERM_IS_LIVE,
        format!(
            "{}: \"{}\" names the retired term \"{}\"",
            rel, heading, text
        ),
        Next::agent(Act::Rewrites(Part::Entry)),
    )
}

pub(super) fn ambiguous_heading_next() -> Next {
    Next::agent(Act::Writes(Part::Entry))
}

/// A heading `depends:` entry's `=A|B` set names a letter its target
/// doesn't have.
pub fn bad_option_letter(rel: &str, dependent: &str, target: &str, letter: char) -> Said {
    said(
        &rule::ANSWER_SET_NAMES_TARGET_OPTIONS,
        format!(
            "{}: \"{}\" depends on \"{}\" under an answer set naming option {}, which it does not have",
            rel, dependent, target, letter
        ),
        Next::agent(Act::Rewrites(Part::AnswerSet)),
    )
}

/// `parent` is the question the group stands on, `None` for terms alone.
/// `open` are the questions of the group that are not answered yet, in file
/// order; `answered` counts the ones the owner answered, which hold their
/// place and are never moved.
pub fn stand_cap_exceeded(
    rel: &str,
    parent: Option<&str>,
    open: &[String],
    answered: usize,
    cap: usize,
) -> Said {
    let names = open
        .iter()
        .map(|h| format!("\"{}\"", h))
        .collect::<Vec<_>>()
        .join(", ");
    let on = match parent {
        Some(h) => format!("on \"{}\"", h),
        None => "on terms alone".to_string(),
    };
    let found = if answered == 0 {
        format!(
            "{}: {} questions stand {}, the most is {}: {}",
            rel,
            open.len(),
            on,
            cap,
            names
        )
    } else {
        format!(
            "{}: {} questions stand {}, {} answered, the most is {}: {}",
            rel,
            open.len() + answered,
            on,
            answered,
            cap,
            names
        )
    };
    let next = Next::agent(Act::Writes(Part::Question));
    said(&rule::STAND_QUESTION_LIMIT, found, next)
}

// `derive_tiers`' three failure shapes (record::DeriveError), each with two
// wordings: the plain one (`approve`/`check`'s `plan` category) and the one approve's asking uses,
// which always points at re-approving afterward.

use crate::record::DeriveError;

fn derive_what(rel: &str, e: &DeriveError) -> (&'static rule::Rule, String, Next) {
    match e {
        DeriveError::Dangling { section, entry } => (
            &rule::DEPENDS_ENTRY_RESOLVES,
            format!(
                "{}: \"{}\" depends on \"{}\" which is no heading in this file",
                rel, section, entry
            ),
            Next::agent(Act::Rewrites(Part::Entry)),
        ),
        DeriveError::Cycle(names) => (
            &rule::DEPENDS_LINES_HAVE_NO_CYCLE,
            format!(
                "{}: depends lines lead back to their own question: {}",
                rel,
                names
                    .iter()
                    .map(|n| format!("\"{}\"", n))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Next::agent(Act::Rewrites(Part::LineOf(LineKind::Depends))),
        ),
        DeriveError::SelfDep(h) => (
            &rule::QUESTION_NOT_ON_ITSELF,
            format!("{}: \"{}\" depends on itself", rel, h),
            Next::agent(Act::Removes(Part::Entry)),
        ),
    }
}

pub fn derive_error(rel: &str, e: &DeriveError) -> Said {
    let (r, found, next) = derive_what(rel, e);
    said(r, found, next)
}

pub fn answered_id_missing(id: &str, heading: &str) -> Said {
    said(
        &crate::rule::ANSWERED_QUESTION_STAYS_IN_FILES,
        format!(
            "id {} (\"{}\") is answered and current but missing from every topic file",
            id, heading
        ),
        Next::agent(Act::Restores(Part::Section)),
    )
}

/// The kind of a reference line by the word that opens it.
fn line_kind(word: &str) -> LineKind {
    match word {
        "depends" => LineKind::Depends,
        "supersedes" => LineKind::Supersedes,
        "blame" => LineKind::Blame,
        _ => LineKind::Terms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    #[test]
    fn a_second_blame_line_is_a_reference_line_given_twice() {
        assert_eq!(
            section_shape_error("x.md", "Does X?", "blame: second blame line, line 6").text(),
            "x.md: \"Does X?\": blame second line, line 6; agent rewrites the blame line"
        );
    }

    #[test]
    fn a_tell_and_a_length_hit_pick_their_rule_from_their_kind() {
        use crate::tells::{LengthKind, TellKind};
        assert_eq!(
            tell("x.md", 1, "H?", "said", TellKind::Provenance)
                .rule
                .name,
            rule::TELL_NAMES_SPEAKER.name
        );
        assert_eq!(
            tell("x.md", 1, "H?", "and", TellKind::Join).rule.name,
            rule::TELL_JOINS_TWO_QUESTIONS.name
        );
        assert_eq!(
            too_long("x.md", 1, "H?", LengthKind::OneWordOption, 3)
                .rule
                .name,
            rule::DOES_OPTION_IS_ONE_WORD.name
        );
        assert_eq!(
            too_long("x.md", 1, "H?", LengthKind::Option, 30).rule.name,
            rule::OPTION_WORD_LIMIT.name
        );
        assert_eq!(
            too_long("x.md", 1, "H?", LengthKind::Why, 30).rule.name,
            rule::WHY_WORD_LIMIT.name
        );
        assert_eq!(
            too_long("x.md", 1, "H?", LengthKind::Costs, 30).rule.name,
            rule::COSTS_WORD_LIMIT.name
        );
    }
}
