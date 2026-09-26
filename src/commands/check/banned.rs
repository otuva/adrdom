use super::Failure;
use crate::banned;
use crate::ledger::Ledger;
use crate::msg;
use crate::record;
use crate::repo;
use crate::state;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(super) fn check_banned(root: &Path, staged: bool, message: Option<&Path>) -> Vec<Failure> {
    let mut out = Vec::new();
    let banned_path = root.join("adrdom/state/banned.sha256");
    let entries = banned::read_entries(&banned_path);
    if entries.is_empty() {
        return out;
    }
    let hash_map: HashMap<String, Option<String>> = entries.into_iter().collect();
    let tf = state::load_terms(root);

    for (name, content) in collect_targets(root, staged, message) {
        for (i, line) in content.lines().enumerate() {
            let mut seen: HashSet<String> = HashSet::new();
            for tok in banned::tokenize(line) {
                let h = record::sha256_hex(&tok);
                let Some(uuid_opt) = hash_map.get(&h) else {
                    continue;
                };
                if !seen.insert(tok.clone()) {
                    continue;
                }
                let suffix = uuid_opt
                    .as_ref()
                    .and_then(|uuid| {
                        tf.as_ref().and_then(|f| {
                            f.terms
                                .iter()
                                .find(|t| t.id.as_deref() == Some(uuid.as_str()))
                                .map(|t| t.name.clone())
                        })
                    })
                    .map(|n| format!(" (term {})", n))
                    .unwrap_or_default();
                out.push(Failure {
                    check: "banned".to_string(),
                    id: None,
                    message: msg::banned_word_at(&format!("{}:{}", name, i + 1), &suffix),
                });
            }
        }
    }
    out
}

/// The ban list against the ledger's `banned`/`unbanned` history: every
/// hash the ledger currently considers banned (a `banned` row with no
/// later `unbanned` row) must be in the file, and every hash in the file
/// must trace back to some `banned` row (regardless of a later `unbanned`
/// row, which would otherwise have removed it from the file already).
pub(super) fn check_banned_ledger(led: &Ledger, root: &Path) -> Vec<Failure> {
    let mut out = Vec::new();
    let banned_path = root.join("adrdom/state/banned.sha256");
    let file_hashes = banned::read_set(&banned_path);

    let mut ever_banned: HashSet<String> = HashSet::new();
    let mut currently_banned: HashSet<String> = HashSet::new();
    for row in &led.rows {
        match row.event.as_str() {
            "banned" => {
                ever_banned.insert(row.hash.clone());
                currently_banned.insert(row.hash.clone());
            }
            "unbanned" => {
                currently_banned.remove(&row.hash);
            }
            _ => {}
        }
    }

    let mut missing_from_file: Vec<&String> = currently_banned
        .iter()
        .filter(|h| !file_hashes.contains(*h))
        .collect();
    missing_from_file.sort();
    for h in missing_from_file {
        out.push(Failure {
            check: "banned".to_string(),
            id: None,
            message: msg::banned_missing_from_file(hash_prefix(h)),
        });
    }

    let mut missing_from_ledger: Vec<&String> = file_hashes
        .iter()
        .filter(|h| !ever_banned.contains(*h))
        .collect();
    missing_from_ledger.sort();
    for h in missing_from_ledger {
        out.push(Failure {
            check: "banned".to_string(),
            id: None,
            message: msg::banned_missing_from_ledger(hash_prefix(h)),
        });
    }

    out
}

fn hash_prefix(h: &str) -> &str {
    &h[..h.len().min(12)]
}

pub(crate) fn collect_targets(
    root: &Path,
    staged: bool,
    message: Option<&Path>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(msg_path) = message {
        if let Ok(content) = std::fs::read_to_string(msg_path) {
            out.push((msg_path.display().to_string(), content));
        }
        return out;
    }
    if staged {
        if let Some(list) = repo::git(
            root,
            &["diff", "--cached", "--name-only", "--diff-filter=d"],
        ) {
            for path in list.lines() {
                if skip_path(path) {
                    continue;
                }
                if let Some(content) = repo::git(root, &["show", &format!(":{}", path)]) {
                    out.push((path.to_string(), content));
                }
            }
        }
        return out;
    }

    let mut paths: Vec<String> = Vec::new();
    if let Some(list) = repo::git(root, &["ls-files"]) {
        paths.extend(list.lines().map(|s| s.to_string()));
    }
    for f in repo::list_decision_files(root) {
        if let Ok(rel) = f.strip_prefix(root) {
            let s = rel.to_string_lossy().replace('\\', "/");
            if !paths.contains(&s) {
                paths.push(s);
            }
        }
    }
    for p in paths {
        if skip_path(&p) {
            continue;
        }
        let full = root.join(&p);
        if let Ok(content) = std::fs::read_to_string(&full) {
            out.push((p, content));
        }
    }
    out
}

fn skip_path(p: &str) -> bool {
    p == "adrdom/state/banned.sha256"
        || p == "adrdom/state/ledger.tsv"
        // A banned word removed from adrdom/terms.md would otherwise stay
        // visible in the snapshot (`adrdom/state/terms.approved.md`) until
        // the next approve: skipped here the same way ledger.tsv is, since
        // it holds terms text from a point in time, not an agent's own
        // words to scan.
        || p == "adrdom/state/terms.approved.md"
        || p.starts_with(".git/")
}
