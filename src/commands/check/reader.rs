use crate::rules::Class;
use super::ledger::{head_parents, parent_object_present};
use super::Failure;
use crate::msg;
use crate::reader::{self, Row};
use crate::repo::{self, Source};
use std::collections::HashSet;
use std::path::Path;

/// "What does check hold of the reader's file?"
/// (abb6faed-96f7-406d-8384-002ee81bb157 A): every row at HEAD, at the other
/// side of a merge, at `--since` and at the remote of a push stays byte for
/// byte or is gone whole; no row is edited or added by hand. A malformed row
/// fails; a line new to every comparison point is judged by its shape alone.
/// A missing file is fine: a fresh repository has no rows.
pub(super) fn check_reader(root: &Path, source: &Source, since: Option<&str>, stops: &mut Vec<String>) -> Vec<Failure> {
    let _ = stops;
    let content = source.read(root, repo::READER_PATH).unwrap_or_default();
    let parsed = reader::parse(&content);
    let mut out: Vec<Failure> = parsed
        .malformed
        .iter()
        .map(|(lineno, raw)| Failure::said(Class::Reader, msg::malformed_reader_row(*lineno, raw)))
        .collect();
    let present: HashSet<&str> = content.lines().collect();
    let identities: HashSet<Identity> = parsed.rows.iter().map(identity).collect();
    let edited_at = |refname: &str| ref_has_edited_row(root, repo::READER_PATH, refname, &present, &identities);

    let head_edited = edited_at("HEAD");
    if head_edited {
        out.push(Failure::said(Class::Reader, msg::reader_row_edited()));
    }

    // At a merge, a line byte-identical to a row of any parent (HEAD
    // included) is no edit; a pending merge names its other side in
    // MERGE_HEAD, a merge commit names its parents. The merge finding adds
    // nothing when the HEAD finding was printed for the same edit.
    let merged: Vec<String> = if repo::git(root, &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_some() {
        vec!["HEAD".to_string(), "MERGE_HEAD".to_string()]
    } else {
        let parents = head_parents(root);
        if parents.len() >= 2 {
            // A parent whose commit object is missing is the ledger's report.
            parents.into_iter().filter(|sha| parent_object_present(root, sha)).collect()
        } else {
            Vec::new()
        }
    };
    if !head_edited && !merged.is_empty() {
        let contents: Vec<String> = merged
            .iter()
            .map(|r| repo::git(root, &["show", &format!("{}:{}", r, repo::READER_PATH)]).unwrap_or_default())
            .collect();
        if merge_edits_row(&content, &contents) {
            out.push(Failure::said(Class::Reader, msg::reader_merge_row_edited()));
        }
    }

    if let Some(commit) = since {
        if !repo::is_zero_sha(commit) && parent_object_present(root, commit) && edited_at(commit) {
            out.push(Failure::said(Class::Reader, msg::reader_since_row_edited(commit)));
        }
    }
    out
}

/// What a reading is: its time, the text it read, the neighbour, the prompt
/// and the model. Its word, reason and version are what a hand can edit.
type Identity = (String, String, String, String, String);

fn identity(r: &Row) -> Identity {
    (r.time.clone(), r.text_hash.clone(), r.neighbour.clone(), r.prompt_hash.clone(), r.model.clone())
}

/// Whether `other_content` holds a row whose line is not in `present`
/// byte for byte but whose identity is that of a row of the judged file: the
/// row was edited. A row gone whole is no edit.
pub(super) fn rows_edited(other_content: &str, present: &HashSet<&str>, identities: &HashSet<Identity>) -> bool {
    other_content
        .lines()
        .filter(|l| !l.is_empty() && !present.contains(l))
        .filter_map(reader::parse_row)
        .any(|r| identities.contains(&identity(&r)))
}

fn ref_has_edited_row(root: &Path, path: &str, refname: &str, present: &HashSet<&str>, identities: &HashSet<Identity>) -> bool {
    let Some(other) = repo::git(root, &["show", &format!("{}:{}", refname, path)]) else {
        return false;
    };
    rows_edited(&other, present, identities)
}

/// The merge test, judged from the judged file's lines: a line byte-identical
/// to a line of any parent is no edit; a line that is byte-identical to none
/// but shares its identity with a row of some parent is an edit.
fn merge_edits_row(content: &str, parents: &[String]) -> bool {
    let lines: HashSet<&str> = parents.iter().flat_map(|c| c.lines()).collect();
    let identities: HashSet<Identity> = parents.iter().flat_map(|c| c.lines()).filter_map(reader::parse_row).map(|r| identity(&r)).collect();
    content
        .lines()
        .filter(|l| !l.is_empty() && !lines.contains(l))
        .filter_map(reader::parse_row)
        .any(|r| identities.contains(&identity(&r)))
}

/// The same test for a push: the remote's copy against the local one.
pub(super) fn push_edits_row(remote_content: &str, local_content: &str) -> bool {
    let present: HashSet<&str> = local_content.lines().collect();
    let identities: HashSet<Identity> = reader::parse(local_content).rows.iter().map(identity).collect();
    rows_edited(remote_content, &present, &identities)
}
