//! The messages about the blame line of a question, the blame files and the blame check.

use super::*;
use crate::status::FileStatus;

// --- the blame line of a question -------------------------------------------
// One line under a heading naming the approved blame file and the row ranges
// the question was born from (per "What does a question carry of the blame
// file it came from?"), judged by `rules::blame_line_refusals`.

/// A `blame:` entry that names no row range (`entry` as the line writes it):
/// a name alone.
pub fn blame_entry_without_range(rel: &str, heading: &str, entry: &str) -> Said {
    blame_entry_incomplete(rel, heading, entry, "row range")
}

fn blame_entry_incomplete(rel: &str, heading: &str, entry: &str, lacks: &str) -> Said {
    said(
        &rule::BLAME_LINE_NAMES_ROW_RANGES,
        format!(
            "{}: \"{}\": the blame entry \"{}\" names no {}",
            rel, heading, entry, lacks
        ),
        Next::agent(Act::Writes(
            "the name of a blame file and the ranges of its rows".into(),
        )),
    )
}

/// A `blame:` entry whose name or identifier is no blame file.
pub fn blame_names_no_file(rel: &str, heading: &str, name: &str) -> Said {
    said(
        &rule::BLAME_LINE_NAMES_APPROVED_FILE,
        format!(
            "{}: \"{}\": the blame entry \"{}\" names no blame file",
            rel, heading, name
        ),
        Next::agent(Act::Writes(
            "the path of a blame file under adrdom/blame without .md".into(),
        )),
    )
}

/// A `blame:` entry names the blame file `blame_rel`, which awaits its
/// approval (`status` is new or changed): the line waits for it.
pub fn blame_file_awaits_approval(
    rel: &str,
    heading: &str,
    blame_rel: &str,
    status: FileStatus,
) -> Said {
    said(
        &rule::BLAME_LINE_NAMES_APPROVED_FILE,
        format!(
            "{}: \"{}\": the blame line names {}, which is {}",
            rel,
            heading,
            blame_rel,
            status.name()
        ),
        crate::moves::approval_wait(),
    )
}

/// A `blame:` entry names a range that is the range of no row of the blame
/// file `blame_rel`.
pub fn blame_range_of_no_row(
    rel: &str,
    heading: &str,
    blame_rel: &str,
    from: usize,
    to: usize,
) -> Said {
    said(
        &rule::BLAME_LINE_NAMES_ROW_RANGES,
        format!(
            "{}: \"{}\": the blame line names {}-{}, the range of no row of {}",
            rel, heading, from, to, blame_rel
        ),
        Next::agent(Act::Writes("the range of a row of that blame file".into())),
    )
}

/// A question not yet asked has no neighbours on record for its heading.
pub fn plan_neighbours_missing(rel: &str, heading: &str) -> Said {
    said(
        &rule::PLAN_STEP_HAS_ITS_RECORD,
        format!(
            "{}: \"{}\" has no neighbours on record for its heading",
            rel, heading
        ),
        crate::moves::clause(MoveKind::Plan {
            rel: rel.to_string(),
        }),
    )
}

/// A question not yet asked has no rows of the reader at its body hash.
pub fn plan_rows_missing(rel: &str, heading: &str) -> Said {
    said(
        &rule::PLAN_STEP_HAS_ITS_RECORD,
        format!(
            "{}: \"{}\" has no reader rows for its heading with its body",
            rel, heading
        ),
        crate::moves::clause(MoveKind::Plan {
            rel: rel.to_string(),
        }),
    )
}

/// A neighbour in the reader's rows is on no depends, supersedes or
/// unrelated line of the question.
pub fn plan_line_missing(rel: &str, heading: &str, kind: &str, name: &str) -> Said {
    said(
        &rule::NEIGHBOUR_IS_ON_A_LINE,
        format!(
            "{}: the {} \"{}\" is a neighbour of \"{}\" and is on no line",
            rel, kind, name, heading
        ),
        Next::agent(Act::Writes(format!(
            "the missing line under \"{}\"",
            heading
        ))),
    )
}

