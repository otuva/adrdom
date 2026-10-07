use super::{is_note, Failure};
use crate::rules::Finding;
use crate::ledger::Ledger;
use crate::record::RecordFile;

/// `parse`: a topic file's shape errors (missing/duplicate h1, a `## `
/// tier heading, a section's own `shape_errors`) plus each section's own
/// `parse_errors` ("option X has no decision sentence", a duplicate `id:`
/// line, an answer naming a missing option) -- the `parse` category of
/// `record_refusals`, computed once in `mod.rs` and shared with `approve`'s
/// `do_approve_record` and `ask` (178a232e A point 1): both lists now
/// live in the shared `Finding` list, so this is a plain filter over it
/// rather than its own scan over `records`.
///
/// On a file awaiting approval every one of these is a note, unless it is
/// about a question with force; on an approved file each is a failure (per
/// "What does check do with a fault on a file awaiting approval?", 05141f49
/// A, and "What is the shape of a topic file?", 9913568f A).
pub(super) fn check_parse(
    records: &[RecordFile],
    refusals: &[Vec<Finding>],
    awaiting: &[String],
    led: &Ledger,
) -> (Vec<Failure>, Vec<Failure>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    for (rf, refs) in records.iter().zip(refusals) {
        let draft = awaiting.contains(&rf.rel);
        for r in refs.iter().filter(|r| r.rule == "parse") {
            if is_note(rf, r, draft, led) {
                notes.push(Failure { check: "parse", message: r.message.clone() });
            } else {
                out.push(Failure {
                    check: "parse",
                    message: r.message.clone(),
                });
            }
        }
    }
    (out, notes)
}
