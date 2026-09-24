use super::{dropped_state, Failure};
use crate::ledger::Ledger;
use crate::record::RecordFile;
use std::collections::{HashMap, HashSet};

pub(super) fn check_ids(records: &[RecordFile], led: &Ledger) -> Vec<Failure> {
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
                    message: format!(
                        "{}: {}: plan not approved; owner runs: adr approve {}",
                        rf.rel, sec.heading_text, rf.rel
                    ),
                }),
                Some(id) => {
                    if let Some(prev) = seen.get(id) {
                        out.push(Failure {
                            check: "ids".to_string(),
                            id: Some(id.clone()),
                            message: format!(
                                "duplicate id {} in {} and {}:{}",
                                id,
                                prev,
                                rf.rel,
                                sec.heading_text
                            ),
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
    for id in ledger_ids {
        if dropped_state(led, &id) {
            continue;
        }
        if !seen.contains_key(&id) {
            out.push(Failure {
                check: "ids".to_string(),
                id: Some(id.clone()),
                message: format!("ledger id {} not found in any topic file", id),
            });
        }
    }

    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if let Some(row) = led.latest_state(id) {
                if row.event == "withdrawn" {
                    out.push(Failure {
                        check: "ids".to_string(),
                        id: Some(id.clone()),
                        message: format!(
                            "withdrawn id {} still present in {}",
                            id,
                            rf.rel
                        ),
                    });
                }
            }
        }
    }

    out
}
