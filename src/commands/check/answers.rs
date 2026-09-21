use super::Failure;
use crate::ledger::Ledger;
use crate::record::{self, RecordFile};

pub(super) fn check_answers(records: &[RecordFile], led: &Ledger) -> Vec<Failure> {
    let mut out = Vec::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            let cur = record::section_hash(&rf.lines, sec);
            match sec.answer_line {
                Some(_) => {
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
                }
                None => {
                    let has_answered_row = led
                        .rows
                        .iter()
                        .any(|r| r.event == "answered" && &r.id == id);
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
            }
        }
    }
    out
}
