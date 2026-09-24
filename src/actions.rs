//! One code path for every owner act: the CLI and the page both call these (7.2 A).
use crate::depends::{self, DepTarget};
use crate::ledger::{self, Ledger};
use crate::msg::{self, Next};
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state::{self, TermsLoad};
use crate::tells;
use crate::terms;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;

pub struct Outcome {
    pub summary: String,
}

/// One refusal a topic file earns, tagged with the `check` category under
/// which the identical text is filed (`parse`, `tells`, `plan`, `ids`):
/// `approve` prefixes each with `approve: `, `check` with `<category>: `,
/// so the same line reads either way regardless of which one printed it
/// (E1.2, one tag richer than `terms_refusals`'s plain `Vec<String>`,
/// E1.1, since a topic file's refusals span four of `check`'s labels
/// where `adrdom/terms.md`'s all live under `terms:`).
pub struct Refusal {
    pub category: &'static str,
    pub message: String,
}

/// The terms.md gates shared by `do_answer` and `do_refuse` (5): missing,
/// present but unreadable/shape-invalid, or not approved, each refusing with
/// the same message either caller would print. `rel` is the topic file's
/// path, named in the not-approved message exactly as both callers already
/// print it. On success, the loaded, approved terms file is returned (only
/// `do_answer` has further use for it).
fn require_terms(root: &Path, led: &Ledger, rel: &str) -> Result<terms::TermsFile, String> {
    match state::load_terms_checked(root) {
        TermsLoad::Missing => Err(msg::terms_missing()),
        TermsLoad::LoadError(e) => Err(msg::terms_load_error(
            &e,
            Next::owner_runs("the same command again once adrdom/terms.md can be read"),
        )),
        TermsLoad::Ok(tf) => {
            if !tf.errors.is_empty() {
                let out = tf
                    .errors
                    .iter()
                    .map(|e| msg::terms_shape_error(e))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(out);
            }
            if !state::terms_approved(led, &tf) {
                return Err(msg::terms_not_approved(rel));
            }
            Ok(tf)
        }
    }
}

pub fn do_answer(root: &Path, id: &str, letter: char) -> Result<Outcome, String> {
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);

    let files = repo::list_decision_files(root);
    let mut found = None;
    for f in &files {
        if let Ok(rf) = record::load(f) {
            if let Some(idx) = rf.sections.iter().position(|s| s.id.as_deref() == Some(id)) {
                found = Some((rf, idx));
                break;
            }
        }
    }
    let Some((mut rf, idx)) = found else {
        return Err(msg::id_not_found(id));
    };
    let rel = state::rel_path(root, &rf.path);

    // Refuse when terms.md is missing, has shape errors, or is not
    // approved: the same messages `ask` uses, prefixed `answer:` by the
    // caller in `commands::answer::run`. The loaded, approved terms file is
    // kept for the terms-line validity check below.
    let tf = require_terms(root, &led, &rel)?;

    let cur_hash;
    let supersedes_val;
    {
        let sec = &rf.sections[idx];
        if sec.is_refused() {
            return Err(msg::already_refused(&sec.heading_text));
        }
        if sec.answer_line.is_some() {
            return Err(msg::already_answered(&sec.heading_text));
        }
        if !sec.options.iter().any(|o| o.letter == letter) {
            return Err(msg::letter_not_an_option(letter, &sec.heading_text));
        }
        cur_hash = record::section_hash(&rf.lines, sec);
        match led.latest_state(id) {
            Some(row) if row.event == "asked" && row.hash == cur_hash => {}
            _ => {
                return Err(msg::not_in_asked_state(&sec.heading_text));
            }
        }
        let Some(terms_line_idx) = sec.terms_line else {
            return Err(msg::no_terms_line(id));
        };
        let Some(pairs) = terms::parse_terms_line(rf.lines[terms_line_idx].trim_end()) else {
            return Err(msg::malformed_terms_line(id));
        };
        for (_, uuid) in &pairs {
            match tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str())) {
                None => {
                    return Err(msg::terms_line_names_gone_term(id, uuid));
                }
                Some(t) if t.retired => {
                    return Err(msg::terms_line_names_retired_term(id, &t.name));
                }
                _ => {}
            }
        }
        supersedes_val = sec.supersedes.clone();
        for old in &supersedes_val {
            // A supersedes target must be current when this question was
            // asked (E3, founding 1.3 A/4.14 A, owner note 2026-09-24): the
            // supersedes line freezes at ask, which `ask` itself now
            // enforces. Here at answer time the target may since have moved
            // to `superseded` -- accepted only when that happened after
            // this question's own latest `asked` row, so a split (several
            // newer questions each naming the same older target, asked
            // together and answered one after another) goes through for
            // every one of them. Compared by ledger order (row position),
            // never the `time` string: two rows written in the same run can
            // share a timestamp. Anything else -- never asked/answered,
            // refused, withdrawn, or superseded before this question was
            // asked -- refuses exactly as before.
            let target_ok = match led.latest_state(old) {
                Some(row) if row.event == "answered" => true,
                Some(row) if row.event == "superseded" => {
                    match (led.latest_asked_pos(id), led.latest_superseded_pos(old)) {
                        (Some(asked_pos), Some(superseded_pos)) => superseded_pos > asked_pos,
                        _ => false,
                    }
                }
                _ => false,
            };
            if !target_ok {
                return Err(msg::supersedes_not_current(old));
            }
        }
    }

    let heading_for_print;
    let sentence_for_print;
    {
        let sec = &rf.sections[idx];
        let (_, hash_end) = record::hash_range(&rf.lines, sec);
        heading_for_print = sec.heading_text.clone();
        sentence_for_print = sec
            .options
            .iter()
            .find(|o| o.letter == letter)
            .and_then(|o| o.sentence.clone())
            .unwrap_or_default();
        rf.lines.splice(
            hash_end..hash_end,
            [String::new(), format!("**Answer:** {}", letter)],
        );
    }

    record::save(&rf).map_err(|e| {
        msg::write_failed(
            "topic file",
            e,
            Next::owner_runs(format!("adr answer {} <letter> again", id)),
        )
    })?;

    let mut rows = vec![ledger::Row {
        time: ledger::now_rfc3339(),
        event: "answered".to_string(),
        id: id.to_string(),
        hash: cur_hash,
        detail: letter.to_string(),
        version: crate::version::VERSION.to_string(),
    }];
    for old in &supersedes_val {
        rows.push(ledger::Row {
            time: ledger::now_rfc3339(),
            event: "superseded".to_string(),
            id: old.clone(),
            hash: "-".to_string(),
            detail: id.to_string(),
            version: crate::version::VERSION.to_string(),
        });
    }
    ledger::append(&ledger_path, &rows).map_err(|e| {
        msg::update_failed(
            "ledger",
            e,
            Next::owner_runs(format!("adr answer {} <letter> again", id)),
        )
    })?;

    Ok(Outcome {
        summary: format!("answered: {}\n  {}", heading_for_print, sentence_for_print),
    })
}

