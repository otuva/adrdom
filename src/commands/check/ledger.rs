use super::{dropped_state, Failure};
use crate::ledger::{Ledger, Row};
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The state-row events `check_same_time_competition`'s rule (1) matches
/// (ce19c883-c61d-477b-a6d4-fee5134c6e9a T): the same set `Ledger::latest_state`
/// already reads a question's state from.
const COMPETING_STATE_EVENTS: [&str; 6] = ["asked", "answered", "refused", "superseded", "withdrawn", "dropped"];

/// The events rule (3) matches: every event whose `detail` column carries a
/// file path (src/actions.rs's `approved`, `context`, `terms` and `blame`
/// writers), per the dry run this decision's build asked for.
const COMPETING_PATH_EVENTS: [&str; 4] = ["approved", "context", "terms", "blame"];

pub(super) fn check_ledger(
    led: &Ledger,
    records: &[RecordFile],
    root: &Path,
    ledger_content: &str,
    since: Option<&str>,
    removed_files: &[(String, Vec<crate::moves::RemovalHold>)],
) -> Vec<Failure> {
    let mut out = Vec::new();
    for (lineno, raw) in &led.malformed {
        out.push(Failure {
            check: "ledger".to_string(),
            message: msg::malformed_ledger_row(*lineno, raw),
        });
    }

    out.extend(check_held(root, repo::LEDGER_PATH, ledger_content, since, "ledger", &LEDGER_MSGS));

    out.extend(check_same_time_competition(led));
    out.extend(check_dropped(led, records, root));

    let all_ids: HashSet<String> = records
        .iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.clone()))
        .collect();

    for row in &led.rows {
        if row.event != "asked" || all_ids.contains(&row.id) || dropped_state(led, &row.id) {
            continue;
        }
        // An id whose approved file is gone, its removal awaiting approval:
        // every state it can be in (asked, refused, answered) is reported
        // once by the ids check with the file's removal move, so the ledger
        // check leaves it out (one finding per fault).
        let in_removed = led
            .latest_for_id("approved", &row.id)
            .is_some_and(|a| removed_files.iter().any(|(rel, _)| *rel == a.detail));
        if in_removed {
            continue;
        }
        out.push(Failure {
            check: "ledger".to_string(),
            message: msg::ledger_asked_id_missing(&row.id, Some(row.detail.as_str())),
        });
    }

    for row in &led.rows {
        if row.event == "answered" {
            let has_match = led
                .rows
                .iter()
                .any(|r| r.event == "asked" && r.id == row.id && r.hash == row.hash);
            if !has_match {
                out.push(Failure {
                    check: "ledger".to_string(),
                    message: msg::ledger_answered_without_asked(
                        &row.id,
                        led.latest_asked(&row.id).map(|r| r.detail.as_str()),
                    ),
                });
            }
        }
    }

    out
}

