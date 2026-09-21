use super::{dropped_state, Failure};
use crate::ledger::Ledger;
use crate::record::RecordFile;
use crate::repo;
use std::collections::HashSet;
use std::path::Path;

pub(super) fn check_ledger(
    led: &Ledger,
    records: &[RecordFile],
    root: &Path,
    ledger_path: &Path,
) -> Vec<Failure> {
    let mut out = Vec::new();
    for (lineno, raw) in &led.malformed {
        out.push(Failure {
            check: "ledger".to_string(),
            id: None,
            message: format!("line {}: malformed row: {}", lineno, raw),
        });
    }

    if let Some(head_content) = repo::git(root, &["show", "HEAD:adrdom/state/ledger.tsv"]) {
        let working = std::fs::read_to_string(ledger_path).unwrap_or_default();
        if !working.starts_with(&head_content) {
            out.push(Failure {
                check: "ledger".to_string(),
                id: None,
                message: "not append-only: working copy diverges from HEAD".to_string(),
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
                id: Some(row.id.clone()),
                message: format!("asked row id {} not found in any topic file", row.id),
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
                    id: Some(row.id.clone()),
                    message: format!(
                        "answered row for {} has no matching earlier asked hash",
                        row.id
                    ),
                });
            }
        }
    }

    out
}