/// `adr answer <id> refuse` (8.1 A). The refusals shared with the letter
/// path (id not found, already answered, terms, hash state) apply; the
/// letter-specific option check and the section's own terms-line/pairing
/// checks do not, since a refusal commits no decision sentence bound to a
/// term. On success: `**Answer:** refused` is appended at the end of the
/// hash range, and a `refused` row is written (hash = the section hash
/// before the insert, detail = heading). A `supersedes:` line on a refused
/// question is never acted on (no target is checked, no `superseded` row
/// is written); its presence only earns a printed note.
pub fn do_refuse(root: &Path, id: &str) -> Result<Outcome, String> {
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);

    let files = repo::list_decision_files(root);
    let mut found = None;
    for f in &files {
        if let Ok(rf) = record::load(f) {
            if let Some(idx) = rf.sections.iter().position(|s| s.id.as_deref() == Some(id)) {
                found = Some((rf, idx));
                break;
            }
        }
    }
    let Some((mut rf, idx)) = found else {
        return Err(msg::id_not_found(id));
    };
    let rel = state::rel_path(root, &rf.path);

    // Same terms.md gates as the letter path (missing/shape/not approved);
    // the section's own terms-line/pairing checks are not shared (5).
    require_terms(root, &led, &rel)?;

    let cur_hash;
    let supersedes_val;
    {
        let sec = &rf.sections[idx];
        if sec.is_refused() {
            return Err(msg::already_refused(&sec.heading_text));
        }
        if sec.answer_line.is_some() {
            return Err(msg::already_answered(&sec.heading_text));
        }
        cur_hash = record::section_hash(&rf.lines, sec);
        match led.latest_state(id) {
            Some(row) if row.event == "asked" && row.hash == cur_hash => {}
            _ => {
                return Err(msg::not_in_asked_state(&sec.heading_text));
            }
        }
        supersedes_val = sec.supersedes.clone();
    }

    let heading_for_print;
    let mut note: Option<String> = None;
    {
        let sec = &rf.sections[idx];
        let (_, hash_end) = record::hash_range(&rf.lines, sec);
        heading_for_print = sec.heading_text.clone();
        if !supersedes_val.is_empty() {
            note = Some(format!(
                "answer: {}",
                msg::supersedes_ignored_on_refusal(&heading_for_print)
            ));
        }
        rf.lines.splice(
            hash_end..hash_end,
            [String::new(), "**Answer:** refused".to_string()],
        );
    }

    record::save(&rf).map_err(|e| {
        msg::write_failed(
            "topic file",
            e,
            Next::owner_runs(format!("adr answer {} refuse again", id)),
        )
    })?;

    let row = ledger::Row {
        time: ledger::now_rfc3339(),
        event: "refused".to_string(),
        id: id.to_string(),
        hash: cur_hash,
        detail: heading_for_print.clone(),
        version: crate::version::VERSION.to_string(),
    };
    ledger::append(&ledger_path, &[row]).map_err(|e| {
        msg::update_failed(
            "ledger",
            e,
            Next::owner_runs(format!("adr answer {} refuse again", id)),
        )
    })?;

    let mut summary = format!("answer: {}", msg::refused_summary(&heading_for_print, &rel));
    if let Some(n) = note {
        summary.push('\n');
        summary.push_str(&n);
    }

    Ok(Outcome { summary })
}

pub fn do_approve(root: &Path, file: &Path) -> Result<Outcome, String> {
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);
    let rel = state::rel_path(root, file);

    if rel == "adrdom/terms.md" {
        do_approve_terms(root, file, &led, &ledger_path)
    } else {
        do_approve_record(root, file, &rel, &led, &ledger_path)
    }
}

