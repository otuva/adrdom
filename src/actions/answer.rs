//! Answer and refuse: the owner picks an option of an asked question, or refuses it.
use super::{event_names_in, finding, findings_of, require_terms, stop, Outcome, Stops};
use crate::depends;
use crate::ledger::{self, Ledger};
use crate::msg::{self, Next};
use crate::record::{self, RecordFile};
use crate::rules::{self, Class, Command};
use crate::state;
use crate::terms;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
fn locate_asked(
    root: &Path,
    id: &str,
    version: &str,
    letter: Option<char>,
) -> Result<Located, Stops> {
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
        // A superseded question's section is struck from every file (adr-code:
        // b5ca161f-c88c-4f02-a9d9-1d406cf3a252): the ledger still says it is
        // closed, by the heading its `asked` row carries.
        if unread.is_empty() && state::is_superseded(&led, id) {
            let heading = led
                .latest_asked(id)
                .map(|r| r.detail.clone())
                .unwrap_or_default();
            return Err(stop(
                Command::Answer,
                msg::already_closed(&heading, crate::status::QuestionStatus::Superseded),
            ));
        }
        if unread.is_empty() {
            return Err(stop(Command::Answer, msg::id_not_found(id)));
        }
        return Err(stop(
            Command::Answer,
            msg::id_not_found_some_unread(id, &unread),
        ));
    };
    // `record::load` (unlike `load_all`/`load_all_via`) leaves `rf.rel` as
    // the raw path it was given, so it is recomputed here the way every
    // other caller of the single-file loader already does.
    let rel = crate::state::rel_path(root, &rf.path);
    rf.rel = rel.clone();

    // Refuse when terms.md is missing, unreadable, or not approved (per
    // "Does the tool stop while the terms file waits for approval?",
    // aa475aa3 A) -- the same gate `approve` applies. The returned terms file is
    // kept for the terms-line validity check `do_answer` makes afterward.
    let tf = require_terms(root, &led, &rel, Command::Answer)?;

    let cur_hash;
    {
        let sec = &rf.sections[idx];
        use crate::status::QuestionStatus;
        // Closed by the ledger, the status show prints (per "Does a command
        // name a status of its own?", f53ac3b9 A).
        if let st @ (QuestionStatus::Answered
        | QuestionStatus::Refused
        | QuestionStatus::Superseded
        | QuestionStatus::Withdrawn
        | QuestionStatus::Dropped) = crate::status::question_status(&led, sec)
        {
            return Err(stop(
                Command::Answer,
                msg::already_closed(&sec.heading_text, st),
            ));
        }
        // A closing line the ledger does not hold is hand-written: the
        // finding `check` and `show` print for it, the same move.
        if sec.is_refused() {
            return Err(finding(
                Class::Answers,
                msg::refusal_without_row(&rel, &sec.heading_text),
            ));
        }
        if sec.answer_line.is_some() {
            return Err(finding(
                Class::Answers,
                msg::answer_without_row(&rel, &sec.heading_text),
            ));
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
            return Err(stop(Command::Answer, msg::invalid_version_format(version)));
        }
        // The text stands as it was asked (the version the owner would read
        // is the current one) or it does not (never asked, or edited since:
        // the owner's `approve` is the move).
        let stands = crate::state::asked_text_stands(&led, id, &cur_hash);
        if !stands {
            // The move is the one `show` names for the question's file.
            let moves = || {
                let all = record::load_all(root);
                let index = crate::rules::RepoIndex::new(root, &all, &tf.terms, &led);
                let j = crate::moves::Judging {
                    root,
                    all: &all,
                    terms_list: &tf.terms,
                    led: &led,
                    index: &index,
                };
                crate::moves::of_unasked_question_in(&j, &rf, sec.is_skeleton)
            };
            let report = moves();
            let mut lines = stop(
                Command::Answer,
                if sec.is_skeleton {
                    msg::not_written(&sec.heading_text, &report)
                } else if led.latest_asked(id).is_none() {
                    msg::not_asked_yet(&sec.heading_text, &rf.rel, &report)
                } else {
                    msg::not_in_asked_state(&sec.heading_text, &report)
                },
            );
            lines.extend(findings_of(report.findings));
            return Err(lines);
        }
        if version_lower != cur_hash[..record::VERSION_LEN] {
            return Err(stop(
                Command::Answer,
                msg::stale_version(&sec.heading_text, &rel),
            ));
        }
        // Checked once the identifier and the version are good, so the
        // move it names runs as printed with a letter of the options.
        if let Some(letter) = letter {
            if !sec.options.iter().any(|o| o.letter == letter) {
                let letters: Vec<char> = sec.options.iter().map(|o| o.letter).collect();
                return Err(stop(
                    Command::Answer,
                    msg::letter_not_an_option(letter, &sec.heading_text, &letters, id, version),
                ));
            }
        }
    }

    if let Some(r) = answer_gate(root, &rf, &led, &tf, idx, &rel) {
        return Err(r);
    }

    Ok(Located {
        led,
        rf,
        idx,
        rel,
        tf,
        cur_hash,
    })
}

