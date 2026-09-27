//! One code path for every owner act: the CLI and the page both call these
//! (per "Does the adr binary serve the ui?").
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
/// (one tag richer than `terms_refusals`'s plain `Vec<String>`, since a
/// topic file's refusals span four of `check`'s labels where
/// `adrdom/terms.md`'s all live under `terms:`).
pub struct Refusal {
    pub category: &'static str,
    pub message: String,
}

/// The missing/unreadable half of the terms.md gate, shared by every
/// `require_terms` caller: missing entirely, or present but a read/shape
/// error, each refuses the same way regardless of which caller hit it. The
/// next move is the agent's either way (agents edit `adrdom/terms.md`, per
/// "ask's next step is right" -- there is no owner re-run that fixes a
/// file only an edit can fix).
fn load_terms_or_refuse(root: &Path) -> Result<terms::TermsFile, String> {
    match state::load_terms_checked(root) {
        TermsLoad::Missing => Err(msg::terms_missing()),
        TermsLoad::LoadError(e) => Err(msg::terms_load_error(
            &e,
            Next::agent("fixes adrdom/terms.md and retries"),
        )),
        TermsLoad::Ok(tf) => Ok(tf),
    }
}

/// The strict terms.md gates (5): missing, present but unreadable/
/// shape-invalid, or not approved, each refusing with the same message the
/// caller would print. `rel` is the topic file's path, named in the
/// not-approved message exactly as the caller already prints it. On
/// success, the loaded, approved terms file is returned. The one gate for
/// `ask`, `answer`, `refuse`, and approve-of-a-topic-file (per "Does the
/// tool stop while the terms file waits for approval?", aa475aa3 A).
pub fn require_terms(root: &Path, led: &Ledger, rel: &str) -> Result<terms::TermsFile, String> {
    let tf = load_terms_or_refuse(root)?;
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

/// Exactly one ASCII uppercase character; anything else (empty, more than
/// one character, lowercase, a digit, punctuation) is not a letter. Shared
/// by the CLI (`commands/answer.rs`) and the page (`commands/serve/mod.rs`)
/// so "answer takes that version from the terminal and the page alike"
/// extends to how the letter itself is read: the page refuses a lowercase
/// or multi-character selection the same way the CLI does.
pub fn parse_letter(s: &str) -> Option<char> {
    let mut chars = s.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if c.is_ascii_uppercase() {
        Some(c)
    } else {
        None
    }
}

/// What `do_answer` and `do_refuse` both need once a question has been
/// found, gated on terms, and confirmed to still be in the `asked` state at
/// the version given -- the steps neither letter-specific nor refusal-
/// specific (id lookup, terms gate, already-refused/already-answered,
/// section hash, version format/staleness, ledger state). `tf` is unused by
/// `do_refuse`, which has no terms-line pairing of its own to check.
struct Located {
    led: Ledger,
    rf: RecordFile,
    idx: usize,
    rel: String,
    tf: terms::TermsFile,
    cur_hash: String,
}

/// `letter` is the option `do_answer` will pick; it is checked against the
/// section's options right after the already-answered checks and before
/// the version check, as before answer and refuse shared this path.
/// `do_refuse` passes `None`.
fn locate_asked(root: &Path, id: &str, version: &str, letter: Option<char>) -> Result<Located, String> {
    let ledger_path = root.join(crate::repo::LEDGER_PATH);
    let led = ledger::read(&ledger_path);

    let mut found = None;
    for rf in record::load_all(root) {
        if let Some(idx) = rf.sections.iter().position(|s| s.id.as_deref() == Some(id)) {
            found = Some((rf, idx));
            break;
        }
    }
    let Some((rf, idx)) = found else {
        return Err(msg::id_not_found(id));
    };
    let rel = rf.rel.clone();

    // Refuse when terms.md is missing, unreadable, or not approved (per
    // "Does the tool stop while the terms file waits for approval?",
    // aa475aa3 A) -- the same gate `ask` applies. The returned terms file is
    // kept for the terms-line validity check `do_answer` makes afterward.
    let tf = require_terms(root, &led, &rel)?;

    let cur_hash;
    {
        let sec = &rf.sections[idx];
        if sec.is_refused() {
            return Err(msg::already_refused(&sec.heading_text));
        }
        if sec.answer_line.is_some() {
            return Err(msg::already_answered(&sec.heading_text));
        }
        if let Some(letter) = letter {
            if !sec.options.iter().any(|o| o.letter == letter) {
                return Err(msg::letter_not_an_option(letter, &sec.heading_text));
            }
        }
        cur_hash = record::section_hash(&rf.lines, sec);
        // Per "Does an answer carry the version of the question the owner
        // read?": the version on the answer must match this question's
        // current text before anything else about ledger state is even
        // considered -- a mismatch means the text moved since it was read,
        // regardless of what the ledger currently says. The given version
        // is lowercased first (hex case shouldn't matter) and validated as
        // exactly `VERSION_LEN` hex characters before ever being compared
        // to the hash -- a malformed argument (wrong length, non-hex
        // characters) is a different problem than a real version that has
        // gone stale, so it earns its own message.
        let version_lower = version.to_lowercase();
        if version_lower.len() != record::VERSION_LEN
            || !version_lower.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(msg::invalid_version_format(version));
        }
        if version_lower != cur_hash[..record::VERSION_LEN] {
            return Err(msg::stale_version(&sec.heading_text));
        }
        match led.latest_state(id) {
            Some(row) if row.event == "asked" && row.hash == cur_hash => {}
            _ => {
                return Err(msg::not_in_asked_state(&sec.heading_text));
            }
        }
    }

    Ok(Located { led, rf, idx, rel, tf, cur_hash })
}