/// One dependent's pairs (`word=uuid`), together with a description of
/// where they were carried, for approve's step 4.
struct Dependent {
    /// `<rel>: "<heading>"` for a decision, or `term "<name>"` for a term:
    /// in both cases the recorded pairing, read from the dependent's own
    /// `terms:` line exactly as it stood in the file before this approve,
    /// never re-resolved from today's definition/section text.
    source: String,
    /// The dependent's own decision id when it is a decision (`Some`), so a
    /// retired-term refusal can point at the exact question to supersede;
    /// `None` for a term dependent, which has no id and nothing to
    /// supersede.
    id: Option<String>,
    pairs: Vec<(String, String)>,
}

/// Every refusal `adrdom/terms.md` earns under steps 1, 3, 4 and 5 of
/// `do_approve_terms` (shape errors, circular definitions, a dependent's
/// pairing that no longer holds, banned words in the file's own text),
/// collected all at once rather than returned at the first (E1.1) --
/// `do_approve_terms` prints every one and writes nothing on any refusal;
/// `check` reports the same list, each line prefixed `terms: ` instead of
/// `approve: `, minus the approval-state lines that only make sense after a
/// real write (`already approved`/`approved`, the definition-changed
/// notes), which stay `check`'s own.
///
/// `tf` must already have every id minted in memory (step 2, see
/// `terms::mint_ids`/`terms::minted_copy`): `do_approve_terms` mints
/// straight into the file it is about to write before calling this; `check`
/// -- which never writes -- calls this on a `terms::minted_copy` instead.
/// Either way, a cycle or a moved/retired dependent's pairing in a terms
/// file that has never been approved, and so carries no ids on disk, is
/// caught the same way both callers look for it (`terms::cycles` and
/// `terms::phrase_lookup` only see a term that has an id).
///
/// Returns every refusal message (`adrdom/terms.md: ...`, no caller
/// prefix) plus each term's freshly resolved definition pairs
/// (`terms::definition_pairs`, in `tf.terms` order), computed once here
/// and reused by `do_approve_terms`'s write step so it is never resolved
/// twice.
///
/// `records` is every topic file in the repo, already parsed by the
/// caller (E1.3): `do_approve_terms` parses them once itself and reuses
/// that same parse for its step-7 report; `check` passes the same
/// `RecordFile`s it built for its own record checks. Neither re-reads a
/// topic file from disk just to compute a dependent's terms line here.
pub fn terms_refusals(
    root: &Path,
    records: &[RecordFile],
    tf: &terms::TermsFile,
    led: &Ledger,
) -> (Vec<String>, Vec<Vec<(String, String)>>) {
    let mut refusals: Vec<String> = Vec::new();

    // Step 1 (shape): "missing id" cannot appear here, since `tf` is
    // already minted.
    for e in &tf.errors {
        refusals.push(msg::terms_shape_error(e));
    }

    // Step 3: definition pairings; a self-pairing or any cycle refuses.
    // Every cycle is reported, not just the first.
    for cycle in terms::cycles(tf) {
        let mut chain = cycle.clone();
        chain.push(cycle[0].clone());
        refusals.push(msg::circular_definition(&chain.join(" -> ")));
    }

    // Fresh definition pairs for every term (never re-derived from a stale
    // terms line in the file). Computed once and reused for the dependents
    // check below and, by the caller, for the write step.
    let pairs_per_entry: Vec<Vec<(String, String)>> =
        (0..tf.terms.len()).map(|i| terms::definition_pairs(tf, i)).collect();

    // Every current decision's terms line, and every non-retired term's
    // fresh definition pairs, is a dependent.
    let mut dependents: Vec<Dependent> = Vec::new();
    for rf in records {
        let frel = &rf.rel;
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !state::is_current(led, id) {
                continue;
            }
            let Some(tl) = sec.terms_line else { continue };
            let Some(pairs) = terms::parse_terms_line(rf.lines[tl].trim_end()) else {
                continue;
            };
            dependents.push(Dependent {
                source: format!("{}: \"{}\"", frel, sec.heading_text),
                id: Some(id.clone()),
                pairs,
            });
        }
    }
    for t in &tf.terms {
        if t.retired {
            continue;
        }
        // The recorded pairing, exactly as decisions' frozen dependents are
        // read: the term's own `terms:` line as it already stood in the
        // file before this approve, not a fresh re-resolve (9.3 B;
        // terms-walk: "definitions are dependents for retire/move
        // refusals"). A term with no terms line yet has no dependents.
        let Some(pairs) = t.terms_line.as_deref().and_then(terms::parse_terms_line) else {
            continue;
        };
        dependents.push(Dependent {
            source: format!("term \"{}\"", t.name),
            id: None,
            pairs,
        });
    }

    // Step 4: every pair on a dependent must still hold in the new file.
    // `lookup` answers "what does this exact word/phrase resolve to" for
    // every dependent pair, built once rather than once per pair.
    let lookup = terms::phrase_lookup(&tf.terms);
    for dep in &dependents {
        for (word, uuid) in &dep.pairs {
            let Some(target) = tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str()))
            else {
                refusals.push(msg::term_removed_but_carried(uuid, &dep.source));
                continue;
            };
            if target.retired {
                // A decision dependent: the agent's move is to write a new
                // question that supersedes it, not the owner's to make on
                // the spot. A term dependent (its own definition still
                // carries the retired word) is never superseded (9.3 B):
                // the agent rewords the definition and the owner re-approves.
                let next = match &dep.id {
                    Some(id) => Next::agent(format!("supersedes {} with a new question first", id)),
                    None => msg::term_reword_and_approve(),
                };
                refusals.push(msg::term_retired_but_carried(&target.name, &dep.source, next));
                continue;
            }
            let still_resolves = lookup.get(word) == Some(uuid);
            if !still_resolves {
                // Same split as the retired case just above: a decision
                // dependent re-asks so its terms line matches the current
                // wording; a term dependent's own definition is reworded
                // and the owner re-approves.
                let next = match &dep.id {
                    Some(id) => Next::agent(format!(
                        "re-asks {} so its terms line matches the current wording",
                        id
                    )),
                    None => msg::term_reword_and_approve(),
                };
                refusals.push(msg::word_moved_off_term(word, &target.name, &dep.source, next));
            }
        }
    }

    // Step 5: banned hits in terms.md's own text, from the single ban list.
    let banned_path = root.join("adrdom/state/banned.sha256");
    let banned_entries = crate::banned::read_entries(&banned_path);
    if !banned_entries.is_empty() {
        let hash_map: std::collections::HashMap<String, Option<String>> =
            banned_entries.into_iter().collect();
        for (i, line) in tf.lines.iter().enumerate() {
            for tok in crate::banned::tokenize(line) {
                let h = record::sha256_hex(&tok);
                if let Some(uuid_opt) = hash_map.get(&h) {
                    let suffix = uuid_opt
                        .as_ref()
                        .and_then(|u| {
                            tf.terms
                                .iter()
                                .find(|t| t.id.as_deref() == Some(u.as_str()))
                                .map(|t| t.name.clone())
                        })
                        .map(|n| format!(" (term {})", n))
                        .unwrap_or_default();
                    refusals.push(msg::terms_banned_word(i + 1, &tok, &suffix));
                }
            }
        }
    }

    (refusals, pairs_per_entry)
}