/// One question `adr plan` read the neighbours of, and the step it read them
/// at: its heading, or its heading with its body.
pub fn plan_question_read(heading: &str, what: &str) -> String {
    format!("\"{}\": {}", heading, what)
}

/// One neighbour of a question as every command prints it (`adr plan` for
/// what it read, `adr show --neighbours`): its kind and name, the step whose
/// text it was brought for and the reader's word (a neighbour of a heading
/// has none), the line the agent put it on, and the reader's sentence of
/// reason (the same).
pub fn neighbour(
    kind: &str,
    name: &str,
    step: &str,
    word: Option<&str>,
    line: Option<&str>,
    reason: &str,
) -> String {
    let on = match line {
        Some(l) => format!("on the {} line", l),
        None => "no line".to_string(),
    };
    let at = match word {
        Some(w) => format!("{} {}", step, w),
        None => step.to_string(),
    };
    let text = format!("{} \"{}\": {}; {}", kind, name, at, on);
    if reason.is_empty() {
        text
    } else {
        format!("{}; {}", text, reason)
    }
}

/// One pair of current decisions the reader marked same or inconsistent,
/// as `adr read` prints it; no other word marks a pair.
pub fn read_pair(word: &str, first: &str, second: &str, reason: &str, next: Next) -> Said {
    debug_assert!(matches!(word, "same" | "inconsistent"), "{}", word);
    let rule = match word {
        "same" => &crate::rule::READ_PAIR_SAME,
        _ => &crate::rule::READ_PAIR_INCONSISTENT,
    };
    said(
        rule,
        format!("{}: \"{}\" and \"{}\": {}", word, first, second, reason),
        next,
    )
}

pub fn ask_supersedes_not_current(rel: &str, heading: &str, old: &str, name: Option<&str>) -> Said {
    let target = if crate::record::is_uuid(old) {
        named(old, name)
    } else {
        format!("\"{}\"", old)
    };
    said(
        &rule::SUPERSEDES_TARGET_IS_CURRENT,
        format!(
            "{}: \"{}\": supersedes {}, which is not a current decision",
            rel, heading, target
        ),
        Next::agent(Act::Rewrites("the supersedes entry".into())),
    )
}

/// The moment a refusal about the supersedes line is printed: it decides
/// whether the agent's move ends in asking again.
#[derive(Clone, Copy)]
pub enum SupersedesMoment {
    /// `approve`: nothing of the question has been asked yet.
    Approve,
    /// `ask` or `answer`: the question was, or is about to be, asked.
    Asked,
}

