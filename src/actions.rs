//! One code path for every owner act: the CLI and the page both call these
//! (per "Does the adr binary serve the ui?").
use crate::blame;
use crate::depends::{self, DepTarget};
use crate::ledger::{self, Ledger};
use crate::msg::{self, Next};
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state::{self, TermsLoad};
use crate::rules::{self, empty_plan_hash, is_topic_file_path, removal_holds, terms_refusals, RemovalHolds};
use crate::terms;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;

/// What an owner act wrote: one event per ledger row it appended, in the
/// order written (per "What does a command that writes print?", 069a6b4a
/// A). The terminal prints each as one line; the page renders the same
/// events.
pub struct Outcome {
    pub events: Vec<crate::out::Event>,
    /// Findings on what the owner gave the act that stopped none of it
    /// (a word given to unban that was not banned), each with its move:
    /// printed apart from the events, on standard error.
    pub input_findings: Vec<String>,
}

impl Outcome {
    /// The events of `rows`, each named in words by `names` (`event_names`).
    pub fn of_rows(rows: &[ledger::Row], names: &HashMap<String, String>) -> Outcome {
        Outcome {
            events: rows.iter().filter_map(|r| crate::out::Event::from_row(r, names)).collect(),
            input_findings: Vec::new(),
        }
    }
}

/// Every question heading of `all` and term name of `terms` by
/// identifier: what turns the identifiers of the rows an act wrote into
/// words, from what the act already holds.
pub fn event_names(all: &[RecordFile], terms: &[terms::Term]) -> HashMap<String, String> {
    let mut names = crate::view::heading_texts(all);
    names.extend(terms.iter().filter_map(|t| Some((t.id.clone()?, t.name.clone()))));
    names
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
            msg::fix_then_same_command("adrdom/terms.md so it can be read"),
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
        return Err(msg::terms_not_approved(rel, crate::status::terms_file_status(led, &tf)));
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
    let Some((mut rf, idx)) = found else {
        if unread.is_empty() {
            return Err(msg::id_not_found(id));
        }
        return Err(msg::id_not_found_some_unread(id, &unread));
    };
    // `record::load` (unlike `load_all`/`load_all_via`) leaves `rf.rel` as
    // the raw path it was given, so it is recomputed here the way every
    // other caller of the single-file loader already does.
    let rel = crate::state::rel_path(root, &rf.path);
    rf.rel = rel.clone();

    // Refuse when terms.md is missing, unreadable, or not approved (per
    // "Does the tool stop while the terms file waits for approval?",
    // aa475aa3 A) -- the same gate `ask` applies. The returned terms file is
    // kept for the terms-line validity check `do_answer` makes afterward.
    let tf = require_terms(root, &led, &rel)?;

    let cur_hash;
    {
        let sec = &rf.sections[idx];
        use crate::status::QuestionStatus;
        // Closed by the ledger, the status show prints (per "Does a command
        // name a status of its own?", f53ac3b9 A).
        match crate::status::question_status(&led, sec) {
            st @ (QuestionStatus::Answered
            | QuestionStatus::Refused
            | QuestionStatus::Superseded
            | QuestionStatus::Withdrawn
            | QuestionStatus::Dropped) => return Err(msg::already_closed(&sec.heading_text, st)),
            _ => {}
        }
        // A closing line the ledger does not hold is hand-written: the
        // finding `check` and `show` print for it, the same move.
        if sec.is_refused() {
            return Err(msg::refusal_without_row(&rel, &sec.heading_text));
        }
        if sec.answer_line.is_some() {
            return Err(msg::answer_without_row(&rel, &sec.heading_text));
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
            // The move is the one `show` names for the question's file.
            let moves = || {
                let all = record::load_all(root);
                let index = crate::rules::RepoIndex::new(root, &all, &tf.terms, &led);
                let j = crate::moves::Judging { root, all: &all, terms_list: &tf.terms, led: &led, index: &index };
                crate::moves::of_unasked_question_in(&j, &rf, &sec.heading_text, sec.is_skeleton)
            };
            return Err(if sec.is_skeleton {
                msg::not_written(&sec.heading_text, &moves())
            } else if led.latest_asked(id).is_none() {
                msg::not_asked_yet(&sec.heading_text, &moves())
            } else {
                msg::not_in_asked_state(&sec.heading_text, &moves())
            });
        }
        if version_lower != cur_hash[..record::VERSION_LEN] {
            return Err(msg::stale_version(&sec.heading_text, &rel));
        }
        // Checked once the identifier and the version are good, so the
        // move it names runs as printed with a letter of the options.
        if let Some(letter) = letter {
            if !sec.options.iter().any(|o| o.letter == letter) {
                let letters: Vec<char> = sec.options.iter().map(|o| o.letter).collect();
                return Err(msg::letter_not_an_option(letter, &sec.heading_text, &letters, id, version));
            }
        }
    }

    if let Some(r) = answer_gate(root, &rf, &led, &tf, idx, &rel) {
        return Err(r);
    }

    Ok(Located { led, rf, idx, rel, tf, cur_hash })
}