/// `adrdom/terms.md`: the seven steps of 4 in the spec, in order, nothing
/// written to disk before step 6 (a shape/cycle/dependent/banned refusal
/// leaves the file byte-identical to what was on disk before the call).
/// Steps 1, 3, 4 and 5 are `terms_refusals`, shared with `check` (E1.1).
fn do_approve_terms(
    root: &Path,
    file: &Path,
    led: &Ledger,
    ledger_path: &Path,
) -> Result<Outcome, String> {
    // Step 1: parse.
    let mut tf = terms::load(file).map_err(|e| {
        format!(
            "approve: {}",
            msg::read_failed(
                &file.display().to_string(),
                e,
                Next::owner_runs("adr approve adrdom/terms.md again"),
            )
        )
    })?;

    // Every topic file, parsed once here and reused both by
    // `terms_refusals` (E1.3: no re-reading them from disk itself) and by
    // step 7's dependent report below.
    let mut records: Vec<RecordFile> = Vec::new();
    for f in &repo::list_decision_files(root) {
        if let Ok(rf) = record::load_in(root, f) {
            records.push(rf);
        }
    }

    // The pre-existing latest `terms` row, captured before anything below
    // can change what "latest" means, for step 7's before/after compare.
    let prev_terms_row = led.latest_for_path("terms", "adrdom/terms.md").cloned();

    // Step 2: mint ids in memory only; nothing is written to disk yet.
    let inserted = terms::mint_ids(&mut tf);

    // Steps 1 (shape, post-mint), 3 (cycles), 4 (dependents), 5 (banned):
    // every refusal, all at once.
    let (refusals, pairs_per_entry) = terms_refusals(root, &records, &tf, led);
    if !refusals.is_empty() {
        let msg = refusals
            .iter()
            .map(|r| format!("approve: {}", r))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(msg);
    }

    // Step 6: write the file. `terms:` lines use the fresh pairs computed
    // above, one term at a time, in the same order as `tf.terms`.
    let mut new_lines = tf.lines.clone();
    terms::write_terms_lines(&mut new_lines, &tf, &pairs_per_entry);
    tf.lines = new_lines;

    terms::save(&tf).map_err(|e| {
        format!(
            "approve: {}",
            msg::write_failed(
                &file.display().to_string(),
                e,
                Next::owner_runs("adr approve adrdom/terms.md again"),
            )
        )
    })?;

    let mut out: Vec<String> = Vec::new();
    if inserted > 0 {
        out.push(msg::terms_ids_filled(inserted));
    }

    if state::terms_approved(led, &tf) {
        out.push(msg::terms_already_approved());
    } else {
        let row = ledger::Row {
            time: ledger::now_rfc3339(),
            event: "terms".to_string(),
            id: "-".to_string(),
            hash: terms::file_hash(&tf),
            detail: "adrdom/terms.md".to_string(),
            version: crate::version::VERSION.to_string(),
        };
        ledger::append(ledger_path, &[row]).map_err(|e| {
            format!(
                "approve: {}",
                msg::update_failed(
                    "ledger",
                    e,
                    Next::owner_runs("adr approve adrdom/terms.md again"),
                )
            )
        })?;
        out.push(msg::terms_approved_now());
    }

    // Step 7: informational, never a refusal. Only when the version on
    // HEAD is genuinely the one just superseded (its hash matches the
    // ledger row that was latest before this call) do we have anything
    // trustworthy to diff definitions against.
    if let Some(prev_row) = &prev_terms_row {
        if let Some(head_content) = repo::git(root, &["show", "HEAD:adrdom/terms.md"]) {
            let old_tf = terms::parse(&record::split_lines(&head_content));
            if terms::file_hash(&old_tf) == prev_row.hash {
                for t in &tf.terms {
                    let Some(tid) = &t.id else { continue };
                    let Some(old_t) = old_tf.terms.iter().find(|o| o.id.as_deref() == Some(tid.as_str()))
                    else {
                        continue;
                    };
                    if old_t.definition == t.definition {
                        continue;
                    }
                    out.push(msg::term_definition_changed(&t.name));
                    for rf in &records {
                        for sec in &rf.sections {
                            let Some(id) = &sec.id else { continue };
                            if !state::is_current(led, id) {
                                continue;
                            }
                            let Some(tl) = sec.terms_line else { continue };
                            let pairs =
                                terms::parse_terms_line(rf.lines[tl].trim_end()).unwrap_or_default();
                            if pairs.iter().any(|(_, pid)| pid == tid) {
                                out.push(msg::term_used_by_current(&rf.rel, &sec.heading_text));
                            }
                        }
                    }
                    for (i, t2) in tf.terms.iter().enumerate() {
                        if t2.retired {
                            continue;
                        }
                        if pairs_per_entry[i].iter().any(|(_, pid)| pid == tid) {
                            out.push(msg::term_used_by_term(&t2.name));
                        }
                    }
                }
            }
        }
    }

    Ok(Outcome {
        summary: out.join("\n"),
    })
}

