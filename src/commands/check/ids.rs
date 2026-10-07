use crate::rules::Rule;
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
    // Every identifier a topic file holds.
    let held: HashSet<&str> =
        records.iter().flat_map(|rf| rf.sections.iter().filter(|s| !s.is_skeleton).filter_map(|s| s.id.as_deref())).collect();
    // In ledger order, each once: two runs print the same lines in the
    // same order.
    let mut ledger_ids: Vec<String> = Vec::new();
    for row in &led.rows {
        if (row.event == "asked" || row.event == "answered") && row.id != "-" && !ledger_ids.contains(&row.id) {
            ledger_ids.push(row.id.clone());
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

    // A question asked in a file awaiting approval and struck from its
    // plan: the one judgement show and plan print too (`rules::struck_asked`).
    let mut struck: HashSet<String> = HashSet::new();
    let sole = stale.len() == 1;
    for (_, rel) in &stale {
        struck.extend(crate::rules::struck_asked_ids(rel, sole, records, led).into_iter().map(|(id, _, _)| id));
        for f in crate::rules::struck_asked(rel, true, sole, records, led) {
            out.push(Failure::new(f.rule, f.message));
        }
    }

    for id in ledger_ids {
        if held.contains(id.as_str()) {
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

        // asked / refused, missing from every current topic file. The file
        // the id was approved in is gone, its removal awaiting approval:
        // name that file's removal move. Struck from the plan of a file
        // still there and awaiting approval: reported above, once. Else the
        // id is not found anywhere.
        if let Some((rel, holds)) = led
            .latest_for_id("approved", &id)
            .map(|r| r.detail.as_str())
            .and_then(|rel| removed.iter().find(|(d, _)| d == rel))
        {
            let heading = led
                .latest_asked(&id)
                .map(|r| r.detail.clone())
                .unwrap_or_else(|| id.clone());
            out.push(Failure::new(Rule::Ids, msg::removed_with_file(&id, &heading, event.unwrap_or("unknown"), crate::moves::removal_next(rel, holds))));
            continue;
        }
        if struck.contains(&id) {
            continue;
        }
        out.push(Failure::new(Rule::Ids, msg::ledger_id_not_found(&id, led.latest_asked(&id).map(|r| r.detail.as_str()))));
    }

    for r in missing_answered {
        out.push(Failure::new(Rule::Ids, r.message.clone()));
    }

    out
}