/// The refusal when `answer` does not take question `idx` now (per "Does
/// answer take a question standing on a question with no decision?" and
/// "Does a refused question hold the tiers above it?"): `state::answer_waits`
/// is the one test, the `next` lines and the page read it too. Shared by
/// a letter and a refusal. An unanswered target names the owner's
/// `adr answer` of the first question in its chain that `answer` takes, once
/// that question is asked and stands as asked, else the agent's `adr ask`
/// (or the writing of a body first); a chain that ends on a refused question
/// names the plan change and the approval.
fn answer_gate(root: &Path, rf: &RecordFile, led: &Ledger, tf: &terms::TermsFile, idx: usize, rel: &str) -> Option<String> {
    let w = state::answer_waits(rf, idx);
    if w.is_empty() {
        return None;
    }
    use crate::moves::{Move, MoveKind, Waits};
    let mut waits = Vec::new();
    // What the plan waits for, read off the file as loaded, the way `show`
    // reads it: the move to ask a leaf is the agent's `adr ask` only when
    // ask would take it (`moves::of_unasked_question`, the pieces
    // `moves::of_topic_file` uses).
    let all = std::cell::OnceCell::new();
    let index = std::cell::OnceCell::new();
    let moves_for = |leaf: usize, unwritten: bool| {
        let all = all.get_or_init(|| record::load_all(root));
        let index = index.get_or_init(|| crate::rules::RepoIndex::new(root, all, &tf.terms, led));
        let j = crate::moves::Judging { root, all, terms_list: &tf.terms, led, index };
        crate::moves::of_unasked_question_in(&j, rf, &rf.sections[leaf].heading_text, unwritten)
    };
    for &t in &w.unanswered {
        let heading = rf.sections[t].heading_text.clone();
        waits.push(match answer_leaf(rf, t) {
            Err(refused) => Waits::Stuck { heading, refused: rf.sections[refused].heading_text.clone() },
            Ok(leaf) => {
                let sec = &rf.sections[leaf];
                match &sec.id {
                    _ if sec.is_skeleton => Waits::Unwritten { heading, report: moves_for(leaf, true) },
                    Some(id) if crate::view::asked_text_stands(led, rf, sec) => {
                        let answer = Move::new(MoveKind::Answer {
                            id: id.clone(),
                            version: record::version_of(&rf.lines, sec),
                            heading: sec.heading_text.clone(),
                        });
                        Waits::Unanswered { heading, answer }
                    }
                    _ => Waits::Asking { heading, report: moves_for(leaf, false) },
                }
            }
        });
    }
    for &t in &w.refused_target {
        waits.push(Waits::RefusedTarget { heading: rf.sections[t].heading_text.clone() });
    }
    for &t in &w.refused_tier {
        waits.push(Waits::RefusedTier { heading: rf.sections[t].heading_text.clone() });
    }
    Some(crate::moves::answer_waits(&rf.sections[idx].heading_text, rel, waits))
}

/// The question `answer` would take first when it starts from question `t`
/// and follows what each waiting question waits on: `Ok` with that question,
/// or `Err` with the refused question the chain ends on.
fn answer_leaf(rf: &RecordFile, t: usize) -> Result<usize, usize> {
    let mut seen = vec![t];
    let mut cur = t;
    loop {
        let w = state::answer_waits(rf, cur);
        if w.is_empty() {
            return Ok(cur);
        }
        match w.unanswered.iter().find(|n| !seen.contains(n)) {
            Some(&n) => {
                seen.push(n);
                cur = n;
            }
            None => match w.refused_target.first().or(w.refused_tier.first()) {
                Some(&r) => return Err(r),
                None => return Ok(cur),
            },
        }
    }
}

/// The write step `do_answer` and `do_refuse` both end with: splice the
/// `**Answer:** <answer_text>` line in at the end of the section's hash
/// range, save the topic file, then append `rows` to the ledger -- with the
/// same error handling and the same command to run again either way,
/// `given` being what the owner gave after the version (`"A"` for the
/// letter, `"refuse"` for a refusal). When the ledger cannot take the rows
/// the topic file is put back as it was, so the answer is all written or
/// not at all and the command the refusal names runs as printed (per "Does
/// a refusal name the command that clears it?"). Eight plain, once-each
/// arguments over a params struct: the same shape `check_plan` accepts
/// this same lint for.
#[allow(clippy::too_many_arguments)]
fn finish_answer(
    root: &Path,
    id: &str,
    version: &str,
    given: &str,
    mut rf: RecordFile,
    idx: usize,
    answer_text: &str,
    rows: Vec<ledger::Row>,
    drop_ids: &[String],
) -> Result<(), String> {
    let ledger_path = root.join(crate::repo::LEDGER_PATH);
    let before = record::join_lines(&rf.lines);
    let path = rf.path.clone();
    let again = || {
        msg::after_fix_owner_runs(crate::moves::command(&["adr", "answer", &format!("{}@{}", id, version), given]))
    };

    let hash_end = {
        let sec = &rf.sections[idx];
        record::hash_range(&rf.lines, sec).1
    };
    rf.lines.splice(
        hash_end..hash_end,
        [String::new(), format!("**Answer:** {}", answer_text)],
    );

    // Per "Does answer remove a question whose answer set lacks the
    // answer?": each dropped section leaves the file, heading through the
    // line before the next heading, in the same write as the answer line.
    if !drop_ids.is_empty() {
        let mut rebuilt = record::build(rf.path.clone(), rf.rel.clone(), rf.lines.clone());
        let mut ranges: Vec<(usize, usize)> = rebuilt
            .sections
            .iter()
            .filter(|s| s.id.as_ref().is_some_and(|i| drop_ids.contains(i)))
            .map(|s| (s.start, s.end))
            .collect();
        ranges.sort_unstable_by(|a, b| b.cmp(a));
        for (start, end) in ranges {
            rebuilt.lines.drain(start..end);
        }
        rf = record::build(rebuilt.path.clone(), rebuilt.rel.clone(), rebuilt.lines);
    }

    record::save(&rf).map_err(|e| msg::write_failed("topic file", e, again()))?;

    if let Err(e) = ledger::append(&ledger_path, &rows) {
        // The answer line and the removed questions go back out, so the
        // file waits for the same answer it waited for before.
        if let Err(back) = std::fs::write(&path, &before) {
            return Err(msg::update_failed(
                "ledger",
                format!("{}; {} holds the answer line and could not be put back: {}", e, rf.rel, back),
                Next::agent(format!("fixes what the error names and removes the answer line from {}", rf.rel))
                    .and_owner_runs(crate::moves::command(&["adr", "answer", &format!("{}@{}", id, version), given])),
            ));
        }
        return Err(msg::update_failed("ledger", e, again()));
    }

    Ok(())
}