/// Per "What does answer write for a removed question?": each `dropped`
/// row is verified from the record alone. Its detail is `<id> <letter>` (a
/// direct drop) or `<id>` (a dependent). The cause exists (an `answered`
/// row for that id with that letter, or a `dropped` row for that id), an
/// `approved` row of the question holds the row's hash, and no topic file
/// holds the id. At the commit that removes the section, HEAD's copy of
/// the file still holds it, and then HEAD's text must hash to the row's
/// hash and its depends entry must show the set lacked the letter (direct)
/// or the entry on the dropped question (dependent).
fn check_dropped(led: &Ledger, records: &[RecordFile], root: &Path) -> Vec<Failure> {
    let mut out = Vec::new();
    let fail = |message: String| Failure { check: "ledger".to_string(), message };
    for (at, row) in led.rows.iter().enumerate().filter(|(_, r)| r.event == "dropped") {
        // Per "Does a decision change after it is answered?": a question
        // that was answered or refused is never removed. `led.rows` is
        // ordered by time when read (per "Does the tool take the order of
        // ledger rows from their time?"), so `[..at]` is "earlier in time".
        if let Some(prior) = led.rows[..at]
            .iter()
            .find(|r| r.id == row.id && matches!(r.event.as_str(), "answered" | "refused"))
        {
            out.push(fail(msg::ledger_dropped_closed(&row.id, &prior.event)));
        }
        let parts: Vec<&str> = row.detail.split(' ').collect();
        let cause: (&str, Option<char>) = match parts.as_slice() {
            [id] if record::is_uuid(id) => (id, None),
            [id, l] if record::is_uuid(id) && l.chars().count() == 1 => (id, l.chars().next()),
            _ => {
                out.push(fail(msg::ledger_dropped_malformed(&row.id, &row.detail)));
                continue;
            }
        };
        let (cause_id, letter) = cause;
        let cause_ok = match letter {
            Some(l) => led
                .rows
                .iter()
                .any(|r| r.event == "answered" && r.id == cause_id && r.detail == l.to_string()),
            None => led.rows.iter().any(|r| r.event == "dropped" && r.id == cause_id),
        };
        if !cause_ok {
            let what = match letter {
                Some(l) => format!("no answered row for {} with the letter {}", cause_id, l),
                None => format!("no dropped row for {}", cause_id),
            };
            out.push(fail(msg::ledger_dropped_without_cause(&row.id, &what)));
        }
        let approved_rows: Vec<&Row> = led
            .rows
            .iter()
            .filter(|r| r.event == "approved" && r.id == row.id && r.hash == row.hash)
            .collect();
        if approved_rows.is_empty() {
            out.push(fail(msg::ledger_dropped_not_approved(&row.id)));
        }
        for rf in records {
            if rf.sections.iter().any(|s| s.id.as_deref() == Some(row.id.as_str())) {
                out.push(fail(msg::ledger_dropped_still_in_file(&row.id, &rf.rel)));
            }
        }
        // The commit that removes the section: HEAD's copy still holds it.
        for a in &approved_rows {
            let Some(content) = repo::git(root, &["show", &format!("HEAD:{}", a.detail)]) else {
                continue;
            };
            let head = record::build(root.join(&a.detail), a.detail.clone(), record::split_lines(&content));
            let Some(sec) = head.sections.iter().find(|s| s.id.as_deref() == Some(row.id.as_str())) else {
                continue;
            };
            if record::question_hash(&head, sec) != row.hash {
                out.push(fail(msg::ledger_dropped_text_differs(&row.id)));
            }
            let entries: Vec<&record::Dep> = sec
                .depends
                .iter()
                .filter(|d| matches!(d, record::Dep::Id { id, .. } if id == cause_id))
                .collect();
            match letter {
                Some(l) => {
                    let lacks = entries
                        .iter()
                        .any(|d| matches!(d.set(), Some(record::AnswerSet::Letters(ls)) if !ls.contains(&l)));
                    if !lacks {
                        out.push(fail(msg::ledger_dropped_set_holds_letter(&row.id, &l.to_string())));
                    }
                }
                None => {
                    if entries.is_empty() {
                        out.push(fail(msg::ledger_dropped_not_standing_on(&row.id, cause_id)));
                    }
                }
            }
        }
    }
    out
}

/// The refusals of one held file: the ledger's and the reader file's differ
/// in what they name, not in the rule.
pub(super) struct HeldMsgs {
    pub not_append_only: fn() -> String,
    pub merge_drops_rows: fn() -> String,
    pub since_drops_rows: fn(&str) -> String,
    /// Whether this file reports a merge parent or a `since` commit that
    /// cannot be read; the ledger reports it, and the reader file leaves it
    /// to that one report, since both hit the same commit.
    pub report_unreadable: bool,
}

const LEDGER_MSGS: HeldMsgs = HeldMsgs {
    not_append_only: msg::ledger_not_append_only,
    merge_drops_rows: msg::ledger_merge_drops_rows,
    since_drops_rows: msg::ledger_since_drops_rows,
    report_unreadable: true,
};

pub(super) const READER_MSGS: HeldMsgs = HeldMsgs {
    not_append_only: msg::reader_not_append_only,
    merge_drops_rows: msg::reader_merge_drops_rows,
    since_drops_rows: msg::reader_since_drops_rows,
    report_unreadable: false,
};

