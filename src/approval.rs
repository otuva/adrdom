//! What a file the owner approves holds beyond its latest approval (per
//! "Does show print what a file holds beyond its latest approval?", bceceda8
//! A): for a topic file its prose and each question new, changed or struck
//! since the plan was approved; for the terms file each entry new, changed or
//! removed; for a blame file its rows. The ledger holds hashes only, so the
//! approved text is the newest committed version of the path whose hash is
//! the one the ledger approved (`approved_text`). Each part is named by a
//! status of `status::FileStatus`, never by words of its own.
use crate::ledger::Ledger;
use crate::record::{self, RecordFile, Section};
use crate::repo;
use crate::state;
use crate::status::FileStatus;
use crate::terms::{self, TermsFile};
use std::path::Path;

/// The side of one line of a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// In the approved text and in the file.
    Both,
    /// In the approved text, gone from the file.
    Removed,
    /// In the file, not in the approved text.
    Added,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub side: Side,
    pub text: String,
}

impl DiffLine {
    /// The line as `show` prints it: `+ `, `- ` or two spaces, then the text,
    /// the marks of a diff.
    pub fn marked(&self) -> String {
        let mark = match self.side {
            Side::Both => " ",
            Side::Removed => "-",
            Side::Added => "+",
        };
        format!("{} {}", mark, self.text)
    }
}

/// One part of a file beyond its latest approval: its status (`New`,
/// `Changed` or `Removed`) and its lines, marked.
#[derive(Debug, Clone)]
pub struct Change {
    pub status: FileStatus,
    pub lines: Vec<DiffLine>,
}

/// The line diff of `old` against `new` (longest common subsequence): every
/// line of both, each marked by its side, in order.
pub fn diff(old: &[String], new: &[String]) -> Vec<DiffLine> {
    let (n, m) = (old.len(), new.len());
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if old[i] == new[j] { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < n || j < m {
        if i < n && j < m && old[i] == new[j] {
            out.push(DiffLine { side: Side::Both, text: new[j].clone() });
            i += 1;
            j += 1;
        } else if i < n && (j == m || lcs[i + 1][j] >= lcs[i][j + 1]) {
            out.push(DiffLine { side: Side::Removed, text: old[i].clone() });
            i += 1;
        } else {
            out.push(DiffLine { side: Side::Added, text: new[j].clone() });
            j += 1;
        }
    }
    out
}

/// Every line of `new` as added: a part never approved.
fn added(new: &[String]) -> Vec<DiffLine> {
    new.iter().map(|t| DiffLine { side: Side::Added, text: t.clone() }).collect()
}

/// Every line of `old` as removed: a part gone from the file.
fn removed(old: &[String]) -> Vec<DiffLine> {
    old.iter().map(|t| DiffLine { side: Side::Removed, text: t.clone() }).collect()
}

/// The change of a part whose approved text is `old` (`None` when no
/// committed version holds it: the change is then the part as it stands).
fn changed(old: Option<Vec<String>>, new: &[String]) -> Vec<DiffLine> {
    match old {
        Some(old) => diff(&old, new),
        None => added(new),
    }
}

/// The committed texts of `rel`, newest first, read one at a time: one for
/// each commit that touched the path. The one walk
/// of git history every reader of an approved text goes through; a reader
/// that stops at its match reads no older version.
pub fn committed_texts<'a>(root: &'a Path, rel: &'a str) -> impl Iterator<Item = String> + 'a {
    let commits: Vec<String> = repo::git(root, &["log", "--format=%H", "--", rel])
        .map(|log| log.lines().filter(|l| !l.is_empty()).map(str::to_string).collect())
        .unwrap_or_default();
    commits.into_iter().filter_map(move |commit| repo::git(root, &["show", &format!("{}:{}", commit, rel)]))
}

/// The newest committed text of `rel` that `matches` takes.
pub fn approved_text(root: &Path, rel: &str, matches: &dyn Fn(&str) -> bool) -> Option<String> {
    committed_texts(root, rel).find(|text| matches(text))
}

fn parse_topic(root: &Path, rel: &str, text: &str) -> RecordFile {
    record::build(root.join(rel), rel.to_string(), record::split_lines(text))
}

/// A question's lines as approve hashes them (`record::question_hash`), less
/// its `id:` line: the heading, the reference lines and the body.
pub fn question_lines(rf: &RecordFile, sec: &Section) -> Vec<String> {
    let (s, e) = record::hash_range(&rf.lines, sec);
    rf.lines[s..e]
        .iter()
        .map(|l| l.trim_end().to_string())
        .filter(|l| {
            !l.trim().is_empty() && !l.starts_with("terms: ") && !l.starts_with("**Answer:**") && !l.starts_with("id: ")
        })
        .collect()
}

/// The prose above the first question, its blank lines left out.
fn prose_lines(rf: &RecordFile) -> Vec<String> {
    let (s, e) = rf.context_range;
    let e = e.min(rf.lines.len());
    rf.lines[s.min(e)..e].iter().map(|l| l.trim_end().to_string()).filter(|l| !l.trim().is_empty()).collect()
}

/// The latest hash approve wrote for the question `id` of `rel`.
fn approved_hash<'a>(led: &'a Ledger, rel: &str, id: &str) -> Option<&'a str> {
    led.rows
        .iter()
        .rfind(|r| r.event == "approved" && r.id == id && r.detail == rel)
        .map(|r| r.hash.as_str())
}

