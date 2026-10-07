use super::Failure;
use crate::rules::Finding;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::RecordFile;
use crate::state;
use std::collections::{HashMap, HashSet};

pub(super) fn check_ids(
    records: &[RecordFile],
    led: &Ledger,
    missing_answered: &[Finding],
    raw_hashes: &HashMap<String, (Vec<String>, String)>,
    removed: &[(String, Vec<crate::moves::RemovalHold>)],
) -> Vec<Failure> {
    let mut out = Vec::new();
    let mut seen: HashMap<String, String> = HashMap::new();
    for rf in records {
        for sec in &rf.sections {
            // Skeleton headings never fail anything, including ids.
            if sec.is_skeleton {
                continue;
            }
            match &sec.id {
                // A heading with no id is a question of a file awaiting
                // approval (per "Does check fail a topic file awaiting
                // approval?"): approve gives it one, and the file is counted
                // as awaiting, never failed.
                None => {}
                Some(id) => {
                    if let Some(prev) = seen.get(id) {
                        out.push(Failure {
                            check: "ids",
                            message: msg::duplicate_id(id, prev, &rf.rel, &sec.heading_text),
                        });
                    } else {
                        seen.insert(
                            id.clone(),
                            format!("{}:{}", rf.rel, sec.heading_text),
                        );
                    }
                }
            }
        }
    }

    let mut ledger_ids: HashSet<String> = HashSet::new();
    for row in &led.rows {
        if (row.event == "asked" || row.event == "answered") && row.id != "-" {
            ledger_ids.insert(row.id.clone());
        }
    }

    // Every currently-unapproved topic file that was once approved: a plan
    // change awaiting re-approval. This doesn't depend on which id is being
    // looked at, so it's computed once for the whole loop below rather than
    // per id.
    // Read off `raw_hashes` (computed before the normalize pass mutates
    // each `RecordFile`'s parsed `depends:` entries in memory) rather than
    // `state::plan_approved(led, rf, &rf.rel)`, which would otherwise
    // recompute the plan hash from the by-then-normalized `rf`: approval
    // state must always be read off the file exactly as it sits on disk
    // (per "...").
    let stale: Vec<(usize, &str)> = records
        .iter()
        .filter(|rf| {
            let plan_h: &[String] = raw_hashes.get(&rf.rel).map(|(p, _)| p.as_slice()).unwrap_or_default();
            !state::plan_approved_hashes(led, &rf.rel, plan_h)
        })
        .filter_map(|rf| {
            led.rows
                .iter()
                .rposition(|r| r.event == "approved" && r.detail == rf.rel)
                .map(|pos| (pos, rf.rel.as_str()))
        })
        .collect();

    for id in ledger_ids {
        if seen.contains_key(&id) {
            continue;
        }
        let event = led.latest_state(&id).map(|r| r.event.as_str());
        // Withdrawn is `approve`'s own formal record that it already saw
        // this heading gone and accounted for it: fully processed, nothing
        // left to flag.
        if event == Some("withdrawn") {
            continue;
        }
        // A dropped id left its file with an answer (per "Does answer remove
        // a question whose answer set lacks the answer?"), asked or not;
        // `check_dropped` verifies the row, nothing is owed here.
        if event == Some("dropped") {
            continue;
        }
        // An answered-and-current id missing from every file is reported
        // below, from `missing_answered` (shared with `approve`'s
        // `missing_answered_refusals`), so it isn't printed twice here.
        // Unless its file is gone with the removal awaiting approval: the
        // removed-file branch below names that file's move for it.
        let in_removed = led
            .latest_for_id("approved", &id)
            .is_some_and(|r| removed.iter().any(|(d, _)| *d == r.detail));
        if event == Some("answered") && !in_removed {
            continue;
        }
        // A superseded id outside a stale edit is its ordinary, permanent
        // end state (`approve`'s "allow a superseded one silently") and
        // earns nothing at all -- checked before `stale` is consulted, so
        // an unrelated unapproved edit elsewhere never turns this into a
        // "removed from the plan" failure.
        if event == Some("superseded") {
            continue;
        }

        // asked / refused, missing from every current topic file: when a
        // file that was once approved now has a stale, still unapproved
        // plan, the heading was very likely just edited out of it, so say
        // what happened and point at the fix instead of the bare "not
        // found" line. Name the file only when exactly one such file
        // exists; with more than one, which one dropped this id can't be
        // told apart, so nothing is named. A file that was never approved
        // at all names nothing here -- there is no earlier approved plan
        // for this id to have dropped out of.
        // The file the id was approved in is gone, its removal awaiting
        // approval: name that file, the same line a struck heading earns.
        if let Some((rel, holds)) = led
            .latest_for_id("approved", &id)
            .map(|r| r.detail.as_str())
            .and_then(|rel| removed.iter().find(|(d, _)| d == rel))
        {
            let heading = led
                .latest_asked(&id)
                .map(|r| r.detail.clone())
                .unwrap_or_else(|| id.clone());
            out.push(Failure {
                check: "ids",
                message: msg::removed_with_file(&id, &heading, event.unwrap_or("unknown"), crate::moves::removal_next(rel, holds)),
            });
            continue;
        }
        if !stale.is_empty() {
            let heading = led
                .latest_asked(&id)
                .map(|r| r.detail.clone())
                .unwrap_or_else(|| id.clone());
            let removed_file = match stale.as_slice() {
                [(_, rel)] => Some(*rel),
                _ => None,
            };
            out.push(Failure {
                check: "ids",
                message: msg::removed_from_plan(&id, &heading, event.unwrap_or("unknown"), removed_file),
            });
            continue;
        }
        out.push(Failure {
            check: "ids",
            message: msg::ledger_id_not_found(&id, led.latest_asked(&id).map(|r| r.detail.as_str())),
        });
    }

    for r in missing_answered {
        out.push(Failure {
            check: "ids",
            message: r.message.clone(),
        });
    }

    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if let Some(row) = led.latest_state(id) {
                if row.event == "withdrawn" {
                    out.push(Failure {
                        check: "ids",
                        message: msg::withdrawn_id_present(id, &sec.heading_text, &rf.rel),
                    });
                }
            }
        }
    }

    out
}