/// The rows of the file at `path` (judged as `content`) that HEAD, the
/// other side of a merge and `since` hold must all still be there, byte for
/// byte: the one rule the ledger and the reader file are held to.
pub(super) fn check_held(
    root: &Path,
    path: &str,
    content: &str,
    since: Option<&str>,
    check: &str,
    msgs: &HeldMsgs,
) -> Vec<Failure> {
    let ledger_content = content;
    let mut out = Vec::new();
    // Built once and shared by every presence test below (the HEAD test
    // right here, and the merge tests just after): all of them ask the same
    // question, "is this line somewhere in `ledger_content`?", against the
    // same `ledger_content`, so the line set only needs building once per
    // run rather than once per ref compared.
    let present: HashSet<&str> = ledger_content.lines().collect();

    // Compared against `ledger_content` -- the same staged-or-working-tree
    // bytes `led` was itself parsed from in `build` (per "Does check at a
    // commit read the staged files?") -- rather than re-reading the
    // filesystem here, so this never disagrees with `led` about which copy
    // of the ledger it is judging. Per "Does check fail a ledger whose rows
    // changed place?" (c10c3a49-e993-4c9f-a46b-4a1ba3646dcf A): a presence
    // test, not a byte-prefix test -- every row line HEAD's own ledger
    // holds must still be a line somewhere in `ledger_content`, wherever it
    // sits; only a row that is missing or changed fails this, never one
    // that only moved. Shares `ref_ledger_has_rows_missing` with the merge
    // test just below -- the same rule, just against a different ref.
    if let Some(head_content) = repo::git(root, &["show", &format!("HEAD:{}", path)]) {
        if lines_missing(&head_content, &present) {
            out.push(Failure {
                check: check.to_string(),
                message: (msgs.not_append_only)(),
            });
        }
    }

    // Per "Does check at a merge look for the ledger rows of both
    // branches?": at a merge, every row either side wrote must survive
    // into the ledger being judged. The HEAD side is already the presence
    // test just above; this covers the other side(s) -- `MERGE_HEAD`
    // while a merge is still pending, or, once the merge commit itself
    // exists (and nothing is pending any more), every parent named in
    // HEAD's own commit object.
    //
    // The committed side reads those parents from `git cat-file commit
    // HEAD` rather than `HEAD^1`/`HEAD^2`: in a shallow clone, `rev-parse`
    // simply fails to resolve a parent whose object was never fetched, the
    // same way it fails for a ref that plain doesn't exist, and treating
    // that as "no such ref, nothing to compare" would let a shallow clone
    // pass a merge check that never actually looked at the other side. The
    // commit object itself still names both parents' shas regardless of
    // which of them were fetched, so those shas -- checked one at a time
    // with `git cat-file -e <sha>^{commit}` for whether the object is
    // actually present -- tell the two cases apart: a parent that's fully
    // there but just holds no ledger file at that path is nothing to
    // compare (as before); a parent whose commit object is missing outright
    // is a blind spot the check refuses on, once, instead of quietly
    // passing.
    if repo::git(root, &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_some() {
        if ref_ledger_has_rows_missing(root, path, "MERGE_HEAD", &present) {
            out.push(Failure {
                check: check.to_string(),
                message: (msgs.merge_drops_rows)(),
            });
        }
    } else {
        let parents = head_parents(root);
        if parents.len() >= 2 {
            let mut unreadable = false;
            let mut drops = false;
            for sha in &parents {
                if !parent_object_present(root, sha) {
                    unreadable = true;
                    continue;
                }
                if ref_ledger_has_rows_missing(root, path, sha, &present) {
                    drops = true;
                }
            }
            if unreadable && !msgs.report_unreadable {
            } else if unreadable {
                out.push(Failure {
                    check: check.to_string(),
                    message: msg::ledger_merge_parents_unreadable(),
                });
            } else if drops {
                out.push(Failure {
                    check: check.to_string(),
                    message: (msgs.merge_drops_rows)(),
                });
            }
        }
    }

    // Per "Does check in CI compare the ledger with the commit before the
    // push?" (a26876ba-5789-4d05-965e-853d00d9edd2 C): `--since <commit>`
    // adds one more presence test, against `<commit>`'s own ledger, on top
    // of the plain check's own HEAD/merge ones above. `is_zero_sha` covers
    // a branch's first push (CI's `github.event.before` is all zeros then):
    // nothing to compare. `--push` runs this same presence test itself, per
    // ref line, through `ref_ledger_has_rows_missing`/`lines_missing`
    // directly (see `push.rs`) rather than through this `since` parameter,
    // since a push judges only that one comparison, never the rest of
    // `check_ledger`.
    if let Some(commit) = since {
        if !repo::is_zero_sha(commit) {
            if !parent_object_present(root, commit) {
                if msgs.report_unreadable {
                    out.push(Failure {
                        check: check.to_string(),
                        message: msg::ledger_since_unreadable(commit),
                    });
                }
            } else if ref_ledger_has_rows_missing(root, path, commit, &present) {
                out.push(Failure {
                    check: check.to_string(),
                    message: (msgs.since_drops_rows)(commit),
                });
            }
        }
    }

    out
}

/// Whether `refname`'s copy of the ledger (`git show <refname>:adrdom/state/ledger.tsv`)
/// holds any non-empty row line missing from `present` -- the ledger being
/// judged, already split into a line set by the caller. `refname` names
/// `MERGE_HEAD`, one of `HEAD`'s parents, or a plain commit sha (the
/// `since` comparison above, and `push.rs`'s own per-ref comparison) -- the
/// caller has already confirmed the sha's commit object is present when it
/// matters (every caller but the `MERGE_HEAD` one); when the ref itself
/// can't be read at all (`MERGE_HEAD` unset) or simply holds no ledger file
/// at that path, there's nothing to compare and this counts as nothing
/// missing, never a failure. `pub(super)`: the one comparison shared by the
/// HEAD test above, the merge tests, `since`, and `push.rs`'s own presence
/// test.
pub(super) fn ref_ledger_has_rows_missing(root: &Path, path: &str, refname: &str, present: &HashSet<&str>) -> bool {
    let Some(other_content) = repo::git(root, &["show", &format!("{}:{}", refname, path)]) else {
        return false;
    };
    lines_missing(&other_content, present)
}

/// Whether any non-empty line of `other_content` is missing, byte for
/// byte, from `present` -- the ledger being judged's own lines, split once
/// by the caller -- the one presence rule "Does check fail a ledger whose
/// rows changed place?" (c10c3a49-e993-4c9f-a46b-4a1ba3646dcf A) puts
/// behind both the HEAD test above and `ref_ledger_has_rows_missing`'s
/// merge-side test: a row's place in either file is free, so this never
/// looks at where a line sits, only whether it is there at all. `pub(super)`
/// so `push.rs` can run the same test directly against a sha it already
/// read itself, without going through `ref_ledger_has_rows_missing`'s own
/// `git show`.
pub(super) fn lines_missing(other_content: &str, present: &HashSet<&str>) -> bool {
    other_content.lines().any(|l| !l.is_empty() && !present.contains(l))
}

/// The `parent <sha>` lines of HEAD's own raw commit object, in order --
/// read through `git cat-file commit HEAD` rather than derived from
/// `HEAD^1`/`HEAD^2`, since a shallow clone hides a parent it never
/// fetched from `rev-parse` (which just fails to resolve it) while the
/// commit object itself still names it. Empty for a root commit, one entry
/// for an ordinary commit, two or more for a merge commit; a commit's
/// header lines end at the first blank line, so nothing past that (the
/// commit message) is mistaken for a `parent` line.
fn head_parents(root: &Path) -> Vec<String> {
    let Some(raw) = repo::git(root, &["cat-file", "commit", "HEAD"]) else {
        return Vec::new();
    };
    raw.lines()
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.strip_prefix("parent "))
        .map(|s| s.trim().to_string())
        .collect()
}

