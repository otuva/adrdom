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
use crate::state;
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
    let any_headings = blame::any_headings_with_id(records);
    let known_ids = blame::known_heading_ids(&any_headings);

    for rel in &rels {
        let Some(content) = source.read(root, rel) else { continue };
        let bf = blame::parse(root.join(rel), rel.clone(), record::split_lines(&content));

        if !bf.shape_errors.is_empty() {
            for e in &bf.shape_errors {
                out.push(Failure {
                    check: "blame".to_string(),
                    message: msg::blame_shape_error(rel, e),
                });
            }
            continue;
        }
        // No shape error means both header fields parsed.
        let origin = bf.origin.clone().expect("no shape error means origin is set");
        let blob = bf.blob.clone().expect("no shape error means blob is set");

        match source.read(root, &origin) {
            None => {
                // 4bb5ef1b A: the blame file's own draft state is one more
                // "still open" reason, alongside any unanswered `decision:`
                // row.
                let mut open = blame::open_decision_rows(&bf, records, led, &any_headings, &known_ids);
                if !state::blame_approved(led, rel, &content) {
                    open.push("the blame file's own approval".to_string());
                    open.sort();
                }
                if !open.is_empty() {
                    out.push(Failure {
                        check: "blame".to_string(),
                        message: msg::blame_origin_missing(rel, &origin, &open),
                    });
                }
            }
            Some(origin_content) => {
                // 5fddeeb5 A: the origin is still here, so its current
                // content's own git blob sha must match what the file
                // names -- read through the same `source` as everything
                // else `check` judges (a staged edit to the origin is
                // judged too, per "Does check at a commit read the staged
                // files?").
                if let Some(actual) = repo::git_hash_object_stdin(root, &origin_content) {
                    if actual != blob {
                        out.push(Failure {
                            check: "blame".to_string(),
                            message: msg::blame_blob_mismatch(rel, &origin),
                        });
                    }
                }
                // 9c3cc17e A: a draft never fails on its own -- only a
                // count, gathered below.
                if !state::blame_approved(led, rel, &content) {
                    awaiting.push(rel.clone());
                }
            }
        }
    }

    if !awaiting.is_empty() {
        notes.push(msg::blame_awaiting(awaiting.len(), &awaiting.join(", ")));
    }

    (out, notes)
}
