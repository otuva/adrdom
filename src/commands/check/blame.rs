//! `blame`: holds every `adrdom/blame/*.md` file to its origin (per "Does
//! check hold a blame file to its origin?", 5fddeeb5 A) and refuses an
//! origin that has left while its blame file still has an open row (per
//! "Does an origin leave the repository while its blame file has an open
//! row?", 4bb5ef1b A): those fail whatever the file's approval. The rules
//! `approve` refuses a blame file on (its shape, its coverage of the origin
//! and the destination of each row, per "Does a command refuse what check
//! passes?", fe14991d A) are read from the same judgement: a note on a file
//! awaiting approval, a failure on an approved one (per "What does check do
//! with a fault on a file awaiting approval?"); the destinations resolve
//! against the tree check reads.
use super::Failure;
use crate::blame;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo::{self, Source};
use crate::rules::{self, Finding};
use std::path::Path;

/// The failures, and the notes: the findings on a blame file awaiting
/// approval.
pub(super) fn check_blame(
    root: &Path,
    source: &Source,
    records: &[RecordFile],
    led: &Ledger,
) -> (Vec<Failure>, Vec<Finding>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();

    let mut rels: Vec<String> = source
        .list_under(root, repo::BLAME_DIR)
        .into_iter()
        .filter(|p| p.ends_with(".md"))
        .collect();
    rels.sort();

    // Built once for every file this run, not once per row (`decision:`/
    // `dropped: copy of <heading>` resolution needs both).
    let (any_headings, known_ids) = blame::heading_ids(records);
    // What every row's destination resolves against, read once for the run
    // rather than once per blame file.
    let names = (!rels.is_empty()).then(|| rules::BlameNames::read(root, source));

    for rel in &rels {
        let Some(content) = source.read(root, rel) else {
            // Present (it came from `list_under`) but unreadable as text:
            // its own failure, named, never silently skipped.
            out.push(Failure {
                check: "blame",
                message: msg::blame_file_unreadable(rel),
            });
            continue;
        };
        let bf = blame::parse(root.join(rel), rel.clone(), record::split_lines(&content));

        let st = blame::file_state(root, source, &bf, &content, records, led, &any_headings, &known_ids);
        for message in &st.failures {
            out.push(Failure { check: "blame", message: message.clone() });
        }
        let mut found: Vec<Finding> = st
            .shape
            .iter()
            .map(|m| Finding::new("blame", m.clone()).about(rules::Subject::BlameFile(rel.clone())))
            .collect();
        if let Some(origin_content) = &st.origin_content {
            found.extend(rules::judge_blame_coverage(&bf, origin_content.lines().count()));
            if let Some(origin) = &bf.origin {
                if let Some(names) = &names {
                    found.extend(rules::judge_blame_destinations_in(root, &bf, origin, names).0);
                }
            }
        }
        if st.awaiting {
            notes.extend(found);
        } else {
            out.extend(found.into_iter().map(|f| Failure { check: "blame", message: f.message }));
        }
    }

    (out, notes)
}
