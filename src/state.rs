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

/// True when `hash` matches the latest ledger row for `event`/`rel`: the
/// comparison `plan_approved`/`context_approved` make, factored out so a
/// caller that already holds the hash -- computed from the file exactly as
/// it sits on disk, before any in-memory normalization
/// (`depends::normalize_cross_file_deps`) touched it -- can make the same
/// comparison without recomputing the hash from a `RecordFile` that may by
/// then have been normalized. Approval state must always read the file
/// exactly as written, never the resolved-for-tiers copy: normalization is
/// only for resolving depends targets and deriving tiers.
fn hash_matches_latest(led: &Ledger, event: &str, rel: &str, hash: &str) -> bool {
    match led.latest_for_path(event, rel) {
        Some(row) => row.hash == hash,
        None => false,
    }
}

/// True when `plan_hash` (computed by the caller) matches the latest
/// `approved` ledger row for `rel`. See `hash_matches_latest`.
pub fn plan_approved_hash(led: &Ledger, rel: &str, plan_hash: &str) -> bool {
    hash_matches_latest(led, "approved", rel, plan_hash)
}

/// True when `context_hash` (computed by the caller) matches the latest
/// `context` ledger row for `rel`. See `hash_matches_latest`.
pub fn context_approved_hash(led: &Ledger, rel: &str, context_hash: &str) -> bool {
    hash_matches_latest(led, "context", rel, context_hash)
}

/// True when the topic file's current plan hash matches the latest `approved`
/// ledger row for `rel` (the topic file's path relative to the repo root).
/// `rf` must not have gone through `depends::normalize_cross_file_deps` --
/// that mutates the parsed `depends:` entries this hashes, in memory, so a
/// caller that normalizes its `rf` must compute the hash before doing so
/// and call `plan_approved_hash` with it instead of this.
pub fn plan_approved(led: &Ledger, rf: &RecordFile, rel: &str) -> bool {
    plan_approved_hash(led, rel, &record::plan_hash(rf))
}

