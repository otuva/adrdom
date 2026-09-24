use crate::ledger::Ledger;
use crate::msg;
use crate::record::RecordFile;
use crate::view;

/// `moot` (8.7 A): a note, never a failure -- one line for every section
/// whose heading `depends:` entry carries an answer set that excludes the
/// letter the target question was actually answered with. The plan still
/// asks the agent to look, but it is not wrong on its own: the owner may
/// simply not have re-planned yet.
///
/// Shares `view::moot_notes` with the owner's page (rather than
/// recomputing the same note from the file's `**Answer:**` letter alone)
/// so the two cannot diverge on what counts as answered: both gate on the
/// ledger's `qstate`, not the raw text in the file, since a body can be
/// edited to show a letter that was never actually asked/answered through
/// the ledger.
pub(super) fn check_moot(records: &[RecordFile], led: &Ledger) -> Vec<String> {
    let mut out = Vec::new();
    for rf in records {
        for note in view::moot_notes(rf, led) {
            out.push(msg::moot_note(&rf.rel, &note));
        }
    }
    out
}
