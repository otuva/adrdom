//! What a question names of a blame file, and what one blame file waits for.

use super::origin::{plain_path, valid_origin_path};
use super::{BlameFile, Entry, Index, Row, ShapeKind};
use crate::msg;
use crate::record;
use crate::repo;
use crate::rules::{Class, Finding, Subject};
use crate::status::FileStatus;
use std::path::Path;
/// The `decision` rows of the blame file `entry` in file order, each with
/// whether a question names it (per "What does check do with a decision row
/// that no question names?", ee27fd4c A). A question names a row when a
/// `blame:` entry of some question of `topic_files`, whatever its status,
/// resolves to `entry` (`Index::named`: its name, else its identifier) and
/// holds exactly the row's range. The one test `show --questions` counts
/// the rows by.
pub fn decision_rows<'a>(
    entry: &'a Entry,
    index: &Index,
    topic_files: &[record::RecordFile],
) -> Vec<(&'a Row, bool)> {
    let named: Vec<(usize, usize)> = topic_files
        .iter()
        .flat_map(|rf| &rf.sections)
        .flat_map(|sec| &sec.blame)
        .filter(|b| index.named(&b.file).is_some_and(|f| f.rel() == entry.rel()))
        .flat_map(|b| b.ranges.iter().copied())
        .collect();
    entry
        .rows()
        .iter()
        .filter(|r| r.is_decision())
        .map(|r| (r, named.contains(&(r.start, r.end))))
        .collect()
}

// --- what one blame file waits for ------------------------------------------

/// What `approve`, `check` and `show` all read off one blame file against
/// its origin: the findings of its shape and of the origin, what stopped
/// the read of the origin, and the origin's content when it could be read.
pub struct FileState {
    /// What the file breaks, each a finding about it: its shape, an id line
    /// another file keeps, an origin that is no regular file outside adrdom,
    /// an origin missing while the file awaits approval, a blob the origin
    /// no longer matches. A note on a file awaiting approval, a failure on
    /// an approved one (per "What does check do with a fault on a file
    /// awaiting approval?"); coverage is judged apart (`rules::judge_blame_file`).
    pub findings: Vec<Finding>,
    /// What stopped the read of the origin (unreadable as text, git could
    /// not hash it): not a rule broken, a stop, each with its move.
    pub stops: Vec<String>,
    pub origin_content: Option<String>,
}

