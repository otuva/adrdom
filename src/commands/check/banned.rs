use crate::rules::Class;
use super::Failure;
use crate::banned;
use crate::ledger::Ledger;
use crate::msg;
use crate::repo::{self, Source};
use crate::state;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(super) fn check_banned(
    root: &Path,
    staged: bool,
    message: Option<&Path>,
    source: &Source,
    stops: &mut Vec<String>,
) -> Vec<Failure> {
    let mut out = Vec::new();
    // A message file that cannot be read is never scanned: a failure, or
    // the commit would pass with its message unread.
    if let Some(msg_path) = message {
        if let Err(e) = std::fs::read_to_string(msg_path) {
            stops.push(msg::message_file_unreadable(&msg_path.display().to_string(), e));
        }
    }
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
            out.push(Failure::said(Class::Banned, msg::banned_word_at(&msg::location(&name, hit.line), &suffix)));
        }
    }
    out
}

/// The ban list against the ledger's `banned`/`unbanned` history (per "Does
/// check hold the ban list equal to the ledger?"): the file must hold
/// exactly the hashes the ledger says are banned now, no more and no
/// fewer. Every hash the ledger currently considers banned (a `banned` row
/// with no later `unbanned` row) must be in the file; every hash in the
/// file must trace back to some `banned` row; and every hash in the file
/// that the ledger's own latest row calls `unbanned` is flagged too, since
/// the file and the ledger's current state must agree, not just the
/// file and the ledger's history.
pub(super) fn check_banned_ledger(led: &Ledger, root: &Path, source: &Source) -> Vec<Failure> {
    let mut out = Vec::new();
    let file_hashes: HashSet<String> = read_banned_entries(root, source)
        .into_iter()
        .map(|(h, _)| h)
        .collect();

    let mut ever_banned: HashSet<String> = HashSet::new();
    for row in &led.rows {
        if row.event == "banned" {
            ever_banned.insert(row.hash.clone());
        }
    }
    // Shared with `adr ban --rebuild` (`banned::currently_banned`), so the
    // two can never disagree about what "banned now" means.
    let currently_banned: HashSet<String> = banned::currently_banned(led)
        .into_iter()
        .map(|(h, _)| h)
        .collect();

    let mut missing_from_file: Vec<&String> = currently_banned
        .iter()
        .filter(|h| !file_hashes.contains(*h))
        .collect();
    missing_from_file.sort();
    for h in missing_from_file {
        out.push(Failure::said(Class::Banned, msg::banned_missing_from_file(hash_prefix(h))));
    }

    let mut missing_from_ledger: Vec<&String> = file_hashes
        .iter()
        .filter(|h| !ever_banned.contains(*h))
        .collect();
    missing_from_ledger.sort();
    for h in missing_from_ledger {
        out.push(Failure::said(Class::Banned, msg::banned_missing_from_ledger(hash_prefix(h))));
    }

    let mut unbanned_but_in_file: Vec<&String> = file_hashes
        .iter()
        .filter(|h| ever_banned.contains(*h) && !currently_banned.contains(*h))
        .collect();
    unbanned_but_in_file.sort();
    for h in unbanned_but_in_file {
        out.push(Failure::said(Class::Banned, msg::banned_unbanned_but_in_file(hash_prefix(h))));
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
        if let Some(list) = repo::staged_paths(root, false) {
            for path in &list {
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
    if let Some(list) = repo::git_paths(root, &["ls-files"]) {
        paths.extend(list);
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
