use super::Failure;
use crate::actions::Refusal;

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
/// rules and a question that pairs no term.
pub(super) fn check_tells(refusals: &[Vec<Refusal>]) -> Vec<Failure> {
    let mut out = Vec::new();
    for refs in refusals {
        for r in refs {
            if r.category == "tells" {
                out.push(Failure {
                    check: "tells".to_string(),
                    message: r.message.clone(),
                });
            }
        }
    }
    out
}
