use super::Failure;
use crate::ledger::Ledger;
use crate::record::{self, RecordFile};

pub(super) fn check_answers(records: &[RecordFile], led: &Ledger) -> Vec<Failure> {
    let mut out = Vec::new();
    for rf in records {
        for (ti, sec) in rf.sections.iter().enumerate() {
            let Some(id) = &sec.id else { continue };
            let cur = record::section_hash(&rf.lines, sec);

            if sec.is_refused() {
                // 8.1 A: a `**Answer:** refused` line needs a matching
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
                        message: format!(
                            "{}: {}: refusal line without ledger row",
                            rf.rel, sec.heading_text
                        ),
                    });
                }
            } else if sec.answer_line.is_some() {
                let letter = sec.answer_letter();
                let ok = letter
                    .map(|l| {
                        led.rows.iter().any(|r| {
                            r.event == "answered"
                                && &r.id == id
                                && r.hash == cur
                                && r.detail == l.to_string()
                        })
                    })
                    .unwrap_or(false);
                if !ok {
                    out.push(Failure {
                        check: "answers".to_string(),
                        id: Some(id.clone()),
                        message: format!(
                            "{}: {}: answer line without ledger row",
                            rf.rel,
                            sec.heading_text
                        ),
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
                        message: format!(
                            "{}: {}: answered row without answer line",
                            rf.rel,
                            sec.heading_text
                        ),
                    });
                } else if has_refused_row {
                    // 8.1 A, the reverse of the refusal-line check: a
                    // `refused` row exists but the file no longer shows the
                    // `**Answer:** refused` line.
                    out.push(Failure {
                        check: "answers".to_string(),
                        id: Some(id.clone()),
                        message: format!(
                            "{}: {}: refused row without answer line",
                            rf.rel, sec.heading_text
                        ),
                    });
                } else {
                    let latest_asked_hash = led.latest_asked(id).map(|r| r.hash.clone());
                    if latest_asked_hash.as_deref() != Some(cur.as_str()) {
                        out.push(Failure {
                            check: "answers".to_string(),
                            id: Some(id.clone()),
                            message: format!(
                                "{}: {}: edited after asking; run adr ask",
                                rf.rel,
                                sec.heading_text
                            ),
                        });
                    }
                }
            }

            out.extend(check_frozen_sets(rf, ti, sec, id, led));
        }
    }
    out
}

/// 8.7 A: for every `assumes` row of an asked/answered/refused id, the
/// file's current set on that dependent entry must still hash to the row's
/// hash -- a change to a depends line's answer set after the question was
/// asked is a plan change, caught here rather than silently taking effect.
/// A dependent heading that disappeared since (renamed or removed) is fine:
/// the plan changed, and there is nothing left to compare against.
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
        let dep_heading = row.detail.trim();
        let Some((_, set)) = dependents
            .iter()
            .find(|(di, _)| rf.sections[*di].heading_text.trim() == dep_heading)
        else {
            // Disappeared dependent heading: fine, the plan changed.
            continue;
        };
        // No set at all (the depends line lost its `=...` suffix since
        // asking) hashes the same empty-set text `set_text`/`canonical_set`
        // used to: never a match against a recorded `assumes` hash, which
        // is always written for a determined set.
        let expected = match set {
            Some(s) => record::assumes_hash(dep_heading, s),
            None => record::sha256_hex(&format!("{}=", dep_heading)),
        };
        if expected != row.hash {
            out.push(Failure {
                check: "answers".to_string(),
                id: Some(id.to_string()),
                message: format!(
                    "{}: \"{}\" answer set on \"{}\" changed after ask; agent restores it, a change is a plan change",
                    rf.rel, dep_heading, sec.heading_text
                ),
            });
        }
    }
    out
}