/// True when the topic file's current context hash matches the latest
/// `context` ledger row for `rel`. Context hashing never reads a
/// section's `depends:` entries, so it is unaffected by
/// `depends::normalize_cross_file_deps` in practice -- kept alongside
/// `plan_approved` (and `context_approved_hash` given the same way) so
/// nothing about that stays implicit.
pub fn context_approved(led: &Ledger, rf: &RecordFile, rel: &str) -> bool {
    context_approved_hash(led, rel, &record::context_hash(rf))
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

/// Like `load_terms`, but through `source` (per "Does check at a commit
/// read the staged files?"): every `check` caller that needs the terms
/// file uses this instead, so a staged edit to it is judged, not whatever
/// is left on disk. Every other caller (`ask`, `approve`, `ban`, `show`,
/// `serve`) keeps calling `load_terms`, which always wants the working
/// tree.
pub fn load_terms_via(root: &Path, source: &crate::repo::Source) -> Option<TermsFile> {
    let content = source.read(root, "adrdom/terms.md")?;
    let mut tf = terms::parse(&record::split_lines(&content));
    tf.path = root.join("adrdom/terms.md");
    Some(tf)
}

/// `adrdom/state/terms.approved.md`: the exact bytes `approve
/// adrdom/terms.md` last wrote to `adrdom/terms.md`, snapshotted in the same
/// run as its ledger event (`actions::do_approve_terms`). `check`'s only
/// source for what the terms file looked like at its latest approval (per
/// "Does check fail a terms file that differs from its latest approval?") --
/// unlike walking `git log`, this is a plain tracked file, so it survives a
/// shallow checkout, a squash merge, or a rebase that a commit-by-commit
/// history search would lose. `None` when the file has never been
/// approved (no snapshot has ever been written) or can't be read.
pub fn load_terms_snapshot(root: &Path) -> Option<TermsFile> {
    let path = root.join("adrdom/state/terms.approved.md");
    let content = std::fs::read_to_string(path).ok()?;
    Some(terms::parse(&record::split_lines(&content)))
}

/// Like `load_terms_snapshot`, but through `source`: `check`'s own copy,
/// used the same way `load_terms_via` is.
pub fn load_terms_snapshot_via(root: &Path, source: &crate::repo::Source) -> Option<TermsFile> {
    let content = source.read(root, "adrdom/state/terms.approved.md")?;
    Some(terms::parse(&record::split_lines(&content)))
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
    let path = root.join("adrdom/terms.md");
    if !path.exists() {
        return TermsLoad::Missing;
    }
    match terms::load(&path) {
        Ok(tf) => TermsLoad::Ok(tf),
        Err(e) => TermsLoad::LoadError(e.to_string()),
    }
}

/// True when `id`'s latest state row in the ledger is `answered`
/// (current). `superseded`/`withdrawn`/`asked`/never-asked are all false.
pub fn is_current(led: &Ledger, id: &str) -> bool {
    matches!(led.latest_state(id), Some(row) if row.event == "answered")
}

/// True when `sec`'s latest ledger state is `answered`, `refused`, or
/// `superseded`: its decision is closed, one way or another, so its text --
/// heading included -- is never edited again (per "Does a decision change
/// after it is answered?" and "Does the tool stop loaded phrases in a
/// question?"). A
/// section with no id yet, or none of those three states, is unfrozen.
/// The one shared "is this section frozen" test `approve` and `check` both
/// use.
pub fn is_frozen(led: &Ledger, sec: &Section) -> bool {
    sec.id
        .as_ref()
        .map(|id| {
            matches!(
                led.latest_state(id).map(|r| r.event.as_str()),
                Some("answered") | Some("refused") | Some("superseded")
            )
        })
        .unwrap_or(false)
}

/// True when `sec`'s latest ledger state is `refused`, `superseded`, or
/// `withdrawn`: the question is closed and out of play -- a refused
/// question is done, a superseded decision has been replaced, a withdrawn
/// question no longer stands in the file -- so it no longer holds its tier
/// for the cap (per "Does the tool cap the questions in a tier?").
/// `answered` is still live for this purpose (only
/// `is_frozen`'s edit-lock treats it as closed); a section with no id yet,
/// or none of those three states, is live.
pub fn is_dead(led: &Ledger, sec: &Section) -> bool {
    sec.id
        .as_ref()
        .map(|id| {
            matches!(
                led.latest_state(id).map(|r| r.event.as_str()),
                Some("refused") | Some("superseded") | Some("withdrawn")
            )
        })
        .unwrap_or(false)
}

/// `tiers` (index-aligned with `rf.sections`, as `rf.tiers` is) with every
/// dead section's entry (`is_dead`) left out: the tier cap (per "Does the
/// tool cap the questions in a tier?") counts
/// only live questions, while everything else a tier holds -- derivation,
/// a refused question holding its tier until the plan changes -- still
/// reads `rf.tiers` whole. The one place that filters before counting, so
/// `record::over_cap`/`record::tier_sizes` (which take a plain tier list
/// and have no ledger access) stay ignorant of ledger state.
pub fn live_tiers(led: &Ledger, rf: &RecordFile, tiers: &[usize]) -> Vec<usize> {
    rf.sections
        .iter()
        .zip(tiers)
        .filter(|(sec, _)| !is_dead(led, sec))
        .map(|(_, &t)| t)
        .collect()
}

/// The text a section's `terms:` line is (re)computed from, and what the
/// banned-word scan reads (per "Does a reference in a topic file carry the
/// identifier of what it names?"): the section's hashed range (heading
/// through the hash end, see `record::hash_range`) with the `terms:` line
/// itself left out (so re-resolving never matches against a term's own
/// previous resolution rather than the prose that produced it), and with
/// `id:`, `depends:`, and `supersedes:` lines left out too, since those hold
/// ids and references, not prose, once a plan is approved.
pub fn section_text_excluding_terms_line(lines: &[String], sec: &Section) -> String {
    let (s, e) = record::hash_range(lines, sec);
    lines[s..e]
        .iter()
        .enumerate()
        .filter(|&(j, l)| {
            let idx = s + j;
            if Some(idx) == sec.terms_line {
                return false;
            }
            let t = l.trim_start();
            !(t.starts_with("id: ") || t.starts_with("depends: ") || t.starts_with("supersedes: "))
        })
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
