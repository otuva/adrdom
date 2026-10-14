//! One place to resolve a `depends:` entry (per "What does a depends entry
//! name?"): `approve`'s validation,
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
use crate::status;
use crate::terms::Term;
use std::collections::HashMap;

/// What a `depends:` entry resolves to: a heading in the same file (its
/// index into `rf.sections`), a current (non-retired) term (its id), or a
/// current decision of another topic file (its id): the record of what a
/// closed question stood on, never a target a question being judged may name.
pub enum DepTarget {
    Heading(usize),
    Term(String),
    Decision(String),
    /// The identifier of a superseded question of this file whose section
    /// is struck (adr-code: b5ca161f-c88c-4f02-a9d9-1d406cf3a252): the
    /// ledger alone holds it, and the entry resolves as it did when the
    /// section stood in the file.
    Struck(String),
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
    /// An `Id` entry naming a current decision of another topic file, on a
    /// question being judged (`status::open_to_ask_text`): a question stands
    /// on terms and questions of its own file, so the entry resolves to
    /// nothing.
    OtherFile(String),
}

/// Resolves one `depends:` entry, written by `sec`, against `rf` (for
/// same-file heading lookup), `terms_list`, `phrase_lookup`
/// (`terms::phrase_lookup(terms_list)`, built once per command run by the
/// caller and reused across every `depends:` entry in every file --
/// rebuilding it per entry was the expensive path) and `led` (per "What does
/// a depends entry name?": a term, or a question of the same topic file).
///
/// `Id` entries: a heading id in `rf` first, else a non-retired term id in
/// `terms_list`, else a current decision by the ledger; anything else
/// refuses as `NotCurrentDecision`. A current decision found only in another
/// topic file is `OtherFile` on a question being judged
/// (`status::open_to_ask_text`, the moment the blame line is judged too), and
/// `Decision` on any other: an answered, asked-and-unchanged, refused,
/// superseded or withdrawn question keeps the record of what it stood on. An
/// `Id` that names a *retired* term is allowed instead of refused when `sec`
/// itself is no longer open (`state::is_dead`: refused, superseded, or
/// withdrawn) -- that question is closed one way or another, so a depends
/// line frozen back when the term was still current is a record of what it
/// stood on then, not a dangling reference to re-plan. An open (unasked or
/// asked-unanswered) or answered-current `sec` still refuses on it: an open
/// question must be re-planned onto a live term, and an answered-current one
/// naming a retired term should never happen at all (retiring a term a
/// current decision depends on is refused up front, at `approve
/// adrdom/terms.md`).
///
/// `Text` entries: heading-shaped (trimmed text ends in `?`) look up a
/// same-file heading by its exact text: `Ok(Some(Heading(_)))` when found,
/// `Ok(None)` when not (never a refusal here: a dangling/cycle/self-dep
/// heading reference is `derive_tiers`'s single error, reported once from
/// `RecordFile::derive_error` rather than per entry). Anything not
/// heading-shaped must resolve to exactly one current term via a lookup in
/// `phrase_lookup` -- the whole trimmed entry, case-insensitive -- which skips
/// retired terms, not the scanning `terms::resolve`, so a phrase that only
/// contains a term's name (rather than naming it outright) does not resolve
/// here.
pub fn resolve_dep(
    d: &Dep,
    sec: &Section,
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
            if let Some(term) = terms_list.iter().find(|t| t.id.as_deref() == Some(id.as_str())) {
                if !term.retired || state::is_dead(led, sec) {
                    return Ok(Some(DepTarget::Term(id.clone())));
                }
                return Err(DepError::RetiredTermId(id.clone()));
            }
            if state::is_struck_from(led, id, &rf.rel) {
                return Ok(Some(DepTarget::Struck(id.clone())));
            }
            if state::is_current(led, id) {
                if status::open_to_ask_text(led, rf, sec) {
                    return Err(DepError::OtherFile(id.clone()));
                }
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
/// own prefix, e.g. `approve: `).
///
/// `name` is what the identifier of a `NotCurrentDecision` or `OtherFile`
/// names, when the caller can tell: a term's name, or the heading of a
/// question in any state.
pub fn dep_refusal_text(rel: &str, heading: &str, err: &DepError, name: Option<&str>) -> msg::Said {
    match err {
        DepError::NotCurrentDecision(id) => msg::dep_not_current_decision(rel, heading, id, name),
        DepError::OtherFile(id) => msg::dep_names_decision_of_other_file(rel, heading, id, name),
        DepError::RetiredTermId(id) => msg::dep_retired_term_id(rel, heading, id, name),
        DepError::NoTerm(text) => msg::dep_no_term(rel, heading, text),
        DepError::RetiredTerm(text) => msg::dep_retired_term(rel, heading, text),
    }
}

/// Every current decision's heading text, across every topic file in
/// `records`, paired with the file (`rf.rel`) and id it belongs to
/// (several entries share a heading text only when two current decisions
/// happen to carry the exact same one, in different files -- `check`'s
/// cross-file duplicate-heading check flags that separately). Built once
/// per command run and passed to its callers, rather than rebuilt per
/// entry or per file. It serves the `supersedes:` and `unrelated:` lines
/// (an entry may name a current decision of another file by its heading)
/// and the depends finding in `rules::derive_finding`, which uses it to
/// tell a heading entry that names a decision of another file from one
/// that names nothing. A `depends:` entry is resolved against the file and
/// the terms alone. Takes already-loaded records rather than a repo root,
/// so a caller that has every topic file in memory does not read them again.
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

/// Why a `supersedes:` entry names no decision to replace.
pub enum SupersedesError {
    /// It names no current decision, by identifier or by heading.
    NotCurrent,
    /// Its heading is the heading of more than one current decision; the
    /// (sorted) topic files they are in.
    Ambiguous(Vec<String>),
}

/// The identifier of the current decision a `supersedes:` entry names (per
/// "What does a reference in a topic file carry?": the agent writes the name, the tool turns it
/// into the identifier). An entry is the identifier of a current decision, or the heading of
/// exactly one, in this topic file or another; `cross_file_headings` is `cross_file_heading_index`,
/// every current decision by its heading.
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
/// the file exactly as loaded.
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