/// The refusal when `answer` does not take question `idx` now (per "Does answer take a question
/// standing on a question that has no decision?" and "Does a refused question hold the tiers above
/// it?"): `state::answer_waits` is the one test, the `next` lines and the page read it too. Shared
/// by a letter and a refusal. An unanswered target names the owner's `adr answer` of the first
/// question in its chain that `answer` takes, once that question is asked and stands as asked, else
/// the owner's `adr approve` (or the writing of a body first); a chain that ends on a refused question
/// names the plan change and the approval.
fn answer_gate(
    root: &Path,
    rf: &RecordFile,
    led: &Ledger,
    tf: &terms::TermsFile,
    idx: usize,
    rel: &str,
) -> Option<Stops> {
    let w = state::answer_waits(rf, idx);
    if w.is_empty() {
        return None;
    }
    use crate::moves::{Move, MoveKind, Waits};
    let mut waits = Vec::new();
    // What the plan waits for, read off the file as loaded, the way `show`
    // reads it: the move to ask a leaf is the owner's `adr approve` only when
    // approve would take it (`moves::of_unasked_question`, the pieces
    // `moves::of_topic_file` uses).
    let all = std::cell::OnceCell::new();
    let index = std::cell::OnceCell::new();
    let moves_for = |unwritten: bool| {
        let all = all.get_or_init(|| record::load_all(root));
        let index = index.get_or_init(|| crate::rules::RepoIndex::new(root, all, &tf.terms, led));
        let j = crate::moves::Judging {
            root,
            all,
            terms_list: &tf.terms,
            led,
            index,
        };
        crate::moves::of_unasked_question_in(&j, rf, unwritten)
    };
    for &t in &w.unanswered {
        let heading = rf.sections[t].heading_text.clone();
        waits.push(match answer_leaf(rf, t) {
            Err(refused) => Waits::Stuck {
                heading,
                refused: rf.sections[refused].heading_text.clone(),
            },
            Ok(leaf) => {
                let sec = &rf.sections[leaf];
                match &sec.id {
                    _ if sec.is_skeleton => Waits::Unwritten {
                        heading,
                        report: moves_for(true),
                    },
                    Some(id) if crate::view::asked_text_stands(led, rf, sec) => {
                        let answer = Move::new(MoveKind::Answer {
                            id: id.clone(),
                            version: record::version_of(&rf.lines, sec),
                            heading: sec.heading_text.clone(),
                        });
                        Waits::Unanswered { heading, answer }
                    }
                    _ => Waits::Asking {
                        heading,
                        report: moves_for(false),
                    },
                }
            }
        });
    }
    for &t in &w.refused_target {
        waits.push(Waits::RefusedTarget {
            heading: rf.sections[t].heading_text.clone(),
        });
    }
    for &t in &w.refused_tier {
        waits.push(Waits::RefusedTier {
            heading: rf.sections[t].heading_text.clone(),
        });
    }
    let (waits_line, found) =
        crate::moves::answer_waits(&rf.sections[idx].heading_text, rel, waits);
    let mut lines = finding(Class::Answers, waits_line);
    lines.extend(findings_of(found));
    Some(lines)
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
/// a refusal name the command that clears it?"). Ten plain, once-each
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
    strike_ids: &[String],
) -> Result<Vec<String>, Stops> {
    let ledger_path = root.join(crate::repo::LEDGER_PATH);
    let before = record::join_lines(&rf.lines);
    let path = rf.path.clone();
    let again = || {
        msg::after_fix_owner_runs(crate::moves::command(&[
            "answer",
            &format!("{}@{}", id, version),
            given,
        ]))
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
    // Per "What strikes the section of a superseded question from its topic
    // file?": each superseded section leaves every topic file in the same
    // run, the answered file in the same write. adr-code: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
    let leaving: Vec<String> = drop_ids.iter().chain(strike_ids).cloned().collect();
    if !leaving.is_empty() {
        rf = without_sections(rf, &leaving);
    }

    record::save(&rf)
        .map_err(|e| stop(Command::Answer, msg::write_failed("topic file", e, again())))?;

    // The other topic files that hold a struck section, each as it was
    // before, so a ledger that cannot take the rows puts them all back.
    let mut others: Vec<(PathBuf, String, String)> = Vec::new();
    if !strike_ids.is_empty() {
        for other in record::load_all(root) {
            if other.path == path || other.rel == rf.rel {
                continue;
            }
            if !other
                .sections
                .iter()
                .any(|s| s.id.as_ref().is_some_and(|i| strike_ids.contains(i)))
            {
                continue;
            }
            let was = record::join_lines(&other.lines);
            let (opath, orel) = (other.path.clone(), other.rel.clone());
            let cut = without_sections(other, strike_ids);
            // Recorded before the save: a save that fails after truncating
            // the file leaves it to be put back with the others.
            others.push((opath, was, orel));
            if let Err(e) = record::save(&cut) {
                restore_files(&others);
                let _ = std::fs::write(&path, &before);
                return Err(stop(
                    Command::Answer,
                    msg::write_failed("topic file", e, again()),
                ));
            }
        }
    }

    if let Err(e) = ledger::append(&ledger_path, &rows) {
        restore_files(&others);
        // The answer line and the removed questions go back out, so the
        // file waits for the same answer it waited for before.
        if let Err(back) = std::fs::write(&path, &before) {
            return Err(stop(
                Command::Answer,
                msg::update_failed(
                    crate::repo::LEDGER_PATH,
                    format!(
                        "{}; {} holds the answer line and could not be put back: {}",
                        e, rf.rel, back
                    ),
                    Next::agent(msg::Act::Rewrites(msg::Part::Ledger))
                        .then(msg::Act::Removes(msg::Part::LineOf(msg::LineKind::Answer)))
                        .and_owner_runs(crate::moves::command(&[
                            "answer",
                            &format!("{}@{}", id, version),
                            given,
                        ])),
                ),
            ));
        }
        return Err(stop(
            Command::Answer,
            msg::update_failed(crate::repo::LEDGER_PATH, e, again()),
        ));
    }

    Ok(others.into_iter().map(|(_, _, rel)| rel).collect())
}

/// `rf` without the sections of `ids`, each from its heading through the line
/// before the next heading.
fn without_sections(rf: RecordFile, ids: &[String]) -> RecordFile {
    let mut rebuilt = record::build(rf.path.clone(), rf.rel.clone(), rf.lines.clone());
    let mut ranges: Vec<(usize, usize)> = rebuilt
        .sections
        .iter()
        .filter(|s| s.id.as_ref().is_some_and(|i| ids.contains(i)))
        .map(|s| (s.start, s.end))
        .collect();
    ranges.sort_unstable_by(|a, b| b.cmp(a));
    for (start, end) in ranges {
        rebuilt.lines.drain(start..end);
    }
    record::build(rebuilt.path.clone(), rebuilt.rel.clone(), rebuilt.lines)
}

/// Puts back each file written as it was.
fn restore_files(files: &[(PathBuf, String, String)]) {
    for (path, was, _) in files {
        let _ = std::fs::write(path, was);
    }
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
fn collect_drops(
    rf: &RecordFile,
    rel: &str,
    led: &Ledger,
    idx: usize,
    id: &str,
    letter: char,
) -> Result<Vec<Drop>, Stops> {
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
            if sec
                .depends
                .iter()
                .any(|d| rf.dep_target(d) == Some(removed))
            {
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
            return Err(finding(
                Class::Answers,
                msg::drop_without_id(&sec.heading_text, rel),
            ));
        };
        let closed = sec.answer_value.is_some()
            || led
                .rows
                .iter()
                .any(|r| r.id == qid && matches!(r.event.as_str(), "answered" | "refused"));
        if closed {
            return Err(finding(
                Class::Answers,
                msg::drop_of_closed(&sec.heading_text, sec.is_refused(), rel),
            ));
        }
        let hash = record::question_hash(rf, sec);
        let approved = led
            .rows
            .iter()
            .any(|r| r.event == "approved" && r.id == qid && r.hash == hash && r.detail == rel);
        if !approved {
            return Err(finding(
                Class::Answers,
                msg::drop_not_approved(&sec.heading_text, rel),
            ));
        }
        out.push(Drop {
            id: qid,
            hash,
            detail,
        });
    }
    Ok(out)
}

pub fn do_answer(root: &Path, id: &str, version: &str, letter: char) -> Result<Outcome, Stops> {
    let Located {
        led,
        rf,
        idx,
        tf,
        cur_hash,
        rel,
    } = locate_asked(root, id, version, Some(letter))?;

    let supersedes_val;
    {
        let sec = &rf.sections[idx];
        let Some(terms_line_idx) = sec.terms_line else {
            return Err(finding(
                Class::Terms,
                msg::no_terms_line(&rel, id, &sec.heading_text),
            ));
        };
        let problems =
            terms::validate_pairs(rf.lines[terms_line_idx].trim_end(), &tf).map_err(|_| {
                finding(
                    Class::Terms,
                    msg::malformed_terms_line(&rel, id, &sec.heading_text),
                )
            })?;
        if let Some(p) = problems.first() {
            return Err(finding(
                Class::Terms,
                match &p.kind {
                    terms::ProblemKind::Gone => msg::terms_line_names_gone_term(
                        &rel,
                        id,
                        &sec.heading_text,
                        &p.uuid,
                        &p.word,
                    ),
                    terms::ProblemKind::Retired(name) => {
                        msg::terms_line_names_retired_term(&rel, id, &sec.heading_text, name)
                    }
                },
            ));
        }
        supersedes_val = sec.supersedes.clone();
    }
    // `locate_asked` loads only the one file it finds, so every topic file
    // is loaded again here for the repo-wide heading index.
    let mut superseded_ids: Vec<String> = Vec::new();
    let all = record::load_all(root);
    {
        let cross_file_headings = depends::cross_file_heading_index(&all, &led);
        let names = crate::view::heading_names(&all, &led);
        let refusals = rules::supersedes_refusals(
            &rf.rel,
            &rf,
            &cross_file_headings,
            &names,
            &all,
            &led,
            Some(idx),
        );
        if !refusals.is_empty() {
            return Err(findings_of(refusals));
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
    let names = event_names_in(&all, &tf.terms, &led);
    let mut outcome = Outcome::of_rows(&rows, &names);
    for e in &mut outcome.events {
        if let crate::out::Event::Answered {
            heading, sentence, ..
        } = e
        {
            *heading = heading_for_print.clone();
            *sentence = sentence_for_print.clone();
        }
    }

    let letter_str = letter.to_string();
    // Every superseded question's section, the ones this run supersedes and
    // the ones that piled up before it.
    let mut strike_ids = state::superseded_ids(&led);
    strike_ids.extend(superseded_ids.iter().cloned());
    strike_ids.sort();
    strike_ids.dedup();
    let written = finish_answer(
        root,
        id,
        version,
        &letter_str,
        rf,
        idx,
        &letter_str,
        rows,
        &drop_ids,
        &strike_ids,
    )?;
    for path in written {
        outcome.events.push(crate::out::Event::FileWritten { path });
    }

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
pub fn do_refuse(root: &Path, id: &str, version: &str) -> Result<Outcome, Stops> {
    // Same lookup, terms gate, already-refused/already-answered, hash,
    // version, and ledger-state checks as the letter path (`locate_asked`).
    // The section's own terms-line/pairing checks are not shared (5), so
    // the returned terms file has no further use here.
    let Located {
        led,
        rf,
        idx,
        cur_hash,
        ..
    } = locate_asked(root, id, version, None)?;

    let heading_for_print = rf.sections[idx].heading_text.clone();

    let row = ledger::Row {
        time: ledger::write_time(&led),
        event: "refused".to_string(),
        id: id.to_string(),
        hash: cur_hash,
        detail: heading_for_print.clone(),
        version: crate::version::VERSION.to_string(),
    };

    let mut outcome = Outcome::of_rows(std::slice::from_ref(&row), &HashMap::new());
    let strike_ids = state::superseded_ids(&led);
    let written = finish_answer(
        root,
        id,
        version,
        "refuse",
        rf,
        idx,
        "refused",
        vec![row],
        &[],
        &strike_ids,
    )?;
    for path in written {
        outcome.events.push(crate::out::Event::FileWritten { path });
    }

    Ok(outcome)
}
