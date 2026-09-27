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

    // Compared against `ledger_content` -- the same staged-or-working-tree
    // bytes `led` was itself parsed from in `build` (per "Does check at a
    // commit read the staged files?") -- rather than re-reading the
    // filesystem here, so this never disagrees with `led` about which copy
    // of the ledger it is judging.
    if let Some(head_content) = repo::git(root, &["show", &format!("HEAD:{}", repo::LEDGER_PATH)]) {
        if !ledger_content.starts_with(&head_content) {
            out.push(Failure {
                check: "ledger".to_string(),
                message: msg::ledger_not_append_only(),
            });
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
