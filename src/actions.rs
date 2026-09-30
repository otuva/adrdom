//! One code path for every owner act: the CLI and the page both call these
//! (per "Does the adr binary serve the ui?").
use crate::blame;
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

    // Loaded one file at a time, stopping at the first match, rather than
    // `record::load_all` (every topic file, parsed up front): an id names
    // exactly one section in exactly one file, so answer/refuse -- unlike
    // check or ask, which always need every file -- never has a reason to
    // parse the rest once it is found.
    let mut found = None;
    // The files that could not be read: the question may be in one of
    // them, so "not found" names them.
    let mut unread: Vec<String> = Vec::new();
    for f in crate::repo::list_decision_files(root) {
        match record::load(&f) {
            Ok(rf) => {
                if let Some(idx) = rf.sections.iter().position(|s| s.id.as_deref() == Some(id)) {
                    found = Some((rf, idx));
                    break;
                }
            }
            Err(_) => unread.push(crate::state::rel_path(root, &f)),
        }
    }
    let Some((rf, idx)) = found else {
        if unread.is_empty() {
            return Err(msg::id_not_found(id));
        }
        return Err(msg::id_not_found_some_unread(id, &unread));
    };
    // `record::load` (unlike `load_all`/`load_all_via`) leaves `rf.rel` as
    // the raw path it was given, so it is recomputed here the way every
    // other caller of the single-file loader already does.
    let rel = crate::state::rel_path(root, &rf.path);

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
                let letters: Vec<char> = sec.options.iter().map(|o| o.letter).collect();
                return Err(msg::letter_not_an_option(
                    letter,
                    &sec.heading_text,
                    &letters,
                    id,
                    version,
                ));
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
        // The text stands as it was asked (the version the owner would read
        // is the current one) or it does not (never asked, or edited since:
        // the agent's `ask` is the move and the owner has nothing to run).
        let stands = crate::state::asked_text_stands(&led, id, &cur_hash);
        if !stands {
            return Err(if sec.is_skeleton {
                msg::not_written(&sec.heading_text)
            } else if led.latest_asked(id).is_none() {
                msg::not_asked_yet(&sec.heading_text, &rel)
            } else {
                match led.latest_state(id).map(|r| r.event.as_str()) {
                    Some("withdrawn") => msg::already_withdrawn(&sec.heading_text),
                    Some("answered") => msg::already_answered(&sec.heading_text),
                    Some("refused") => msg::already_refused(&sec.heading_text),
                    Some("superseded") => msg::already_superseded(&sec.heading_text),
                    _ => msg::not_in_asked_state(&sec.heading_text, &rel),
                }
            });
        }
        if version_lower != cur_hash[..record::VERSION_LEN] {
            return Err(msg::stale_version(&sec.heading_text, &rel));
        }
    }

    Ok(Located { led, rf, idx, rel, tf, cur_hash })
}