/// The supersedes line of `heading` leaves out current decisions standing
/// on what it replaces. `missing` are `(file, heading, id)`; a heading of
/// another file is printed with its file.
pub fn supersedes_leaves_out(
    rel: &str,
    heading: &str,
    missing: &[(String, String, String)],
    moment: SupersedesMoment,
) -> Said {
    let names = missing
        .iter()
        .map(|(file, h, _)| {
            if file == rel {
                format!("\"{}\"", h)
            } else {
                format!("{}: \"{}\"", file, h)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let add = Act::Rewrites(format!(
        "the supersedes line of \"{}\" to add {}",
        heading, names
    ));
    said(
        &rule::SUPERSEDES_NAMES_DEPENDENTS,
        format!(
            "{}: \"{}\": supersedes a decision that current decisions stand on, and the line leaves out {}",
            rel, heading, names
        ),
        match moment {
            SupersedesMoment::Approve => Next::agent(add),
            SupersedesMoment::Asked => Next::agent_then_runs(add, command(&["adr", "ask", rel])),
        },
    )
}

/// A question whose depends line names a decision that its own supersedes
/// line replaces.
pub fn stands_on_what_it_supersedes(rel: &str, heading: &str, target: &str) -> Said {
    said(
        &rule::QUESTION_NOT_ON_WHAT_IT_SUPERSEDES,
        format!(
            "{}: \"{}\" stands on \"{}\", which it supersedes",
            rel, heading, target
        ),
        Next::agent(Act::Removes("the entry from its depends line".into())),
    )
}

/// A question of the plan stands on a decision that another question of the
/// same plan supersedes.
pub fn stands_on_what_the_plan_supersedes(
    rel: &str,
    heading: &str,
    target: &str,
    replacer: &str,
) -> Said {
    said(
        &rule::NO_QUESTION_ON_WHAT_PLAN_SUPERSEDES,
        format!(
            "{}: \"{}\" stands on \"{}\", which \"{}\" supersedes",
            rel, heading, target, replacer
        ),
        Next::agent(Act::Rewrites(format!(
            "the depends entry to name \"{}\"",
            replacer
        ))),
    )
}

/// A `supersedes:` entry written as a heading that more than one current
/// decision carries.
pub fn ask_supersedes_ambiguous(rel: &str, heading: &str, entry: &str, files: &[String]) -> Said {
    said(
        &rule::SUPERSEDES_ENTRY_NAMES_ONE_DECISION,
        format!(
            "{}: \"{}\": supersedes \"{}\", which is a current decision in more than one place: {}",
            rel,
            heading,
            entry,
            files.join(", ")
        ),
        ambiguous_heading_next(),
    )
}

pub fn banned_word_in_candidate(rel: &str, heading: &str, suffix: &str) -> Said {
    said(
        &rule::BANNED_WORD_ABSENT_FROM_RECORDS,
        format!("{}: \"{}\": banned word{}", rel, heading, suffix),
        Next::agent(Act::Rewrites("the line".into())),
    )
}

pub fn pairs_no_term(rel: &str, heading: &str) -> Said {
    said(
        &rule::QUESTION_PAIRS_TO_TERM,
        format!("{}: \"{}\" uses no term", rel, heading),
        Next::agent(Act::Writes("the term in adrdom/terms.md".into())),
    )
}

pub fn pairs_no_term_at_write(rel: &str, heading: &str) -> Said {
    pairs_no_term(rel, heading)
}

// --- adrdom/blame/*.md (blame files) --------------------------------------
// Shared by `blame::file_state` and the rules on a blame file (per "What
// does a blame file hold of its origin?", "Does a blame file cover its whole
// origin?"), read by `approve`, `check` and `show` alike.

/// A fault of the structure of a blame file: a header line missing, the
/// blank line after the header, a line that is no row.
pub fn blame_shape_error(rel: &str, e: &str) -> Said {
    said(
        &crate::rule::BLAME_FILE_SHAPE,
        format!("{}: {}", rel, e),
        Next::agent(Act::Rewrites("the blame file".into())),
    )
}

/// A row whose value is neither `decision` nor `detail`.
pub fn blame_row_value_error(rel: &str, e: &str) -> Said {
    said(
        &crate::rule::BLAME_ROW_SAYS_DECISION_OR_DETAIL,
        format!("{}: {}", rel, e),
        Next::agent(Act::Writes("decision or detail after the tab".into())),
    )
}

/// An id line that holds no identifier, or sits below the header: the tool
/// writes the identifier of a blame file, at approval, as line 3.
pub fn blame_id_error(rel: &str, e: &str, approved: Option<&str>) -> Said {
    // `e` opens with the line it found, as "line N: ...". A file with an
    // approval gets its approved id line back instead.
    let next = match approved {
        Some(id) => Next::agent(Act::Writes(format!("back the id line \"id: {}\"", id))),
        None => match e
            .strip_prefix("line ")
            .and_then(|r| r.split(':').next())
            .filter(|n| n.parse::<usize>().is_ok())
        {
            Some(n) => Next::agent(Act::Removes(format!("the id line, line {}", n))),
            None => Next::agent(Act::Removes("the id line".into())),
        },
    };
    said(
        &crate::rule::BLAME_ID_ASSIGNED_BY_TOOL,
        format!("{}: {}", rel, e),
        next,
    )
}

/// `rel` was approved and has no id line now: the identifier its approval
/// gave is gone, and approve does not assign another.
pub fn blame_id_line_gone(rel: &str, id: &str) -> Said {
    said(
        &crate::rule::BLAME_ID_ASSIGNED_BY_TOOL,
        format!("{}: the id line of its approval is gone", rel),
        Next::agent(Act::Writes(format!("back the id line \"id: {}\"", id))),
    )
}

/// The id line of `rel` holds an identifier no `blame` ledger row holds: the
/// tool did not write it.
pub fn blame_id_not_written(rel: &str) -> Said {
    said(
        &crate::rule::BLAME_ID_ASSIGNED_BY_TOOL,
        format!(
            "{}: its id line holds an identifier the tool did not write",
            rel
        ),
        Next::agent(Act::Removes("the id line".into())),
    )
}

/// `rel` was approved and its id line differs from the identifier its
/// approval gave (`written`).
pub fn blame_id_edited(rel: &str, written: &str) -> Said {
    said(
        &crate::rule::BLAME_ID_ASSIGNED_BY_TOOL,
        format!(
            "{}: its id line differs from the identifier of its approval",
            rel
        ),
        Next::agent(Act::Writes(format!("back the id line \"id: {}\"", written))),
    )
}

/// The id line of `rel` holds the identifier of another blame file
/// (`other_rel`, the one that keeps it): a copy of that file.
pub fn blame_id_shared(rel: &str, other_rel: &str) -> Said {
    said(
        &crate::rule::BLAME_ID_NAMES_ONE_FILE,
        format!("{}: its id line holds the identifier of {}", rel, other_rel),
        Next::agent(Act::Removes(format!("the id line from {}", rel))),
    )
}

fn range_text(start: usize, end: usize) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{}-{}", start, end)
    }
}

pub fn blame_coverage_gap(rel: &str, start: usize, end: usize) -> Said {
    said(
        &crate::rule::BLAME_COVERS_ORIGIN,
        format!(
            "{}: line(s) {} of the origin are covered by no row",
            rel,
            range_text(start, end)
        ),
        Next::agent(Act::Writes(format!(
            "a row for {} {}",
            if start == end { "line" } else { "lines" },
            range_text(start, end)
        ))),
    )
}

pub fn blame_range_beyond(rel: &str, start: usize, end: usize, n: usize) -> Said {
    said(
        &crate::rule::BLAME_ROW_RANGE_WITHIN_ORIGIN,
        format!(
            "{}: row {} is beyond the origin's {} line(s)",
            rel,
            range_text(start, end),
            n
        ),
        Next::agent(Act::Rewrites("the row".into())),
    )
}

/// The origin line names a path under `repo::ADRDOM_DIR`, or one that exists
/// but is not a regular file (per the "origin" term: "a prose file outside
/// adrdom whose text the records take in"). `blame::valid_origin_path` is
/// the one place `approve`, `check` and `show` read this rule from.
pub fn blame_bad_origin_path(rel: &str, path: &str) -> Said {
    said(
        &crate::rule::BLAME_ORIGIN_IS_A_FILE,
        format!("{}: \"{}\" is not a regular file outside adrdom", rel, path),
        Next::agent(Act::Rewrites("the origin line".into())),
    )
}

// --- commands/check/blame.rs -----------------------------------------------
// Shared by `check`'s `blame` category (per "What does check do when an origin
// differs from its blame file's blob identifier?", 6f6e7f4a A; "What does
// check do when an origin is absent?", 0c3d498a A).

/// `git hash-object`'s own blob sha for `origin`'s current content no
/// longer matches the blame file's `blob:` line (6f6e7f4a A). A missing
/// origin is never reported here -- that is `blame_origin_missing`.
pub fn blame_blob_mismatch(rel: &str, origin: &str) -> Said {
    said(
        &crate::rule::BLAME_BLOB_MATCHES_ORIGIN,
        format!("{}: blob no longer matches {}", rel, origin),
        Next::agent(Act::Rewrites(format!(
            "the rows of {} to the current origin",
            rel
        )))
        .and_owner_runs(command(&["adr", "approve", rel])),
    )
}

/// `origin` is missing while the blame file `rel` awaits approval (`status`,
/// new or changed): an origin stays until its blame file is approved
/// (0c3d498a A). Once the file is approved the origin may go, and nothing
/// is reported.
pub fn blame_origin_missing(rel: &str, origin: &str, status: FileStatus) -> Said {
    said(
        &crate::rule::BLAME_ORIGIN_STAYS_UNTIL_APPROVED,
        format!(
            "{}: origin {} is missing while the blame file is {}",
            rel,
            origin,
            status.name()
        ),
        Next::agent(Act::Restores(origin.to_string())),
    )
}

/// A blame file itself (`rel`) could not be read as text at all (per "Does
/// check compare a blame file's blob identifier with its origin?", 6f6e7f4a
/// A: fail-open here means never silently skipping the file). Distinct from
/// `blame_shape_error` -- there is no content to have a shape at all.
pub fn blame_file_unreadable(rel: &str) -> String {
    refuse(
        format!("{}: cannot be read as text (e.g. its encoding)", rel),
        Next::agent(Act::Rewrites(rel.to_string())),
    )
}

/// `git hash-object --stdin` itself failed on `origin`'s current content
/// (6f6e7f4a A: the blob comparison must fail closed, never silently pass
/// when git could not be asked at all).
pub fn blame_hash_failed(rel: &str, origin: &str) -> String {
    refuse(
        format!(
            "{}: git could not hash the current content of {}",
            rel, origin
        ),
        Next::agent_then_runs(Act::Rewrites(origin.to_string()), Cmd::fixed("adr check")),
    )
}

/// `origin` sits at its path (per `Source::exists`) but its content could
/// not be read as text (0c3d498a A: distinct from the origin being
/// missing outright -- a present, unreadable origin is its own failure,
/// never routed through the missing-origin branch).
pub fn blame_origin_unreadable(rel: &str, origin: &str) -> String {
    refuse(
        format!(
            "{}: origin {} exists but cannot be read as text (e.g. its encoding)",
            rel, origin
        ),
        Next::agent(Act::Rewrites(origin.to_string())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    //
    // The five findings of a blame line, word for word: each ends in a move,
    // and the one that waits for a blame file's approval names the status of
    // the file by its name in status.rs and ends in the tail moves.rs holds.
    #[test]
    fn the_blame_line_findings_read_as_the_decision_words_them() {
        let (r, h) = ("x.md", "Does X?");
        assert_eq!(
            blame_entry_without_range(r, h, "notes").text(),
            "x.md: \"Does X?\": the blame entry \"notes\" names no row range; agent writes the name of a blame file and the ranges of its rows"
        );
        assert_eq!(
            blame_names_no_file(r, h, "notes").text(),
            "x.md: \"Does X?\": the blame entry \"notes\" names no blame file; agent writes the path of a blame file under adrdom/blame without .md"
        );
        for status in [FileStatus::New, FileStatus::Changed] {
            assert_eq!(
                blame_file_awaits_approval(r, h, "adrdom/blame/notes.md", status).text(),
                format!(
                    "x.md: \"Does X?\": the blame line names adrdom/blame/notes.md, which is {}; {}",
                    status.name(),
                    crate::moves::approval_wait().render()
                )
            );
        }
        assert_eq!(
            blame_range_of_no_row(r, h, "adrdom/blame/notes.md", 3, 3).text(),
            "x.md: \"Does X?\": the blame line names 3-3, the range of no row of adrdom/blame/notes.md; agent writes the range of a row of that blame file"
        );
    }
}
