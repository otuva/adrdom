use super::{is_note, Failure};
use crate::ledger::Ledger;
use crate::rules::Finding;
use crate::record::RecordFile;

/// `tells` (per "Does the tool stop loaded phrases in a question?"): the
/// same provenance/code-state/date/loaded-heading scan
/// `approve` and `ask` refuse on, reported here instead -- only for text not
/// yet frozen: a section whose latest ledger state is not `answered`,
/// `refused`, or `superseded`, plus the context prose, which is never
/// frozen (it carries no ledger state of its own). A frozen section never
/// starts failing because a later release grew the list.
///
/// Every line is the `tells` category of `record_refusals` (`refusals`,
/// computed once in `mod.rs`, shared with `approve`'s `do_approve_record`):
/// the heading, the context prose, each body below its heading, the length
/// rules and a question that pairs no term. On a file awaiting approval each
/// is a note, not a failure (per "Does check fail a topic file awaiting
/// approval?"); the notes come back as the second value.
pub(super) fn check_tells(
    records: &[RecordFile],
    refusals: &[Vec<Finding>],
    awaiting: &[String],
    led: &Ledger,
) -> (Vec<Failure>, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    for (rf, refs) in records.iter().zip(refusals) {
        let draft = awaiting.contains(&rf.rel);
        for r in refs {
            if r.rule == "tells" && is_note(rf, r, draft, led) {
                notes.push(format!("tells: {}", r.message));
            } else if r.rule == "tells" {
                out.push(Failure {
                    check: "tells".to_string(),
                    message: r.message.clone(),
                });
            }
        }
    }
    (out, notes)
}