/// The write step `do_answer` and `do_refuse` both end with: splice the
/// `**Answer:** <answer_text>` line in at the end of the section's hash
/// range, save the topic file, then append `rows` to the ledger -- with the
/// same error handling and the same "try again" command shape either way,
/// `retry_words` filling in the part that differs (`"A again"` for the
/// letter, `"refuse again"` for a refusal).
fn finish_answer(
    root: &Path,
    id: &str,
    version: &str,
    retry_words: &str,
    mut rf: RecordFile,
    idx: usize,
    answer_text: &str,
    rows: Vec<ledger::Row>,
) -> Result<(), String> {
    let ledger_path = root.join(crate::repo::LEDGER_PATH);

    let hash_end = {
        let sec = &rf.sections[idx];
        record::hash_range(&rf.lines, sec).1
    };
    rf.lines.splice(
        hash_end..hash_end,
        [String::new(), format!("**Answer:** {}", answer_text)],
    );

    record::save(&rf).map_err(|e| {
        msg::write_failed(
            "topic file",
            e,
            Next::owner_runs(format!("adr answer {}@{} {}", id, version, retry_words)),
        )
    })?;

    ledger::append(&ledger_path, &rows).map_err(|e| {
        msg::update_failed(
            "ledger",
            e,
            Next::owner_runs(format!("adr answer {}@{} {}", id, version, retry_words)),
        )
    })?;

    Ok(())
}

