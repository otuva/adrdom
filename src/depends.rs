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
    /// An `Id` entry naming a retired term, on a live section.
    RetiredTermId(String),
    /// A term-shaped `Text` entry (not heading-shaped, i.e. not ending in
    /// `?`) that names no current term.
    NoTerm(String),
    /// A term-shaped `Text` entry naming a retired term's name or alias
    /// and no current term.
    RetiredTerm(String),
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
                return Err(DepError::RetiredTermId(id.clone()));
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
                None => {
                    let retired = crate::terms::is_retired_phrase(terms_list, t);
                    if retired {
                        Err(DepError::RetiredTerm(t.to_string()))
                    } else {
                        Err(DepError::NoTerm(t.to_string()))
                    }
                }
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
pub fn dep_refusal_text(rel: &str, heading: &str, err: &DepError, name: Option<&str>) -> msg::Said {
    match err {
        DepError::NotCurrentDecision(id) => msg::dep_not_current_decision(rel, heading, id, name),
        DepError::RetiredTermId(id) => msg::dep_retired_term_id(rel, heading, id, name),
        DepError::NoTerm(text) => msg::dep_no_term(rel, heading, text),
        DepError::RetiredTerm(text) => msg::dep_retired_term(rel, heading, text),
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

/// Why a `supersedes:` entry names no decision to replace.
pub enum SupersedesError {
    /// It names no current decision, by identifier or by heading.
    NotCurrent,
    /// Its heading is the heading of more than one current decision; the
    /// (sorted) topic files they are in.
    Ambiguous(Vec<String>),
}

/// The identifier of the current decision a `supersedes:` entry names (per
/// "Does a reference in a topic file carry the identifier of what it
/// names?": the agent writes the name, the tool turns it into the
/// identifier). An entry is the identifier of a current decision, or the
/// heading of exactly one, in this topic file or another;
/// `cross_file_headings` is `cross_file_heading_index`, every current
/// decision by its heading.
pub fn resolve_supersedes(
    entry: &str,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> Result<String, SupersedesError> {
    let entry = entry.trim();
    if state::is_current(led, entry) {
        return Ok(entry.to_string());
    }
    let matches: &[(String, String)] = cross_file_headings
        .get(entry)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    match matches {
        [] => Err(SupersedesError::NotCurrent),
        [(_, id)] => Ok(id.clone()),
        many => {
            let mut files: Vec<String> = many.iter().map(|(rel, _)| rel.clone()).collect();
            files.sort();
            files.dedup();
            Err(SupersedesError::Ambiguous(files))
        }
    }
}

/// The entries of a `supersedes:` line, from the pieces a split at every
/// comma gave. A heading may hold a comma itself: pieces are joined again,
/// the longest run first, while the joined text names a current decision
/// (one or several), the same way a `depends:` line's pieces are joined
/// against the headings of its file. A run that names none stays one entry
/// for each piece.
pub fn supersedes_entries(
    pieces: &[String],
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < pieces.len() {
        let mut chosen = i + 1;
        for j in (i + 2..=pieces.len()).rev() {
            let joined = pieces[i..j].join(", ");
            if !matches!(
                resolve_supersedes(&joined, cross_file_headings, led),
                Err(SupersedesError::NotCurrent)
            ) {
                chosen = j;
                break;
            }
        }
        out.push(pieces[i..chosen].join(", "));
        i = chosen;
    }
    out
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
) -> Vec<msg::Said> {
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
) -> Vec<msg::Said> {
    let Some(target) = records
        .iter()
        .find_map(|rf| rf.sections.iter().find(|s| s.id.as_deref() == Some(id)))
    else {
        return Vec::new();
    };
    bad_letters(rel, dependent_heading, target, letters)
}

/// The current decisions standing on `target` (an id), through the current
/// decisions standing on those: `(rel, heading, id)`, sorted. A current
/// decision stands on another when a `depends:` entry of it is that
/// decision's id, or -- as a same-file heading written as text -- its
/// heading. Questions not answered, refused, superseded or withdrawn, and
/// headings with no body, are not part of it.
pub fn standing_on(records: &[RecordFile], led: &Ledger, target: &str) -> Vec<(String, String, String)> {
    let mut found: Vec<(String, String, String)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    seen.insert(target.to_string());
    let mut queue: Vec<String> = vec![target.to_string()];
    while let Some(cur) = queue.pop() {
        let cur_heading = records
            .iter()
            .flat_map(|rf| rf.sections.iter().map(move |s| (rf.rel.as_str(), s)))
            .find(|(_, s)| s.id.as_deref() == Some(cur.as_str()))
            .map(|(rel, s)| (rel.to_string(), s.heading_text.trim().to_string()));
        for rf in records {
            for sec in &rf.sections {
                let Some(id) = &sec.id else { continue };
                if seen.contains(id) || !state::is_current(led, id) {
                    continue;
                }
                let at = cur_heading.as_ref().map(|(rel, h)| (rel.as_str(), h.as_str()));
                let stands = sec.depends.iter().any(|d| entry_names_target(d, &rf.rel, &cur, at));
                if stands {
                    seen.insert(id.clone());
                    queue.push(id.clone());
                    found.push((rf.rel.clone(), sec.heading_text.trim().to_string(), id.clone()));
                }
            }
        }
    }
    found.sort();
    found
}

/// True when the `depends:` entry `d`, of a section in the file `holder_rel`,
/// names the decision `target_id`: an id entry that is that id, or a
/// heading written as text that is the heading of the target, which is
/// `target_at` (its file's `rel` and trimmed heading) and must sit in the
/// same file as the entry.
pub fn entry_names_target(
    d: &Dep,
    holder_rel: &str,
    target_id: &str,
    target_at: Option<(&str, &str)>,
) -> bool {
    match d {
        Dep::Id { id, .. } => id == target_id,
        Dep::Text { text, .. } => {
            target_at.is_some_and(|(rel, heading)| rel == holder_rel && text.trim() == heading)
        }
    }
}

/// What a `supersedes:` line over `ids` leaves out: every decision standing
/// on one of them (`standing_on`) that is not itself on the line.
pub fn left_out_of_supersedes(
    records: &[RecordFile],
    led: &Ledger,
    ids: &[String],
) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = Vec::new();
    for id in ids {
        for t in standing_on(records, led, id) {
            if !ids.contains(&t.2) && !out.iter().any(|o| o.2 == t.2) {
                out.push(t);
            }
        }
    }
    out.sort();
    out
}

/// The `rel` of every topic file in `all` whose plan is approved, judged on
/// the file exactly as loaded. `all` must not have gone through
/// `normalize_cross_file_deps` (that rewrites the entries the plan hash
/// covers); a caller that normalizes its records takes the hashes before
/// doing so and calls `state::plan_approved_hashes` itself.
pub fn approved_plan_rels(all: &[RecordFile], led: &Ledger) -> std::collections::HashSet<String> {
    all.iter()
        .filter(|f| state::plan_approved(led, f, &f.rel))
        .map(|f| f.rel.clone())
        .collect()
}

/// The ids named on the `supersedes:` lines of the live, unanswered
/// questions of the files in `all` whose `rel` is in `approved`. Built once
/// per command run; `replaced_ids` adds the file under judgment.
pub fn replaced_base(
    all: &[RecordFile],
    approved: &std::collections::HashSet<String>,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for file in all.iter().filter(|f| approved.contains(&f.rel)) {
        collect_replaced(file, cross_file_headings, led, &mut out);
    }
    out
}

/// The ids of the decisions named on the `supersedes:` line of a question
/// that is live and not answered (no body, not asked, or asked), in every
/// topic file whose plan is approved (`base`, from `replaced_base`) and in
/// `rf`, the file under judgment. An entry is an id, or a heading of a
/// current decision (an entry that resolves to none is left out: the
/// not-current refusal reports it).
pub fn replaced_ids(
    base: &std::collections::HashSet<String>,
    rf: &RecordFile,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> std::collections::HashSet<String> {
    let mut out = base.clone();
    collect_replaced(rf, cross_file_headings, led, &mut out);
    out
}

fn collect_replaced(
    file: &RecordFile,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
    out: &mut std::collections::HashSet<String>,
) {
    for sec in &file.sections {
        if sec.supersedes.is_empty() || state::is_dead(led, sec) {
            continue;
        }
        if sec.id.as_deref().is_some_and(|id| state::is_current(led, id)) {
            continue;
        }
        for entry in supersedes_entries(&sec.supersedes, cross_file_headings, led) {
            if let Ok(id) = resolve_supersedes(&entry, cross_file_headings, led) {
                out.insert(id);
            }
        }
    }
}
