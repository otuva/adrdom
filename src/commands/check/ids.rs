use super::Failure;
use crate::actions::Refusal;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::RecordFile;
use crate::state;
use std::collections::{HashMap, HashSet};

pub(super) fn check_ids(records: &[RecordFile], led: &Ledger, missing_answered: &[Refusal]) -> Vec<Failure> {
    let mut out = Vec::new();
    let mut seen: HashMap<String, String> = HashMap::new();
    for rf in records {
        for sec in &rf.sections {
            // Skeleton headings never fail anything, including ids.
            if sec.is_skeleton {
                continue;
            }
            match &sec.id {
                // 9.1 A point 9: a section with no id means the plan was
                // never approved (or an edit since moved the plan hash) --
                // `approve`, not `ask`, is what gives it one.
                None => out.push(Failure {
                    check: "ids".to_string(),
                    id: None,
                    message: msg::heading_plan_not_approved(&rf.rel, &sec.heading_text),
                }),
                Some(id) => {
                    if let Some(prev) = seen.get(id) {
                        out.push(Failure {
                            check: "ids".to_string(),
                            id: Some(id.clone()),
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
    let stale: Vec<(usize, &str)> = records
        .iter()
        .filter(|rf| !state::plan_approved(led, rf, &rf.rel))
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
        // An answered-and-current id missing from every file is reported
        // below, from `missing_answered` (E1.2, shared with `approve`'s
        // `missing_answered_refusals`), so it isn't printed twice here.
        if event == Some("answered") {
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
                check: "ids".to_string(),
                id: Some(id.clone()),
                message: msg::removed_from_plan(&id, &heading, event.unwrap_or("unknown"), removed_file),
            });
            continue;
        }
        out.push(Failure {
            check: "ids".to_string(),
            id: Some(id.clone()),
            message: msg::ledger_id_not_found(&id),
        });
    }

    for r in missing_answered {
        out.push(Failure {
            check: "ids".to_string(),
            id: None,
            message: r.message.clone(),
        });
    }

    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if let Some(row) = led.latest_state(id) {
                if row.event == "withdrawn" {
                    out.push(Failure {
                        check: "ids".to_string(),
                        id: Some(id.clone()),
                        message: msg::withdrawn_id_present(id, &rf.rel),
                    });
                }
            }
        }
    }

    out
}