pub fn do_answer(root: &Path, id: &str, version: &str, letter: char) -> Result<Outcome, String> {
    let Located { led, rf, idx, tf, cur_hash, .. } = locate_asked(root, id, version, Some(letter))?;

    let supersedes_val;
    {
        let sec = &rf.sections[idx];
        let Some(terms_line_idx) = sec.terms_line else {
            return Err(msg::no_terms_line(id));
        };
        let problems = terms::validate_pairs(rf.lines[terms_line_idx].trim_end(), &tf)
            .map_err(|_| msg::malformed_terms_line(id))?;
        if let Some(p) = problems.first() {
            return Err(match &p.kind {
                terms::ProblemKind::Gone => msg::terms_line_names_gone_term(id, &p.uuid),
                terms::ProblemKind::Retired(name) => msg::terms_line_names_retired_term(id, name),
            });
        }
        supersedes_val = sec.supersedes.clone();
        for old in &supersedes_val {
            // A supersedes target must be current when this question was
            // asked (per "Does one decision supersede several?", owner note
            // 2026-09-24): the supersedes line freezes at ask, which `ask` itself now
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

    let heading_for_print = rf.sections[idx].heading_text.clone();
    let sentence_for_print = rf.sections[idx]
        .options
        .iter()
        .find(|o| o.letter == letter)
        .and_then(|o| o.sentence.clone())
        .unwrap_or_default();

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

    let letter_str = letter.to_string();
    finish_answer(root, id, version, &format!("{} again", letter_str), rf, idx, &letter_str, rows)?;

    Ok(Outcome {
        summary: msg::answered_summary(&heading_for_print, &sentence_for_print),
    })
}

/// `adr answer <id>@<version> refuse` (per "Does the owner answer a
/// question with a refusal?"). The refusals shared with the letter
/// path (id not found, already answered, terms, hash state) apply; the
/// letter-specific option check and the section's own terms-line/pairing
/// checks do not, since a refusal commits no decision sentence bound to a
/// term. On success: `**Answer:** refused` is appended at the end of the
/// hash range, and a `refused` row is written (hash = the section hash
/// before the insert, detail = heading). A `supersedes:` line on a refused
/// question is never acted on (no target is checked, no `superseded` row
/// is written); its presence only earns a printed note.
pub fn do_refuse(root: &Path, id: &str, version: &str) -> Result<Outcome, String> {
    // Same lookup, terms gate, already-refused/already-answered, hash,
    // version, and ledger-state checks as the letter path (`locate_asked`).
    // The section's own terms-line/pairing checks are not shared (5), so
    // the returned terms file has no further use here.
    let Located { rf, idx, rel, cur_hash, .. } = locate_asked(root, id, version, None)?;

    let supersedes_val = rf.sections[idx].supersedes.clone();
    let heading_for_print = rf.sections[idx].heading_text.clone();
    let mut note: Option<String> = None;
    if !supersedes_val.is_empty() {
        note = Some(format!(
            "answer: {}",
            msg::supersedes_ignored_on_refusal(&heading_for_print)
        ));
    }

    let row = ledger::Row {
        time: ledger::now_rfc3339(),
        event: "refused".to_string(),
        id: id.to_string(),
        hash: cur_hash,
        detail: heading_for_print.clone(),
        version: crate::version::VERSION.to_string(),
    };

    finish_answer(root, id, version, "refuse again", rf, idx, "refused", vec![row])?;

    let mut summary = format!("answer: {}", msg::refused_summary(&heading_for_print, &rel));
    if let Some(n) = note {
        summary.push('\n');
        summary.push_str(&n);
    }

    Ok(Outcome { summary })
}

pub fn do_approve(root: &Path, file: &Path) -> Result<Outcome, String> {
    let ledger_path = root.join(crate::repo::LEDGER_PATH);
    let led = ledger::read(&ledger_path);
    let rel = state::rel_path(root, file);

    if rel == crate::repo::TERMS_PATH {
        do_approve_terms(root, file, &led, &ledger_path)
    } else {
        do_approve_record(root, file, &rel, &led, &ledger_path)
    }
}

/// Every refusal `adrdom/terms.md` earns under steps 1, 3, 4 and 5 of
/// `do_approve_terms` (shape errors, circular definitions, a dependent's
/// pairing that no longer holds, banned words in the file's own text),
/// plus a definition running past two sentences
/// (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76, not one of the numbered steps
/// above), collected all at once rather than returned at the first --
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
/// caller: `do_approve_terms` parses them once itself and reuses
/// that same parse for its step-7 report; `check` passes the same
/// `RecordFile`s it built for its own record checks. Neither re-reads a
/// topic file from disk just to compute a dependent's terms line here.
///
/// `source` is who the ban list (step 5) is read through: `approve` passes
/// `Source::WorkingTree` (it always reads `adrdom/terms.md` straight off
/// disk), `check` passes whichever source it is judging (per "Does check at
/// a commit read the staged files?"), so a staged edit to the ban list is
/// judged, not whatever is left on disk. `scan_banned` is false only for
/// `check`'s own call: `check_banned` already scans `adrdom/terms.md` as a
/// tracked file, so step 5 would otherwise report the same hit twice, once
/// prefixed `banned:` and once prefixed `terms:`; `approve` (the one caller
/// that never runs `check_banned` itself) always passes `true`, since step 5
/// is its only banned-word gate on the file.
pub fn terms_refusals(
    root: &Path,
    records: &[RecordFile],
    tf: &terms::TermsFile,
    led: &Ledger,
    source: &repo::Source,
    scan_banned: bool,
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
        refusals.push(msg::circular_definition(&msg::cycle_chain(&cycle)));
    }

    // Fresh definition pairs for every term (never re-derived from a stale
    // terms line in the file). Computed once and reused for the dependents
    // check below and, by the caller, for the write step.
    let pairs_per_entry: Vec<Vec<(String, String)>> =
        (0..tf.terms.len()).map(|i| terms::definition_pairs(tf, i)).collect();

    // Every current decision's terms line, every current decision's
    // `depends:` line (once approved, an id -- possibly a term's), and
    // every non-retired term's recorded pairing, is a dependent -- the
    // widest reach of `state::dependent_entries`' five readers.
    let dependents: Vec<state::DependentEntry> =
        state::dependent_entries(records, led, tf, true, true, None);

    // Step 4: every pair on a dependent must still hold in the new file.
    // `lookup` answers "what does this exact word/phrase resolve to" for
    // every dependent pair, built once rather than once per pair.
    let lookup = terms::phrase_lookup(&tf.terms);
    for dep in &dependents {
        let label = dep.source.label();
        for (word, uuid) in &dep.pairs {
            let Some(target) = tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str()))
            else {
                refusals.push(msg::term_removed_but_carried(uuid, &label));
                continue;
            };
            if target.retired {
                // A decision dependent: the agent's move is to write a new
                // question that supersedes it, not the owner's to make on
                // the spot. A term dependent (its own definition still
                // carries the retired word) is never superseded (per "Does
                // a retired term keep its words?"): the agent rewords the
                // definition and the owner re-approves.
                let next = match dep.source.decision_id() {
                    Some(id) => Next::agent(format!("supersedes {} with a new question first", id)),
                    None => msg::term_reword_and_approve(),
                };
                refusals.push(msg::term_retired_but_carried(&target.name, &label, next));
                continue;
            }
            // A `depends:`-line dependent's pair is an id read straight off
            // the entry, never a resolved word -- there is no word to
            // drift, only the target's removal/retirement above, so this
            // check is skipped for it (`from_word` is false).
            if dep.from_word {
                let still_resolves = lookup.get(word) == Some(uuid);
                if !still_resolves {
                    // A decision dependent here is always current, since
                    // only `is_current` sections become one above -- so it
                    // can never be re-asked (per "Does a decision change
                    // after it is answered?"; "Does a refusal name the
                    // command that clears it?" means a refusal about an
                    // answered question never tells the agent to re-ask it,
                    // since that can't clear it). What moved is
                    // adrdom/terms.md, not the decision's own text, so the
                    // fix is there: restore the wording so the word
                    // resolves to this term again, or, when the move is
                    // meant, supersede the decision with a new question
                    // that pairs the new wording. A term dependent (its own
                    // definition still carries the drifted word) is
                    // reworded by the agent and the owner re-approves, same
                    // as the retired case above.
                    let next = match dep.source.decision_id() {
                        Some(id) => Next::agent(format!(
                            "restores the wording in adrdom/terms.md so \"{}\" still names \"{}\", or writes a question that supersedes {} with a new terms pairing",
                            word, target.name, id
                        )),
                        None => msg::term_reword_and_approve(),
                    };
                    refusals.push(msg::word_moved_off_term(word, &target.name, &label, next));
                }
            }
        }
    }

    // Step 5: banned hits in terms.md's own text, from the single ban list
    // (skipped for `check`'s call -- see `scan_banned` above -- since
    // `check_banned` already scans this same file as a tracked file).
    if scan_banned {
        let banned_entries = crate::banned::read_entries_via(root, source);
        if !banned_entries.is_empty() {
            let hash_map: std::collections::HashMap<String, Option<String>> =
                banned_entries.into_iter().collect();
            let text = tf.lines.join("\n");
            for hit in crate::banned::hits(&text, &hash_map, &tf.terms) {
                let suffix = msg::term_suffix(hit.term.as_deref());
                refusals.push(msg::terms_banned_word(hit.line, &suffix));
            }
        }
    }

    // Every term's own definition is one or two sentences
    // (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76; not one of the spec's
    // numbered steps). Checked on every entry, retired or not, the same
    // way the shape checks in `terms::parse` run on every entry regardless
    // of `retired`.
    for t in &tf.terms {
        let count = terms::sentence_count(&t.definition);
        if count > 2 {
            refusals.push(msg::definition_too_long(&t.name, count));
        }
    }

    (refusals, pairs_per_entry)
}

