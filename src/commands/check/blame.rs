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
use crate::rules::Class;
use super::Failure;
use crate::blame;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo::{self, Source};
use crate::rules::{self, Finding};
use std::path::Path;

/// The failures, and the notes: the findings on a blame file awaiting
/// approval. `as_loaded` is every topic file as read, before the normalize
/// pass rewrites `records`: what a row's destination resolves against, the
/// same files `show` resolves it against.
pub(super) fn check_blame(
    root: &Path,
    source: &Source,
    records: &[RecordFile],
    as_loaded: &[RecordFile],
    terms_list: &[crate::terms::Term],
    led: &Ledger,
    stops: &mut Vec<String>,
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
    let names = (!rels.is_empty()).then(|| rules::BlameNames::of(root, source, as_loaded, terms_list.to_vec()));

    for rel in &rels {
        let Some(content) = source.read(root, rel) else {
            // Present (it came from `list_under`) but unreadable as text:
            // its own failure, named, never silently skipped.
            stops.push(msg::blame_file_unreadable(rel));
            continue;
        };
        let bf = blame::parse(root.join(rel), rel.clone(), record::split_lines(&content));

        let st = blame::file_state(root, source, &bf, &content, records, led, &any_headings, &known_ids);
        stops.extend(st.stops.iter().cloned());
        out.extend(st.failures.iter().map(|f| f.clone().class(Class::Blame)));
        let mut found: Vec<Finding> = st.shape.clone();
        // A row whose destination does not resolve: approve refuses it on a
        // draft, so on an approved file its target went since (a term
        // retired, a question dropped by an answer, a topic file removed by
        // the owner). It is named with its move, and blocks no commit of
        // the owner's own act: a note either way.
        let mut destinations: Vec<Finding> = Vec::new();
        if let Some(origin_content) = &st.origin_content {
            found.extend(rules::judge_blame_coverage(&bf, origin_content.lines().count()));
            if let Some(origin) = &bf.origin {
                if let Some(names) = &names {
                    destinations = rules::judge_blame_destinations_in(root, &bf, origin, names).0;
                }
            }
        }
        notes.extend(destinations);
        if st.awaiting {
            notes.extend(found);
        } else {
            out.extend(found.into_iter().map(|f| f.class(Class::Blame)));
        }
    }

    (out, notes)
}
