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
/// own prefix, e.g. `approve: `). `DepError::AmbiguousHeading` never
/// reaches here: `resolve_dep` only returns it for a heading-shaped `Text`
/// entry (trimmed text ending in `?`), and `record_refusals` -- the one
/// caller of this function -- always resolves that shape of entry through
/// `rf.derive_error` (via `normalize_cross_file_deps`) instead, skipping it
/// before ever calling `resolve_dep` for the purpose of a per-entry
/// message.
///
/// `name` is what the identifier of a `NotCurrentDecision` names, when the
/// caller can tell: a term's name, or the heading of a question in any
/// state.
pub fn dep_refusal_text(rel: &str, heading: &str, err: &DepError, name: Option<&str>) -> String {
    match err {
        DepError::NotCurrentDecision(id) => msg::dep_not_current_decision(rel, heading, id, name),
        DepError::NoTerm(text) => msg::dep_no_term(rel, heading, text),
        DepError::AmbiguousHeading { .. } => {
            unreachable!("a heading-shaped entry never reaches dep_refusal_text; see the doc comment above")
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
    heading_index(records, Some(led))
}

/// The traversal `cross_file_heading_index` (current decisions only, via
/// `Some(led)`) and `blame::any_headings_with_id` (any state at all, via
/// `None`) both build from: every heading text carrying an id, across
/// every topic file in `records`, paired with its file and id, filtered to
/// current decisions only when a ledger is given.
pub fn heading_index(
    records: &[RecordFile],
    led: Option<&Ledger>,
) -> HashMap<String, Vec<(String, String)>> {
    let mut out: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if let Some(led) = led {
                if !state::is_current(led, id) {
                    continue;
                }
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
/// would slip through unrefused.
///
/// An entry naming a current decision in more than one other file is never
/// substituted (guessing which one it means is exactly the bug this
/// leaves unresolved on purpose): the first such entry found (in file
/// order) is remembered, and `derive_tiers` is still run afterward, on the
/// entries as they stand (every unambiguous substitution applied, every
/// ambiguous entry left as the unresolved `Text` it was written as) --
/// `derive_tiers` has no cross-file knowledge of its own, so it always
/// calls that unresolved entry `Dangling`. When that is the *only* problem
/// `derive_tiers` finds (`Ok`, or an `Err` that is exactly this entry's own
/// `Dangling`), the real reason is more informative and replaces it as
/// `rf.derive_error` (`DeriveError::AmbiguousHeading`). But when
/// `derive_tiers` finds a *different* same-file problem first -- a cycle, a
/// self-dep, or a dangling entry on some other heading, reached before the
/// ambiguous one in file order -- that real bug is kept instead: an
/// ambiguous cross-file reference elsewhere in the file must never hide a
/// same-file defect the agent still has to fix. `derive_error`/`tiers`
/// remain one file-wide slot each (never both a same-file error and the
/// ambiguity at once -- the design already reports at most one derive
/// error per file, same as before this fix), so every caller that already
/// reads them for any other `DeriveError` (approve and check through
/// `record_refusals`'s "plan" category, ask through its own `derive_error`
/// check, show and the page through `rf.tiers` being `None`) sees whichever
/// one is chosen the same way, without needing a side list of its own to
/// remember to fold in -- the gap that let `show`/the page silently print a
/// guessed "stands on" line for the ambiguous case before this fix.
pub fn normalize_cross_file_deps(
    rf: &mut RecordFile,
    terms_list: &[Term],
    phrase_lookup: &HashMap<String, String>,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) {
    let mut substitutions: Vec<(usize, usize, String)> = Vec::new();
    let mut ambiguous: Option<record::DeriveError> = None;

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
                Err(DepError::AmbiguousHeading { text: entry, files }) => {
                    if ambiguous.is_none() {
                        ambiguous = Some(record::DeriveError::AmbiguousHeading {
                            section: sec.heading_text.clone(),
                            entry,
                            files,
                        });
                    }
                }
                Err(_) => {}
            }
        }
    }

    for (si, di, id) in substitutions {
        if let Dep::Text { set, .. } = &rf.sections[si].depends[di] {
            let set = set.clone();
            rf.sections[si].depends[di] = Dep::Id { id, set };
        }
    }

    let Some(ambiguous_err) = ambiguous else {
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
        return;
    };

    // `ambiguous` is only ever set from this variant, above.
    let record::DeriveError::AmbiguousHeading {
        section: a_section,
        entry: a_entry,
        ..
    } = &ambiguous_err
    else {
        unreachable!("ambiguous is only ever set to DeriveError::AmbiguousHeading")
    };
    let (a_section, a_entry) = (a_section.clone(), a_entry.clone());

    rf.tiers = None;
    rf.derive_error = Some(match record::derive_tiers(rf) {
        // Nothing else wrong: the ambiguity is the file's one problem.
        Ok(_) => ambiguous_err,
        // The artifact `Dangling` the ambiguous entry always earns on its
        // own (it names no heading in `rf`, by construction -- that is
        // exactly what made it a cross-file candidate): the ambiguity is
        // still the more informative reason.
        Err(record::DeriveError::Dangling { section, entry })
            if section == a_section && entry == a_entry =>
        {
            ambiguous_err
        }
        // A different same-file problem -- a cycle, a self-dep, or a
        // dangling entry on some other heading, reached before the
        // ambiguous one in file order -- is a real bug and must not be
        // hidden behind the ambiguity.
        Err(e) => e,
    });
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

/// `bad_letters`'s cross-file counterpart: the same check, for a depends
/// entry that resolves to a current decision elsewhere (`DepTarget::Decision`)
/// rather than a same-file heading -- `bad_letters` alone can't reach it,
/// since its `target: &Section` always comes out of the same `RecordFile`.
/// `records` is every topic file in the repo (already loaded by the
/// caller, the same shape `cross_file_heading_index`'s callers already
/// have); empty when `id` names no section anywhere in it (should not
/// happen once it has resolved as a current decision, but never assumed).
pub fn bad_letters_for_decision(
    rel: &str,
    dependent_heading: &str,
    records: &[RecordFile],
    id: &str,
    letters: &[char],
) -> Vec<String> {
    let Some(target) = records
        .iter()
        .find_map(|rf| rf.sections.iter().find(|s| s.id.as_deref() == Some(id)))
    else {
        return Vec::new();
    };
    bad_letters(rel, dependent_heading, target, letters)
}
