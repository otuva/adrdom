// Shared helpers for the commands that gate on approval/terms state
// (`ask`, `approve`, and anything else that needs the same reads). Kept
// here so `check` and other slices can reuse them instead of duplicating
// this logic.
use crate::ledger::{Ledger, Row};
use crate::record::{self, RecordFile, Section};
use crate::terms::{self, TermsFile};
use std::collections::BTreeSet;
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
    match led.latest_for_path("terms", crate::repo::TERMS_PATH) {
        Some(row) => row.hash == terms::file_hash(tf),
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

/// `adrdom/state/terms.approved.md`: the exact bytes `approve
/// adrdom/terms.md` last wrote to `adrdom/terms.md`, snapshotted in the same
/// run as its ledger event (`actions::do_approve_terms`). Every reader that
/// needs the terms file's text as it stood at its latest approval (per
/// "Does check fail a terms file that differs from its latest approval?")
/// reads it through here -- unlike walking `git log`, this is a plain
/// tracked file, so it survives a shallow checkout, a squash merge, or a
/// rebase that a commit-by-commit history search would lose. `source`
/// picks where the bytes come from (`Source::WorkingTree` for every caller
/// but `check`, which passes whichever source it is judging, per "Does
/// check at a commit read the staged files?"). `None` when the file has
/// never been approved (no snapshot has ever been written) or can't be
/// read this way.
pub fn approved_terms(root: &Path, source: &crate::repo::Source) -> Option<TermsFile> {
    let content = source.read(root, crate::repo::TERMS_APPROVED_PATH)?;
    Some(terms::parse(&record::split_lines(&content)))
}

/// One term whose definition text in `current` differs from its text in
/// `approved` -- matched by id only, never by name (per "What does a
/// definition say?": the definition, not the aliases, is what a decision's
/// meaning rests on). A term present in `current` but missing from
/// `approved` entirely is not a "change" here and is left out; callers that
/// need to tell the two apart (`check`'s `EditImpact::MissingFromApprovedCopy`)
/// still make that distinction themselves.
pub struct DefinitionChange {
    pub id: String,
    pub name: String,
    pub old_definition: String,
    pub new_definition: String,
}

/// Every term in `current` whose definition differs from its counterpart in
/// `approved`, matched by id. Shared by `approve`'s step 7 report and the
/// page's before/after diff.
pub fn definition_changes(current: &TermsFile, approved: &TermsFile) -> Vec<DefinitionChange> {
    let mut out = Vec::new();
    for t in &current.terms {
        let Some(tid) = &t.id else { continue };
        let Some(old) = approved.terms.iter().find(|o| o.id.as_deref() == Some(tid.as_str())) else {
            continue;
        };
        if old.definition == t.definition {
            continue;
        }
        out.push(DefinitionChange {
            id: tid.clone(),
            name: t.name.clone(),
            old_definition: old.definition.clone(),
            new_definition: t.definition.clone(),
        });
    }
    out
}

/// One current decision's, or one non-retired term's, declared use of a
/// term: what to print (`label`, always the quoted form used everywhere a
/// dependent is printed) and, for a decision, its own id -- named so a
/// caller that must supersede it to clear a refusal has something to point
/// at (a term has nothing to supersede, so `decision_id` is `None` for
/// one).
pub enum DependentSource {
    Decision { rel: String, heading: String, id: String },
    Term { name: String },
}

impl DependentSource {
    /// `"<rel>: \"<heading>\""` for a decision, `"term \"<name>\""` for a
    /// term -- the one label format used everywhere a dependent is printed.
    pub fn label(&self) -> String {
        match self {
            DependentSource::Decision { rel, heading, .. } => format!("{}: \"{}\"", rel, heading),
            DependentSource::Term { name } => format!("term \"{}\"", name),
        }
    }

    pub fn decision_id(&self) -> Option<&str> {
        match self {
            DependentSource::Decision { id, .. } => Some(id),
            DependentSource::Term { .. } => None,
        }
    }
}

/// One source's pairing to one or more terms: the pairs themselves, and
/// whether each is a resolved word (subject to "did this word move off the
/// term" checks, since a terms-line entry is a snapshot of what some prose
/// used to resolve to) or a `depends:` id read straight off the entry
/// (never a resolved word, so only the target's removal/retirement applies
/// to it).
pub struct DependentEntry {
    pub source: DependentSource,
    pub pairs: Vec<(String, String)>,
    pub from_word: bool,
}

/// Every current decision's, and (when `include_terms`) every non-retired
/// term's, declared use of a term in `tf` -- the widest reach any of the
/// five readers of this need: a current decision's own `terms:` line
/// always counts; a `depends:` entry naming a term directly counts too when
/// `include_depends_ids` is set (skipped when the same id is already on
/// that decision's terms line, so retiring a term used both ways earns one
/// entry, not two); a non-retired term's own recorded pairing counts when
/// `include_terms` is set, from `term_pairs_override` when the caller
/// supplies one (`approve`'s step 7, reporting against pairs about to be
/// written rather than what is already on disk, indexed the same as
/// `tf.terms`) or from the term's own `terms_line` otherwise. Never
/// transitive on its own -- a caller that needs the closure over the term
/// graph runs `transitive_term_ids` on the pairs this returns.
pub fn dependent_entries(
    records: &[RecordFile],
    led: &Ledger,
    tf: &TermsFile,
    include_depends_ids: bool,
    include_terms: bool,
    term_pairs_override: Option<&[Vec<(String, String)>]>,
) -> Vec<DependentEntry> {
    let mut out = Vec::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !is_current(led, id) {
                continue;
            }
            let terms_line_pairs: Vec<(String, String)> = sec
                .terms_line
                .and_then(|tl| terms::parse_terms_line(rf.lines[tl].trim_end()))
                .unwrap_or_default();
            if !terms_line_pairs.is_empty() {
                out.push(DependentEntry {
                    source: DependentSource::Decision {
                        rel: rf.rel.clone(),
                        heading: sec.heading_text.clone(),
                        id: id.clone(),
                    },
                    pairs: terms_line_pairs.clone(),
                    from_word: true,
                });
            }
            if include_depends_ids {
                for d in &sec.depends {
                    let record::Dep::Id { id: dep_id, .. } = d else { continue };
                    if terms_line_pairs.iter().any(|(_, u)| u == dep_id) {
                        continue;
                    }
                    if !tf.terms.iter().any(|t| t.id.as_deref() == Some(dep_id.as_str())) {
                        continue;
                    }
                    out.push(DependentEntry {
                        source: DependentSource::Decision {
                            rel: rf.rel.clone(),
                            heading: sec.heading_text.clone(),
                            id: id.clone(),
                        },
                        pairs: vec![(dep_id.clone(), dep_id.clone())],
                        from_word: false,
                    });
                }
            }
        }
    }
    if include_terms {
        for (i, t) in tf.terms.iter().enumerate() {
            if t.retired {
                continue;
            }
            let pairs = match term_pairs_override {
                Some(fresh) => fresh.get(i).cloned().unwrap_or_default(),
                None => t
                    .terms_line
                    .as_deref()
                    .and_then(terms::parse_terms_line)
                    .unwrap_or_default(),
            };
            if pairs.is_empty() {
                continue;
            }
            out.push(DependentEntry {
                source: DependentSource::Term { name: t.name.clone() },
                pairs,
                from_word: true,
            });
        }
    }
    out
}

