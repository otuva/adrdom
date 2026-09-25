//! One place to resolve a `depends:` entry (per "What does a depends line
//! name?"): `approve`'s validation,
//! `approve`'s rewrite, and `check`'s `plan` check all read the same target
//! by the same rules and print the same refusal text, instead of each
//! carrying its own copy of "what does this entry point to". `record.rs`
//! deliberately has no ledger/terms access (see `Dep`'s own doc comment),
//! so this lives alongside `state.rs`'s other ledger-aware helpers rather
//! than there.
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, Dep, RecordFile, Section};
use crate::state;
use crate::terms::Term;
use std::collections::HashMap;

/// What a `depends:` entry resolves to: a heading in the same file (its
/// index into `rf.sections`), a current (non-retired) term (its id), or a
/// current decision elsewhere (its id).
pub enum DepTarget {
    Heading(usize),
    Term(String),
    Decision(String),
}

/// Why a `depends:` entry did not resolve, carrying just what
/// `dep_refusal_text` needs to fill in the message.
pub enum DepError {
    /// An `Id` entry naming nothing this file/terms/ledger knows as
    /// current: neither a heading id in this file, a non-retired term id,
    /// nor a current decision.
    NotCurrentDecision(String),
    /// A term-shaped `Text` entry (not heading-shaped, i.e. not ending in
    /// `?`) that names no current term.
    NoTerm(String),
    /// A heading-shaped `Text` entry naming no heading in its own file, but
    /// a current decision's heading text in more than one *other* topic
    /// file (the gap "Does a reference in a topic file carry the identifier
    /// of what it names?" left open): which one it means is ambiguous, so
    /// this refuses rather than silently picking one. Carries the entry's
    /// own text and the (sorted) relative paths of every file it matched.
    AmbiguousHeading { text: String, files: Vec<String> },
}