/// One question an answer removes: its id, its hash from approval, and the
/// detail its `dropped` row carries.
struct Drop {
    id: String,
    hash: String,
    detail: String,
}

/// The questions leaving `rf` with the answer `letter` to section `idx`
/// (per "Does answer remove a question whose answer set lacks the
/// answer?" and "What does answer write for a removed question?"): first
/// every question with a depends entry on it whose set lists letters
/// without `letter` (`=*` never drops), detail `<answered id> <letter>`;
/// then, repeatedly, every question with a depends entry on a removed one,
/// detail the id of that removed question. Only questions of this file
/// are searched: an entry on a question of another file names a current
/// decision and carries no set. A question whose text has no `approved`
/// row is not removed, and neither is one that holds an answer line or has
/// an `answered` or `refused` row (per "Does a decision change after it is
/// answered?"): the answer refuses.
fn collect_drops(rf: &RecordFile, rel: &str, led: &Ledger, idx: usize, id: &str, letter: char) -> Result<Vec<Drop>, String> {
    let mut found: Vec<usize> = Vec::new();
    let mut details: Vec<String> = Vec::new();
    for (j, sec) in rf.sections.iter().enumerate() {
        if j == idx {
            continue;
        }
        let lacks = sec.depends.iter().any(|d| {
            rf.dep_target(d) == Some(idx)
                && matches!(d.set(), Some(record::AnswerSet::Letters(l)) if !l.contains(&letter))
        });
        if lacks {
            found.push(j);
            details.push(format!("{} {}", id, letter));
        }
    }
    let mut at = 0;
    while at < found.len() {
        let removed = found[at];
        let removed_id = rf.sections[removed].id.clone();
        for (j, sec) in rf.sections.iter().enumerate() {
            if j == idx || found.contains(&j) {
                continue;
            }
            if sec.depends.iter().any(|d| rf.dep_target(d) == Some(removed)) {
                found.push(j);
                details.push(removed_id.clone().unwrap_or_default());
            }
        }
        at += 1;
    }
    let mut out = Vec::new();
    for (j, detail) in found.into_iter().zip(details) {
        let sec = &rf.sections[j];
        let Some(qid) = sec.id.clone() else {
            return Err(msg::drop_without_id(&sec.heading_text));
        };
        let closed = sec.answer_value.is_some()
            || led.rows.iter().any(|r| r.id == qid && matches!(r.event.as_str(), "answered" | "refused"));
        if closed {
            return Err(msg::drop_of_closed(&sec.heading_text, sec.is_refused()));
        }
        let hash = record::question_hash(rf, sec);
        let approved = led
            .rows
            .iter()
            .any(|r| r.event == "approved" && r.id == qid && r.hash == hash && r.detail == rel);
        if !approved {
            return Err(msg::drop_not_approved(&sec.heading_text, rel));
        }
        out.push(Drop { id: qid, hash, detail });
    }
    Ok(out)
}

