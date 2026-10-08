use super::{is_note, Failure};
use crate::ledger::Ledger;
use crate::record::RecordFile;
use crate::rules::{Class, Finding};

/// The findings of the rule `rule` (`parse`: a topic file's shape, its
/// stray lines, the lines the tool writes; `tells`: the provenance, code
/// state, date and loaded-heading scan, the length rules and a question
/// that pairs no term) in the judgement of each topic file `approve`
/// refuses on (`rules::judge_topic_file`, computed once in `mod.rs`). On a
/// file awaiting approval each is a note unless it is about a question with
/// force; on an approved file each is a failure (per "What does check do
/// with a fault on a file awaiting approval?", 05141f49 A, and "What is the
/// shape of a topic file?", 9913568f A). A frozen section never starts
/// failing because a later release grew a list: the rules judge only text
/// not yet frozen.
pub(super) fn check_rule(
    class: Class,
    records: &[RecordFile],
    refusals: &[Vec<Finding>],
    awaiting: &[String],
    led: &Ledger,
) -> (Vec<Failure>, Vec<Failure>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    for (rf, refs) in records.iter().zip(refusals) {
        let draft = awaiting.contains(&rf.rel);
        for r in refs.iter().filter(|r| r.class == class) {
            let f = r.clone();
            if is_note(rf, r, draft, led) {
                notes.push(f);
            } else {
                out.push(f);
            }
        }
    }
    (out, notes)
}
