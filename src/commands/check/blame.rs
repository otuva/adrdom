//! `blame`: holds every `adrdom/blame/*.md` file to its origin (per "Does
//! check hold a blame file to its origin?", 5fddeeb5 A) and refuses an
//! origin that has left while its blame file still has an open row (per
//! "Does an origin leave the repository while its blame file has an open
//! row?", 4bb5ef1b A). A blame file merely awaiting approval is a count,
//! never a failure (per "Does the tool stop while a blame file waits for
//! approval?", 9c3cc17e A) -- a shape error is a failure regardless, the
//! same as a topic file's.
use super::Failure;
use crate::blame;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo::{self, Source};
use std::path::Path;

pub(super) fn check_blame(
    root: &Path,
    source: &Source,
    records: &[RecordFile],
    led: &Ledger,
) -> (Vec<Failure>, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    let mut awaiting: Vec<String> = Vec::new();

    let mut rels: Vec<String> = source
        .list_under(root, repo::BLAME_DIR)
        .into_iter()
        .filter(|p| p.ends_with(".md"))
        .collect();
    rels.sort();

    // Built once for every file this run, not once per row (`decision:`/
    // `dropped: copy of <heading>` resolution needs both).
    let (any_headings, known_ids) = blame::heading_ids(records);

    for rel in &rels {
        let Some(content) = source.read(root, rel) else {
            // Present (it came from `list_under`) but unreadable as text:
            // its own failure, named, never silently skipped.
            out.push(Failure {
                check: "blame".to_string(),
                message: msg::blame_file_unreadable(rel),
            });
            continue;
        };
        let bf = blame::parse(root.join(rel), rel.clone(), record::split_lines(&content));

        let st = blame::file_state(root, source, &bf, &content, records, led, &any_headings, &known_ids);
        for message in st.failures {
            out.push(Failure { check: "blame".to_string(), message });
        }
        if st.awaiting {
            awaiting.push(rel.clone());
        }
    }

    if !awaiting.is_empty() {
        notes.push(msg::blame_awaiting(awaiting.len(), &awaiting.join(", ")));
    }

    (out, notes)
}