/// Resolves one `depends:` entry, written by `sec`, against `rf` (for
/// same-file heading lookup), `terms_list`, `phrase_lookup`
/// (`terms::phrase_lookup(terms_list)`, built once per command run by the
/// caller and reused across every `depends:` entry in every file --
/// rebuilding it per entry was the expensive path), `cross_file_headings`
/// (`cross_file_heading_index(root, led)`, likewise built once per command
/// run: every *current* decision's heading text, repo-wide, paired with its
/// file and id), and `led`.
///
/// `Id` entries: a heading id in `rf` first, else a non-retired term id in
/// `terms_list`, else a current decision by the ledger; anything else
/// refuses as `NotCurrentDecision`. An `Id` that names a *retired* term is
/// allowed instead of refused when `sec` itself is no longer open
/// (`state::is_dead`: refused, superseded, or withdrawn) -- that question
/// is closed one way or another, so a depends line frozen back when the
/// term was still current is a record of what it stood on then, not a
/// dangling reference to re-plan. An open (unasked or asked-unanswered)
/// or answered-current `sec` still refuses on it: an open question must be
/// re-planned onto a live term, and an answered-current one naming a
/// retired term should never happen at all (retiring a term a current
/// decision depends on is refused up front, at `approve adrdom/terms.md`).
///
/// `Text` entries: heading-shaped (trimmed text ends in `?`) look up a
/// same-file heading by its exact text first -- `Ok(Some(Heading(_)))`
/// when found. When it names no heading in `rf` itself, `cross_file_headings`
/// is checked next (the cross-file gap "Does a reference in a topic file
/// carry the identifier of what it names?" left open: a reference still
/// carries the identifier of what it names even when that's a decision in
/// another topic file, so
/// the agent must be able to write that decision's heading text the same
/// way it writes a same-file one): exactly one current decision elsewhere
/// under that heading resolves to `Ok(Some(Decision(id)))`; more than one
/// refuses as `AmbiguousHeading` rather than guessing; none at all is
/// `Ok(None)` (never a refusal here: a dangling/cycle/self-dep heading
/// reference is `derive_tiers`'s single error, reported once from
/// `RecordFile::derive_error` rather than per entry -- unchanged from
/// before this fix). Anything not heading-shaped must resolve to exactly
/// one current term via a lookup in `phrase_lookup` -- the whole trimmed
/// entry, case-insensitive -- which skips retired terms, not the scanning
/// `terms::resolve`, so a phrase that only contains a term's name (rather
/// than naming it outright) no longer resolves here.
pub fn resolve_dep(
    d: &Dep,
    sec: &Section,
    rf: &RecordFile,
    terms_list: &[Term],
    phrase_lookup: &HashMap<String, String>,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> Result<Option<DepTarget>, DepError> {
    match d {
        Dep::Id { id, .. } => {
            if let Some(idx) = rf
                .sections
                .iter()
                .position(|s| s.id.as_deref() == Some(id.as_str()))
            {
                return Ok(Some(DepTarget::Heading(idx)));
            }
            if let Some(term) = terms_list.iter().find(|t| t.id.as_deref() == Some(id.as_str())) {
                if !term.retired || state::is_dead(led, sec) {
                    return Ok(Some(DepTarget::Term(id.clone())));
                }
                return Err(DepError::NotCurrentDecision(id.clone()));
            }
            if state::is_current(led, id) {
                return Ok(Some(DepTarget::Decision(id.clone())));
            }
            Err(DepError::NotCurrentDecision(id.clone()))
        }
        Dep::Text { text, .. } => {
            let t = text.trim();
            if t.ends_with('?') {
                if let Some(idx) = rf.sections.iter().position(|s| s.heading_text.trim() == t) {
                    return Ok(Some(DepTarget::Heading(idx)));
                }
                let matches: &[(String, String)] = cross_file_headings
                    .get(t)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                return match matches {
                    [] => Ok(None),
                    [(_, id)] => Ok(Some(DepTarget::Decision(id.clone()))),
                    many => {
                        let mut files: Vec<String> = many.iter().map(|(rel, _)| rel.clone()).collect();
                        files.sort();
                        files.dedup();
                        Err(DepError::AmbiguousHeading {
                            text: t.to_string(),
                            files,
                        })
                    }
                };
            }
            match phrase_lookup.get(&t.to_lowercase()) {
                Some(id) => Ok(Some(DepTarget::Term(id.clone()))),
                None => Err(DepError::NoTerm(t.to_string())),
            }
        }
    }
}

/// The refusal text for a `resolve_dep` failure, `rel: "<heading>" ...`,
/// the same wording `approve` and `check` already used (callers add their
/// own prefix, e.g. `approve: `).
pub fn dep_refusal_text(rel: &str, heading: &str, err: &DepError) -> String {
    match err {
        DepError::NotCurrentDecision(id) => msg::dep_not_current_decision(rel, heading, id),
        DepError::NoTerm(text) => msg::dep_no_term(rel, heading, text),
        DepError::AmbiguousHeading { text, files } => {
            msg::dep_ambiguous_heading(rel, heading, text, files)
        }
    }
}

/// Every current decision's heading text, across every topic file in
/// `records`, paired with the file (`rf.rel`) and id it belongs to
/// (several entries share a heading text only when two current decisions
/// happen to carry the exact same one, in different files -- `check`'s
/// cross-file duplicate-heading check flags that separately). Built once
/// per command run, like `terms::phrase_lookup`, and passed to every
/// `resolve_dep` call and to `normalize_cross_file_deps` in that run,
/// rather than rebuilt per entry or per file. Takes already-loaded records
/// rather than a repo root: every caller either already has every topic
/// file loaded for its own purposes (`check`, and `approve`'s
/// `missing_answered_refusals` pass) or is the one place left that has to
/// load them for this alone (`ask`, `show`, the page's `snapshot`) --
/// either way, this used to re-read every file off disk itself regardless,
/// doubling that read in the callers that already had it in memory.
pub fn cross_file_heading_index(records: &[RecordFile], led: &Ledger) -> HashMap<String, Vec<(String, String)>> {
    let mut out: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !state::is_current(led, id) {
                continue;
            }
            out.entry(sec.heading_text.trim().to_string())
                .or_default()
                .push((rf.rel.clone(), id.clone()));
        }
    }
    out
}

