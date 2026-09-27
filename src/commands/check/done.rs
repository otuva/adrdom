use super::file_approval;
use crate::ledger::Ledger;
use crate::record::RecordFile;
use crate::state;
use std::collections::HashMap;

/// `done`: every file whose plan and context are approved and whose every
/// h3 section carries a letter answer (per "Does a topic file close?",
/// updated by "Does the owner answer a question with a refusal?": a refused
/// section is unanswered for `done`, same as an unasked one).
pub(super) fn check_done(
    records: &[RecordFile],
    led: &Ledger,
    raw_hashes: &HashMap<String, (String, String)>,
) -> Vec<String> {
    let mut out = Vec::new();
    for rf in records {
        let (rel, plan_ok, ctx_ok) = file_approval(rf, raw_hashes, led);
        if state::file_done(rf, plan_ok, ctx_ok) {
            out.push(rel);
        }
    }
    out.sort();
    out
}
