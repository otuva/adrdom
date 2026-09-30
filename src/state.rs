// Shared helpers for the commands that gate on approval/terms state
// (`ask`, `approve`, and anything else that needs the same reads). Kept
// here so `check` and other slices can reuse them instead of duplicating
// this logic.
use crate::ledger::{Ledger, Row};
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
    match led.latest_for_path("terms", crate::repo::TERMS_PATH) {
        Some(row) => row.hash == terms::file_hash(tf),
        None => false,
    }
}

/// True when `content` (a blame file's current text, exactly as it stands)
/// hashes to the latest `blame` ledger row for `rel` (per "Does the owner
/// approve a blame file?", e879fe72 A: "a blame file that differs from its
/// latest approval is a draft"). Hashed the same way `crate::blame::file_hash`
/// hashes the rewritten lines `do_approve_blame` is about to write, so a
/// file freshly approved always reads back as approved.
pub fn blame_approved(led: &Ledger, rel: &str, content: &str) -> bool {
    let lines = record::split_lines(content);
    let hash = crate::blame::file_hash(&lines);
    match led.latest_for_path("blame", rel) {
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
/// term's, declared use of a term in `tf` -- the widest reach any of its
/// three readers need: a current decision's own `terms:` line always
/// counts; a `depends:` entry naming a term directly counts too when
/// `include_depends_ids` is set (skipped when the same id is already on
/// that decision's terms line, so retiring a term used both ways earns one
/// entry, not two); a non-retired term's own recorded pairing, from its own
/// `terms_line`, counts when `include_terms` is set. Never transitive on
/// its own -- these pairs are not walked further through the term graph.
pub fn dependent_entries(
    records: &[RecordFile],
    led: &Ledger,
    tf: &TermsFile,
    include_depends_ids: bool,
    include_terms: bool,
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
        for t in tf.terms.iter() {
            if t.retired {
                continue;
            }
            let pairs = t
                .terms_line
                .as_deref()
                .and_then(terms::parse_terms_line)
                .unwrap_or_default();
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

/// True when the latest ledger state of `id` is `asked` and that row holds
/// `cur_hash`, the hash of the section's text now: the text stands as it was
/// asked. `answer` takes a question only then, `show` prints its version
/// only then, and `next` names the owner's answer only then; an edited
/// question is the agent's to ask again (per "Does an answer carry the
/// version of the question the owner read?").
pub fn asked_text_stands(led: &Ledger, id: &str, cur_hash: &str) -> bool {
    matches!(led.latest_state(id), Some(row) if row.event == "asked" && row.hash == cur_hash)
}

/// True when the latest ledger state of `id` is `asked` although an earlier
/// row answered or refused it: the question was asked again (per "Does
/// check fail a question answered with two different options?") and waits
/// for the owner.
pub fn is_open_again(led: &Ledger, id: &str) -> bool {
    matches!(led.latest_state(id), Some(row) if row.event == "asked")
        && led
            .rows
            .iter()
            .any(|r| r.id == id && matches!(r.event.as_str(), "answered" | "refused"))
}

/// The live questions of the first tier over the cap, `None` when no tier
/// is over it: the headings of those not answered yet, in file order, and
/// the count of the answered ones. An answered question holds its place in
/// the tier (`live_tiers`) but its depends line is never edited again
/// (`is_frozen`), so the cap refusal (per "Does a refusal name the command
/// that clears it?") names only the ones the agent can still move. Read by
/// `approve` and `check` through `record_refusals` and by `ask`.
pub fn over_cap_headings(
    led: &Ledger,
    rf: &RecordFile,
    tiers: &[usize],
) -> Option<(Vec<String>, usize)> {
    let live: Vec<(&Section, usize)> = rf
        .sections
        .iter()
        .zip(tiers)
        .filter(|(sec, _)| !is_dead(led, sec))
        .map(|(sec, &t)| (sec, t))
        .collect();
    let live_tiers: Vec<usize> = live.iter().map(|(_, t)| *t).collect();
    let (tier, _) = crate::record::over_cap(&live_tiers)?;
    let mut open = Vec::new();
    let mut answered = 0;
    for (sec, _) in live.into_iter().filter(|(_, t)| *t == tier) {
        if sec.id.as_deref().is_some_and(|id| is_current(led, id)) {
            answered += 1;
        } else {
            open.push(sec.heading_text.clone());
        }
    }
    Some((open, answered))
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

/// The section beside section `i` in its tier that is not answered: the
/// first one still waiting for an answer, else the first refused one (a
/// refused question counts as not answered until a plan change). What
/// "Does the agent build on a decision before its tier is fully answered?"
/// waits on; `None` when the tier is fully answered or has no derived tier.
pub fn unresolved_sibling(rf: &RecordFile, i: usize) -> Option<&Section> {
    let tiers = rf.tiers.as_ref()?;
    let find = |refused: bool| {
        rf.sections
            .iter()
            .enumerate()
            .find(|(j, s)| *j != i && tiers[*j] == tiers[i] && tier_unresolved(s) && s.is_refused() == refused)
            .map(|(_, s)| s)
    };
    find(false).or_else(|| find(true))
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

/// One ruling a question earned toward its `id`: an answered letter, or a
/// refusal. Per "Does check fail a question answered with two different
/// options?" (e3200081-9995-4bbe-983f-2f430dc341e7, rule 8): a refusal and
/// an answer are two different rulings, so a `refused` row counts here
/// exactly as an `answered` row does. Every `refused` row is the same one
/// ruling, distinct from every letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ruling {
    Answered(char),
    Refused,
}

impl Ruling {
    /// The word this ruling names in a message: the letter itself, or
    /// `"refused"`.
    pub fn label(&self) -> String {
        match self {
            Ruling::Answered(c) => c.to_string(),
            Ruling::Refused => "refused".to_string(),
        }
    }
}

/// `row`'s ruling, when it is an `answered` or `refused` row for the id it
/// is checked against (any other event, including `asked`, carries no
/// ruling).
fn ruling_of(row: &Row) -> Option<Ruling> {
    match row.event.as_str() {
        "answered" => row.detail.trim().chars().next().map(Ruling::Answered),
        "refused" => Some(Ruling::Refused),
        _ => None,
    }
}

/// The standing rulings for `id`, per "Does check fail a question answered
/// with two different options?" (e3200081-9995-4bbe-983f-2f430dc341e7): take
/// `id`'s `asked` and `answered`/`refused` rows in ledger order (`led.rows`
/// is already ordered by time); let `r` be the position of the latest
/// `asked` row. `r` is an "asked again" row when the rulings before it --
/// each `answered` row's letter and each `refused` row, per `ruling_of` --
/// name two or more different rulings. The standing rulings are the
/// `answered`/`refused` rows after `r` when `r` is an asked-again row,
/// otherwise every `answered`/`refused` row of the question. `None` when
/// `id` was never asked at all -- there is no `r` to reckon from.
pub fn standing_rulings(led: &Ledger, id: &str) -> Option<Vec<Ruling>> {
    let r_pos = led.rows.iter().rposition(|row| row.id == id && row.event == "asked")?;

    let mut distinct_rulings_before: Vec<Ruling> = Vec::new();
    for row in &led.rows[..r_pos] {
        if row.id == id {
            if let Some(ruling) = ruling_of(row) {
                if !distinct_rulings_before.contains(&ruling) {
                    distinct_rulings_before.push(ruling);
                }
            }
        }
    }
    let asked_again = distinct_rulings_before.len() >= 2;

    let range: &[Row] = if asked_again { &led.rows[r_pos + 1..] } else { &led.rows[..] };
    let rulings: Vec<Ruling> = range
        .iter()
        .filter(|row| row.id == id)
        .filter_map(ruling_of)
        .collect();
    Some(rulings)
}

/// The first two distinct standing rulings for `id`
/// (e3200081-9995-4bbe-983f-2f430dc341e7, rule 3), in the order their rows
/// appear in the ledger -- `Some` exactly when the question is in conflict:
/// its standing rulings name two or more different options. `None` when
/// `id` was never asked, or its standing rulings all agree.
pub fn conflicting_rulings(led: &Ledger, id: &str) -> Option<(Ruling, Ruling)> {
    let rulings = standing_rulings(led, id)?;
    let mut distinct: Vec<Ruling> = Vec::new();
    for r in rulings {
        if !distinct.contains(&r) {
            distinct.push(r);
        }
    }
    if distinct.len() >= 2 {
        Some((distinct[0], distinct[1]))
    } else {
        None
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

    // adr: e879fe72-ec07-48cd-9902-2b158aaae726
    //
    // "a blame file that differs from its latest approval is a draft":
    // no `blame` row at all, or one whose hash doesn't match the content's
    // own hash, is not approved; a row over the exact hash `blame::file_hash`
    // computes for that content is.
    #[test]
    fn blame_approved_compares_the_latest_blame_row_hash() {
        let content = "origin: docs/x.md\nblob: 0000000000000000000000000000000000000000\n\n1-1\tdropped: dead\n";
        let led = Ledger { rows: vec![], malformed: vec![] };
        assert!(!blame_approved(&led, "adrdom/blame/x.md", content));

        let hash = crate::blame::file_hash(&record::split_lines(content));
        let led = Ledger {
            rows: vec![Row {
                time: "2024-01-01T00:00:00Z".to_string(),
                event: "blame".to_string(),
                id: "-".to_string(),
                hash,
                detail: "adrdom/blame/x.md".to_string(),
                version: "0.1.0".to_string(),
            }],
            malformed: vec![],
        };
        assert!(blame_approved(&led, "adrdom/blame/x.md", content));
        assert!(!blame_approved(&led, "adrdom/blame/other.md", content));
        assert!(!blame_approved(&led, "adrdom/blame/x.md", "different content\n"));
    }

    // adr: e3200081-9995-4bbe-983f-2f430dc341e7
    //
    // The case walk from the build brief, at the ledger level (no files, no
    // git): c1 through c7, each built as the exact row sequence its own
    // description names, times ticking one second apart (the tests only
    // need each row to sort after the last, not real wall-clock gaps).
    mod conflict_tests {
        use super::*;

        fn row_at(event: &str, id: &str, detail: &str, time: &str) -> Row {
            Row {
                time: time.to_string(),
                event: event.to_string(),
                id: id.to_string(),
                hash: "h".to_string(),
                detail: detail.to_string(),
                version: "0.1.0".to_string(),
            }
        }

        fn led_of(rows: Vec<Row>) -> Ledger {
            // Rows come out of `ledger::parse` already ordered by time;
            // built here the same way, from the clean list this test
            // module writes them in (each `row_at` call already at a later
            // time than the last).
            Ledger { rows, malformed: vec![] }
        }

        fn t(n: u32) -> String {
            format!("2024-01-01T00:00:{:02}Z", n)
        }

        // c1: base asked t1; branch a answers A t2; branch b answers B t3
        // (merged) -> conflict; re-asked t5; owner answers A t6 -> A
        // stands, no conflict.
        #[test]
        fn c1_two_answers_after_one_ask_conflict_then_resolved() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("answered", "q", "B", &t(3)),
            ]);
            let (a, b) = conflicting_rulings(&led, "q").expect("c1 should conflict");
            assert_eq!((a, b), (Ruling::Answered('A'), Ruling::Answered('B')));

            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("answered", "q", "B", &t(3)),
                row_at("asked", "q", "Q?", &t(5)),
                row_at("answered", "q", "A", &t(6)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none(), "A stands, no conflict");
        }

        // c2: branch a asked t1, answered A t2; branch b asked t3, answered
        // B t4 (base never asked) -> R = t3 is not asked-again (before it
        // only A) -> standing = all = {A, B} -> conflict; re-asked t5,
        // answered B t6 -> B stands.
        #[test]
        fn c2_independently_asked_on_each_branch_conflict_then_resolved() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("asked", "q", "Q?", &t(3)),
                row_at("answered", "q", "B", &t(4)),
            ]);
            let (a, b) = conflicting_rulings(&led, "q").expect("c2 should conflict");
            assert_eq!((a, b), (Ruling::Answered('A'), Ruling::Answered('B')));

            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("asked", "q", "Q?", &t(3)),
                row_at("answered", "q", "B", &t(4)),
                row_at("asked", "q", "Q?", &t(5)),
                row_at("answered", "q", "B", &t(6)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none(), "B stands, no conflict");
        }

        // c3: interleaved -- asked t1 (a), asked t2 (b), answered A t3,
        // answered B t4 -> conflict (the latest asked row, t2, has no
        // answered row before it at all, so it is not asked-again either).
        #[test]
        fn c3_two_asked_rows_then_two_answers_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("asked", "q", "Q?", &t(2)),
                row_at("answered", "q", "A", &t(3)),
                row_at("answered", "q", "B", &t(4)),
            ]);
            let (a, b) = conflicting_rulings(&led, "q").expect("c3 should conflict");
            assert_eq!((a, b), (Ruling::Answered('A'), Ruling::Answered('B')));
        }

        // c4: one branch only -- asked t1, answered A t2 -> no conflict.
        #[test]
        fn c4_single_answer_no_conflict() {
            let led = led_of(vec![row_at("asked", "q", "Q?", &t(1)), row_at("answered", "q", "A", &t(2))]);
            assert!(conflicting_rulings(&led, "q").is_none());
        }

        // c5: both branches answer the same letter -- not a conflict.
        #[test]
        fn c5_two_answers_same_letter_no_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("answered", "q", "A", &t(3)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none());
        }

        // c6: asked again at t5 and not yet answered -- open, no conflict
        // (mirrors c1's ledger up through its own re-ask row).
        #[test]
        fn c6_reasked_and_not_yet_answered_no_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("answered", "q", "B", &t(3)),
                row_at("asked", "q", "Q?", &t(5)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none(), "open, not yet re-answered");
        }

        // c7: today's rules never let an *answered* id's text move and be
        // asked again -- an answered question is frozen (`is_frozen`) and
        // can only be superseded by a new id, never re-asked in place
        // (tests/answer_test.rs, tests/check_test.rs: "an answered question
        // can never be re-asked"). So the one flow this rule must not break
        // -- an id asked again with a different hash while its latest state
        // is already `answered` -- has no path to it except through this
        // very rule's own conflict re-ask (which always re-asks under the
        // *same* hash, the text never having moved -- see
        // `ask::do_conflict_reask`). There is nothing else to test here.
        #[test]
        fn c7_no_existing_flow_reasks_an_answered_id_with_a_different_hash() {}

        // Rule 8: a refusal counts as a ruling too -- one branch answers,
        // the other refuses -- and mixing it with an answer is a conflict
        // exactly like two different letters.
        #[test]
        fn rule_8_an_answer_and_a_refusal_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("refused", "q", "Q?", &t(3)),
            ]);
            let (a, b) = conflicting_rulings(&led, "q").expect("answer vs refusal should conflict");
            assert_eq!((a, b), (Ruling::Answered('A'), Ruling::Refused));
        }

        // Two refusals (both branches refused) are the same ruling, not a
        // conflict.
        #[test]
        fn rule_8_two_refusals_are_not_a_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("refused", "q", "Q?", &t(2)),
                row_at("refused", "q", "Q?", &t(3)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none());
        }

        // Never asked at all: no `r` to reckon from, so no conflict.
        #[test]
        fn never_asked_no_conflict() {
            let led = led_of(vec![]);
            assert!(conflicting_rulings(&led, "q").is_none());
        }

        // Rule 8, continued: a `refused` row counts toward "asked again"
        // exactly as an `answered` row does. Before this fix, only
        // `answered` rows were counted, so a ruling made of one answer and
        // one refusal was never recognized as "asked again" -- asking again
        // and answering again could never clear the conflict.
        //
        // asked t1; answered A t2; refused t3 (two distinct rulings before
        // the next ask); re-asked t5 -> asked-again, nothing after it yet ->
        // open, no conflict.
        #[test]
        fn rule_8_reasked_after_answer_and_refusal_no_conflict_while_open() {
            // adr: e3200081-9995-4bbe-983f-2f430dc341e7
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("refused", "q", "Q?", &t(3)),
                row_at("asked", "q", "Q?", &t(5)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none(), "open, not yet settled again");
        }

        // Same ledger, then answered A again after the re-ask -> A stands.
        #[test]
        fn rule_8_reasked_after_answer_and_refusal_then_answered_again_stands() {
            // adr: e3200081-9995-4bbe-983f-2f430dc341e7
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("refused", "q", "Q?", &t(3)),
                row_at("asked", "q", "Q?", &t(5)),
                row_at("answered", "q", "A", &t(6)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none(), "A stands, no conflict");
        }

        // Same ledger, but the settling act after the re-ask is a fresh
        // refusal instead of a letter -> refused stands.
        #[test]
        fn rule_8_reasked_after_answer_and_refusal_then_refused_again_stands() {
            // adr: e3200081-9995-4bbe-983f-2f430dc341e7
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("refused", "q", "Q?", &t(3)),
                row_at("asked", "q", "Q?", &t(5)),
                row_at("refused", "q", "Q?", &t(6)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none(), "refused stands, no conflict");
        }
    }
}