/// Whether `sha`'s commit object actually exists in this repository --
/// `false` in exactly the shallow-clone case `head_parents` is read to
/// catch: the sha is named in HEAD's commit object, but the object itself
/// was never fetched. Also the `since`/`push.rs` blind-spot test: a `since`
/// commit or a pre-push line's remote sha that names a commit this
/// repository never fetched at all. `pub(super)` for `push.rs`'s own use.
pub(super) fn parent_object_present(root: &Path, sha: &str) -> bool {
    repo::git(root, &["cat-file", "-e", &format!("{}^{{commit}}", sha)]).is_some()
}

/// Per "Does check fail two ledger rows on one subject with the same
/// time?" (ce19c883-c61d-477b-a6d4-fee5134c6e9a T): every pair of rows that
/// carry the same time and compete for one subject -- and are the latest
/// rows for that subject, so nothing later has already settled which one
/// stands. Three rules, each grouping the ledger by its own notion of
/// "subject" first:
///
/// 1. state rows (`COMPETING_STATE_EVENTS`) sharing an id, the id not `-`;
/// 2. `banned`/`unbanned` rows sharing a hash;
/// 3. rows of one `COMPETING_PATH_EVENTS` event sharing its `detail` (a
///    file path) -- an `approved` row and a `context` row for the same
///    file are two different subjects (different events), so they never
///    compete here.
///
/// Within a subject's own rows, only the ones at that subject's own latest
/// time are ever compared: a same-time tie further back in a subject's
/// history stops mattering the moment a later row exists for it, since
/// that later row is what "the command run again" (this failure's own
/// remedy) writes, and it already settles the subject going forward.
fn check_same_time_competition(led: &Ledger) -> Vec<Failure> {
    // One subject string per finding, sorted before it's ever turned into a
    // `Failure` (`by_id`/`by_hash`/`by_path` are `HashMap`s, so their own
    // iteration order carries no meaning) -- the same determinism every
    // other multi-row category in `check` already sorts its own findings
    // for.
    let mut subjects: Vec<(String, String)> = Vec::new();

    let mut by_id: HashMap<&str, Vec<&Row>> = HashMap::new();
    for r in &led.rows {
        if r.id != "-" && COMPETING_STATE_EVENTS.contains(&r.event.as_str()) {
            by_id.entry(r.id.as_str()).or_default().push(r);
        }
    }
    for (id, rows) in &by_id {
        if let Some(time) = competing_at_latest_time(rows) {
            subjects.push((time, format!("id {}", id)));
        }
    }

    let mut by_hash: HashMap<&str, Vec<&Row>> = HashMap::new();
    for r in &led.rows {
        if matches!(r.event.as_str(), "banned" | "unbanned") {
            by_hash.entry(r.hash.as_str()).or_default().push(r);
        }
    }
    for (hash, rows) in &by_hash {
        if let Some(time) = competing_at_latest_time(rows) {
            subjects.push((time, format!("hash {}", short_hash(hash))));
        }
    }

    let mut by_path: HashMap<(&str, &str), Vec<&Row>> = HashMap::new();
    for r in &led.rows {
        if r.id == "-" && COMPETING_PATH_EVENTS.contains(&r.event.as_str()) {
            by_path.entry((r.event.as_str(), r.detail.as_str())).or_default().push(r);
        }
    }
    for ((event, detail), rows) in &by_path {
        if let Some(time) = competing_at_latest_time(rows) {
            subjects.push((time, format!("{} file {}", event, detail)));
        }
    }

    subjects.sort();
    subjects
        .into_iter()
        .map(|(time, subject)| Failure {
            check: "ledger".to_string(),
            message: msg::ledger_same_time_competition(&time, &subject),
        })
        .collect()
}