/// The write step `do_answer` and `do_refuse` both end with: splice the
/// `**Answer:** <answer_text>` line in at the end of the section's hash
/// range, save the topic file, then append `rows` to the ledger -- with the
/// same error handling and the same "try again" command shape either way,
/// `retry_words` filling in the part that differs (`"A again"` for the
/// letter, `"refuse again"` for a refusal). Eight plain, once-each
/// arguments over a params struct: the same shape `check_plan` accepts
/// this same lint for.
#[allow(clippy::too_many_arguments)]
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
            return Err(msg::no_terms_line(id, &sec.heading_text));
        };
        let problems = terms::validate_pairs(rf.lines[terms_line_idx].trim_end(), &tf)
            .map_err(|_| msg::malformed_terms_line(id, &sec.heading_text))?;
        if let Some(p) = problems.first() {
            return Err(match &p.kind {
                terms::ProblemKind::Gone => {
                    msg::terms_line_names_gone_term(id, &sec.heading_text, &p.uuid, &p.word)
                }
                terms::ProblemKind::Retired(name) => {
                    msg::terms_line_names_retired_term(id, &sec.heading_text, name)
                }
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
                let names = crate::view::all_heading_texts(root);
                return Err(msg::supersedes_not_current(old, names.get(old).map(String::as_str)));
            }
        }
        let all = record::load_all(root);
        let missing = depends::left_out_of_supersedes(&all, &led, &supersedes_val);
        if !missing.is_empty() {
            return Err(msg::supersedes_leaves_out(&rf.rel, &sec.heading_text, &missing));
        }
    }

    let heading_for_print = rf.sections[idx].heading_text.clone();
    let sentence_for_print = rf.sections[idx]
        .options
        .iter()
        .find(|o| o.letter == letter)
        .and_then(|o| o.sentence.clone())
        .unwrap_or_default();

    // One time for every row this run writes (per "Does a command write a
    // time later than the last ledger row?", aefa7f3c-70c3-412e-8d73-16ff9e2b703a
    // A): the "answered" row and any "superseded" rows it carries all share
    // it, taken once here from `led` rather than the clock directly.
    let write_time = ledger::write_time(&led);
    let mut rows = vec![ledger::Row {
        time: write_time.clone(),
        event: "answered".to_string(),
        id: id.to_string(),
        hash: cur_hash,
        detail: letter.to_string(),
        version: crate::version::VERSION.to_string(),
    }];
    for old in &supersedes_val {
        rows.push(ledger::Row {
            time: write_time.clone(),
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
    let Located { led, rf, idx, rel, cur_hash, .. } = locate_asked(root, id, version, None)?;

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
        time: ledger::write_time(&led),
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
    } else if rel.starts_with(&format!("{}/", crate::repo::BLAME_DIR)) {
        do_approve_blame(root, file, &rel, &led, &ledger_path)
    } else {
        do_approve_record(root, file, &rel, &led, &ledger_path)
    }
}

/// `adrdom/blame/<stem>.md` (per "Does the owner approve a blame file?",
/// e879fe72 A): parse the fixed shape, read the origin's content at the
/// blob the file names, check that the rows' ranges cover it with no gap
/// (5c399a14 A), resolve every row's destination against the closed set
/// (38216d39 A) -- collecting every refusal at once, nothing written to
/// disk on any of them, the same contract `do_approve_record` keeps -- then
/// rewrite every row to its canonical (id-bearing, where the set names one)
/// form, save, and write one `blame` ledger row over the saved content's
/// hash unless it already matches the latest one for this path.
fn do_approve_blame(
    root: &Path,
    file: &Path,
    rel: &str,
    led: &Ledger,
    ledger_path: &Path,
) -> Result<Outcome, String> {
    let bf = blame::load(file, rel).map_err(|e| {
        msg::approve_prefixed(msg::read_failed(
            &file.display().to_string(),
            e,
            Next::owner_runs(format!("adr approve {}", rel)),
        ))
    })?;

    let mut refusals: Vec<String> = bf
        .shape_errors
        .iter()
        .map(|e| msg::approve_prefixed(msg::blame_shape_error(rel, e)))
        .collect();
    if !refusals.is_empty() {
        return Err(refusals.join("\n"));
    }

    // Shape is clean, so both header fields are present (`blame::parse`
    // only leaves either `None` alongside the shape error that reports it).
    let blob = bf.blob.clone().expect("no shape error means blob is set");

    // The origin's content comes from the file at its path when it is
    // there, checked against the blob line by `git hash-object`; only a
    // gone origin falls back to the object store. A blob reaches the store
    // only through a stage or a commit, and a fresh repository's first
    // blame file is approved before either.
    let origin_rel = bf.origin.clone().expect("no shape error means origin is set");
    if !blame::valid_origin_path(root, &repo::Source::WorkingTree, &origin_rel) {
        return Err(msg::approve_prefixed(msg::blame_bad_origin_path(rel, &origin_rel)));
    }
    let origin_path = root.join(&origin_rel);
    let content = if origin_path.exists() {
        // Present: must read as text, or this refuses outright (fail
        // closed, never a silent fall-through to the object store, which
        // would let a present-but-corrupt origin sail through unnoticed).
        match std::fs::read_to_string(&origin_path) {
            Ok(text) => {
                let actual = repo::git_hash_object_stdin(root, &text).unwrap_or_default();
                if actual != blob {
                    return Err(msg::approve_prefixed(msg::blame_blob_differs(rel, &origin_rel)));
                }
                text
            }
            Err(e) => {
                return Err(msg::approve_prefixed(msg::read_failed(
                    &origin_rel,
                    e,
                    Next::agent("fixes the origin file so it reads as text (e.g. its encoding)"),
                )))
            }
        }
    } else {
        // Gone: fall back to the object store -- only reached when the
        // path truly has nothing at all, never for a present file this
        // just failed to read.
        match repo::git(root, &["cat-file", "-p", &blob]) {
            Some(text) => text,
            None => return Err(msg::approve_prefixed(msg::blame_blob_not_found(rel, &blob))),
        }
    };
    let n = content.lines().count();

    let (beyond, gaps) = blame::coverage_errors(&bf.rows, n);
    for b in &beyond {
        refusals.push(msg::approve_prefixed(msg::blame_range_beyond(rel, b.start, b.end, n)));
    }
    for g in &gaps {
        refusals.push(msg::approve_prefixed(msg::blame_coverage_gap(rel, g.start, g.end)));
    }

    // Every set `resolve_destination` reads, built once for this file's own
    // rows rather than once per row.
    let all_records: Vec<RecordFile> = record::load_all(root);
    let (any_headings, known_heading_ids) = blame::heading_ids(&all_records);
    let terms_list: Vec<terms::Term> = state::load_terms(root)
        .map(|tf| terms::minted_copy(&tf).terms)
        .unwrap_or_default();
    let phrase_lookup = terms::phrase_lookup(&terms_list);
    let decision_stems: HashSet<String> = repo::list_decision_files(root)
        .iter()
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_string))
        .collect();
    let known_origins = blame::known_origins(root);

    let ctx = blame::ResolveCtx {
        any_headings,
        known_heading_ids,
        terms_list: terms_list.as_slice(),
        phrase_lookup: &phrase_lookup,
        decision_stems,
        known_origins,
        own_origin: origin_rel.clone(),
        root: root.to_path_buf(),
    };

    let mut resolved: Vec<blame::Resolved> = Vec::new();
    for row in &bf.rows {
        match blame::resolve_destination(&row.dest, &ctx) {
            Ok(r) => resolved.push(r),
            Err(e) => refusals.push(msg::approve_prefixed(blame::dest_refusal_text(rel, row, &e))),
        }
    }

    if !refusals.is_empty() {
        return Err(refusals.join("\n"));
    }

    let mut work_lines = bf.lines.clone();
    for (row, r) in bf.rows.iter().zip(resolved.iter()) {
        work_lines[row.line_idx] = format!("{}-{}\t{}", row.start, row.end, r.canonical());
    }

    // Written only when a row changed, so the line below can say whether
    // the file on disk was touched.
    let rewritten = work_lines != bf.lines;
    if rewritten {
        std::fs::write(&bf.path, record::join_lines(&work_lines)).map_err(|e| {
            msg::approve_prefixed(msg::write_failed(
                &file.display().to_string(),
                e,
                Next::owner_runs(format!("adr approve {}", rel)),
            ))
        })?;
    }

    let new_hash = blame::file_hash(&work_lines);
    let unchanged = led.latest_for_path("blame", rel).map(|r| r.hash.as_str()) == Some(new_hash.as_str());

    let summary = if unchanged && rewritten {
        msg::blame_rows_rewritten(rel)
    } else if unchanged {
        msg::blame_unchanged(rel)
    } else {
        let row = ledger::Row {
            time: ledger::write_time(led),
            event: "blame".to_string(),
            id: "-".to_string(),
            hash: new_hash,
            detail: rel.to_string(),
            version: crate::version::VERSION.to_string(),
        };
        ledger::append(ledger_path, &[row]).map_err(|e| {
            msg::approve_prefixed(msg::update_failed(
                "ledger",
                e,
                Next::owner_runs(format!("adr approve {}", rel)),
            ))
        })?;
        msg::blame_approved(rel)
    };

    Ok(Outcome { summary })
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
    approved_plans: &std::collections::HashSet<String>,
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
    // widest reach of `state::dependent_entries`' three readers.
    // Asked questions and not-yet-asked headings of approved plans (in the
    // files of `approved_plans`, read as written on disk) name terms too.
    let mut dependents: Vec<state::DependentEntry> =
        state::dependent_entries(records, led, tf, true, true);
    dependents.extend(state::unsettled_entries(records, led, tf, approved_plans));

    // Step 4: every pair on a dependent must still hold in the new file.
    // `lookup` answers "what does this exact word/phrase resolve to" for
    // every dependent pair, built once rather than once per pair.
    let lookup = terms::phrase_lookup(&tf.terms);
    for dep in &dependents {
        let label = dep.source.label();
        for (word, uuid) in &dep.pairs {
            let Some(target) = tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str()))
            else {
                let word = dep.from_word.then_some(word.as_str());
                refusals.push(msg::term_removed_but_carried(uuid, word, &label));
                continue;
            };
            if target.retired {
                // A decision dependent: the agent's move is to write a new
                // question that supersedes it, not the owner's to make on
                // the spot. A term dependent (its own definition still
                // carries the retired word) is never superseded (per "Does
                // a retired term keep its words?"): the agent rewords the
                // definition and the owner re-approves.
                let next = match &dep.source {
                    state::DependentSource::Decision { id, heading, .. } => {
                        msg::term_retired_decision_next(id, heading)
                    }
                    state::DependentSource::Asked { rel, heading } => {
                        msg::term_retired_asked_next(heading, &target.name, rel)
                    }
                    state::DependentSource::PlanHeading { rel, .. } => {
                        msg::term_retired_heading_next(rel)
                    }
                    state::DependentSource::Term { .. } => msg::term_reword_and_approve(),
                };
                refusals.push(msg::term_retired_but_carried(&target.name, &label, next));
                continue;
            }
            // A `depends:`-line dependent's pair is an id read straight off
            // the entry, never a resolved word -- there is no word to
            // drift, only the target's removal/retirement above, so this
            // check is skipped for it (`from_word` is false).
            let is_decision_or_term = matches!(
                dep.source,
                state::DependentSource::Decision { .. } | state::DependentSource::Term { .. }
            );
            if dep.from_word && is_decision_or_term {
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
                            "restores the wording in adrdom/terms.md so \"{}\" still names \"{}\", or writes a question that supersedes {}",
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
            Next::owner_runs("adr approve adrdom/terms.md"),
        ))
    })?;

    // Every topic file, parsed once here and reused by `terms_refusals`
    // (no re-reading them from disk itself).
    let records: Vec<RecordFile> = record::load_all(root);
    // Files whose plan is approved, judged on the files as written (before
    // any normalization).
    let approved_plans: std::collections::HashSet<String> = records
        .iter()
        .filter(|rf| state::plan_approved(led, rf, &rf.rel))
        .map(|rf| rf.rel.clone())
        .collect();

    // Step 2: mint ids in memory only; nothing is written to disk yet.
    let inserted = terms::mint_ids(&mut tf);

    // Steps 1 (shape, post-mint), 3 (cycles), 4 (dependents), 5 (banned):
    // every refusal, all at once.
    let (refusals, pairs_per_entry) =
        terms_refusals(root, &records, &tf, led, &repo::Source::WorkingTree, true, &approved_plans);
    if !refusals.is_empty() {
        let msg = refusals
            .iter()
            .map(msg::approve_prefixed)
            .collect::<Vec<_>>()
            .join("\n");
        return Err(msg);
    }

    // Which terms changed since their latest `term` row (information only),
    // and which need a new row. A term with no `term` row yet is new: it
    // gets a row and prints nothing.
    let term_hashes: Vec<(String, String, String)> = tf
        .terms
        .iter()
        .filter_map(|t| t.id.clone().map(|id| (id, t.name.clone(), terms::term_hash(t))))
        .collect();
    let mut term_rows_due: Vec<(String, String, String)> = Vec::new();
    let mut changed: Vec<(String, String)> = Vec::new();
    for (id, name, hash) in &term_hashes {
        match led.latest_for_id("term", id) {
            Some(r) if r.hash == *hash => {}
            Some(_) => {
                changed.push((id.clone(), name.clone()));
                term_rows_due.push((id.clone(), name.clone(), hash.clone()));
            }
            None => term_rows_due.push((id.clone(), name.clone(), hash.clone())),
        }
    }
    let mut changed_lines: Vec<String> = Vec::new();
    if !changed.is_empty() {
        let mut entries = state::dependent_entries(&records, led, &tf, true, false);
        entries.extend(state::unsettled_entries(&records, led, &tf, &approved_plans));
        for (id, name) in &changed {
            let mut named_by: Vec<String> = Vec::new();
            for e in &entries {
                if matches!(e.source, state::DependentSource::Term { .. }) {
                    continue;
                }
                if e.pairs.iter().any(|(_, u)| u == id) {
                    let label = e.source.label();
                    if !named_by.contains(&label) {
                        named_by.push(label);
                    }
                }
            }
            changed_lines.push(msg::term_definition_changed(name, &named_by));
        }
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
            Next::owner_runs("adr approve adrdom/terms.md"),
        ))
    })?;

    // Per "Does approve write the terms brief?" (fcab4bf9 A): written on
    // every run, whether or not the terms file changed, so a repository
    // whose terms file is approved gets its terms brief from one run.
    let brief_path = root.join(crate::repo::TERMS_BRIEF_PATH);
    let write_brief = || -> std::io::Result<()> {
        if let Some(dir) = brief_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&brief_path, terms::brief_text(&tf))
    };
    write_brief().map_err(|e| {
        msg::approve_prefixed(msg::write_failed(
            crate::repo::TERMS_BRIEF_PATH,
            e,
            Next::owner_runs("adr approve adrdom/terms.md"),
        ))
    })?;

    let mut out: Vec<String> = Vec::new();
    if inserted > 0 {
        out.push(msg::terms_ids_filled(inserted));
    }
    out.extend(changed_lines);

    let write_time = ledger::write_time(led);
    let term_rows: Vec<ledger::Row> = term_rows_due
        .iter()
        .map(|(id, name, hash)| ledger::Row {
            time: write_time.clone(),
            event: "term".to_string(),
            id: id.clone(),
            hash: hash.clone(),
            detail: name.clone(),
            version: crate::version::VERSION.to_string(),
        })
        .collect();
    let append_failed = |e: std::io::Error| {
        msg::approve_prefixed(msg::update_failed(
            "ledger",
            e,
            Next::owner_runs("adr approve adrdom/terms.md"),
        ))
    };

    if state::terms_approved(led, &tf) {
        // The file already matches the ledger's latest `terms` row:
        // nothing is written. A ledger from before `term` rows existed
        // gets them at its next real approval, with no change listed
        // for that one edit (no migration code in the tool).
        out.push(msg::terms_already_approved());
    } else {
        let row = ledger::Row {
            time: write_time.clone(),
            event: "terms".to_string(),
            id: "-".to_string(),
            hash: terms::file_hash(&tf),
            detail: crate::repo::TERMS_PATH.to_string(),
            version: crate::version::VERSION.to_string(),
        };
        let mut rows = vec![row];
        rows.extend(term_rows);
        ledger::append(ledger_path, &rows).map_err(append_failed)?;
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
///
/// `names` is every heading by its id, of any state and any topic file
/// (`view::all_heading_texts`), built once per command run by the caller:
/// a refusal that names an identifier names the question beside it.
/// `all` is every topic file loaded: a heading-shaped entry that names no
/// current decision but a question of another topic file that is not
/// answered (or is refused) is refused with the state the tool holds.
#[allow(clippy::too_many_arguments)]
pub fn record_refusals(
    rel: &str,
    rf: &RecordFile,
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    names: &HashMap<String, String>,
    all: &[RecordFile],
    led: &Ledger,
) -> Vec<Refusal> {
    let mut out: Vec<Refusal> = Vec::new();

    for e in &rf.shape_errors {
        out.push(Refusal {
            category: "parse",
            message: msg::shape_error(rel, e),
        });
    }
    for i in record::non_question_heading_lines(&rf.lines) {
        out.push(Refusal {
            category: "parse",
            message: msg::heading_not_a_question(rel, i + 1),
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
        // A frozen section whose text (the depends line included) is no
        // longer what the ledger holds: `check` fails it right after, so
        // approve refuses it before writing anything. A `Text` entry in
        // it keeps its own refusal below.
        if frozen
            && !sec.depends.iter().any(|d| matches!(d, record::Dep::Text { .. }))
            && state::frozen_text_edited(led, &rf.lines, sec)
        {
            out.push(Refusal {
                category: "plan",
                message: msg::answered_text_changed(
                    rel,
                    &sec.heading_text,
                    sec.id.as_deref().unwrap_or(""),
                ),
            });
        }
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
            // A dead section (refused, superseded, withdrawn) is history:
            // what its line stood on when it was live is not resolved
            // against the current decisions again, the same exemption
            // `resolve_dep` gives a retired term.
            if state::is_dead(led, sec) {
                continue;
            }
            if let Err(e) =
                depends::resolve_dep(d, sec, rf, terms_list, phrase_lookup, cross_file_headings, led)
            {
                // What the identifier names, for the refusal only: a term
                // (a retired one too), else the heading of a question in
                // any state.
                let name = match &e {
                    depends::DepError::NotCurrentDecision(id) => terms_list
                        .iter()
                        .find(|t| t.id.as_deref() == Some(id.as_str()))
                        .map(|t| t.name.as_str())
                        .or_else(|| names.get(id).map(String::as_str)),
                    _ => None,
                };
                out.push(Refusal {
                    category: "plan",
                    message: depends::dep_refusal_text(rel, &sec.heading_text, &e, name),
                });
            }
        }
    }
    if let Some(err) = &rf.derive_error {
        let waiting = match err {
            record::DeriveError::Dangling { section, entry } => {
                waiting_on_other_file(rel, section, entry, all, led)
            }
            _ => None,
        };
        out.push(Refusal {
            category: "plan",
            message: waiting.unwrap_or_else(|| msg::derive_error(rel, err)),
        });
    }

    // Cap (per "Does the tool cap the questions in a tier?"): no derived tier may hold more than `TIER_CAP` live
    // questions. Only checkable when tiers were actually derived: a
    // `derive_error` above leaves `rf.tiers` `None`, already refused
    // there. A refused/superseded/withdrawn question is dead and does not
    // count (`state::live_tiers`).
    if let Some(tiers) = &rf.tiers {
        let replaced = depends::replaced_ids(all, rf, cross_file_headings, led);
        if let Some((open, answered)) = state::over_cap_headings(led, rf, tiers, &replaced) {
            out.push(Refusal {
                category: "plan",
                message: msg::tier_cap_exceeded(rel, &open, answered, record::TIER_CAP),
            });
        }
    }

    out
}

/// The refusal for a heading-shaped `depends:` entry that names a question
/// of another topic file that is not a current decision: not answered yet
/// (unasked or asked), or refused. `None` when no other file holds an
/// unanswered or refused question under that heading.
fn waiting_on_other_file(
    rel: &str,
    section: &str,
    entry: &str,
    all: &[RecordFile],
    led: &Ledger,
) -> Option<String> {
    let mut refused: Option<&str> = None;
    for other in all.iter().filter(|o| o.rel != rel) {
        for sec in other.sections.iter().filter(|s| s.heading_text.trim() == entry.trim()) {
            match crate::view::qstate(led, sec) {
                crate::view::QState::Skeleton
                | crate::view::QState::Unasked
                | crate::view::QState::Open => {
                    return Some(msg::dep_waits_on_other_file(rel, section, entry, &other.rel));
                }
                crate::view::QState::Refused => refused = Some(other.rel.as_str()),
                _ => {}
            }
        }
    }
    refused.map(|r| msg::dep_on_refused_in_other_file(rel, section, entry, r))
}

/// Refusals about a topic file's own name and its own headings, one
/// function for `approve` and `check` so the two print the same lines
/// (76bd7ef0 A): a name that is not lowercase words joined by hyphens, a
/// topic no name can carry, a name that is not the topic, and a heading
/// text used twice in the file. `topic` is the file's `# ` title when it
/// has one.
pub fn own_name_and_heading_refusals(
    rel: &str,
    topic: Option<&str>,
    sections: &[record::Section],
) -> Vec<String> {
    let mut out = own_name_refusals(rel, topic);
    out.extend(duplicate_heading_refusals(rel, sections));
    out
}

/// The naming half of `own_name_and_heading_refusals`.
pub fn own_name_refusals(rel: &str, topic: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    let name = rel.rsplit('/').next().unwrap_or(rel);
    if !repo::valid_topic_file_name(name) {
        out.push(msg::bad_topic_file_name(rel));
    } else if let Some(topic) = topic {
        let expected = repo::expected_topic_file_stem(topic);
        if !repo::valid_topic_file_name(&format!("{}.md", expected)) {
            out.push(msg::topic_unnameable(rel, topic));
        } else if name.strip_suffix(".md") != Some(expected.as_str()) {
            out.push(msg::topic_file_name_mismatch(rel, topic, &expected));
        }
    }
    out
}

/// The in-file duplicate heading half of `own_name_and_heading_refusals`.
pub fn duplicate_heading_refusals(rel: &str, sections: &[record::Section]) -> Vec<String> {
    let mut out = Vec::new();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for sec in sections {
        *counts.entry(sec.heading_text.as_str()).or_insert(0) += 1;
    }
    let mut dups: Vec<&str> = counts.into_iter().filter(|(_, c)| *c > 1).map(|(h, _)| h).collect();
    dups.sort();
    for h in dups {
        out.push(msg::duplicate_heading_in_file(h, rel));
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
/// `depends:` line (or its `supersedes:` line, directly under it) (per "Does a reference in a topic file carry the
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
            // The plan lines come first, then the id: after the
            // `supersedes:` line when the heading has one (which is
            // directly under `depends:`), else after `depends:`.
            let after = if work_lines.get(depends_idx + 1).is_some_and(|l| l.starts_with("supersedes: ")) {
                depends_idx + 1
            } else {
                depends_idx
            };
            let new_id = Uuid::new_v4().to_string();
            work_lines.insert(after + 1, format!("id: {}", new_id));
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
            Next::owner_runs(format!("adr approve {}", rel)),
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
    let names = crate::view::heading_texts(&all_records);
    let mut refusals: Vec<String> =
        record_refusals(rel, &rf, terms_list, &phrase_lookup, &cross_file_headings, &names, &all_records, led)
            .into_iter()
            .map(|r| msg::approve_prefixed(r.message))
            .collect();

    for m in own_name_and_heading_refusals(rel, rf.topic.as_deref(), &rf.sections) {
        refusals.push(msg::approve_prefixed(m));
    }

    // Per "Does approve turn the supersedes line into identifiers?" (B):
    // each entry of a `supersedes:` line is the heading of a current
    // decision (in any topic file) or its id, and becomes the id, the way
    // a `depends:` entry does. A section that is already frozen keeps its
    // line as it is: its target is superseded by now, and the section's
    // own hash guards the line.
    let mut supersedes_ids: HashMap<usize, Vec<String>> = HashMap::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        // Once asked, the line is frozen with the body (per "Does one
        // decision supersede several?"): a split sibling's target may be
        // superseded by now, and approve leaves the line as it is.
        // An asked section whose text no longer stands as asked is being
        // edited to be asked again: its line is turned like an unasked one's.
        let stands_as_asked = sec.id.as_ref().is_some_and(|id| {
            state::asked_text_stands(led, id, &record::section_hash(&rf.lines, sec))
        });
        if sec.supersedes.is_empty() || stands_as_asked || state::is_frozen(led, sec) {
            continue;
        }
        let mut ids = Vec::new();
        for old in &depends::supersedes_entries(&sec.supersedes, &cross_file_headings, led) {
            match depends::resolve_supersedes(old, &cross_file_headings, led) {
                Ok(id) => ids.push(id),
                Err(depends::SupersedesError::NotCurrent) => refusals.push(msg::approve_prefixed(
                    msg::ask_supersedes_not_current(
                        rel,
                        &sec.heading_text,
                        old,
                        names.get(old).map(String::as_str),
                    ),
                )),
                Err(depends::SupersedesError::Ambiguous(files)) => refusals.push(msg::approve_prefixed(
                    msg::ask_supersedes_ambiguous(rel, &sec.heading_text, old, &files),
                )),
            }
        }
        supersedes_ids.insert(i, ids);
    }

    // The supersedes line names every current decision standing on what it
    // replaces (through the ones standing on those), and nothing of the plan
    // stands on what the plan replaces (796a2544): collected with the other
    // refusals, before anything is written.
    {
        let stands_on = |d: &record::Dep, target: &str| -> bool {
            match d {
                record::Dep::Id { id, .. } => id == target,
                record::Dep::Text { text, .. } => rf
                    .sections
                    .iter()
                    .any(|s| s.id.as_deref() == Some(target) && s.heading_text.trim() == text.trim()),
            }
        };
        let mut idxs: Vec<&usize> = supersedes_ids.keys().collect();
        idxs.sort();
        for &i in idxs {
            let sec = &rf.sections[i];
            let ids = &supersedes_ids[&i];
            let missing = depends::left_out_of_supersedes(&all_records, led, ids);
            if !missing.is_empty() {
                refusals.push(msg::approve_prefixed(msg::supersedes_leaves_out(
                    rel,
                    &sec.heading_text,
                    &missing,
                )));
            }
            for target in ids {
                let target_name = names.get(target).map(String::as_str).unwrap_or("");
                if sec.depends.iter().any(|d| stands_on(d, target)) {
                    refusals.push(msg::approve_prefixed(msg::stands_on_what_it_supersedes(
                        rel,
                        &sec.heading_text,
                        target_name,
                    )));
                }
                for (j, other) in rf.sections.iter().enumerate() {
                    let open = other.id.as_deref().map_or(true, |id| !state::is_current(led, id))
                        && !state::is_dead(led, other);
                    if j != i && open && other.depends.iter().any(|d| stands_on(d, target)) {
                        refusals.push(msg::approve_prefixed(msg::stands_on_what_the_plan_supersedes(
                            rel,
                            &other.heading_text,
                            target_name,
                            &sec.heading_text,
                        )));
                    }
                }
            }
        }
    }

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

    // One time for every row this run writes (per "Does a command write a
    // time later than the last ledger row?", aefa7f3c-70c3-412e-8d73-16ff9e2b703a
    // A), taken once here from `led` (loaded before this run made any
    // change) and reused below for both the withdrawn rows and, further
    // down, the approved/context rows -- two separate `append` calls this
    // one run can make, both carrying this same time.
    let write_time = ledger::write_time(led);

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
                    time: write_time.clone(),
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
    // The `supersedes:` line, kept where the writer put it (under `id:`
    // and `terms:` once asked), holds ids from here on.
    for (i, sec) in sections_snapshot.iter().enumerate() {
        let Some(ids) = supersedes_ids.get(&i) else { continue };
        if let Some(j) = (sec.start + 1..sec.end).find(|&j| work_rf.lines[j].starts_with("supersedes: ")) {
            work_rf.lines[j] = format!("supersedes: {}", ids.join(", "));
        }
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
            Next::owner_runs(format!("adr approve {}", rel)),
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
            time: write_time.clone(),
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
            time: write_time.clone(),
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
            Next::owner_runs(format!("adr approve {}", rel)),
        ))
    })?;

    // `withdrawn_rows` was computed, and any refusal already returned,
    // above -- before either write this function has made so far.
    if !withdrawn_rows.is_empty() {
        ledger::append(ledger_path, &withdrawn_rows).map_err(|e| {
            msg::approve_prefixed(msg::update_failed(
                "ledger",
                e,
                Next::owner_runs(format!("adr approve {}", rel)),
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
    let replaced = depends::replaced_ids(&all_records, &work_rf, &cross_file_headings, led);
    let live = state::live_tiers(led, &work_rf, tiers, &replaced);
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
    // A ban list may hold one hash on two lines (a merge, an edit by
    // hand): it is one word, removed once.
    removed.dedup();
    // The words given that were not banned: said as a count beside what
    // was removed, never dropped without a word.
    let not_banned = to_remove.len() - removed.len();
    let not_banned_suffix = if not_banned == 0 {
        String::new()
    } else {
        msg::not_banned_suffix(not_banned)
    };

    if removed.is_empty() {
        return Ok(Outcome {
            summary: format!("{}{}", msg::nothing_to_remove(), not_banned_suffix),
        });
    }

    let remaining: Vec<(String, Option<String>)> = existing
        .into_iter()
        .filter(|(h, _)| !to_remove.contains(h))
        .collect();
    crate::banned::write_sorted(&banned_path, remaining).map_err(|e| {
        msg::update_failed("banned list", e, Next::owner_runs("adr unban"))
    })?;

    // One time for every row this run writes (per "Does a command write a
    // time later than the last ledger row?", aefa7f3c-70c3-412e-8d73-16ff9e2b703a
    // A), whichever hash each row names.
    let led = ledger::read(&ledger_path);
    let write_time = ledger::write_time(&led);
    let rows: Vec<ledger::Row> = removed
        .iter()
        .map(|h| ledger::Row {
            time: write_time.clone(),
            event: "unbanned".to_string(),
            id: "-".to_string(),
            hash: h.clone(),
            detail: String::new(),
            version: crate::version::VERSION.to_string(),
        })
        .collect();
    ledger::append(&ledger_path, &rows)
        .map_err(|e| msg::update_failed("ledger", e, Next::owner_runs("adr unban")))?;

    Ok(Outcome {
        summary: format!("{}{}", msg::removed_entries(removed.len()), not_banned_suffix),
    })
}