/// Every refusal one topic file (`rf`, at `rel`) earns on its own: shape
/// errors (`parse`), a heading/context-prose tell on text not yet frozen
/// (`tells`), a missing `depends:` line, an entry that fails to resolve, a
/// frozen section's edited depends entry, the derive error, and the tier
/// cap (all `plan`) -- every category `do_approve_record` refuses on
/// except the cross-file "answered id missing from every topic file"
/// check, which needs every topic file, not just this one
/// (`missing_answered_refusals`, below). Shared with `check` (E1.2, one
/// function for both, mirroring `terms_refusals`, E1.1): `approve` prefixes
/// each message `approve: `, `check` prefixes it `<category>: `, so the
/// two print the identical line.
///
/// `terms_list`/`phrase_lookup` are supplied by the caller rather than
/// loaded here: `approve` passes its own already-approved terms (gated by
/// `require_terms` before this is ever called), `check` a copy with every
/// id minted in memory so a real term in a terms.md that has never been
/// approved still resolves (E1.3) instead of drowning this in "which is no
/// term" refusals.
pub fn record_refusals(
    rel: &str,
    rf: &RecordFile,
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
    led: &Ledger,
) -> Vec<Refusal> {
    let mut out: Vec<Refusal> = Vec::new();

    for e in &rf.shape_errors {
        out.push(Refusal {
            category: "parse",
            message: msg::shape_error(rel, e),
        });
    }
    for sec in &rf.sections {
        for e in &sec.shape_errors {
            out.push(Refusal {
                category: "parse",
                message: msg::section_shape_error(rel, &sec.heading_text, e),
            });
        }
    }

    // 9.5 A: a loaded phrase refuses before any write -- a heading with any
    // tell (loaded opening, "and"/"or", provenance, code state, or a date),
    // or a context prose line with a provenance, code-state, or date tell.
    // A frozen heading (state::is_frozen) is skipped: it can never be
    // edited, so refusing on it would block this file forever once the
    // list grows to include a word it already carried.
    for sec in &rf.sections {
        if state::is_frozen(led, sec) {
            continue;
        }
        for (line, phrase, what) in tells::heading_tells(sec.start, &sec.heading_text) {
            out.push(Refusal {
                category: "tells",
                message: msg::tell(rel, line, &sec.heading_text, &phrase, what),
            });
        }
    }
    {
        let (cs, ce) = rf.context_range;
        for (line, phrase, what) in tells::prose_tells_in(&rf.lines, cs, ce) {
            out.push(Refusal {
                category: "tells",
                message: msg::tell(rel, line, "context", &phrase, what),
            });
        }
    }

    // 8.3/8.6 B: every section, skeleton or not, needs at least one
    // `depends:` line naming what the question stands on, before anything
    // about it is resolved or written.
    for sec in &rf.sections {
        if sec.depends.is_empty() {
            out.push(Refusal {
                category: "plan",
                message: msg::no_depends_line(rel, &sec.heading_text),
            });
        }
    }

    // Resolve every entry (8.3/9.1 A). An `Id` entry is kept as written and
    // must already be a heading id in this file, a non-retired term id, or a
    // current decision. A `Text` entry ending in `?` is a heading reference,
    // whose dangling/cycle/self-dep is `derive_tiers`'s single error
    // (computed at parse time into `rf.derive_error`); any other `Text`
    // entry must resolve to exactly one non-retired term. A `Text` entry
    // (of either shape) found inside a section whose latest ledger state is
    // `answered`/`refused`/`superseded` refuses instead of being silently
    // rewritten (9.1 A point 5): that section is frozen -- its decision is
    // already closed, one way or another -- and an edit there is the
    // agent's to remove, not approve's to carry forward.
    for sec in &rf.sections {
        let frozen = state::is_frozen(led, sec);
        for d in &sec.depends {
            if frozen && matches!(d, record::Dep::Text { .. }) {
                out.push(Refusal {
                    category: "plan",
                    message: msg::frozen_depends_edited(rel, &sec.heading_text),
                });
                continue;
            }
            if let record::Dep::Text { text, .. } = d {
                if text.trim().ends_with('?') {
                    // Dangling/cycle/self-dep is reported once, below, via
                    // `rf.derive_error`.
                    continue;
                }
            }
            if let Err(e) = depends::resolve_dep(d, rf, terms_list, phrase_lookup, led) {
                out.push(Refusal {
                    category: "plan",
                    message: depends::dep_refusal_text(rel, &sec.heading_text, &e),
                });
            }
        }
    }
    if let Some(err) = &rf.derive_error {
        out.push(Refusal {
            category: "plan",
            message: msg::derive_error(rel, err),
        });
    }

    // Cap (8.8 A): no derived tier may hold more than `TIER_CAP` live
    // questions. Only checkable when tiers were actually derived: a
    // `derive_error` above leaves `rf.tiers` `None`, already refused
    // there. A refused/superseded/withdrawn question is dead and does not
    // count (`state::live_tiers`).
    if let Some(tiers) = &rf.tiers {
        let live = state::live_tiers(led, rf, tiers);
        if let Some((tier, count)) = record::over_cap(&live) {
            out.push(Refusal {
                category: "plan",
                message: msg::tier_cap_exceeded(rel, tier, count, record::TIER_CAP),
            });
        }
    }

    out
}