/// `adrdom/terms.md` (per "Does the owner approve the terms file as a
/// whole?"): the six steps below, in order, nothing written to disk before
/// step 6 (a shape/cycle/dependent/banned refusal leaves the file
/// byte-identical to what was on disk before the call). Steps 1, 3, 4 and 5
/// (plus the definition-length check above them) are `terms_refusals`,
/// shared with `check`.
fn do_approve_terms(
    root: &Path,
    file: &Path,
    led: &Ledger,
    ledger_path: &Path,
) -> Result<Outcome, String> {
    // Step 1: parse.
    let mut tf = terms::load(file).map_err(|e| {
        msg::approve_prefixed(msg::read_failed(
            &file.display().to_string(),
            e,
            Next::owner_runs("adr approve adrdom/terms.md again"),
        ))
    })?;

    // Every topic file, parsed once here and reused by `terms_refusals`
    // (no re-reading them from disk itself).
    let records: Vec<RecordFile> = record::load_all(root);

    // Step 2: mint ids in memory only; nothing is written to disk yet.
    let inserted = terms::mint_ids(&mut tf);

    // Steps 1 (shape, post-mint), 3 (cycles), 4 (dependents), 5 (banned):
    // every refusal, all at once.
    let (refusals, pairs_per_entry) =
        terms_refusals(root, &records, &tf, led, &repo::Source::WorkingTree, true);
    if !refusals.is_empty() {
        let msg = refusals
            .iter()
            .map(msg::approve_prefixed)
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
        msg::approve_prefixed(msg::write_failed(
            &file.display().to_string(),
            e,
            Next::owner_runs("adr approve adrdom/terms.md again"),
        ))
    })?;

    let mut out: Vec<String> = Vec::new();
    if inserted > 0 {
        out.push(msg::terms_ids_filled(inserted));
    }

    if state::terms_approved(led, &tf) {
        // No new ledger row is written in this branch: the file already
        // matches the ledger's latest `terms` row.
        out.push(msg::terms_already_approved());
    } else {
        let row = ledger::Row {
            time: ledger::now_rfc3339(),
            event: "terms".to_string(),
            id: "-".to_string(),
            hash: terms::file_hash(&tf),
            detail: crate::repo::TERMS_PATH.to_string(),
            version: crate::version::VERSION.to_string(),
        };
        ledger::append(ledger_path, &[row]).map_err(|e| {
            msg::approve_prefixed(msg::update_failed(
                "ledger",
                e,
                Next::owner_runs("adr approve adrdom/terms.md again"),
            ))
        })?;
        out.push(msg::terms_approved_now());
    }

    Ok(Outcome {
        summary: out.join("\n"),
    })
}

