use super::Failure;
use crate::actions::Refusal;
use crate::record::RecordFile;

/// `parse`: a topic file's shape errors (missing/duplicate h1, a `## `
/// tier heading, a section's own `shape_errors`) -- the `parse` category
/// of `record_refusals`, computed once in `mod.rs` and shared with
/// `approve`'s `do_approve_record` (E1.2) -- plus each section's own
/// `parse_errors` ("option X has no decision sentence", a duplicate `id:`
/// line, an answer naming a missing option): a check-only detail `approve`
/// never reads at all, so it stays a plain scan over `records` rather than
/// living in the shared `Refusal` list.
pub(super) fn check_parse(records: &[RecordFile], refusals: &[Vec<Refusal>]) -> Vec<Failure> {
    let mut out = Vec::new();
    for refs in refusals {
        for r in refs {
            if r.category == "parse" {
                out.push(Failure {
                    check: "parse".to_string(),
                    id: None,
                    message: r.message.clone(),
                });
            }
        }
    }
    for rf in records {
        for sec in &rf.sections {
            for e in &sec.parse_errors {
                out.push(Failure {
                    check: "parse".to_string(),
                    id: sec.id.clone(),
                    message: format!("{}: {}: {}", rf.rel, sec.heading_text, e),
                });
            }
        }
    }
    out
}
