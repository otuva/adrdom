// Shared helpers for the commands that gate on approval/terms state
// (`ask`, `approve`, and anything else that needs the same reads). Kept
// here so `check` and other slices can reuse them instead of duplicating
// this logic.
use crate::ledger::Ledger;
use crate::record::{self, RecordFile, Section};
use crate::terms::{self, TermsFile};
use std::path::{Component, Path};

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

/// True when the record's current plan hash matches the latest `approved`
/// ledger row for `rel` (the record's path relative to the repo root).
pub fn plan_approved(led: &Ledger, rf: &RecordFile, rel: &str) -> bool {
    match led.latest_for_path("approved", rel) {
        Some(row) => row.hash == record::plan_hash(rf),
        None => false,
    }
}

/// True when the record's current context hash matches the latest
/// `context` ledger row for `rel`.
pub fn context_approved(led: &Ledger, rf: &RecordFile, rel: &str) -> bool {
    match led.latest_for_path("context", rel) {
        Some(row) => row.hash == record::context_hash(rf),
        None => false,
    }
}

/// True when the terms file's current hash matches the latest `terms`
/// ledger row for `adrdom/terms.md`.
pub fn terms_approved(led: &Ledger, tf: &TermsFile) -> bool {
    match led.latest_for_path("terms", "adrdom/terms.md") {
        Some(row) => row.hash == terms::file_hash(tf),
        None => false,
    }
}

/// Loads `adrdom/terms.md` under `root`, or `None` when the repo has no
/// terms file at all (terms are optional).
pub fn load_terms(root: &Path) -> Option<TermsFile> {
    let path = root.join("adrdom/terms.md");
    if !path.exists() {
        return None;
    }
    terms::load(&path).ok()
}

/// The three ways a caller that requires `adrdom/terms.md` (`ask`, `lock`)
/// can find it: missing outright, present but unreadable/shape-invalid, or
/// loaded cleanly.
pub enum TermsLoad {
    Missing,
    LoadError(String),
    Ok(TermsFile),
}

/// Loads `adrdom/terms.md` under `root`, distinguishing "the file does not
/// exist" from "it exists but failed to read" so callers that must refuse
/// differently on each (`ask` 3.1/3.2, `lock` 6) don't have to duplicate
/// the `Path::exists` check `load_terms` collapses away.
pub fn load_terms_checked(root: &Path) -> TermsLoad {
    let path = root.join("adrdom/terms.md");
    if !path.exists() {
        return TermsLoad::Missing;
    }
    match terms::load(&path) {
        Ok(tf) => TermsLoad::Ok(tf),
        Err(e) => TermsLoad::LoadError(e.to_string()),
    }
}

/// True when `id`'s latest state row in the ledger is `answered` (in
/// force). `superseded`/`withdrawn`/`asked`/never-asked are all false.
pub fn is_in_force(led: &Ledger, id: &str) -> bool {
    matches!(led.latest_state(id), Some(row) if row.event == "answered")
}

/// The text a section's `terms:` line is (re)computed from: the section's
/// hashed range (heading through the hash end, see `record::hash_range`)
/// with the `terms:` line itself left out, so re-resolving never matches
/// against a term's own previous resolution rather than the prose that
/// produced it.
pub fn section_text_excluding_terms_line(lines: &[String], sec: &Section) -> String {
    let (s, e) = record::hash_range(lines, sec);
    lines[s..e]
        .iter()
        .enumerate()
        .filter(|&(j, _)| Some(s + j) != sec.terms_line)
        .map(|(_, l)| l.as_str())
        .collect::<Vec<_>>()
        .join("\n")
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
