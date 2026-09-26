use super::Failure;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};

pub(super) fn check_answers(records: &[RecordFile], led: &Ledger) -> Vec<Failure> {
    let mut out = Vec::new();
    for rf in records {
        for (ti, sec) in rf.sections.iter().enumerate() {
            let Some(id) = &sec.id else { continue };
            let cur = record::section_hash(&rf.lines, sec);

            if sec.is_refused() {
                // Per "Does the owner answer a question with a refusal?":
                // a `**Answer:** refused` line needs a matching
                // `refused` row (same id, hash = the section hash before
                // the insert -- the same `cur` a letter answer is checked
                // against).
                let has_row = led
                    .rows
                    .iter()
                    .any(|r| r.event == "refused" && &r.id == id && r.hash == cur);
                if !has_row {
                    out.push(Failure {
                        check: "answers".to_string(),
                        id: Some(id.clone()),
                        message: msg::refusal_without_row(&rf.rel, &sec.heading_text),
                    });
                }
            } else if sec.answer_line.is_some() {
                let letter = sec.answer_letter();
                // Every `answered` row for this id at all, so an edit that
                // moves the letter the file's `**Answer:**` line names out
                // from under `answer_letter()` (e.g. renaming the answered
                // option) still finds the row: `letter` going `None` there
                // is itself evidence the options changed, not evidence the
                // answer was never genuine.
                let rows_for_id: Vec<_> = led
                    .rows
                    .iter()
                    .filter(|r| r.event == "answered" && &r.id == id)
                    .collect();
                // Among those, the ones at the letter the line currently
                // names -- there may be more than one (a text edit followed
                // by a genuine re-answer would add a second), so any of them
                // matching the current hash is enough to call the line
                // genuine.
                let rows_for_letter: Vec<_> = letter
                    .map(|l| {
                        rows_for_id
                            .iter()
                            .filter(|r| r.detail == l.to_string())
                            .copied()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if rows_for_letter.iter().any(|r| r.hash == cur) {
                    // Genuine and current: nothing to report.
                } else if !rows_for_id.is_empty() {
                    // A genuine `answered` row exists for this id -- at the
                    // line's letter but a different hash, at a different
                    // letter entirely (a hand-changed answer line), or the
                    // line's letter no longer parses at all (the answered
                    // option was renamed) -- so the answer itself is real;
                    // the question's text moved since (per "Does check fail
                    // an answered question whose text changed?"). Never
                    // `answer_without_row`, whose fix would remove a real
                    // answer.
                    out.push(Failure {
                        check: "answers".to_string(),
                        id: Some(id.clone()),
                        message: msg::answered_text_changed(&rf.rel, &sec.heading_text, id),
                    });
                } else {
                    // No genuine `answered` row for this id at all (at any
                    // letter): a hand-written answer line with nothing
                    // backing it.
                    out.push(Failure {
                        check: "answers".to_string(),
                        id: Some(id.clone()),
                        message: msg::answer_without_row(&rf.rel, &sec.heading_text),
                    });
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
                        id: Some(id.clone()),
                        message: msg::answered_row_without_line(&rf.rel, &sec.heading_text),
                    });
                } else if has_refused_row {
                    // Per "Does the owner answer a question with a
                    // refusal?", the reverse of the refusal-line check: a
                    // `refused` row exists but the file no longer shows the
                    // `**Answer:** refused` line.
                    out.push(Failure {
                        check: "answers".to_string(),
                        id: Some(id.clone()),
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
                            id: Some(id.clone()),
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
        // asking) hashes the same empty-set text `set_text`/`canonical_set`
        // used to: never a match against a recorded `assumes` hash, which
        // is always written for a determined set.
        let expected = match set {
            Some(s) => record::assumes_hash(dep_id, s),
            None => record::sha256_hex(&format!("{}=", dep_id)),
        };
        if expected != row.hash {
            out.push(Failure {
                check: "answers".to_string(),
                id: Some(id.to_string()),
                message: msg::answer_set_changed(&rf.rel, &dep_heading, &sec.heading_text),
            });
        }
    }
    out
}
