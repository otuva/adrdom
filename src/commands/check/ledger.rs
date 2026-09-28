use super::{dropped_state, Failure};
use crate::ledger::Ledger;
use crate::msg;
use crate::record::RecordFile;
use crate::repo;
use std::collections::HashSet;
use std::path::Path;

pub(super) fn check_ledger(
    led: &Ledger,
    records: &[RecordFile],
    root: &Path,
    ledger_content: &str,
) -> Vec<Failure> {
    let mut out = Vec::new();
    for (lineno, raw) in &led.malformed {
        out.push(Failure {
            check: "ledger".to_string(),
            message: msg::malformed_ledger_row(*lineno, raw),
        });
    }

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
    if let Some(head_content) = repo::git(root, &["show", &format!("HEAD:{}", repo::LEDGER_PATH)]) {
        if lines_missing(&head_content, &present) {
            out.push(Failure {
                check: "ledger".to_string(),
                message: msg::ledger_not_append_only(),
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
        if ref_ledger_has_rows_missing(root, "MERGE_HEAD", &present) {
            out.push(Failure {
                check: "ledger".to_string(),
                message: msg::ledger_merge_drops_rows(),
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
                if ref_ledger_has_rows_missing(root, sha, &present) {
                    drops = true;
                }
            }
            if unreadable {
                out.push(Failure {
                    check: "ledger".to_string(),
                    message: msg::ledger_merge_parents_unreadable(),
                });
            } else if drops {
                out.push(Failure {
                    check: "ledger".to_string(),
                    message: msg::ledger_merge_drops_rows(),
                });
            }
        }
    }

    let all_ids: HashSet<String> = records
        .iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.clone()))
        .collect();

    for row in &led.rows {
        if row.event == "asked" && !all_ids.contains(&row.id) && !dropped_state(led, &row.id) {
            out.push(Failure {
                check: "ledger".to_string(),
                message: msg::ledger_asked_id_missing(&row.id),
            });
        }
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
                    message: msg::ledger_answered_without_asked(&row.id),
                });
            }
        }
    }

    out
}

/// Whether `refname`'s copy of the ledger (`git show <refname>:adrdom/state/ledger.tsv`)
/// holds any non-empty row line missing from `present` -- the ledger being
/// judged, already split into a line set by the caller. `refname` names
/// `MERGE_HEAD` or one of `HEAD`'s parents by sha (the caller has already
/// confirmed the parent's commit object is present when it calls this);
/// when the ref itself can't be read at all (`MERGE_HEAD` unset) or simply
/// holds no ledger file at that path, there's nothing to compare and this
/// counts as nothing missing, never a failure.
fn ref_ledger_has_rows_missing(root: &Path, refname: &str, present: &HashSet<&str>) -> bool {
    let Some(other_content) = repo::git(root, &["show", &format!("{}:{}", refname, repo::LEDGER_PATH)]) else {
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
/// looks at where a line sits, only whether it is there at all.
fn lines_missing(other_content: &str, present: &HashSet<&str>) -> bool {
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
/// was never fetched.
fn parent_object_present(root: &Path, sha: &str) -> bool {
    repo::git(root, &["cat-file", "-e", &format!("{}^{{commit}}", sha)]).is_some()
}
