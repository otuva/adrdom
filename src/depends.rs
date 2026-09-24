//! One place to resolve a `depends:` entry (9.1 A): `approve`'s validation,
//! `approve`'s rewrite, and `check`'s `plan` check all read the same target
//! by the same rules and print the same refusal text, instead of each
//! carrying its own copy of "what does this entry point to". `record.rs`
//! deliberately has no ledger/terms access (see `Dep`'s own doc comment),
//! so this lives alongside `state.rs`'s other ledger-aware helpers rather
//! than there.
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{Dep, RecordFile, Section};
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
}

/// Resolves one `depends:` entry against `rf` (for same-file heading
/// lookup), `terms_list`, `phrase_lookup` (`terms::phrase_lookup(terms_list)`,
/// built once per command run by the caller and reused across every
/// `depends:` entry in every file -- rebuilding it per entry was the
/// expensive path), and `led`.
///
/// `Id` entries: a heading id in `rf` first, else a non-retired term id in
/// `terms_list`, else a current decision by the ledger; anything else
/// refuses as `NotCurrentDecision`.
///
/// `Text` entries: heading-shaped (trimmed text ends in `?`) look up a
/// same-file heading by its exact text -- `Ok(Some(Heading(_)))` when
/// found, `Ok(None)` when not (never a refusal here: a dangling/cycle/
/// self-dep heading reference is `derive_tiers`'s single error, reported
/// once from `RecordFile::derive_error` rather than per entry). Anything
/// else must resolve to exactly one current term via a lookup in
/// `phrase_lookup` -- the whole trimmed entry, case-insensitive -- which
/// skips retired terms, not the scanning `terms::resolve`, so a phrase
/// that only contains a term's name (rather than naming it outright) no
/// longer resolves here.
pub fn resolve_dep(
    d: &Dep,
    rf: &RecordFile,
    terms_list: &[Term],
    phrase_lookup: &HashMap<String, String>,
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
            if terms_list
                .iter()
                .any(|t| t.id.as_deref() == Some(id.as_str()) && !t.retired)
            {
                return Ok(Some(DepTarget::Term(id.clone())));
            }
            if state::is_current(led, id) {
                return Ok(Some(DepTarget::Decision(id.clone())));
            }
            Err(DepError::NotCurrentDecision(id.clone()))
        }
        Dep::Text { text, .. } => {
            let t = text.trim();
            if t.ends_with('?') {
                return Ok(rf
                    .sections
                    .iter()
                    .position(|s| s.heading_text.trim() == t)
                    .map(DepTarget::Heading));
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
    }
}

/// One `bad_option_letter` message per letter in `letters` that isn't
/// one of `target`'s options (8.7 A); empty when `target` has no options
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
