// Shared helpers for the commands that gate on approval/terms state
// (`ask`, `approve`, and anything else that needs the same reads). Kept
// here so `check` and other slices can reuse them instead of duplicating
// this logic.
use crate::ledger::Ledger;
use crate::record;
use crate::terms::{self, TermsFile};
use std::path::{Component, Path};

mod files;
mod questions;
mod rulings;
mod term_entries;

pub use files::{
    blame_approved, context_approved, context_approved_hash, dropped_hashes, is_dropped,
    is_struck_from, is_superseded, plan_approved, plan_approved_hashes, superseded_ids,
    terms_approved,
};
pub use questions::{
    answer_waits, asked_text_stands, closed_match, frozen_text_edited, is_current, is_dead,
    is_frozen, is_open_again, over_cap_headings, section_text_excluding_terms_line,
    tier_unresolved, unresolved_sibling, AnswerWaits, ClosedMatch,
};
pub use rulings::{conflicting_rulings, current_successors, standing_rulings, Ruling};
pub use term_entries::{
    approve_retires_or_removes, dependent_entries, removed_terms, retire_unrecorded,
    unreadable_asked_terms_lines, unsettled_entries, waiting_terms, DependentEntry,
    DependentSource, TERM_REMOVED,
};

/// `path` relative to `root`, with forward slashes, e.g.
/// `adrdom/decisions/plan.md`. Works whether `path` is already relative
/// (left as-is) or absolute under `root` (the `root` prefix is stripped).
pub fn rel_path(root: &Path, path: &Path) -> String {
    let rel: &Path = path.strip_prefix(root).unwrap_or(path);
    let mut parts: Vec<String> = Vec::new();
    for comp in rel.components() {
        if let Component::Normal(s) = comp {
            parts.push(s.to_string_lossy().into_owned());
        }
    }
    parts.join("/")
}

/// A path a command was given, from the repository root: read the way the
/// shell that ran the command reads it (relative to the current directory
/// unless absolute, `.` and `..` resolved), `None` when it lies outside the
/// repository. What `show`, `plan`, `ask` and `approve` read a path
/// argument by, so `cd adrdom/decisions && adr ask gate.md` names the file
/// every other spelling names.
pub fn given_rel(root: &Path, file: &Path) -> Option<String> {
    let abs = if file.is_absolute() {
        file.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|d| d.join(file))
            .unwrap_or_else(|_| root.join(file))
    };
    let normal = lexical(&abs);
    let root_abs = lexical(&std::path::Path::new(std::path::MAIN_SEPARATOR_STR).join(root));
    // A root reached through a link (the current directory canonical, the
    // root as found, or the other way round) still holds its files.
    let inside = normal
        .strip_prefix(&root_abs)
        .ok()
        .map(Path::to_path_buf)
        .or_else(|| {
            let canon_root = std::fs::canonicalize(root).ok()?;
            let canon_parent = abs.parent().and_then(|p| std::fs::canonicalize(p).ok())?;
            let canon = canon_parent.join(abs.file_name()?);
            canon.strip_prefix(&canon_root).ok().map(Path::to_path_buf)
        })?;
    Some(rel_path(Path::new(""), &inside))
}

/// `path` with `.` dropped and `..` resolved against the component before
/// it, text only.
fn lexical(path: &Path) -> std::path::PathBuf {
    let mut parts: Vec<std::ffi::OsString> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                parts.pop();
            }
            Component::CurDir => {}
            Component::Normal(p) => parts.push(p.to_os_string()),
            Component::RootDir | Component::Prefix(_) => parts.clear(),
        }
    }
    let mut normal = std::path::PathBuf::from(std::path::MAIN_SEPARATOR_STR);
    normal.extend(parts);
    normal
}

/// True when `hash` matches the latest ledger row for `event`/`rel`: the
/// comparison `plan_approved`/`context_approved` make, factored out so a
/// caller that already holds the hash -- computed from the file exactly as
/// it sits on disk -- can make the same comparison without recomputing the
/// hash from a `RecordFile`. Approval state must always read the file
/// exactly as written.
fn hash_matches_latest(led: &Ledger, event: &str, rel: &str, hash: &str) -> bool {
    match led.latest_for_path(event, rel) {
        Some(row) => row.hash == hash,
        None => false,
    }
}

/// Loads `adrdom/terms.md` under `root`, or `None` when the repo has no
/// terms file at all (terms are optional).
pub fn load_terms(root: &Path) -> Option<TermsFile> {
    let path = root.join(crate::repo::TERMS_PATH);
    if !path.exists() {
        return None;
    }
    terms::load(&path).ok()
}

/// Like `load_terms`, but through `source` (per "Does check at a commit
/// read the staged files?"): every `check` caller that needs the terms
/// file uses this instead, so a staged edit to it is judged, not whatever
/// is left on disk. Every other caller (`ask`, `approve`, `ban`, `show`,
/// `serve`) keeps calling `load_terms`, which always wants the working
/// tree.
pub fn load_terms_via(root: &Path, source: &crate::repo::Source) -> Option<TermsFile> {
    let content = source.read(root, crate::repo::TERMS_PATH)?;
    let mut tf = terms::parse(&record::split_lines(&content));
    tf.path = root.join(crate::repo::TERMS_PATH);
    Some(tf)
}

/// The three ways a caller that requires `adrdom/terms.md` (`ask`, `answer`)
/// can find it: missing outright, present but unreadable/shape-invalid, or
/// loaded cleanly.
pub enum TermsLoad {
    Missing,
    LoadError(String),
    Ok(TermsFile),
}

/// Loads `adrdom/terms.md` under `root`, distinguishing "the file does not
/// exist" from "it exists but failed to read" so callers that must refuse
/// differently on each (`ask` 3.1/3.2, `answer` 6) don't have to duplicate
/// the `Path::exists` check `load_terms` collapses away.
pub fn load_terms_checked(root: &Path) -> TermsLoad {
    let path = root.join(crate::repo::TERMS_PATH);
    if !path.exists() {
        return TermsLoad::Missing;
    }
    match terms::load(&path) {
        Ok(tf) => TermsLoad::Ok(tf),
        Err(e) => TermsLoad::LoadError(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rel_path_strips_root_and_uses_forward_slashes() {
        let root = PathBuf::from("/repo");
        let path = PathBuf::from("/repo/adrdom/decisions/plan.md");
        assert_eq!(rel_path(&root, &path), "adrdom/decisions/plan.md");
    }

    #[test]
    fn rel_path_leaves_an_already_relative_path_alone() {
        let root = PathBuf::from("/repo");
        let path = PathBuf::from("adrdom/decisions/plan.md");
        assert_eq!(rel_path(&root, &path), "adrdom/decisions/plan.md");
    }
}