/// Reads `bf`, whose status is `status` and which sits in `index`, against
/// its origin in `source` (per "What does check do when an origin
/// differs from its blame file's blob identifier?", 6f6e7f4a A, and "What does
/// check do when an origin is absent?", 0c3d498a A): an approved
/// file whose origin is gone has nothing to say, and its blob and coverage
/// are not judged.
pub fn file_state(
    root: &Path,
    source: &repo::Source,
    bf: &BlameFile,
    status: FileStatus,
    index: &Index,
) -> FileState {
    let rel = &bf.rel;
    let mut st = FileState {
        findings: Vec::new(),
        stops: Vec::new(),
        origin_content: None,
    };
    let about =
        |said: msg::Said| Finding::said(Class::Blame, said).about(Subject::BlameFile(rel.clone()));
    st.findings = bf
        .shape_errors
        .iter()
        .map(|e| about(e.said(rel, index.approved_id(rel))))
        .collect();
    if let Some(keeper) = index.id_taken_from(bf) {
        st.findings
            .push(about(msg::blame_id_shared(rel, keeper.rel())));
    }
    // An approved file keeps the identifier its approval gave: approve writes
    // one only for a file never approved (d2851ea1 A). A faulty id line is
    // an id line, and its own finding says what is wrong with it.
    if let (FileStatus::Changed, None, Some(_)) = (status, &bf.id, index.approved_id(rel)) {
        if !bf.shape_errors.iter().any(|e| e.kind == ShapeKind::Id) {
            st.findings.push(about(msg::blame_id_line_gone(rel)));
        }
    }
    // The tool wrote the id line and the ledger holds what it wrote: an id
    // line no `blame` row holds was written by hand, and an approved file
    // whose id line differs from its approval's was edited since. A file
    // renamed after approval holds an identifier of a row for another path
    // and has no row of its own: neither.
    if let Some(id) = bf.id.as_deref() {
        match index.approved_id(rel) {
            Some(written) if !written.eq_ignore_ascii_case(id) => {
                st.findings.push(about(msg::blame_id_edited(rel)))
            }
            Some(_) => {}
            None if !index.row_holds(id) => st.findings.push(about(msg::blame_id_not_written(rel))),
            None => {}
        }
    }
    let Some(origin) = bf.origin.clone() else {
        return st;
    };
    // The git index holds plain paths only, so the origin is looked up, read
    // and hashed by its plain path in every source; a message names it as
    // written.
    let plain = plain_path(&origin);
    if !valid_origin_path(root, source, &origin) {
        st.findings
            .push(about(msg::blame_bad_origin_path(rel, &origin)));
        return st;
    }
    if !source.exists(root, &plain) {
        if status.awaits_approval() {
            st.findings
                .push(about(msg::blame_origin_missing(rel, &origin, status)));
        }
        return st;
    }
    // A fault of the structure leaves nothing to compare the origin with; a
    // row value or an id line leaves the ranges and the blob as they are.
    if bf.shape_errors.iter().any(|e| e.kind == ShapeKind::File) {
        return st;
    }
    let blob = bf
        .blob
        .clone()
        .expect("no structure fault means blob is set");
    match source.read(root, &plain) {
        None => st.stops.push(msg::blame_origin_unreadable(rel, &origin)),
        Some(origin_content) => {
            // The working tree file is hashed the way git stores it (its
            // filters applied); the index holds the stored blob already.
            let actual = match source {
                repo::Source::WorkingTree => {
                    repo::git_hash_object_filtered(root, &origin_content, &plain)
                }
                repo::Source::Index => repo::git_hash_object_stdin(root, &origin_content),
            };
            match actual {
                // The `blob:` line may hold upper case hex; git prints lower.
                Some(actual) if !actual.eq_ignore_ascii_case(&blob) => st
                    .findings
                    .push(about(msg::blame_blob_mismatch(rel, &origin))),
                Some(_) => {}
                None => st.stops.push(msg::blame_hash_failed(rel, &origin)),
            }
            st.origin_content = Some(origin_content);
        }
    }
    st
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;
    use std::path::PathBuf;

    // adr: 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70
    // The `blob:` line takes hex in either case and git prints lower case:
    // the comparison with the origin ignores the case.
    #[test]
    fn an_upper_case_blob_matches_the_origin() {
        let root = std::env::temp_dir().join(format!("adr-blame-blob-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/x.md"), "one\ntwo\n").unwrap();
        let blob = repo::git_hash_object_stdin(&root, "one\ntwo\n").unwrap();
        let findings = |blob: &str| {
            let text = format!("origin: docs/x.md\nblob: {}\n\n1-2\tdetail\n", blob);
            let bf = parsed("adrdom/blame/x.md", &text);
            let index = Index::of(vec![bf.clone()], &ledger_approving(&[]));
            file_state(
                &root,
                &repo::Source::WorkingTree,
                &bf,
                FileStatus::New,
                &index,
            )
            .findings
            .len()
        };
        assert_eq!(findings(&blob), 0);
        assert_eq!(findings(&blob.to_uppercase()), 0);
        assert_eq!(
            findings(ZERO_BLOB),
            1,
            "a blob of another content still mismatches"
        );
        assert_eq!(findings(&ZERO_BLOB.replacen('0', "A", 1)), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    // adr: a469ecb9-a4f1-4a2a-89c5-7378cc1138b4
    //
    // A question names a row when its blame line writes the file's name or
    // its identifier with exactly the row's range, whatever the question's
    // status; a detail row is no decision row, and a range of another file
    // or one that only overlaps the row's names nothing.
    #[test]
    fn a_decision_row_is_named_by_a_question_holding_exactly_its_range() {
        let rows = "1-1\tdecision\n2-2\tdetail\n3-4\tdecision\n5-5\tdecision\n6-6\tdecision\n";
        let file = |rel: &str, id: Option<&str>| {
            let id_line = id.map(|i| format!("id: {}\n", i)).unwrap_or_default();
            parsed(
                rel,
                &format!(
                    "origin: docs/x.md\nblob: {}\n{}\n{}",
                    ZERO_BLOB, id_line, rows
                ),
            )
        };
        let index = Index::of(
            vec![
                file("adrdom/blame/x.md", Some(ID_A)),
                file("adrdom/blame/o.md", None),
            ],
            &ledger_approving(&[]),
        );
        let topic = |text: &str| {
            record::build(
                PathBuf::from("t.md"),
                "adrdom/decisions/t.md".to_string(),
                lines_of(text),
            )
        };
        let topic_files = vec![
            topic("# T\n\n### Q?\nblame: x 1-1 2-2 3-3, o 5-5\n"),
            topic(&format!("# U\n\n### R?\nblame: {} 5-5\n", ID_A)),
        ];
        let listed = |name: &str| -> Vec<(usize, usize, bool)> {
            let entry = index.by_name(name).unwrap();
            decision_rows(entry, &index, &topic_files)
                .into_iter()
                .map(|(r, named)| (r.start, r.end, named))
                .collect()
        };
        assert_eq!(
            listed("x"),
            vec![(1, 1, true), (3, 4, false), (5, 5, true), (6, 6, false)]
        );
        assert_eq!(
            listed("o"),
            vec![(1, 1, false), (3, 4, false), (5, 5, true), (6, 6, false)]
        );
        assert!(decision_rows(index.by_name("x").unwrap(), &index, &[])
            .iter()
            .all(|&(_, named)| !named));
    }
}