/// Every term id reached starting at `start` (already-resolved uuids) and,
/// while `transitive`, repeatedly following each landed term's own
/// recorded `terms:` line onward through `tf` -- cycle-safe. `start`
/// themselves are always included. Non-transitive, this is just `start`
/// deduplicated.
pub fn transitive_term_ids(
    start: impl IntoIterator<Item = String>,
    tf: &TermsFile,
    transitive: bool,
) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack: Vec<String> = start.into_iter().collect();
    while let Some(uuid) = stack.pop() {
        if !seen.insert(uuid.clone()) {
            continue;
        }
        if !transitive {
            continue;
        }
        if let Some(t) = tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str())) {
            if let Some(more) = t.terms_line.as_deref().and_then(terms::parse_terms_line) {
                stack.extend(more.into_iter().map(|(_, u)| u));
            }
        }
    }
    seen
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

/// `done` (per "Does a topic file close?"): a file is done once its plan
/// and context are approved and every section carries a letter answer.
/// `plan_ok`/`ctx_ok` are supplied by the caller (`check`'s `file_approval`
/// reads them off a hash captured before any in-memory normalize pass,
/// `view`'s off the working tree the same way) rather than recomputed here.
pub fn file_done(rf: &RecordFile, plan_ok: bool, ctx_ok: bool) -> bool {
    plan_ok && ctx_ok && rf.sections.iter().all(|s| s.answer_letter().is_some())
}