/// The approved lines of the question `id` of `rel`: from `plan` when it
/// holds that question at its approved hash, else from the newest committed
/// version that does.
fn approved_question(root: &Path, rel: &str, id: &str, hash: &str, plan: Option<&RecordFile>) -> Option<Vec<String>> {
    let lines_in = |rf: &RecordFile| {
        rf.sections
            .iter()
            .find(|s| s.id.as_deref() == Some(id) && record::question_hash(rf, s) == hash)
            .map(|s| question_lines(rf, s))
    };
    if let Some(found) = plan.and_then(lines_in) {
        return Some(found);
    }
    let text = approved_text(root, rel, &|t| lines_in(&parse_topic(root, rel, t)).is_some())?;
    lines_in(&parse_topic(root, rel, &text))
}

/// What a topic file holds beyond its latest approval.
#[derive(Debug, Clone, Default)]
pub struct TopicChanges {
    /// The prose, when it differs from its latest approval.
    pub prose: Option<Change>,
    /// Index-aligned with the file's sections: the change of each question
    /// new or changed since its latest approval, `None` for one approved as
    /// it stands.
    pub questions: Vec<Option<Change>>,
    /// Each question of the latest approved plan gone from the file and not
    /// dropped: its heading and its approved lines, all removed.
    pub struck: Vec<(String, Change)>,
}

/// What the topic file `rf` (as loaded, before
/// `depends::normalize_cross_file_deps`) holds beyond its latest approval.
pub fn topic_file(root: &Path, rf: &RecordFile, led: &Ledger) -> TopicChanges {
    let rel = rf.rel.as_str();
    let plan_ok = state::plan_approved(led, rf, rel);
    let prose = if state::context_approved(led, rf, rel) {
        None
    } else {
        let new = prose_lines(rf);
        match led.latest_for_path("context", rel) {
            None if new.is_empty() => None,
            None => Some(Change { status: FileStatus::New, lines: added(&new) }),
            Some(row) => {
                let hash = row.hash.clone();
                let old = approved_text(root, rel, &|t| record::context_hash(&parse_topic(root, rel, t)) == hash)
                    .map(|t| prose_lines(&parse_topic(root, rel, &t)));
                Some(Change { status: FileStatus::Changed, lines: changed(old, &new) })
            }
        }
    };
    if plan_ok {
        return TopicChanges { prose, questions: vec![None; rf.sections.len()], struck: Vec::new() };
    }
    // The committed version of the latest approved plan, when one holds it
    // exactly: the source of the approved text of every question.
    let plan = latest_plan(root, rel, led);
    let questions = rf
        .sections
        .iter()
        .map(|sec| {
            let new = question_lines(rf, sec);
            let hash = record::question_hash(rf, sec);
            match sec.id.as_deref().and_then(|id| approved_hash(led, rel, id).map(|h| (id, h))) {
                None => Some(Change { status: FileStatus::New, lines: added(&new) }),
                Some((_, h)) if h == hash => None,
                Some((id, h)) => {
                    let old = approved_question(root, rel, id, h, plan.as_ref());
                    Some(Change { status: FileStatus::Changed, lines: changed(old, &new) })
                }
            }
        })
        .collect();
    let in_file = |id: &str| rf.sections.iter().any(|s| s.id.as_deref() == Some(id));
    let dropped = |id: &str| led.rows.iter().any(|r| r.event == "dropped" && r.id == id);
    let struck = match &plan {
        Some(old) => old
            .sections
            .iter()
            .filter(|s| s.id.as_deref().is_some_and(|id| !in_file(id) && !dropped(id)))
            .map(|s| (s.heading_text.clone(), Change { status: FileStatus::Removed, lines: removed(&question_lines(old, s)) }))
            .collect(),
        // No committed text holds the approved plan (approved and not yet
        // committed): a struck question is still never silent. Its
        // identifier stands for its heading, which only that text held.
        None => struck_ids(rf, led)
            .into_iter()
            .map(|id| (id, Change { status: FileStatus::Removed, lines: Vec::new() }))
            .collect(),
    };
    TopicChanges { prose, questions, struck }
}

