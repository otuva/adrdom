use super::Failure;
use crate::actions::Refusal;

/// `parse`: a topic file's shape errors (missing/duplicate h1, a `## `
/// tier heading, a section's own `shape_errors`) plus each section's own
/// `parse_errors` ("option X has no decision sentence", a duplicate `id:`
/// line, an answer naming a missing option) -- the `parse` category of
/// `record_refusals`, computed once in `mod.rs` and shared with `approve`'s
/// `do_approve_record` and `ask` (055d303b A point 1): both lists now
/// live in the shared `Refusal` list, so this is a plain filter over it
/// rather than its own scan over `records`.
pub(super) fn check_parse(refusals: &[Vec<Refusal>]) -> Vec<Failure> {
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
    out
}