/// A current (answered, not superseded) decision id now named by no topic
/// file's heading at all, across every topic file in the repo (`records`):
/// `approve` refuses on it rather than silently withdrawing it the way an
/// asked/refused id missing from every file is (that withdrawal stays
/// `do_approve_record`'s own bookkeeping, not a refusal, since striking a
/// question that was never answered loses nothing). Global, so computed
/// once per run rather than once per file -- `do_approve_record` calls it
/// once regardless of which file was named; `check` likewise once for the
/// whole repo. Shared with `check`'s `ids` category (E1.2).
pub fn missing_answered_refusals(records: &[RecordFile], led: &Ledger) -> Vec<Refusal> {
    let mut present_ids: HashSet<String> = HashSet::new();
    for rf in records {
        for sec in &rf.sections {
            if let Some(id) = &sec.id {
                present_ids.insert(id.clone());
            }
        }
    }

    let mut state_ids: BTreeSet<String> = BTreeSet::new();
    for row in &led.rows {
        if matches!(
            row.event.as_str(),
            "asked" | "answered" | "refused" | "superseded" | "withdrawn"
        ) {
            state_ids.insert(row.id.clone());
        }
    }

    let mut out = Vec::new();
    for id in &state_ids {
        if present_ids.contains(id) {
            continue;
        }
        if let Some(r) = led.latest_state(id) {
            if r.event == "answered" {
                let heading = led
                    .latest_asked(id)
                    .map(|a| a.detail.clone())
                    .unwrap_or_else(|| id.clone());
                out.push(Refusal {
                    category: "ids",
                    message: msg::answered_id_missing(id, &heading),
                });
            }
        }
    }
    out
}

/// Inserts a freshly minted id directly after each skeleton section's
/// `depends:` line (9.1 A/8.1 A point 4, no blank padding, the same bare
/// single-line insert `ask` uses for `terms:` after `id:`). Every section
/// here has already passed `record_refusals`'s "has no depends line"
/// check, so the search below should never fail -- but that invariant
/// lives in a different function now, so a future reorder of the two
/// reports an error instead of panicking.
fn insert_missing_ids(rf: &RecordFile, mut work_lines: Vec<String>) -> Result<Vec<String>, String> {
    let needs_id: Vec<bool> = rf.sections.iter().map(|s| s.id.is_none()).collect();
    for i in (0..rf.sections.len()).rev() {
        if needs_id[i] {
            let sec = &rf.sections[i];
            let depends_idx = (sec.start + 1..sec.end)
                .find(|&j| work_lines[j].starts_with("depends: "))
                .ok_or_else(|| format!("approve: {}", msg::no_depends_line_bare(&sec.heading_text)))?;
            let new_id = Uuid::new_v4().to_string();
            work_lines.insert(depends_idx + 1, format!("id: {}", new_id));
        }
    }
    Ok(work_lines)
}

