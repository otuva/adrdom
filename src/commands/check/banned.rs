use super::Failure;
use crate::banned;
use crate::ledger::Ledger;
use crate::msg;
use crate::repo::{self, Source};
use crate::state;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(super) fn check_banned(root: &Path, staged: bool, message: Option<&Path>, source: &Source) -> Vec<Failure> {
    let mut out = Vec::new();
    let entries = read_banned_entries(root, source);
    if entries.is_empty() {
        return out;
    }
    let hash_map: HashMap<String, Option<String>> = entries.into_iter().collect();
    let tf = state::load_terms_via(root, source);
    let terms_list: &[crate::terms::Term] = tf.as_ref().map(|f| f.terms.as_slice()).unwrap_or(&[]);

    for (name, content) in collect_targets(root, staged, message) {
        for hit in banned::hits(&content, &hash_map, terms_list) {
            let suffix = msg::term_suffix(hit.term.as_deref());
            out.push(Failure {
                check: "banned".to_string(),
                message: msg::banned_word_at(&msg::location(&name, hit.line), &suffix),
            });
        }
    }
    out
}

/// The ban list against the ledger's `banned`/`unbanned` history: every
/// hash the ledger currently considers banned (a `banned` row with no
/// later `unbanned` row) must be in the file, and every hash in the file
/// must trace back to some `banned` row (regardless of a later `unbanned`
/// row, which would otherwise have removed it from the file already).
pub(super) fn check_banned_ledger(led: &Ledger, root: &Path, source: &Source) -> Vec<Failure> {
    let mut out = Vec::new();
    let file_hashes: HashSet<String> = read_banned_entries(root, source)
        .into_iter()
        .map(|(h, _)| h)
        .collect();

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
            message: msg::banned_missing_from_ledger(hash_prefix(h)),
        });
    }

    out
}

fn hash_prefix(h: &str) -> &str {
    &h[..h.len().min(12)]
}

/// `adrdom/state/banned.sha256` through `source` (per "Does check at a
/// commit read the staged files?"): a staged edit to the ban list is
/// judged, not whatever is left on disk. Every entry, hash plus term uuid;
/// empty when the file doesn't exist in `source` at all.
fn read_banned_entries(root: &Path, source: &Source) -> Vec<(String, Option<String>)> {
    banned::read_entries_via(root, source)
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
                if let Some(content) = Source::Index.read(root, path) {
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
    for p in paths {
        if skip_path(&p) {
            continue;
        }
        if let Some(content) = Source::WorkingTree.read(root, &p) {
            out.push((p, content));
        }
    }
    out
}

fn skip_path(p: &str) -> bool {
    p == banned::BAN_LIST || p == repo::LEDGER_PATH || p.starts_with(".git/")
}
