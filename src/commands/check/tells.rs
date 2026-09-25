use super::Failure;
use crate::actions::Refusal;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::state::{self, rel_path};
use crate::tells;
use std::path::Path;

/// `tells` (per "Does the tool stop loaded phrases in a question?"): the
/// same provenance/code-state/date/loaded-heading scan
/// `approve` and `ask` refuse on, reported here instead -- only for text not
/// yet frozen: a section whose latest ledger state is not `answered`,
/// `refused`, or `superseded`, plus the context prose, which is never
/// frozen (it carries no ledger state of its own). A frozen section never
/// starts failing because a later release grew the list.
///
/// The heading-tell and context-prose-tell lines are the `tells` category
/// of `record_refusals` (`refusals`, computed once in `mod.rs`, shared
/// with `approve`'s `do_approve_record`). Each section's body below
/// the heading (Problem/Standard/Why/Costs) is scanned here too, for
/// provenance/code-state/date only -- `approve` never reads this far into
/// a body, only the heading and the file's context prose, so it stays a
/// check-only extra rather than living in the shared list.
pub(super) fn check_tells(root: &Path, records: &[RecordFile], led: &Ledger, refusals: &[Vec<Refusal>]) -> Vec<Failure> {
    let mut out = Vec::new();
    for refs in refusals {
        for r in refs {
            if r.category == "tells" {
                out.push(Failure {
                    check: "tells".to_string(),
                    id: None,
                    message: r.message.clone(),
                });
            }
        }
    }

    for rf in records {
        let rel = rel_path(root, &rf.path);
        for sec in &rf.sections {
            if state::is_frozen(led, sec) {
                continue;
            }
            let (s, e) = record::hash_range(&rf.lines, sec);
            for (line, phrase, what) in tells::prose_tells_in(&rf.lines, s + 1, e) {
                out.push(Failure {
                    check: "tells".to_string(),
                    id: sec.id.clone(),
                    message: msg::tell(&rel, line, &sec.heading_text, &phrase, what),
                });
            }
        }
    }
    out
}
