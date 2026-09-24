use super::Failure;
use crate::ledger::Ledger;
use crate::record::{self, RecordFile};
use crate::state::{self, rel_path};
use crate::tells;
use std::path::Path;

/// `tells` (9.5 A): the same provenance/code-state/date/loaded-heading scan
/// `approve` and `ask` refuse on, reported here instead -- only for text not
/// yet frozen: a section whose latest ledger state is not `answered`,
/// `refused`, or `superseded`, plus the context prose, which is never
/// frozen (it carries no ledger state of its own). A frozen section never
/// starts failing because a later release grew the list.
///
/// A section's heading is scanned for every tell (loaded opening, "and"/
/// "or", provenance, code state, a date); its body below the heading line
/// only for provenance/code-state/date, so a heading hit is reported once,
/// not twice.
pub(super) fn check_tells(root: &Path, records: &[RecordFile], led: &Ledger) -> Vec<Failure> {
    let mut out = Vec::new();
    for rf in records {
        let rel = rel_path(root, &rf.path);

        let (cs, ce) = rf.context_range;
        for (line, phrase, what) in tells::prose_tells_in(&rf.lines, cs, ce) {
            out.push(Failure {
                check: "tells".to_string(),
                id: None,
                message: tells::tell_text(&rel, line, "context", &phrase, what),
            });
        }

        for sec in &rf.sections {
            if state::is_frozen(led, sec) {
                continue;
            }

            for (line, phrase, what) in tells::heading_tells(sec.start, &sec.heading_text) {
                out.push(Failure {
                    check: "tells".to_string(),
                    id: sec.id.clone(),
                    message: tells::tell_text(&rel, line, &sec.heading_text, &phrase, what),
                });
            }

            let (s, e) = record::hash_range(&rf.lines, sec);
            for (line, phrase, what) in tells::prose_tells_in(&rf.lines, s + 1, e) {
                out.push(Failure {
                    check: "tells".to_string(),
                    id: sec.id.clone(),
                    message: tells::tell_text(&rel, line, &sec.heading_text, &phrase, what),
                });
            }
        }
    }
    out
}