/// `rows` are already every row sharing one subject (whichever of the
/// three groupings above). Its own latest time -- the ledger's rows are
/// RFC3339 strings in one fixed UTC form (`ledger::now_rfc3339`/
/// `ledger::write_time`), so plain string comparison already orders them
/// correctly, without reparsing. `Some(time)` when two or more of the rows
/// at that latest time are not byte-identical to each other (`row_line`
/// equal); `None` when only one row sits at the latest time, or every row
/// there is the same row repeated.
fn competing_at_latest_time(rows: &[&Row]) -> Option<String> {
    let latest = rows.iter().map(|r| r.time.as_str()).max()?;
    let at_latest: Vec<&&Row> = rows.iter().filter(|r| r.time == latest).collect();
    if at_latest.len() < 2 {
        return None;
    }
    let first = at_latest[0];
    if at_latest.iter().all(|r| rows_identical(first, r)) {
        None
    } else {
        Some(latest.to_string())
    }
}

/// Whether `a` and `b` are the same row, byte for byte -- `Row` no longer
/// carries the raw line it was parsed from (`ledger::order_by_time` drops
/// it), but every field `ledger::row_line` writes is compared here instead,
/// which is the same test for any row that parsed cleanly (`time` is
/// already equal, both rows coming from the same time-bucket).
fn rows_identical(a: &Row, b: &Row) -> bool {
    a.event == b.event && a.id == b.id && a.hash == b.hash && a.detail == b.detail && a.version == b.version
}

/// A hash's first 12 hex characters, the same prefix length
/// `commands/check/banned.rs`'s own `hash_prefix` reports a banned hash
/// with -- kept as its own copy here rather than importing that (private)
/// helper, so this file and that one stay free to change independently.
fn short_hash(h: &str) -> &str {
    &h[..h.len().min(12)]
}