pub fn do_answer(root: &Path, id: &str, version: &str, letter: char) -> Result<Outcome, String> {
    let Located { led, mut rf, idx, tf, cur_hash, rel } = locate_asked(root, id, version, Some(letter))?;

    let supersedes_val;
    {
        let sec = &rf.sections[idx];
        let Some(terms_line_idx) = sec.terms_line else {
            return Err(msg::no_terms_line(&rel, id, &sec.heading_text));
        };
        let problems = terms::validate_pairs(rf.lines[terms_line_idx].trim_end(), &tf)
            .map_err(|_| msg::malformed_terms_line(&rel, id, &sec.heading_text))?;
        if let Some(p) = problems.first() {
            return Err(match &p.kind {
                terms::ProblemKind::Gone => {
                    msg::terms_line_names_gone_term(&rel, id, &sec.heading_text, &p.uuid, &p.word)
                }
                terms::ProblemKind::Retired(name) => {
                    msg::terms_line_names_retired_term(&rel, id, &sec.heading_text, name)
                }
            });
        }
        supersedes_val = sec.supersedes.clone();
    }
    // `locate_asked` loads only the one file it finds, so every topic file
    // is loaded again here for the repo-wide heading index.
    let mut superseded_ids: Vec<String> = Vec::new();
    let all = record::load_all(root);
    {
        let cross_file_headings = depends::cross_file_heading_index(&all, &led);
        // The same normalization `ask` and `check` run before judging a
        // file, so a cross-file heading-text entry reads the same here.
        let phrase_lookup = terms::phrase_lookup(&tf.terms);
        depends::normalize_cross_file_deps(&mut rf, &tf.terms, &phrase_lookup, &cross_file_headings, &led);
        let names = crate::view::heading_texts(&all);
        let refusals = rules::supersedes_refusals(&rf.rel, &rf, &cross_file_headings, &names, &all, &led, Some(idx));
        if !refusals.is_empty() {
            return Err(refusals.into_iter().map(|r| r.message).collect::<Vec<_>>().join("\n"));
        }
        // The ids the entries resolve to (an entry that no longer resolves
        // is a sibling replacer's target, already superseded: its own id).
        for old in depends::supersedes_entries(&supersedes_val, &cross_file_headings, &led) {
            match depends::resolve_supersedes(&old, &cross_file_headings, &led) {
                Ok(target) => superseded_ids.push(target),
                Err(_) => superseded_ids.push(old.trim().to_string()),
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
    // One row per decision replaced, however many times the line names it.
    let mut once: Vec<&String> = Vec::new();
    for old in &superseded_ids {
        if once.contains(&old) {
            continue;
        }
        once.push(old);
        rows.push(ledger::Row {
            time: write_time.clone(),
            event: "superseded".to_string(),
            id: old.clone(),
            hash: "-".to_string(),
            detail: id.to_string(),
            version: crate::version::VERSION.to_string(),
        });
    }

    // Every question of this file whose set lacks the letter, and every
    // question standing on a removed one, leaves with this answer.
    let drops = collect_drops(&rf, &rel, &led, idx, id, letter)?;
    let drop_ids: Vec<String> = drops.iter().map(|d| d.id.clone()).collect();
    for d in &drops {
        rows.push(ledger::Row {
            time: write_time.clone(),
            event: "dropped".to_string(),
            id: d.id.clone(),
            hash: d.hash.clone(),
            detail: d.detail.clone(),
            version: crate::version::VERSION.to_string(),
        });
    }

    // Named before the write: a dropped question leaves the file with it.
    let names = event_names(&all, &tf.terms);
    let mut outcome = Outcome::of_rows(&rows, &names);
    for e in &mut outcome.events {
        if let crate::out::Event::Answered { heading, sentence, .. } = e {
            *heading = heading_for_print.clone();
            *sentence = sentence_for_print.clone();
        }
    }

    let letter_str = letter.to_string();
    finish_answer(root, id, version, &letter_str, rf, idx, &letter_str, rows, &drop_ids)?;

    Ok(outcome)
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
/// is written).
pub fn do_refuse(root: &Path, id: &str, version: &str) -> Result<Outcome, String> {
    // Same lookup, terms gate, already-refused/already-answered, hash,
    // version, and ledger-state checks as the letter path (`locate_asked`).
    // The section's own terms-line/pairing checks are not shared (5), so
    // the returned terms file has no further use here.
    let Located { led, rf, idx, cur_hash, .. } = locate_asked(root, id, version, None)?;

    let heading_for_print = rf.sections[idx].heading_text.clone();

    let row = ledger::Row {
        time: ledger::write_time(&led),
        event: "refused".to_string(),
        id: id.to_string(),
        hash: cur_hash,
        detail: heading_for_print.clone(),
        version: crate::version::VERSION.to_string(),
    };

    let outcome = Outcome::of_rows(std::slice::from_ref(&row), &HashMap::new());
    finish_answer(root, id, version, "refuse", rf, idx, "refused", vec![row], &[])?;

    Ok(outcome)
}

pub fn do_approve(root: &Path, file: &Path) -> Result<Outcome, String> {
    let ledger_path = root.join(crate::repo::LEDGER_PATH);
    let led = ledger::read(&ledger_path);
    // The path as the repository names it, read from where it was given
    // (relative to the directory the command ran in): approve takes the
    // files `show` takes and no other, so it never writes a ledger row for
    // a path no other command reads (per "Does show print what a file holds
    // beyond its latest approval?", which has show take every file approve
    // takes).
    let rel = approved_rel(root, file).map_err(msg::approve_prefixed)?;
    let file = &root.join(&rel);

    if rel == crate::repo::TERMS_PATH {
        do_approve_terms(root, file, &led, &ledger_path)
    } else if rel.starts_with(&format!("{}/", crate::repo::BLAME_DIR)) {
        do_approve_blame(root, file, &rel, &led, &ledger_path)
    } else if is_topic_file_path(&rel) && !file.exists() {
        do_approve_removal(root, &rel, &led, &ledger_path)
    } else {
        do_approve_record(root, file, &rel, &led, &ledger_path)
    }
}


/// `file` as a path from the repository root, read the way the shell that
/// ran the command reads it (relative to the current directory unless
/// absolute, `.` and `..` resolved), or the refusal when it is outside the
/// repository or no file approve takes: a topic file under
/// adrdom/decisions, adrdom/terms.md, or a blame file under adrdom/blame.
fn approved_rel(root: &Path, file: &Path) -> Result<String, String> {
    let Some(rel) = state::given_rel(root, file) else {
        return Err(msg::approve_not_a_file_approve_takes(&file.display().to_string()));
    };
    if rules::approve_takes(&rel) {
        Ok(rel)
    } else {
        Err(msg::approve_not_a_file_approve_takes(&rel))
    }
}

/// `adr approve <path>` for a topic file that is gone from disk: approves
/// its removal. Refused while the file held a current decision or an open
/// question (asked and not answered, or a heading of its approved plan not
/// yet asked; see `removal_holds`), and for a path the ledger holds no
/// approved plan for: a draft never approved leaves with no owner act.
/// Writes the approved and context rows over an empty plan and empty prose,
/// and a withdrawn row for each refused id left in no file.
fn do_approve_removal(root: &Path, rel: &str, led: &Ledger, ledger_path: &Path) -> Result<Outcome, String> {
    if led.latest_for_path("approved", rel).is_none() {
        return Err(msg::approve_prefixed(msg::removal_not_a_topic_file(rel)));
    }
    let all_records: Vec<RecordFile> = record::load_all(root);
    let present_ids: HashSet<String> = rules::held_ids(&all_records).into_iter().map(str::to_string).collect();
    let RemovalHolds { held, .. } = removal_holds(root, rel, led, &present_ids);

    let refusals: Vec<String> = rules::judge_removed_topic_file(root, rel, led, &present_ids)
        .into_iter()
        .map(|f| msg::approve_prefixed(f.message))
        .collect();
    if !refusals.is_empty() {
        return Err(refusals.join("\n"));
    }
    let mut withdrawn_rows: Vec<ledger::Row> = Vec::new();
    let write_time = ledger::write_time(led);
    for (id, _) in &held {
        if let Some(r) = led.latest_state(id).filter(|r| r.event == "refused") {
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

    let empty = empty_plan_hash();
    let mut rows: Vec<ledger::Row> = Vec::new();
    for event in ["approved", "context"] {
        if led.latest_for_path(event, rel).map(|r| r.hash.as_str()) != Some(empty.as_str()) {
            rows.push(ledger::Row {
                time: write_time.clone(),
                event: event.to_string(),
                id: "-".to_string(),
                hash: empty.clone(),
                detail: rel.to_string(),
                version: crate::version::VERSION.to_string(),
            });
        }
    }
    rows.extend(withdrawn_rows);
    ledger::append(ledger_path, &rows).map_err(|e| {
        msg::approve_prefixed(msg::update_failed(
            "ledger",
            e,
            msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
        ))
    })?;
    Ok(Outcome::of_rows(&rows, &HashMap::new()))
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
    let content = std::fs::read_to_string(file).map_err(|e| {
        msg::approve_prefixed(msg::read_failed(
            rel,
            e,
            msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
        ))
    })?;
    let bf = blame::parse(file.to_path_buf(), rel.to_string(), record::split_lines(&content));

    // A row that names a term resolves it against the approved terms file,
    // and approve writes that term's identifier into the row: the same
    // gate topic-file approve applies (`require_terms`), so no identifier
    // is written that the terms file has not been given yet.
    let names_a_term = bf.rows.iter().any(|r| r.dest.trim_start().starts_with("term:"));
    if names_a_term {
        require_terms(root, led, rel).map_err(msg::approve_prefixed)?;
    }

    // One judgement of the file against its origin, the one `check` and
    // `show` read (`blame::file_state`, `rules::judge_blame_file`): approve
    // refuses on what they report, with the same moves (per "Does a command
    // refuse what check passes?", fe14991d A).
    let source = repo::Source::WorkingTree;
    let (records, _) = record::load_all_via(root, &source);
    let (any_headings, known_ids) = blame::heading_ids(&records);
    let st = blame::file_state(root, &source, &bf, &content, &records, led, &any_headings, &known_ids);
    let terms_list: Vec<terms::Term> =
        state::load_terms(root).map(|tf| terms::minted_copy(&tf).terms).unwrap_or_default();
    let names = rules::BlameNames::of(root, &source, &records, terms_list);
    let refusals: Vec<String> =
        rules::judge_blame_file(root, &st, &bf, &names).into_iter().map(|f| msg::approve_prefixed(f.message)).collect();
    if !refusals.is_empty() {
        return Err(refusals.join("\n"));
    }
    // The origin is gone and nothing holds it (`file_state` names an open
    // row or the file's own approval otherwise): the file stands at its
    // approval, and there is nothing to approve.
    let Some(origin) = st.origin_content.as_ref().and(bf.origin.as_deref()) else {
        return Ok(Outcome::of_rows(&[], &HashMap::new()));
    };
    let (_, resolved) = rules::judge_blame_destinations_in(root, &bf, origin, &names);

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
                msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
            ))
        })?;
    }

    let new_hash = blame::file_hash(&work_lines);
    let unchanged = led.latest_for_path("blame", rel).map(|r| r.hash.as_str()) == Some(new_hash.as_str());

    // No row, no line: a rewrite of the rows into the form approve writes
    // over a hash the ledger already holds is no event.
    let mut rows: Vec<ledger::Row> = Vec::new();
    if !unchanged {
        let row = ledger::Row {
            time: ledger::write_time(led),
            event: "blame".to_string(),
            id: "-".to_string(),
            hash: new_hash,
            detail: rel.to_string(),
            version: crate::version::VERSION.to_string(),
        };
        ledger::append(ledger_path, std::slice::from_ref(&row)).map_err(|e| {
            msg::approve_prefixed(msg::update_failed(
                "ledger",
                e,
                msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
            ))
        })?;
        rows.push(row);
    }

    let mut outcome = Outcome::of_rows(&rows, &HashMap::new());
    // A file written that no row of this run covers is an event of its own:
    // the rows rewritten into the form approve writes, over a hash the
    // ledger already holds.
    if rewritten && rows.is_empty() {
        outcome.events.push(crate::out::Event::FileWritten { path: rel.to_string() });
    }
    Ok(outcome)
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
            msg::after_fix_owner_runs(crate::moves::Cmd::fixed("adr approve adrdom/terms.md")),
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
    terms::mint_ids(&mut tf);

    // Steps 1 (shape, post-mint), 3 (cycles), 4 (dependents), 5 (banned):
    // every refusal, all at once.
    let (refusals, pairs_per_entry) =
        terms_refusals(root, &records, &tf, led, &repo::Source::WorkingTree, true, &approved_plans);
    let mut refusals = refusals;
    // Only an approve that retires or removes a term needs to read the
    // asked questions' terms lines; one that only adds or rewords does not.
    if state::approve_retires_or_removes(led, &tf) {
        for (rel, heading) in state::unreadable_asked_terms_lines(&records, led) {
            refusals.push(msg::asked_terms_line_unreadable(&rel, &heading));
        }
    }
    if !refusals.is_empty() {
        let msg = refusals
            .iter()
            .map(msg::approve_prefixed)
            .collect::<Vec<_>>()
            .join("\n");
        return Err(msg);
    }

    // Which terms need a new `term` row: new ones, and those changed since
    // their latest row.
    let term_rows_due: Vec<(String, String, String)> = tf
        .terms
        .iter()
        .filter_map(|t| t.id.clone().map(|id| (id, t.name.clone(), terms::term_hash(t))))
        .filter(|(id, _, hash)| led.latest_for_id("term", id).is_none_or(|r| r.hash != *hash))
        .collect();

    // Step 6: write the file. `terms:` lines use the fresh pairs computed
    // above, one term at a time, in the same order as `tf.terms`.
    let mut new_lines = tf.lines.clone();
    terms::write_terms_lines(&mut new_lines, &tf, &pairs_per_entry);
    tf.lines = new_lines;

    // The files as they stand, put back when the ledger refuses the rows:
    // never a terms file or terms brief rewritten for rows no ledger holds.
    let terms_before = std::fs::read_to_string(&tf.path).ok();
    let terms_changed = terms_before.as_deref() != Some(tf.lines.join("\n").as_str());
    terms::save(&tf).map_err(|e| {
        msg::approve_prefixed(msg::write_failed(
            &file.display().to_string(),
            e,
            msg::after_fix_owner_runs(crate::moves::Cmd::fixed("adr approve adrdom/terms.md")),
        ))
    })?;

    // Per "Does approve write the terms brief?" (fcab4bf9 A): written on
    // every run, whether or not the terms file changed, so a repository
    // whose terms file is approved gets its terms brief from one run.
    let brief_path = root.join(crate::repo::TERMS_BRIEF_PATH);
    let brief_before = std::fs::read_to_string(&brief_path).ok();
    let brief_changed = brief_before.as_deref() != Some(terms::brief_text(&tf).as_str());
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
            msg::after_fix_owner_runs(crate::moves::Cmd::fixed("adr approve adrdom/terms.md")),
        ))
    })?;

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
            msg::after_fix_owner_runs(crate::moves::Cmd::fixed("adr approve adrdom/terms.md")),
        ))
    };

    // The file already matching the ledger's latest `terms` row writes
    // nothing. A ledger from before `term` rows existed gets them at its
    // next real approval (no migration code in the tool).
    let mut written: Vec<ledger::Row> = Vec::new();
    if !state::terms_approved(led, &tf) {
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
        for (id, name) in state::removed_terms(led, &tf) {
            rows.push(ledger::Row {
                time: write_time.clone(),
                event: "term".to_string(),
                id,
                hash: state::TERM_REMOVED.to_string(),
                detail: name,
                version: crate::version::VERSION.to_string(),
            });
        }
        if let Err(e) = ledger::append(ledger_path, &rows) {
            if let Some(before) = &terms_before {
                let _ = std::fs::write(&tf.path, before);
            }
            match &brief_before {
                Some(before) => {
                    let _ = std::fs::write(&brief_path, before);
                }
                None => {
                    let _ = std::fs::remove_file(&brief_path);
                }
            }
            return Err(append_failed(e));
        }
        written = rows;
    }

    // The terms brief is a file of its own that no row covers: written
    // whenever its text changed (per "What does a command that writes
    // print?", 069a6b4a A, one line per event it wrote).
    let mut outcome = Outcome::of_rows(&written, &HashMap::new());
    if terms_changed && written.is_empty() {
        outcome.events.push(crate::out::Event::FileWritten { path: crate::repo::TERMS_PATH.to_string() });
    }
    if brief_changed {
        outcome.events.push(crate::out::Event::FileWritten { path: crate::repo::TERMS_BRIEF_PATH.to_string() });
    }
    Ok(outcome)
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
            // `supersedes:` and `unrelated:` lines when the heading has
            // them (directly under `depends:`), else after `depends:`.
            let mut after = depends_idx;
            while work_lines
                .get(after + 1)
                .is_some_and(|l| l.starts_with("supersedes: ") || l.starts_with("unrelated: "))
            {
                after += 1;
            }
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
            rel,
            e,
            msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
        ))
    })?;
    // The file as the repository names it, whatever path it was loaded by:
    // every rule that compares a ledger row's path with `rf.rel` reads the
    // same path for it whichever spelling the command was given.
    rf.rel = rel.to_string();

    // Refuse up front, before a single `depends:` entry is resolved, when
    // adrdom/terms.md is missing, shape-broken, or not yet approved: the
    // same gate `ask`/`answer`/`refuse` apply via `require_terms`. Without
    // this, a real term with no id minted yet (an unapproved terms.md mints
    // nothing to disk) silently fails to resolve and every depends entry
    // naming it refuses "which is no term" instead of the one clear "terms
    // not approved" line.
    let tf = require_terms(root, led, rel).map_err(msg::approve_prefixed)?;
    // Read off the file as written, before the normalize pass below.
    let planning = rules::planning_rules_apply(led, &rf, rel);
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
    // `all_records` is still as loaded, so its plan hashes are the raw ones.
    let replaced_base = depends::replaced_base(
        &all_records,
        &depends::approved_plan_rels(&all_records, led),
        &cross_file_headings,
        led,
    );
    let replaced = depends::replaced_ids(&replaced_base, &rf, &cross_file_headings, led);
    // Every rule this file breaks, judged once (`rules::judge_topic_file`,
    // the judgement `check` reads too), and the rules on the records as a
    // whole (`rules::judge_records`): collected, not returned at the first,
    // so one `approve` call tells the agent everything it owes the owner
    // before asking again. Nothing is written while one stands.
    let judge = rules::TopicFileJudge {
        root,
        rel,
        rf: &rf,
        all: &all_records,
        terms_list,
        phrase_lookup: &phrase_lookup,
        cross_file_headings: &cross_file_headings,
        names: &names,
        replaced: &replaced,
        led,
        planning,
        shared: None,
        directory: true,
    };
    let refusals: Vec<String> = rules::judge_topic_file(&judge)
        .into_iter()
        .chain(rules::judge_records(&all_records, led))
        .map(|f| msg::approve_prefixed(f.message))
        .collect();

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
        let stands_as_asked = crate::view::asked_text_stands(led, &rf, sec);
        if sec.supersedes.is_empty() || stands_as_asked || state::is_frozen(led, sec) {
            continue;
        }
        // A target that names no decision (or several) is `record_refusals`'
        // refusal above; only the ones that resolve are turned.
        let ids: Vec<String> = depends::supersedes_entries(&sec.supersedes, &cross_file_headings, led)
            .iter()
            .filter_map(|old| depends::resolve_supersedes(old, &cross_file_headings, led).ok())
            .collect();
        supersedes_ids.insert(i, ids);
    }

    let present_ids: HashSet<String> = rules::held_ids(&all_records).into_iter().map(str::to_string).collect();
    let state_ids = led.state_ids();

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
    // An id of a removed file awaiting its removal's approval is settled by
    // that approval (per "What may leave a topic file by a plan change?"),
    // never by the approval of an unrelated file.
    let removed_held: HashSet<String> = rules::removal_awaiting(led, &|rel| root.join(rel).exists())
        .iter()
        .flat_map(|rel| removal_holds(root, rel, led, &present_ids).held)
        .map(|(id, _)| id)
        .collect();
    for id in &state_ids {
        if present_ids.contains(id) || removed_held.contains(id) {
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
    // The `unrelated:` line holds ids too (per "Does every neighbour get a
    // line under its heading?"): a decision, a term or a question of the
    // file by its identifier, a dropped question as written.
    for (i, sec) in sections_snapshot.iter().enumerate() {
        if sec.unrelated.is_empty() || !crate::status::question_status(led, sec).is_unasked() {
            continue;
        }
        let Some(j) = (sec.start + 1..sec.end).find(|&j| work_rf.lines[j].starts_with("unrelated: ")) else {
            continue;
        };
        let mut ids: Vec<String> = Vec::new();
        for e in crate::plan::section_unrelated_entries(i, &work_rf, terms_list, &cross_file_headings, led) {
            let id = match crate::plan::resolve_ref(&e, &work_rf, i, terms_list, &cross_file_headings, led) {
                Ok(crate::plan::Target::Decision(id))
                | Ok(crate::plan::Target::Term(id))
                | Ok(crate::plan::Target::Dropped(id)) => id,
                Ok(crate::plan::Target::Question(q)) => work_rf.sections[q].id.clone().unwrap_or(e),
                Err(_) => e,
            };
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        work_rf.lines[j] = format!("unrelated: {}", ids.join(", "));
    }
    // Reparse once more so `plan_hash`/`context_hash` and everything below
    // sees the file exactly as it will be written.
    let work_rf = record::build(work_rf.path.clone(), rel.to_string(), work_rf.lines.clone());

    let plan_hash = record::plan_hash(&work_rf);
    let context_hash = record::context_hash(&work_rf);

    // Whether the save below changes the file's bytes (ids written in, a
    // reference line rewritten to identifiers, line ends made one kind).
    let bytes_change = std::fs::read_to_string(&work_rf.path).ok().as_deref()
        != Some(record::join_lines(&work_rf.lines).as_str());
    record::save(&work_rf).map_err(|e| {
        msg::approve_prefixed(msg::write_failed(
            &file.display().to_string(),
            e,
            msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
        ))
    })?;

    let mut rows: Vec<ledger::Row> = Vec::new();

    // One row per question (per "Does approve hash each question on its
    // own?"): its id and its own hash, written when that differs from the
    // question's latest row for this file; then the file's row, whose hash
    // is derived from the sorted question hashes.
    let question_hashes = record::question_hashes(&work_rf);
    for (sec, qh) in work_rf.sections.iter().zip(&question_hashes) {
        let Some(id) = &sec.id else { continue };
        let unchanged = led
            .rows
            .iter()
            .rfind(|r| r.event == "approved" && &r.id == id)
            .is_some_and(|r| &r.hash == qh && r.detail == rel);
        if !unchanged {
            rows.push(ledger::Row {
                time: write_time.clone(),
                event: "approved".to_string(),
                id: id.clone(),
                hash: qh.clone(),
                detail: rel.to_string(),
                version: crate::version::VERSION.to_string(),
            });
        }
    }
    // A set approved while its in-file target is asked and not yet answered
    // stays after the answer (per "Does a depends entry on a question in its
    // file carry an answer set?"): the `assumes` row `ask` writes for an
    // entry, written here for an entry that came after the ask.
    for sec in &work_rf.sections {
        let Some(dep_id) = &sec.id else { continue };
        if led.latest_asked(dep_id).is_some() {
            continue;
        }
        for d in &sec.depends {
            let record::Dep::Id { id: tid, set: Some(set) } = d else { continue };
            let waiting = work_rf.sections.iter().any(|t| t.id.as_ref() == Some(tid))
                && led.latest_state(tid).is_some_and(|r| r.event == "asked");
            let hash = record::assumes_hash(dep_id, set);
            let written = led.assumes_for(tid).iter().any(|r| &r.detail == dep_id && r.hash == hash);
            if waiting && !written {
                rows.push(ledger::Row {
                    time: write_time.clone(),
                    event: "assumes".to_string(),
                    id: tid.clone(),
                    hash,
                    detail: dep_id.clone(),
                    version: crate::version::VERSION.to_string(),
                });
            }
        }
    }
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
    }

    ledger::append(ledger_path, &rows).map_err(|e| {
        msg::approve_prefixed(msg::update_failed(
            "ledger",
            e,
            msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
        ))
    })?;

    // `withdrawn_rows` was computed, and any refusal already returned,
    // above -- before either write this function has made so far.
    if !withdrawn_rows.is_empty() {
        ledger::append(ledger_path, &withdrawn_rows).map_err(|e| {
            msg::approve_prefixed(msg::update_failed(
                "ledger",
                e,
                msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
            ))
        })?;
    }
    rows.extend(withdrawn_rows);

    let mut outcome = Outcome::of_rows(&rows, &crate::view::heading_texts(std::slice::from_ref(&work_rf)));
    // A file written that no row of this run covers is named by one line of
    // its own (per "What does a command that writes print?", 069a6b4a A).
    let covered = rows.iter().any(|r| (r.event == "approved" || r.event == "context") && r.detail == rel);
    if bytes_change && !covered {
        outcome.events.push(crate::out::Event::FileWritten { path: rel.to_string() });
    }
    Ok(outcome)
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
    let led = ledger::read(&ledger_path);
    // What is banned is what the ledger says is banned (per "Does check hold
    // the ban list equal to the ledger?"), the set `ban` reads too: a ban
    // list that drifted from it, or cannot be read, changes nothing here.
    let banned_now = crate::banned::currently_banned(&led);
    let is_banned = |h: &str| banned_now.iter().any(|(b, _)| b == h);

    // Each line given is one word (or phrase) and the hashes `ban` wrote
    // for it: the line, and each run of letters in it.
    let mut to_remove: Vec<String> = Vec::new();
    let mut not_banned = 0;
    for line in input.lines() {
        let hashes: Vec<String> = crate::banned::tokenize(line).iter().map(|t| record::sha256_hex(t)).collect();
        if hashes.is_empty() {
            continue;
        }
        let hit: Vec<String> = hashes.into_iter().filter(|h| is_banned(h)).collect();
        if hit.is_empty() {
            not_banned += 1;
        }
        for h in hit {
            if !to_remove.contains(&h) {
                to_remove.push(h);
            }
        }
    }
    to_remove.sort();
    // The words given that were not banned: a finding on the input, as a
    // count of the words given, never dropped without a word.
    let input_findings: Vec<String> = if not_banned == 0 { Vec::new() } else { vec![msg::unban_not_banned(not_banned)] };

    if to_remove.is_empty() {
        return Ok(Outcome { events: Vec::new(), input_findings });
    }

    // One time for every row this run writes (per "Does a command write a
    // time later than the last ledger row?", aefa7f3c-70c3-412e-8d73-16ff9e2b703a
    // A), whichever hash each row names. The ledger first: a ban list left
    // behind by a failed write after it is brought back by the move check
    // names (`adr ban --rebuild`), from the ledger.
    let write_time = ledger::write_time(&led);
    let rows: Vec<ledger::Row> = to_remove
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
        .map_err(|e| msg::update_failed("ledger", e, msg::after_fix_owner_runs(crate::moves::Cmd::fixed("adr unban"))))?;
    let remaining: Vec<(String, Option<String>)> = banned_now.into_iter().filter(|(h, _)| !to_remove.contains(h)).collect();
    crate::banned::write_sorted(&banned_path, remaining).map_err(|e| {
        msg::update_failed("banned list", e, msg::after_fix_agent_runs(crate::moves::Cmd::fixed("adr ban --rebuild")))
    })?;

    let mut outcome = Outcome::of_rows(&rows, &HashMap::new());
    outcome.input_findings = input_findings;
    Ok(outcome)
}