/// The most identifiers `struck_ids` tries in every combination.
const STRUCK_CANDIDATES: usize = 12;

/// The identifiers of the latest approved plan of `rf` that the file no
/// longer holds, read off the ledger alone: the questions still in the file
/// at their approved hashes, plus the set of identifiers approved for this
/// file and gone from it whose approved hashes complete the file's approved
/// hash. Empty when no set does, or when too many identifiers are gone to
/// try every set.
fn struck_ids(rf: &RecordFile, led: &Ledger) -> Vec<String> {
    let rel = rf.rel.as_str();
    let in_file = |id: &str| rf.sections.iter().any(|s| s.id.as_deref() == Some(id));
    let dropped = |id: &str| led.rows.iter().any(|r| r.event == "dropped" && r.id == id);
    let kept: Vec<String> = rf
        .sections
        .iter()
        .filter_map(|s| s.id.as_deref().and_then(|id| approved_hash(led, rel, id)).map(str::to_string))
        .collect();
    let mut gone: Vec<(String, String)> = Vec::new();
    for r in led.rows.iter().filter(|r| r.event == "approved" && r.detail == rel && r.id != "-") {
        if !in_file(&r.id) && !dropped(&r.id) && !gone.iter().any(|(id, _)| *id == r.id) {
            if let Some(h) = approved_hash(led, rel, &r.id) {
                gone.push((r.id.clone(), h.to_string()));
            }
        }
    }
    if gone.is_empty() || gone.len() > STRUCK_CANDIDATES {
        return Vec::new();
    }
    // The largest set first: a plan that held them all is the likelier one.
    let mut sets: Vec<u32> = (1..(1u32 << gone.len())).collect();
    sets.sort_by_key(|m| std::cmp::Reverse(m.count_ones()));
    for mask in sets {
        let mut hashes = kept.clone();
        hashes.extend(gone.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, (_, h))| h.clone()));
        if state::plan_approved_hashes(led, rel, &hashes) {
            return gone.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, (id, _))| id.clone()).collect();
        }
    }
    Vec::new()
}

/// The newest committed version of `rel` whose plan is exactly the latest
/// approved one.
fn latest_plan(root: &Path, rel: &str, led: &Ledger) -> Option<RecordFile> {
    led.latest_for_path("approved", rel)?;
    let text = approved_text(root, rel, &|t| {
        state::plan_approved_hashes(led, rel, &record::question_hashes(&parse_topic(root, rel, t)))
    })?;
    Some(parse_topic(root, rel, &text))
}

