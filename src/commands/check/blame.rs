//! `blame`: holds every `adrdom/blame/*.md` file to its origin (per "Does
//! check compare a blame file's blob identifier with its origin?", 6f6e7f4a
//! A) and to its origin staying until the file is approved (per "Does an
//! origin leave the repository once its blame file is approved?", 0c3d498a
//! A). Every rule on a blame file -- its shape, its id line, its origin and
//! its coverage of the origin, per "Does a command refuse what check
//! passes?", fe14991d A -- is read from the same judgement: a note on a file
//! awaiting approval, a failure on an approved one (per "What does check do
//! with a fault on a file awaiting approval?"). An approved file whose origin
//! has left says nothing.
use super::Failure;
use crate::blame;
use crate::msg;
use crate::repo::Source;
use crate::rules::{self, Finding};
use std::path::Path;

/// The failures, and the notes: the findings on a blame file awaiting
/// approval. `index` is the blame files as `blame::Index::read` read them
/// through `source`, `unreadable` the ones it could not read: the one read
/// of the run, the same tree the rest of it judges.
pub(super) fn check_blame(
    root: &Path,
    source: &Source,
    index: &blame::Index,
    unreadable: &[String],
    stops: &mut Vec<String>,
) -> (Vec<Failure>, Vec<Finding>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();

    // Present (it came from the listing) but unreadable as text: its own
    // failure, named, never silently skipped.
    stops.extend(unreadable.iter().map(|rel| msg::blame_file_unreadable(rel)));

    for entry in index.entries() {
        let st = blame::file_state(root, source, &entry.file, entry.status, index);
        stops.extend(st.stops.iter().cloned());
        let found = rules::judge_blame_file(&st, &entry.file);
        if entry.status.awaits_approval() {
            notes.extend(found);
        } else {
            out.extend(found);
        }
    }

    (out, notes)
}
