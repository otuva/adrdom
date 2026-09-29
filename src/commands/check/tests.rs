use super::Failure;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::RecordFile;
use crate::repo::{self, Source};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Every id that appears after `adr: ` in a tracked, non-`adrdom/` file, as
/// a full uuid token, mapped to every `path:line` location it was found at
/// (4.10-adjacent bookkeeping used by both `check`'s `tested` check and the
/// page's decision view for a tested count per id). Which files are
/// tracked always comes from `git ls-files` (the index), in both `source`s
/// alike -- a tracked-only scan, per "What ties a test to a decision?"
/// ("every tracked file outside adrdom"), so this never walks a
/// gitignored build directory; only each file's *content* is read through
/// `source`, so a staged edit to a tie is judged, not whatever is left on
/// disk (per "Does check at a commit read the staged files?").
pub fn test_index(root: &Path, source: &Source) -> HashMap<String, Vec<String>> {
    let mut test_ids: HashMap<String, Vec<String>> = HashMap::new();
    let adrdom_prefix = format!("{}/", repo::ADRDOM_DIR);
    if let Some(list) = repo::git_paths(root, &["ls-files"]) {
        for path in &list {
            if path.starts_with(&adrdom_prefix) {
                continue;
            }
            let Some(content) = source.read(root, path) else {
                continue;
            };
            for (i, line) in content.lines().enumerate() {
                if let Some(pos) = line.find("adr: ") {
                    let rest = &line[pos + 5..];
                    let id_tok: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
                    if crate::record::is_uuid(&id_tok) {
                        test_ids
                            .entry(id_tok)
                            .or_default()
                            .push(msg::location(path, i + 1));
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
    source: &Source,
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
            owned = test_index(root, source);
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
                message: msg::untested(id),
            });
        }
    }

    for id in test_ids.keys() {
        if !current.contains(id) {
            out.push(Failure {
                check: "tested".to_string(),
                message: msg::stale_test(id),
            });
        }
    }

    (out, tested_count)
}