/// Rewrites, in memory only, every heading-shaped `Text` entry in `rf`
/// that names no heading in `rf` itself but resolves -- via
/// `cross_file_headings` -- to exactly one current decision elsewhere, to
/// an `Id` entry naming that decision (the same substitution `approve`
/// will later write to disk via its own rewrite pass, done here early
/// enough that tier derivation sees it too): `record::derive_tiers` never
/// treats an `Id` entry that names no heading in `rf` as dangling (it may
/// legitimately resolve outside this file, exactly like a term or decision
/// reference already did before this fix), so a legitimate cross-file
/// heading reference no longer trips `RecordFile::derive_error` the way an
/// unresolved `Text` one always has. `rf.tiers`/`rf.derive_error` are
/// recomputed after any substitution, so every later read of them (the
/// tier cap, the "plan" category, the letter-set check) sees the corrected
/// picture. An entry naming no heading anywhere is left untouched --
/// still dangling, exactly as before this fix. A frozen section's entry
/// (`state::is_frozen`) is left untouched too, whatever it resolves to:
/// that section's decision is already closed, so an edited depends line
/// there is `record_refusals`'s frozen-edit refusal to catch, on the
/// unrewritten `Text` entry it expects -- substituting it here first would
/// turn it into an `Id` entry before that check ever runs and the edit
/// would slip through unrefused. An entry naming a current decision in
/// more than one other file is also substituted -- to the first of those
/// decisions, arbitrarily -- so this entry, too, stops looking like a
/// same-file dangling reference to `derive_tiers`; which one gets picked
/// never matters, since every caller that reads `substitutions` also reads
/// the `AmbiguousHeading` refusal this returns for the same entry and
/// refuses the whole file on it before writing anything. Its
/// `AmbiguousHeading` refusal text is returned, one line per such entry,
/// for the caller to fold into its own refusal list the same way it folds
/// in every other `dep_refusal_text` line.
pub fn normalize_cross_file_deps(
    rel: &str,
    rf: &mut RecordFile,
    terms_list: &[Term],
    phrase_lookup: &HashMap<String, String>,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> Vec<String> {
    let mut ambiguous: Vec<String> = Vec::new();
    let mut substitutions: Vec<(usize, usize, String)> = Vec::new();

    for (si, sec) in rf.sections.iter().enumerate() {
        if state::is_frozen(led, sec) {
            continue;
        }
        for (di, d) in sec.depends.iter().enumerate() {
            let Dep::Text { text, .. } = d else { continue };
            if !text.trim().ends_with('?') {
                continue;
            }
            if rf.dep_target(d).is_some() {
                // Same-file heading wins; nothing for this pass to do.
                continue;
            }
            match resolve_dep(d, sec, rf, terms_list, phrase_lookup, cross_file_headings, led) {
                Ok(Some(DepTarget::Decision(id))) => substitutions.push((si, di, id)),
                Ok(_) => {}
                Err(e @ DepError::AmbiguousHeading { .. }) => {
                    ambiguous.push(dep_refusal_text(rel, &sec.heading_text, &e));
                    // Substituted too (see the doc comment above): picking
                    // the first of `cross_file_headings`'s matches for this
                    // text is always possible here, since `resolve_dep`
                    // only returns `AmbiguousHeading` when that lookup has
                    // more than one entry.
                    if let Some((_, id)) = cross_file_headings.get(text.trim()).and_then(|m| m.first()) {
                        substitutions.push((si, di, id.clone()));
                    }
                }
                Err(_) => {}
            }
        }
    }

    if substitutions.is_empty() {
        return ambiguous;
    }

    for (si, di, id) in substitutions {
        if let Dep::Text { set, .. } = &rf.sections[si].depends[di] {
            let set = set.clone();
            rf.sections[si].depends[di] = Dep::Id { id, set };
        }
    }
    match record::derive_tiers(rf) {
        Ok(t) => {
            rf.tiers = Some(t);
            rf.derive_error = None;
        }
        Err(e) => {
            rf.tiers = None;
            rf.derive_error = Some(e);
        }
    }

    ambiguous
}

/// One `bad_option_letter` message per letter in `letters` that isn't
/// one of `target`'s options (per "Does a depends entry carry the answers
/// it holds under?"); empty when `target` has no options
/// yet (an unopened tier's letters go unchecked). The one place a heading
/// dep's answer-set letters are checked against its target, used by
/// `check`'s `plan` check for both an `Id` entry and a heading-shaped
/// `Text` entry.
pub fn bad_letters(
    rel: &str,
    dependent_heading: &str,
    target: &Section,
    letters: &[char],
) -> Vec<String> {
    if target.options.is_empty() {
        return Vec::new();
    }
    letters
        .iter()
        .filter(|l| !target.options.iter().any(|o| o.letter == **l))
        .map(|l| msg::bad_option_letter(rel, dependent_heading, &target.heading_text, *l))
        .collect()
}
