use super::file_approval;
use crate::ledger::Ledger;
use crate::record::RecordFile;
use std::path::Path;

/// `done`: every file whose plan and context are approved and whose every
/// h3 section carries a letter answer (4.15, updated by 8.1 A: a refused
/// section is unanswered for `done`, same as an unasked one).
pub(super) fn check_done(root: &Path, records: &[RecordFile], led: &Ledger) -> Vec<String> {
    let mut out = Vec::new();
    for rf in records {
        let (rel, plan_ok, ctx_ok) = file_approval(root, rf, led);
        if !plan_ok || !ctx_ok {
            continue;
        }
        if rf.sections.iter().all(|s| s.answer_letter().is_some()) {
            out.push(rel);
        }
    }
    out.sort();
    out
}
