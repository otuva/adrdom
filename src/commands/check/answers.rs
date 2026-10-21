use super::Failure;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::rules::Class;
use crate::state;
use crate::terms::Term;
use std::path::Path;

/// `records` are every topic file as written, so the move an edited asked
/// question names is the one `show` computes for its file
/// (`moves::of_unasked_question_in`).
pub(super) fn check_answers(
    root: &Path,
    records: &[RecordFile],
    terms_list: &[Term],
    led: &Ledger,
    source: &crate::repo::Source,
) -> Vec<Failure> {
    let mut out = Vec::new();
    // Read once, and only when a question edited after asking needs its
    // file's moves.
    let index = std::cell::OnceCell::new();
    for rf in records {
        // The rules on each question's answer or refusal, the judgement
        // approve, ask, show and plan read (`rules::closed_findings`).
        for f in crate::rules::closed_findings(&rf.rel, rf, led) {
            out.push(f);
        }
        // The file's report, judged once for every question of it edited
        // after asking: the moves a refusal's line names are the file's, the
        // same for each of them.
        let file_report = std::cell::OnceCell::new();
        for (ti, sec) in rf.sections.iter().enumerate() {
            let Some(id) = &sec.id else { continue };
            let cur = record::section_hash(&rf.lines, sec);
            // An asked question with no answer line whose text no longer
            // hashes as asked: edited after asking. Per "What does a reference in a topic file
            // carry?", approve gives every heading an id, a question never asked included: only one
            // asked before is edited after asking. The move is the one `show` names for its file.
            let asked_open = sec.answer_line.is_none()
                && (state::is_open_again(led, id)
                    || !led
                        .rows
                        .iter()
                        .any(|r| (r.event == "answered" || r.event == "refused") && &r.id == id));
            if asked_open {
                if let Some(latest_asked_hash) = led.latest_asked(id).map(|r| r.hash.clone()) {
                    if latest_asked_hash != cur {
                        let moves = file_report.get_or_init(|| {
                            let loaded = rf;
                            let index = index.get_or_init(|| {
                                crate::rules::RepoIndex::new_via(
                                    root, records, terms_list, led, source,
                                )
                            });
                            let j = crate::moves::Judging {
                                root,
                                all: records,
                                terms_list,
                                led,
                                index,
                            };
                            crate::moves::of_unasked_question_in(&j, loaded, false)
                        });
                        out.push(Failure::said(
                            Class::Answers,
                            msg::edited_after_asking(&rf.rel, &sec.heading_text, moves),
                        ));
                    }
                }
            }

            out.extend(check_frozen_sets(rf, ti, sec, id, led));
        }
    }
    out
}

/// Per "Does a depends entry carry the answers it holds under?" and "What does a reference in a
/// topic file carry?": for every `assumes` row of an asked/answered/refused id, the file's current
/// set on that dependent entry must still hash to the row's hash -- a change to a depends line's
/// answer set after the question was asked is a plan change, caught here rather than silently
/// taking effect. The row's `detail` is the dependent's own heading id, not its heading text; a
/// dependent id that disappeared since (the heading removed, or the plan otherwise changed) is
/// fine: there is nothing left to compare against.
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
            out.push(Failure::said(
                Class::Answers,
                msg::answer_set_changed(&rf.rel, &dep_heading, &sec.heading_text),
            ));
        }
    }
    out
}