/// An entry's lines as the terms file holds them, less its `id:` and
/// `terms:` lines: the heading, definition, aliases and retired flag.
pub fn entry_lines(tf: &TermsFile, t: &terms::Term) -> Vec<String> {
    let start = t.start_line.saturating_sub(1);
    let end = tf
        .terms
        .iter()
        .map(|o| o.start_line.saturating_sub(1))
        .filter(|&s| s > start)
        .min()
        .unwrap_or(tf.lines.len());
    tf.lines[start..end.min(tf.lines.len())]
        .iter()
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.trim().is_empty() && !l.starts_with("id: ") && !l.starts_with("terms: "))
        .collect()
}

/// The approved lines of the entry `id`: the newest committed terms file
/// whose entry of that id hashes to `hash`.
fn approved_entry(root: &Path, id: &str, hash: &str) -> Option<Vec<String>> {
    let entry = |text: &str| {
        let tf = terms::parse(&record::split_lines(text));
        let t = tf.terms.iter().find(|t| t.id.as_deref() == Some(id) && (hash.is_empty() || terms::term_hash(t) == hash))?;
        Some(entry_lines(&tf, t))
    };
    let text = approved_text(root, repo::TERMS_PATH, &|t| entry(t).is_some())?;
    entry(&text)
}

/// What the terms file holds beyond its latest approval: index-aligned with
/// its entries, the change of each entry new or changed (`None` for one
/// approved as it stands), then each entry approved and gone from the file
/// by name.
pub fn terms_file(root: &Path, tf: &TermsFile, led: &Ledger) -> (Vec<Option<Change>>, Vec<(String, Change)>) {
    let entries = tf
        .terms
        .iter()
        .map(|t| {
            let new = entry_lines(tf, t);
            let row = t.id.as_deref().and_then(|id| led.latest_for_id("term", id).map(|r| (id, r)));
            match row {
                None if state::terms_approved(led, tf) => None,
                None => Some(Change { status: FileStatus::New, lines: added(&new) }),
                Some((_, r)) if r.hash == terms::term_hash(t) => None,
                Some((id, r)) => {
                    let old = approved_entry(root, id, &r.hash);
                    Some(Change { status: FileStatus::Changed, lines: changed(old, &new) })
                }
            }
        })
        .collect();
    let gone = state::removed_terms(led, tf)
        .into_iter()
        .map(|(id, name)| {
            let hash = led.latest_for_id("term", &id).map(|r| r.hash.clone()).unwrap_or_default();
            let old = approved_entry(root, &id, &hash).unwrap_or_else(|| vec![format!("## {}", name)]);
            (name, Change { status: FileStatus::Removed, lines: removed(&old) })
        })
        .collect();
    (entries, gone)
}

/// A blame file's lines, its blank lines left out.
fn blame_lines(lines: &[String]) -> Vec<String> {
    lines.iter().map(|l| l.trim_end().to_string()).filter(|l| !l.trim().is_empty()).collect()
}

/// What the blame file `rel` holds beyond its latest approval: its lines
/// against the approved ones, `None` while it equals its latest approval.
pub fn blame_file(root: &Path, rel: &str, lines: &[String], led: &Ledger) -> Option<Change> {
    let content = record::join_lines(lines);
    if state::blame_approved(led, rel, &content) {
        return None;
    }
    let new = blame_lines(lines);
    let Some(row) = led.latest_for_path("blame", rel) else {
        return Some(Change { status: FileStatus::New, lines: added(&new) });
    };
    let hash = row.hash.clone();
    let old = approved_text(root, rel, &|t| crate::blame::file_hash(&record::split_lines(t)) == hash)
        .map(|t| blame_lines(&record::split_lines(&t)));
    Some(Change { status: FileStatus::Changed, lines: changed(old, &new) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_diff_marks_each_line_by_its_side_in_order() {
        let d = diff(&l(&["a", "b", "c"]), &l(&["a", "x", "c", "d"]));
        let marked: Vec<String> = d.iter().map(|x| x.marked()).collect();
        assert_eq!(marked, vec!["  a", "- b", "+ x", "  c", "+ d"]);
    }

    #[test]
    fn a_part_with_no_approved_text_reads_as_added() {
        let d = changed(None, &l(&["a"]));
        assert_eq!(d[0].side, Side::Added);
    }
}