/// A topic file: parse, refuse on shape errors; resolve every `depends:`
/// entry (9.1 A: the file holds ids, the agent writes words, and `approve`
/// is what turns one into the other); check every ledger id that now
/// appears in no topic file, refusing on a current (answered) one, before
/// anything is written -- withdrawing an asked/refused one and allowing a
/// superseded one silently happen after, since neither is a refusal; only
/// then mint an id for every heading that lacks one, compute plan hash and
/// context hash over the rewritten file, write an `approved` row if it
/// differs from the latest for the path, a `context` row likewise, and
/// finally write the withdrawn rows. Nothing is written to disk (file or
/// ledger) when any check refuses.
fn do_approve_record(
    root: &Path,
    file: &Path,
    rel: &str,
    led: &Ledger,
    ledger_path: &Path,
) -> Result<Outcome, String> {
    let rf = record::load(file).map_err(|e| {
        format!(
            "approve: {}",
            msg::read_failed(
                &file.display().to_string(),
                e,
                Next::owner_runs(format!("adr approve {} again", rel)),
            )
        )
    })?;

    // Refuse up front, before a single `depends:` entry is resolved, when
    // adrdom/terms.md is missing, shape-broken, or not yet approved (the
    // review finding this fixes): the same gate `ask`/`do_answer` apply via
    // `require_terms`. Without this, a real term with no id minted yet (an
    // unapproved terms.md mints nothing to disk) silently fails to resolve
    // and every depends entry naming it refuses "which is no term" instead
    // of the one clear "terms not approved" line.
    let tf = require_terms(root, led, rel).map_err(|e| format!("approve: {}", e))?;
    let terms_list: &[terms::Term] = &tf.terms;
    // Built once per command run (not per depends entry): `resolve_dep`
    // used to call `terms::resolve_word`, which rebuilt this same map from
    // scratch on every entry.
    let phrase_lookup = terms::phrase_lookup(terms_list);

    // E1.2: every refusal this file can earn -- shape, tells, a missing
    // depends line, depends resolution (including a frozen section's
    // edit and the derived-tiers error), and the tier cap -- collected
    // here (`record_refusals`, shared with `check`) instead of returned at
    // the first, so one `approve` call tells the agent everything it owes
    // the owner before asking again.
    let mut refusals: Vec<String> = record_refusals(rel, &rf, terms_list, &phrase_lookup, led)
        .into_iter()
        .map(|r| format!("approve: {}", r.message))
        .collect();

    // Ledger ids (asked/answered/refused/superseded/withdrawn) that now
    // appear in no topic file at all: withdraw the asked/refused ones,
    // refuse on a current (answered, not superseded) one via
    // `missing_answered_refusals` (shared with `check`'s `ids` category),
    // allow a superseded one silently. Computed and checked here, before
    // any write below, so a refusal on this leaves both the topic file and
    // the ledger exactly as they were (the function's own contract:
    // nothing is written when a check refuses). Reads every topic file
    // straight off disk like `file` itself -- still unwritten at this
    // point, so its ids are exactly `rf`'s; minting new ids for skeleton
    // headings below never removes an id already in the ledger's
    // `state_ids`, so reading before that rewrite changes nothing this
    // check depends on.
    let mut all_records: Vec<RecordFile> = Vec::new();
    for f in &repo::list_decision_files(root) {
        if let Ok(rf2) = record::load(f) {
            all_records.push(rf2);
        }
    }
    for r in missing_answered_refusals(&all_records, led) {
        refusals.push(format!("approve: {}", r.message));
    }

    let present_ids: HashSet<String> = all_records
        .iter()
        .flat_map(|rf2| rf2.sections.iter().filter_map(|s| s.id.clone()))
        .collect();
    let mut state_ids: BTreeSet<String> = BTreeSet::new();
    for row in &led.rows {
        if matches!(
            row.event.as_str(),
            "asked" | "answered" | "refused" | "superseded" | "withdrawn"
        ) {
            state_ids.insert(row.id.clone());
        }
    }

    // Always computed (harmless when `refusals` is already non-empty from
    // an earlier category): only acted on -- ledger rows appended -- once
    // every category above has been confirmed clean, below.
    let mut withdrawn_rows: Vec<ledger::Row> = Vec::new();
    for id in &state_ids {
        if present_ids.contains(id) {
            continue;
        }
        // A refused id absent from every file is withdrawn exactly like a
        // struck asked one (8.2 A): the refusal already closed it with no
        // decision, so there's nothing left to protect. An answered one is
        // handled above, by `missing_answered_refusals`.
        if let Some(r) = led.latest_state(id) {
            if r.event == "asked" || r.event == "refused" {
                withdrawn_rows.push(ledger::Row {
                    time: ledger::now_rfc3339(),
                    event: "withdrawn".to_string(),
                    id: id.clone(),
                    hash: "-".to_string(),
                    detail: r.detail.clone(),
                    version: crate::version::VERSION.to_string(),
                });
            }
        }
    }

    if !refusals.is_empty() {
        return Err(refusals.join("\n"));
    }

    // Every check above passed: mint an id for every heading that lacks one
    // (skeletons included), reparse so every heading's id is available, then
    // rewrite each section's single `depends:` line to one line of ids
    // (8.1 A point 3), entry by entry, keeping any `=set` suffix.
    let work_lines = insert_missing_ids(&rf, rf.lines.clone())?;
    let mut work_rf = record::build(rf.path.clone(), rel.to_string(), work_lines);

    let sections_snapshot = work_rf.sections.clone();
    for sec in &sections_snapshot {
        let Some(line_idx) = (sec.start + 1..sec.end).find(|&j| work_rf.lines[j].starts_with("depends: "))
        else {
            continue;
        };
        let parts: Vec<String> = sec
            .depends
            .iter()
            .map(|d| rewrite_dep_entry(d, &work_rf, terms_list, &phrase_lookup, led))
            .collect();
        work_rf.lines[line_idx] = format!("depends: {}", parts.join(", "));
    }
    // Reparse once more so `plan_hash`/`context_hash` and everything below
    // sees the file exactly as it will be written.
    let work_rf = record::build(work_rf.path.clone(), rel.to_string(), work_rf.lines.clone());

    let plan_hash = record::plan_hash(&work_rf);
    let context_hash = record::context_hash(&work_rf);

    record::save(&work_rf).map_err(|e| {
        format!(
            "approve: {}",
            msg::write_failed(
                &file.display().to_string(),
                e,
                Next::owner_runs(format!("adr approve {} again", rel)),
            )
        )
    })?;

    let mut rows: Vec<ledger::Row> = Vec::new();
    let mut messages: Vec<String> = Vec::new();

    let plan_changed = led
        .latest_for_path("approved", rel)
        .map(|r| r.hash.as_str())
        != Some(plan_hash.as_str());
    if plan_changed {
        rows.push(ledger::Row {
            time: ledger::now_rfc3339(),
            event: "approved".to_string(),
            id: "-".to_string(),
            hash: plan_hash.clone(),
            detail: rel.to_string(),
            version: crate::version::VERSION.to_string(),
        });
        messages.push(msg::plan_approved(rel));
    } else {
        messages.push(msg::plan_unchanged(rel));
    }

    let context_changed = led
        .latest_for_path("context", rel)
        .map(|r| r.hash.as_str())
        != Some(context_hash.as_str());
    if context_changed {
        rows.push(ledger::Row {
            time: ledger::now_rfc3339(),
            event: "context".to_string(),
            id: "-".to_string(),
            hash: context_hash.clone(),
            detail: rel.to_string(),
            version: crate::version::VERSION.to_string(),
        });
        messages.push(msg::context_approved(rel));
    } else {
        messages.push(msg::context_unchanged(rel));
    }

    ledger::append(ledger_path, &rows).map_err(|e| {
        format!(
            "approve: {}",
            msg::update_failed(
                "ledger",
                e,
                Next::owner_runs(format!("adr approve {} again", rel)),
            )
        )
    })?;

    // `withdrawn_rows` was computed, and any refusal already returned,
    // above -- before either write this function has made so far.
    if !withdrawn_rows.is_empty() {
        ledger::append(ledger_path, &withdrawn_rows).map_err(|e| {
            format!(
                "approve: {}",
                msg::update_failed(
                    "ledger",
                    e,
                    Next::owner_runs(format!("adr approve {} again", rel)),
                )
            )
        })?;
        for r in &withdrawn_rows {
            messages.push(msg::withdrawn(&r.detail));
        }
    }

    // Printed last, after a successful record approval (spec §3): the
    // tiers this file now derives to, one `N (count)` per non-empty tier,
    // live counts only (8.8 A note; matches `check`'s own "tiers: ..."
    // note, computed the same way via `state::live_tiers`). `work_rf.tiers`
    // is guaranteed `Some` here: a `derive_error` would already be among
    // `refusals`, and the early return above would have fired.
    let tiers: &Vec<usize> = work_rf
        .tiers
        .as_ref()
        .expect("derive_error refuses above whenever tiers is None");
    let live = state::live_tiers(led, &work_rf, tiers);
    let tiers_str = record::tier_sizes(&live)
        .into_iter()
        .map(|(t, c)| format!("{} ({})", t, c))
        .collect::<Vec<_>>()
        .join(" ");
    messages.push(msg::tiers_note_approve(tiers_str.trim_end()));

    Ok(Outcome {
        summary: messages.join("\n"),
    })
}

