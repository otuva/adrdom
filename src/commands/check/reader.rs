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
pub(super) fn check_reader(root: &Path, source: &Source, since: Option<&str>) -> Vec<Failure> {
    let content = source.read(root, repo::READER_PATH).unwrap_or_default();
    let parsed = reader::parse(&content);
    let mut out: Vec<Failure> = parsed
        .malformed
        .iter()
        .map(|(lineno, raw)| Failure::said(Class::Reader, msg::malformed_reader_row(*lineno, raw)))
        .collect();
    let judged = Side::of(&[content.as_str()]);
    let edited_at = |refname: &str| ref_has_edited_row(root, repo::READER_PATH, refname, &judged);

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
        let parents: Vec<&str> = contents.iter().map(|c| c.as_str()).collect();
        if Side::of(&parents).edited_in(&content) {
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

/// The lines of one side of a comparison and the identities of its rows.
/// For a judged file the other side is a ref's copy; at a merge the judged
/// file is the one side and the union of the parents' lines the other.
struct Side<'a> {
    lines: HashSet<&'a str>,
    identities: HashSet<Identity>,
}

impl<'a> Side<'a> {
    fn of(contents: &[&'a str]) -> Side<'a> {
        let lines: HashSet<&str> = contents.iter().flat_map(|c| c.lines()).collect();
        let identities = lines.iter().filter_map(|l| reader::parse_row(l)).map(|r| identity(&r)).collect();
        Side { lines, identities }
    }

    /// Whether `from` holds a row whose line is not on this side byte for
    /// byte but whose identity is that of a row of this side: the row was
    /// edited. A row gone whole is no edit.
    fn edited_in(&self, from: &str) -> bool {
        from.lines()
            .filter(|l| !l.is_empty() && !self.lines.contains(l))
            .filter_map(reader::parse_row)
            .any(|r| self.identities.contains(&identity(&r)))
    }
}

fn ref_has_edited_row(root: &Path, path: &str, refname: &str, judged: &Side) -> bool {
    let Some(other) = repo::git(root, &["show", &format!("{}:{}", refname, path)]) else {
        return false;
    };
    judged.edited_in(&other)
}

/// The push test: the remote's copy against the local one.
pub(super) fn push_edits_row(remote_content: &str, local_content: &str) -> bool {
    Side::of(&[local_content]).edited_in(remote_content)
}