/// Every refusal one topic file (`rf`, at `rel`) earns on its own: shape
/// errors and parse errors -- an option with no decision sentence or more
/// than one, a duplicate id line, an answer naming a missing option --
/// (both `parse`), a heading/context-prose tell on text not yet frozen
/// (`tells`), a missing `depends:` line, an entry that fails to resolve, a
/// frozen section's edited depends entry, the derive error, and the tier
/// cap (all `plan`) -- every category `do_approve_record` refuses on
/// except the cross-file "answered id missing from every topic file"
/// check, which needs every topic file, not just this one
/// (`missing_answered_refusals`, below). Shared with `check` and `ask`
/// (055d303b A point 1, one function for all three, mirroring
/// `terms_refusals`): `approve` prefixes each message `approve: `,
/// `check` prefixes it `<category>: `, `ask` prefixes the "parse" slice
/// `ask: `, so the same line reads identically wherever it is printed.
///
/// `terms_list`/`phrase_lookup` are supplied by the caller rather than
/// loaded here: `approve` passes its own already-approved terms (gated by
/// `require_terms` before this is ever called), `check` a copy with every
/// id minted in memory so a real term in a terms.md that has never been
/// approved still resolves instead of drowning this in "which is no
/// term" refusals. `cross_file_headings` is likewise supplied once per
/// command run (`depends::cross_file_heading_index`), threaded straight
/// into `resolve_dep`.
pub fn record_refusals(
    rel: &str,
    rf: &RecordFile,
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
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
    // 055d303b A point 1: an option with no decision sentence (or more than
    // one), a duplicate id line, and an answer naming a missing option were
    // parse errors only `check` read; now every reader of the shared "parse"
    // category (`ask`, `approve`, `check`) refuses on them the same way it
    // already refuses on a shape error.
    for sec in &rf.sections {
        for e in &sec.parse_errors {
            out.push(Refusal {
                category: "parse",
                message: msg::check_parse_error(rel, &sec.heading_text, e),
            });
        }
    }

    // Per "Does the tool stop loaded phrases in a question?": a loaded
    // phrase refuses before any write -- a heading with any
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

    // Per "What does a depends line name?": every section, skeleton or
    // not, needs at least one `depends:` line naming what the question
    // stands on, before anything about it is resolved or written.
    for sec in &rf.sections {
        if sec.depends.is_empty() {
            out.push(Refusal {
                category: "plan",
                message: msg::no_depends_line(rel, &sec.heading_text),
            });
        }
    }

    // Resolve every entry (per "What does a depends line name?" and "Does a
    // reference in a topic file carry the identifier of what it names?").
    // An `Id` entry is kept as written and must already be a heading id in
    // this file, a non-retired term id, or a current decision. A `Text`
    // entry ending in `?` is a heading reference, whose dangling/cycle/
    // self-dep is `derive_tiers`'s single error (computed at parse time
    // into `rf.derive_error`); any other `Text` entry must resolve to
    // exactly one non-retired term. A `Text` entry (of either shape) found
    // inside a section whose latest ledger state is
    // `answered`/`refused`/`superseded` refuses instead of being silently
    // rewritten (per "Does a decision change after it is answered?"): that
    // section is frozen -- its decision is already closed, one way or
    // another -- and an edit there is the agent's to remove, not approve's
    // to carry forward.
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
            if let Err(e) =
                depends::resolve_dep(d, sec, rf, terms_list, phrase_lookup, cross_file_headings, led)
            {
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

    // Cap (per "Does the tool cap the questions in a tier?"): no derived tier may hold more than `TIER_CAP` live
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
/// whole repo. Shared with `check`'s `ids` category.
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
/// `depends:` line (per "Does a reference in a topic file carry the
/// identifier of what it names?"; no blank padding, the same bare
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
                .ok_or_else(|| msg::approve_prefixed(msg::no_depends_line_bare(&sec.heading_text)))?;
            let new_id = Uuid::new_v4().to_string();
            work_lines.insert(depends_idx + 1, format!("id: {}", new_id));
        }
    }
    Ok(work_lines)
}

