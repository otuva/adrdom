use super::{is_note, Failure};
use crate::actions::Refusal;
use crate::ledger::Ledger;
use crate::record::RecordFile;

/// `parse`: a topic file's shape errors (missing/duplicate h1, a `## `
/// tier heading, a section's own `shape_errors`) plus each section's own
/// `parse_errors` ("option X has no decision sentence", a duplicate `id:`
/// line, an answer naming a missing option) -- the `parse` category of
/// `record_refusals`, computed once in `mod.rs` and shared with `approve`'s
/// `do_approve_record` and `ask` (055d303b A point 1): both lists now
/// live in the shared `Refusal` list, so this is a plain filter over it
/// rather than its own scan over `records`.
///
/// The findings about the plan's body shape (a missing Problem or Standard,
/// an option count, a missing Why or Costs) are notes on a file awaiting
/// approval (per "Does check fail a topic file awaiting approval?"); the
/// rest is the topic file's own fixed shape (per "What does a topic file
/// hold?") and fails check.
pub(super) fn check_parse(
    records: &[RecordFile],
    refusals: &[Vec<Refusal>],
    awaiting: &[String],
    led: &Ledger,
) -> (Vec<Failure>, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    for (rf, refs) in records.iter().zip(refusals) {
        let draft = awaiting.contains(&rf.rel);
        for r in refs.iter().filter(|r| r.category == "parse") {
            if is_note(rf, r, draft, led) {
                notes.push(format!("parse: {}", r.message));
            } else {
                out.push(Failure {
                    check: "parse".to_string(),
                    message: r.message.clone(),
                });
            }
        }
    }
    (out, notes)
}
