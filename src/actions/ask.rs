//! The act ask, performed by approve on a topic file (per "Does approve ask
//! the questions of the file?", 3762e14b A): every unanswered question of
//! the file gets its `terms:` line and an `asked` row, and every dependent
//! with a set an `assumes` row.
use super::{finding, stop, Outcome, Stops};
use crate::ledger::{self, Ledger};
use crate::msg;
use crate::record::{self, RecordFile};
use crate::rules::{Class, Command};
use crate::state;
use crate::status;
use crate::terms;
use std::path::Path;

/// Writes the `terms:` line of every question of `rf` open to ask into
/// `rf` (per "Does ask pair the whole question to terms?") and returns the
/// `asked` and `assumes` rows for the questions still open on the text as
/// rewritten (per "What does ask put to the owner at one run?": every one,
/// whatever its tier). A terms line written again under a question that
/// stands as asked changes the file and writes no row.
pub(super) fn ask_open_questions(
    rf: &mut RecordFile,
    terms_list: &[terms::Term],
    led: &Ledger,
    write_time: &str,
) -> Result<Vec<ledger::Row>, Stops> {
    let candidate: Vec<bool> = rf
        .sections
        .iter()
        .map(|sec| status::open_to_ask(led, rf, sec))
        .collect();

    // Bottom to top, so a section growing by its terms line leaves the
    // line numbers of the sections above it as they are.
    let mut touched = false;
    for i in (0..rf.sections.len()).rev() {
        if !candidate[i] {
            continue;
        }
        let sec = rf.sections[i].clone();
        let text = state::section_text_excluding_terms_line(&rf.lines, &sec);
        let pairs = terms::resolve(&text, terms_list);
        // Approve refuses a question that pairs no term before it gets
        // here (per "Does ask refuse a question with no term?"); never drop
        // a binding if one does.
        let Some(new_line) = terms::format_terms_line(&pairs) else {
            return Err(finding(
                Class::Terms,
                msg::pairs_no_term_at_write(&rf.rel, &sec.heading_text),
            ));
        };
        match sec.terms_line {
            Some(tl) => {
                if rf.lines[tl] != new_line {
                    rf.lines[tl] = new_line;
                    touched = true;
                }
            }
            None => {
                // Approve gives every question its id line.
                if let Some(idx) = find_prefixed_line(&rf.lines, sec.start, sec.end, "id: ") {
                    rf.lines.insert(idx + 1, new_line);
                    touched = true;
                }
            }
        }
    }
    if touched {
        *rf = record::build(rf.path.clone(), rf.rel.clone(), rf.lines.clone());
    }

    let mut rows = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if !candidate[i] {
            continue;
        }
        let Some(id) = sec.id.clone() else { continue };
        // Read again on the text as rewritten: a question whose terms line
        // alone was edited stands as asked once the line is written back.
        if touched && !status::open_to_ask(led, rf, sec) {
            continue;
        }
        rows.push(asked_row(rf, i, &id, write_time));
        rows.extend(assumes_rows(rf, i, &id, write_time));
    }
    Ok(rows)
}

/// The line index of the first line in `[start+1, end)` that starts with
/// `prefix`: the `id:` line the `terms:` line goes under.
fn find_prefixed_line(lines: &[String], start: usize, end: usize, prefix: &str) -> Option<usize> {
    (start + 1..end).find(|&j| lines[j].starts_with(prefix))
}

fn asked_row(rf: &RecordFile, idx: usize, id: &str, write_time: &str) -> ledger::Row {
    let sec = &rf.sections[idx];
    ledger::Row {
        time: write_time.to_string(),
        event: "asked".to_string(),
        id: id.to_string(),
        hash: record::section_hash(&rf.lines, sec),
        detail: sec.heading_text.clone(),
        version: crate::version::VERSION.to_string(),
    }
}

/// One `assumes` row per dependent of section `idx` that carries a set (per
/// "Does a depends entry carry the answers it holds under?"): the set it
/// declares, frozen at ask so check catches it moving. A dependent keys on
/// its own identifier, which approve has given it.
fn assumes_rows(rf: &RecordFile, idx: usize, id: &str, write_time: &str) -> Vec<ledger::Row> {
    rf.dependents_of(idx)
        .into_iter()
        .filter_map(|(di, set)| {
            let set = set?;
            let dep_id = rf.sections[di].id.clone()?;
            Some(ledger::Row {
                time: write_time.to_string(),
                event: "assumes".to_string(),
                id: id.to_string(),
                hash: record::assumes_hash(&dep_id, set),
                detail: dep_id,
                version: crate::version::VERSION.to_string(),
            })
        })
        .collect()
}

/// The first section of `rf` in conflict (per "Does check fail a question
/// answered with two different options?", e3200081): it carries an id, the
/// ledger holds two standing rulings for it, whether or not the file shows
/// one of them on an `**Answer:**` line, and its text is the text it was
/// last asked under. A question edited since is no conflict to ask again:
/// approve's judgement refuses it with the move that restores the text.
pub(super) fn conflicted_section(rf: &RecordFile, led: &Ledger) -> Option<(usize, String)> {
    rf.sections.iter().enumerate().find_map(|(i, sec)| {
        let id = sec.id.as_ref()?;
        state::conflicting_rulings(led, id)?;
        let asked = led.latest_asked(id)?;
        (asked.hash == record::section_hash(&rf.lines, sec)).then(|| (i, id.clone()))
    })
}

/// Asks a question in conflict again: strips its `**Answer:**` line, if it
/// has one (and the blank line `answer` put above it), saves the file, and writes a
/// fresh `asked` row with its `assumes` rows. The text never moved, so the
/// hash is the one it was first asked under. Ahead of every other gate,
/// none of which bears on text that has not changed; the file goes back
/// as it was when the ledger refuses the rows.
pub(super) fn reask_conflict(
    root: &Path,
    mut rf: RecordFile,
    led: &Ledger,
    idx: usize,
    id: &str,
) -> Result<Outcome, Stops> {
    let rel = rf.rel.clone();
    let again = || msg::after_fix_owner_runs(crate::moves::command(&["approve", &*rel]));
    let original = std::fs::read_to_string(&rf.path).ok();

    let sec = &rf.sections[idx];
    if let Some(al) = sec.answer_line {
        let from = if al > sec.start && rf.lines[al - 1].trim().is_empty() {
            al - 1
        } else {
            al
        };
        rf.lines.drain(from..=al);
    }
    let rf = record::build(rf.path.clone(), rel.clone(), rf.lines.clone());

    record::save(&rf).map_err(|e| stop(Command::Approve, msg::write_failed(&rel, e, again())))?;

    let write_time = ledger::write_time(led);
    let mut rows = vec![asked_row(&rf, idx, id, &write_time)];
    rows.extend(assumes_rows(&rf, idx, id, &write_time));
    if let Err(e) = ledger::append(&root.join(crate::repo::LEDGER_PATH), &rows) {
        if let Some(original) = &original {
            let _ = std::fs::write(&rf.path, original);
        }
        return Err(stop(
            Command::Approve,
            msg::update_failed(crate::repo::LEDGER_PATH, e, again()),
        ));
    }
    Ok(Outcome::of_rows(
        &rows,
        &crate::view::heading_texts(std::slice::from_ref(&rf)),
    ))
}