/// A topic file: parse, refuse on shape errors; resolve every `depends:`
/// entry (per "Does a reference in a topic file carry the identifier of
/// what it names?": the file holds ids, the agent writes words, and `approve`
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
    let mut rf = record::load(file).map_err(|e| {
        msg::approve_prefixed(msg::read_failed(
            &file.display().to_string(),
            e,
            Next::owner_runs(format!("adr approve {} again", rel)),
        ))
    })?;

    // Refuse up front, before a single `depends:` entry is resolved, when
    // adrdom/terms.md is missing, shape-broken, or not yet approved: the
    // same gate `ask`/`answer`/`refuse` apply via `require_terms`. Without
    // this, a real term with no id minted yet (an unapproved terms.md mints
    // nothing to disk) silently fails to resolve and every depends entry
    // naming it refuses "which is no term" instead of the one clear "terms
    // not approved" line.
    let tf = require_terms(root, led, rel).map_err(msg::approve_prefixed)?;
    let terms_list: &[terms::Term] = &tf.terms;
    // Built once per command run (not per depends entry): `resolve_dep`
    // used to call `terms::resolve_word`, which rebuilt this same map from
    // scratch on every entry.
    let phrase_lookup = terms::phrase_lookup(terms_list);

    // Every topic file, loaded once here rather than twice: this used to be
    // loaded again, further down, only for `missing_answered_refusals` --
    // after `cross_file_heading_index` had already loaded every file
    // itself to build its own copy of the same thing. Loaded via
    // `record::load_all` (root-relative `.rel`, matching what `cross_file_heading_index`
    // always printed) straight off disk, still unwritten at this point, so
    // its ids are exactly what's on disk; minting new ids for skeleton
    // headings below never removes an id already in the ledger's
    // `state_ids`, so loading before that rewrite changes nothing either
    // read below depends on.
    let all_records: Vec<RecordFile> = record::load_all(root);
    // Every current decision's heading text, repo-wide (the cross-file gap
    // "Does a reference in a topic file carry the identifier of what it
    // names?" left open): built once here too, from `all_records` above,
    // and reused for the normalize pass below and for every `resolve_dep`
    // call this function makes.
    let cross_file_headings = depends::cross_file_heading_index(&all_records, led);

    // Heading-shaped `depends:` entries that name no heading in `rf` but
    // resolve to exactly one current decision elsewhere are rewritten to
    // `Id` entries in memory, and `rf.tiers`/`rf.derive_error` recomputed,
    // before `record_refusals` runs -- otherwise a legitimate cross-file
    // reference would still trip the file-wide `derive_error` a plain
    // `Text` heading dep always has when it names no heading in its own
    // file (`record::derive_tiers` has no cross-file/ledger knowledge of
    // its own). An entry that matches more than one current decision
    // elsewhere becomes `rf.derive_error` instead
    // (`DeriveError::AmbiguousHeading`), which `record_refusals` below
    // turns into this file's "plan" refusal -- no side list to fold in
    // here any more.
    depends::normalize_cross_file_deps(&mut rf, terms_list, &phrase_lookup, &cross_file_headings, led);

    // Every refusal this file can earn -- shape, tells, a missing
    // depends line, depends resolution (including a frozen section's
    // edit and the derived-tiers error), and the tier cap -- collected
    // here (`record_refusals`, shared with `check`) instead of returned at
    // the first, so one `approve` call tells the agent everything it owes
    // the owner before asking again.
    let mut refusals: Vec<String> =
        record_refusals(rel, &rf, terms_list, &phrase_lookup, &cross_file_headings, led)
            .into_iter()
            .map(|r| msg::approve_prefixed(r.message))
            .collect();

    // A letter-set suffix naming a current decision elsewhere
    // (`DepTarget::Decision`, an already-id entry or a heading-shaped one
    // the normalize pass above just resolved) must still only name that
    // decision's own option letters -- `bad_letters` alone never sees this
    // case, since its target always comes from `rf` itself. `check`'s own
    // `plan` check makes the same test (and, for a same-file
    // `DepTarget::Heading`, only `check` makes it -- see its own comment
    // for why that half stays a check-only extra).
    for sec in &rf.sections {
        for d in &sec.depends {
            let Some(record::AnswerSet::Letters(letters)) = d.set() else {
                continue;
            };
            if let Ok(Some(DepTarget::Decision(id))) =
                depends::resolve_dep(d, sec, &rf, terms_list, &phrase_lookup, &cross_file_headings, led)
            {
                for m in
                    depends::bad_letters_for_decision(rel, &sec.heading_text, &all_records, &id, letters)
                {
                    refusals.push(msg::approve_prefixed(m));
                }
            }
        }
    }

    // Ledger ids (asked/answered/refused/superseded/withdrawn) that now
    // appear in no topic file at all: withdraw the asked/refused ones,
    // refuse on a current (answered, not superseded) one via
    // `missing_answered_refusals` (shared with `check`'s `ids` category),
    // allow a superseded one silently. Computed and checked here, before
    // any write below, so a refusal on this leaves both the topic file and
    // the ledger exactly as they were (the function's own contract:
    // nothing is written when a check refuses). `all_records`, loaded
    // above, is still exactly what's on disk at this point.
    for r in missing_answered_refusals(&all_records, led) {
        refusals.push(msg::approve_prefixed(r.message));
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
        // struck asked one (per "Does approve withdraw an asked question
        // struck from the plan?"): the refusal already closed it with no
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
    // (per "Does a reference line hold its whole list on one line?"), entry
    // by entry, keeping any `=set` suffix.
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
            .map(|d| rewrite_dep_entry(d, sec, &work_rf, terms_list, &phrase_lookup, &cross_file_headings, led))
            .collect();
        work_rf.lines[line_idx] = format!("depends: {}", parts.join(", "));
    }
    // Reparse once more so `plan_hash`/`context_hash` and everything below
    // sees the file exactly as it will be written.
    let work_rf = record::build(work_rf.path.clone(), rel.to_string(), work_rf.lines.clone());

    let plan_hash = record::plan_hash(&work_rf);
    let context_hash = record::context_hash(&work_rf);

    record::save(&work_rf).map_err(|e| {
        msg::approve_prefixed(msg::write_failed(
            &file.display().to_string(),
            e,
            Next::owner_runs(format!("adr approve {} again", rel)),
        ))
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
        msg::approve_prefixed(msg::update_failed(
            "ledger",
            e,
            Next::owner_runs(format!("adr approve {} again", rel)),
        ))
    })?;

    // `withdrawn_rows` was computed, and any refusal already returned,
    // above -- before either write this function has made so far.
    if !withdrawn_rows.is_empty() {
        ledger::append(ledger_path, &withdrawn_rows).map_err(|e| {
            msg::approve_prefixed(msg::update_failed(
                "ledger",
                e,
                Next::owner_runs(format!("adr approve {} again", rel)),
            ))
        })?;
        for r in &withdrawn_rows {
            messages.push(msg::withdrawn(&r.detail));
        }
    }

    // Printed last, after a successful record approval: the tiers this
    // file now derives to, one `N (count)` per non-empty tier, live counts
    // only (per "Does the tool cap the questions in a tier?"; matches
    // `check`'s own "tiers: ..." note, computed the same way via
    // `state::live_tiers`). `work_rf.tiers`
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
    messages.push(msg::tiers_note_approve(&tiers_str).trim_end().to_string());

    Ok(Outcome {
        summary: messages.join("\n"),
    })
}

