use super::Failure;
use crate::ledger::Ledger;
use crate::record::RecordFile;
use crate::repo;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Every id that appears after `adr: ` in a tracked, non-`adrdom/` file, as
/// a full uuid token, mapped to every `path:line` location it was found at
/// (4.10-adjacent bookkeeping used by both `check`'s `tested` check and the
/// page's decision view for a tested count per id).
pub fn test_index(root: &Path) -> HashMap<String, Vec<String>> {
    let mut test_ids: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(list) = repo::git(root, &["ls-files"]) {
        for path in list.lines() {
            if path.starts_with("adrdom/") {
                continue;
            }
            let full = root.join(path);
            let Ok(content) = std::fs::read_to_string(&full) else {
                continue;
            };
            for (i, line) in content.lines().enumerate() {
                if let Some(pos) = line.find("adr: ") {
                    let rest = &line[pos + 5..];
                    let id_tok: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
                    if is_uuid(&id_tok) {
                        test_ids
                            .entry(id_tok)
                            .or_default()
                            .push(format!("{}:{}", path, i + 1));
                    }
                }
            }
        }
    }
    test_ids
}

pub(super) fn check_tests(
    root: &Path,
    records: &[RecordFile],
    led: &Ledger,
    tests: Option<&HashMap<String, Vec<String>>>,
) -> (Vec<Failure>, usize) {
    let mut out = Vec::new();

    let current: HashSet<String> = records
        .iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.clone()))
        .filter(|id| matches!(led.latest_state(id), Some(row) if row.event == "answered"))
        .collect();

    let owned;
    let test_ids: &HashMap<String, Vec<String>> = match tests {
        Some(t) => t,
        None => {
            owned = test_index(root);
            &owned
        }
    };

    let mut tested_count = 0;

    for id in &current {
        if test_ids.contains_key(id) {
            tested_count += 1;
        } else {
            out.push(Failure {
                check: "tested".to_string(),
                id: Some(id.clone()),
                message: format!(
                    "{}: untested; agent writes a test that shows the code follows this decision and puts \"adr: {}\" in it",
                    id, id
                ),
            });
        }
    }

    for id in test_ids.keys() {
        if !current.contains(id) {
            out.push(Failure {
                check: "tested".to_string(),
                id: Some(id.clone()),
                message: format!("{}: stale (not a current decision)", id),
            });
        }
    }

    (out, tested_count)
}

/// A test only counts when the token after `adr: ` is a full uuid
/// (8-4-4-4-12 lowercase hex); any other text after `adr:` is prose.
fn is_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && [8, 4, 4, 4, 12]
            .iter()
            .zip(&parts)
            .all(|(n, p)| p.len() == *n && p.chars().all(|c| c.is_ascii_hexdigit()))
}
