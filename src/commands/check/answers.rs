use super::Failure;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::state;

pub(super) fn check_answers(records: &[RecordFile], led: &Ledger) -> Vec<Failure> {
    let mut out = Vec::new();
    for rf in records {
        for (ti, sec) in rf.sections.iter().enumerate() {
            let Some(id) = &sec.id else { continue };
            let cur = record::section_hash(&rf.lines, sec);

            // Per "Does check fail a question answered with two different
            // options?" (e3200081-9995-4bbe-983f-2f430dc341e7): a question
            // whose standing ledger rulings (`state::conflicting_rulings`)
            // name two or more different options fails here, straight from
            // the ledger -- the file's own `**Answer:**` line only shows
            // one side and can never catch this on its own (`closed_match`
            // below only compares that one line against the ledger, so it
            // stays silent about a second, equally genuine row for a
            // different option).
            if let Some((a, b)) = state::conflicting_rulings(led, id) {
                out.push(Failure {
                    check: "answers".to_string(),
                    message: msg::answer_conflict(&rf.rel, &sec.heading_text, id, &a.label(), &b.label()),
                });
            }

            if sec.is_refused() {
                // Per "Does the owner answer a question with a refusal?":
                // a `**Answer:** refused` line needs a matching `refused`
                // row (same id, hash = the section hash before the insert
                // -- the same `cur` a letter answer is checked against),
                // shared with `ask`'s own edited-after-closed gate through
                // `state::closed_match`.
                if !matches!(
                    state::closed_match(led, &cur, sec, id),
                    state::ClosedMatch::Current
                ) {
                    out.push(Failure {
                        check: "answers".to_string(),
                        message: msg::refusal_without_row(&rf.rel, &sec.heading_text),
                    });
                }
            } else if sec.answer_line.is_some() {
                // `state::closed_match` mirrors this exactly: every
                // `answered` row for this id at all, so an edit that moves
                // the letter the file's `**Answer:**` line names out from
                // under `answer_letter()` (e.g. renaming the answered
                // option) still finds a row -- the letter going `None`
                // there is itself evidence the options changed, not
                // evidence the answer was never genuine -- and, among rows
                // at the letter the line currently names, there may be
                // more than one (a text edit followed by a genuine
                // re-answer would add a second), so any of them matching
                // the current hash is enough to call the line genuine.
                match state::closed_match(led, &cur, sec, id) {
                    state::ClosedMatch::Current => {
                        // Genuine and current: nothing to report.
                    }
                    state::ClosedMatch::Edited => {
                        // A genuine `answered` row exists for this id -- at
                        // the line's letter but a different hash, at a
                        // different letter entirely (a hand-changed answer
                        // line), or the line's letter no longer parses at
                        // all (the answered option was renamed) -- so the
                        // answer itself is real; the question's text moved
                        // since (per "Does check fail an answered question
                        // whose text changed?"). Never `answer_without_row`,
                        // whose fix would remove a real answer.
                        out.push(Failure {
                            check: "answers".to_string(),
                            message: msg::answered_text_changed(&rf.rel, &sec.heading_text, id),
                        });
                    }
                    state::ClosedMatch::NotClosed => {
                        // No genuine `answered` row for this id at all (at
                        // any letter): a hand-written answer line with
                        // nothing backing it.
                        out.push(Failure {
                            check: "answers".to_string(),
                            message: msg::answer_without_row(&rf.rel, &sec.heading_text),
                        });
                    }
                }
            } else {
                let has_answered_row = led
                    .rows
                    .iter()
                    .any(|r| r.event == "answered" && &r.id == id);
                let has_refused_row = led.rows.iter().any(|r| r.event == "refused" && &r.id == id);
                if has_answered_row {
                    out.push(Failure {
                        check: "answers".to_string(),
                        message: msg::answered_row_without_line(&rf.rel, &sec.heading_text),
                    });
                } else if has_refused_row {
                    // Per "Does the owner answer a question with a
                    // refusal?", the reverse of the refusal-line check: a
                    // `refused` row exists but the file no longer shows the
                    // `**Answer:** refused` line.
                    out.push(Failure {
                        check: "answers".to_string(),
                        message: msg::refused_row_without_line(&rf.rel, &sec.heading_text),
                    });
                } else if let Some(latest_asked_hash) = led.latest_asked(id).map(|r| r.hash.clone()) {
                    // Per "Does a reference in a topic file carry the
                    // identifier of what it names?": `approve` now gives every heading an id,
                    // including one never asked at all (a skeleton, or a
                    // real question still waiting on an earlier tier) --
                    // only a section that genuinely was asked before, and
                    // no longer hashes the same, is "edited after asking".
                    if latest_asked_hash != cur {
                        out.push(Failure {
                            check: "answers".to_string(),
                            message: msg::edited_after_asking(&rf.rel, &sec.heading_text),
                        });
                    }
                }
            }

            out.extend(check_frozen_sets(rf, ti, sec, id, led));
        }
    }
    out
}

/// Per "Does a depends entry carry the answers it holds under?" and "Does a
/// reference in a topic file carry the identifier of what it names?": for
/// every `assumes` row of an asked/answered/refused id, the file's current
/// set on that dependent entry must still hash to the row's hash -- a
/// change to a depends line's answer set after the question was asked is a
/// plan change, caught here rather than silently taking effect. The row's
/// `detail` is the dependent's own heading id, not its heading text; a dependent id that
/// disappeared since (the heading removed, or the plan otherwise changed)
/// is fine: there is nothing left to compare against.
fn check_frozen_sets(
    rf: &RecordFile,
    ti: usize,
    sec: &record::Section,
    id: &str,
    led: &Ledger,
) -> Vec<Failure> {
    let mut out = Vec::new();
    let Some(state_row) = led.latest_state(id) else {
        return out;
    };
    if !matches!(state_row.event.as_str(), "asked" | "answered" | "refused") {
        return out;
    }
    let assumes_rows = led.assumes_for(id);
    if assumes_rows.is_empty() {
        return out;
    }
    let dependents = rf.dependents_of(ti);
    for row in &assumes_rows {
        let dep_id = row.detail.trim();
        let Some((di, set)) = dependents
            .iter()
            .find(|(di, _)| rf.sections[*di].id.as_deref() == Some(dep_id))
        else {
            // Disappeared dependent id: fine, the plan changed.
            continue;
        };
        let dep_heading = rf.sections[*di].heading_text.clone();
        // No set at all (the depends line lost its `=...` suffix since
        // asking) hashes `"<dep_id>="`, an empty-set text that never
        // matches a recorded `assumes` hash, which is always written for a
        // determined set.
        let expected = match set {
            Some(s) => record::assumes_hash(dep_id, s),
            None => record::sha256_hex(&format!("{}=", dep_id)),
        };
        if expected != row.hash {
            out.push(Failure {
                check: "answers".to_string(),
                message: msg::answer_set_changed(&rf.rel, &dep_heading, &sec.heading_text),
            });
        }
    }
    out
}
