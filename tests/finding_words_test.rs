//! A finding says what the tool found and a move names an act on a part of
//! the records (per "What does a finding line hold?", 50b31713 A): no
//! message claims a cause, an actor or an intent the tool cannot see.
mod common;

use common::*;

/// Words that claim what the tool cannot see: who did a thing, how or why,
/// or what ought to be.
const CLAIMS: &[&str] = &[
    "by hand",
    "hand-",
    "shallow",
    "copied",
    "did not write",
    "manager",
    "probably",
    "should",
    "must",
    "seems",
    "likely",
    "someone",
    "forgot",
    "accident",
    "mistake",
    "on purpose",
    "intended",
    "meant",
    "edited",
    "edits",
    "tampered",
    "corrupt",
    "wrong",
];

// adr: 50b31713-4fea-4aa3-842a-79c2750660ba
#[test]
fn no_message_claims_what_the_tool_cannot_see() {
    let mut hits = Vec::new();
    for (rel, text) in source_files() {
        if !rel.starts_with("src/msg/") {
            continue;
        }
        for (n, line) in code_lines(&text) {
            for lit in string_literals(line) {
                let lower = lit.to_lowercase();
                for w in CLAIMS {
                    if lower.contains(w) {
                        hits.push(format!("{}:{}: {:?} in {:?}", rel, n, w, lit));
                    }
                }
            }
        }
    }
    assert!(hits.is_empty(), "{}", hits.join("\n"));
}