/// True when `sec` has no letter answer yet: no `**Answer:** <letter>`
/// line at all (per "Does a topic file close?" and "Does the owner answer
/// a question with a refusal?") -- a skeleton with nothing written, an
/// unasked or asked-but-open question, and a refused question (whose
/// `**Answer:** refused` line never parses as a letter) are all
/// unresolved; only a genuine letter answer, or a superseded section's old
/// letter still sitting in the file, resolves it. Read straight off the
/// file text, no ledger needed: the one "is this section unresolved" test
/// the tier rule reads.
pub fn tier_unresolved(sec: &Section) -> bool {
    sec.answer_letter().is_none()
}

/// The lowest derived tier (per "Does the tool derive tiers from the
/// depends lines?") holding an unresolved section (`tier_unresolved`; a
/// skeleton heading with no answer keeps its tier open exactly as a
/// refused or still-open question does), or `None` when tier derivation
/// failed (`rf.tiers` is `None`) or every section is resolved. What `ask`
/// computes once for its tier rule, and what the page's `waits_on` reads
/// too.
pub fn open_tier(rf: &RecordFile) -> Option<usize> {
    rf.sections
        .iter()
        .enumerate()
        .filter(|(_, s)| tier_unresolved(s))
        .filter_map(|(i, _)| rf.tier_of(i))
        .min()
}

/// The section in tier `t` of `rf` a candidate outside the open tier is
/// blocked on, by file order: the first non-skeleton unresolved section,
/// or, when the tier holds nothing but a skeleton, the first skeleton in
/// it instead (naming a heading with nothing written yet is still more
/// informative than naming nothing) -- so `ask`'s tier-blocked refusal
/// always has something to name once `open_tier` returns `Some`. `None`
/// only when tier `t` holds no section at all.
pub fn tier_blocker(rf: &RecordFile, t: usize) -> Option<&Section> {
    let in_tier = |i: &usize| rf.tier_of(*i) == Some(t);
    rf.sections
        .iter()
        .enumerate()
        .find(|(i, s)| !s.is_skeleton && tier_unresolved(s) && in_tier(i))
        .or_else(|| {
            rf.sections
                .iter()
                .enumerate()
                .find(|(i, s)| s.is_skeleton && in_tier(i))
        })
        .map(|(_, s)| s)
}

/// What comparing a closed (answered or refused) section's current text
/// against the ledger rows that back it finds (per "Does check fail an
/// answered question whose text changed?" and "Does the owner answer a
/// question with a refusal?"). `check`'s per-section answered/refused
/// comparison and `ask`'s "edited after it closed" gate share this instead
/// of each keeping its own copy, so a genuine re-answer -- a harmless edit
/// followed by a real re-answer at the same letter, which adds a second
/// `answered` row rather than rewriting the first -- never desyncs the
/// two: every row for the id at the relevant event (and, for "answered",
/// at the letter the file currently names) is checked, not only the
/// latest.
pub enum ClosedMatch {
    /// No `**Answer:**` line at all, or one with nothing behind it in the
    /// ledger, ever, at this event.
    NotClosed,
    /// A genuine row backs the current text exactly.
    Current,
    /// The section was genuinely closed at some point (a real row for this
    /// id at this event), but none of those rows' hash matches the current
    /// text: it moved since.
    Edited,
}

/// `sec`'s `ClosedMatch`, from `id`'s ledger rows and `cur_hash` (the
/// caller's already-computed `record::section_hash(lines, sec)`, passed in
/// rather than recomputed here since both callers already have it).
pub fn closed_match(led: &Ledger, cur_hash: &str, sec: &Section, id: &str) -> ClosedMatch {
    if sec.is_refused() {
        let rows_for_id: Vec<&Row> = led
            .rows
            .iter()
            .filter(|r| r.event == "refused" && r.id == id)
            .collect();
        if rows_for_id.iter().any(|r| r.hash == cur_hash) {
            ClosedMatch::Current
        } else if !rows_for_id.is_empty() {
            ClosedMatch::Edited
        } else {
            ClosedMatch::NotClosed
        }
    } else if sec.answer_line.is_some() {
        let rows_for_id: Vec<&Row> = led
            .rows
            .iter()
            .filter(|r| r.event == "answered" && r.id == id)
            .collect();
        let letter_matches = sec.answer_letter().is_some_and(|l| {
            rows_for_id
                .iter()
                .any(|r| r.detail == l.to_string() && r.hash == cur_hash)
        });
        if letter_matches {
            ClosedMatch::Current
        } else if !rows_for_id.is_empty() {
            ClosedMatch::Edited
        } else {
            ClosedMatch::NotClosed
        }
    } else {
        ClosedMatch::NotClosed
    }
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
            !crate::tells::starts_with_any(
                l,
                &[crate::tells::ID_PREFIX, crate::tells::DEPENDS_PREFIX, crate::tells::SUPERSEDES_PREFIX],
            )
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