/// The final text one `depends:` entry rewrites to when `approve` rewrites
/// a section's single `depends:` line (per "Does a reference line hold its
/// whole list on one line?"): an already-resolved
/// `Id` entry keeps its id and set exactly as written; a `Text` entry
/// resolves to the id its target already carries in `work_rf` (every entry
/// was already validated above, so this should always find one), keeping
/// its own written text as a fallback rather than silently dropping the
/// entry on the one resolution rule this can't see (a dangling/cycle/
/// self-dep heading reference, reported once from `rf.derive_error` instead
/// of per entry).
fn rewrite_dep_entry(
    d: &record::Dep,
    sec: &record::Section,
    work_rf: &RecordFile,
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> String {
    match d {
        record::Dep::Id { id, set } => format!("{}{}", id, record::set_suffix(set)),
        record::Dep::Text { text, set } => {
            let resolved = depends::resolve_dep(
                d,
                sec,
                work_rf,
                terms_list,
                phrase_lookup,
                cross_file_headings,
                led,
            )
                .ok()
                .flatten()
                .and_then(|target| match target {
                    DepTarget::Heading(idx) => work_rf.sections[idx].id.clone(),
                    DepTarget::Term(id) | DepTarget::Decision(id) => Some(id),
                });
            match resolved {
                Some(id) => format!("{}{}", id, record::set_suffix(set)),
                None => format!("{}{}", text, record::set_suffix(set)),
            }
        }
    }
}

pub fn do_unban(root: &Path, input: &str) -> Result<Outcome, String> {
    let banned_path = root.join(crate::banned::BAN_LIST);
    let ledger_path = root.join(crate::repo::LEDGER_PATH);

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