/// The final text one `depends:` entry rewrites to when `approve` rewrites
/// a section's single `depends:` line (8.1 A point 3): an already-resolved
/// `Id` entry keeps its id and set exactly as written; a `Text` entry
/// resolves to the id its target already carries in `work_rf` (every entry
/// was already validated above, so this should always find one), keeping
/// its own written text as a fallback rather than silently dropping the
/// entry on the one resolution rule this can't see (a dangling/cycle/
/// self-dep heading reference, reported once from `rf.derive_error` instead
/// of per entry).
fn rewrite_dep_entry(
    d: &record::Dep,
    work_rf: &RecordFile,
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
    led: &Ledger,
) -> String {
    let set_suffix = |set: &Option<record::AnswerSet>| {
        set.as_ref()
            .map(|s| format!("={}", record::canonical_set(s)))
            .unwrap_or_default()
    };
    match d {
        record::Dep::Id { id, set } => format!("{}{}", id, set_suffix(set)),
        record::Dep::Text { text, set } => {
            let resolved = depends::resolve_dep(d, work_rf, terms_list, phrase_lookup, led)
                .ok()
                .flatten()
                .and_then(|target| match target {
                    DepTarget::Heading(idx) => work_rf.sections[idx].id.clone(),
                    DepTarget::Term(id) | DepTarget::Decision(id) => Some(id),
                });
            match resolved {
                Some(id) => format!("{}{}", id, set_suffix(set)),
                None => format!("{}{}", text, set_suffix(set)),
            }
        }
    }
}

pub fn do_unban(root: &Path, input: &str) -> Result<Outcome, String> {
    let banned_path = root.join("adrdom/state/banned.sha256");
    let ledger_path = root.join("adrdom/state/ledger.tsv");

    let mut to_remove: HashSet<String> = HashSet::new();
    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        for tok in crate::banned::tokenize(line) {
            to_remove.insert(record::sha256_hex(&tok));
        }
    }

    let existing = crate::banned::read_entries(&banned_path);
    let mut removed: Vec<String> = existing
        .iter()
        .filter(|(h, _)| to_remove.contains(h))
        .map(|(h, _)| h.clone())
        .collect();
    removed.sort();

    if removed.is_empty() {
        return Ok(Outcome {
            summary: msg::nothing_to_remove(),
        });
    }

    let mut remaining: Vec<(String, Option<String>)> = existing
        .into_iter()
        .filter(|(h, _)| !to_remove.contains(h))
        .collect();
    remaining.sort_by(|a, b| a.0.cmp(&b.0));
    let mut content = String::new();
    for (h, u) in &remaining {
        content.push_str(&crate::banned::format_line(h, u));
        content.push('\n');
    }
    std::fs::write(&banned_path, content).map_err(|e| {
        msg::update_failed("banned list", e, Next::owner_runs("adr unban again"))
    })?;

    let rows: Vec<ledger::Row> = removed
        .iter()
        .map(|h| ledger::Row {
            time: ledger::now_rfc3339(),
            event: "unbanned".to_string(),
            id: "-".to_string(),
            hash: h.clone(),
            detail: String::new(),
            version: crate::version::VERSION.to_string(),
        })
        .collect();
    ledger::append(&ledger_path, &rows)
        .map_err(|e| msg::update_failed("ledger", e, Next::owner_runs("adr unban again")))?;

    Ok(Outcome {
        summary: msg::removed_entries(removed.len()),
    })
}
