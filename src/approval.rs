//! What a file the owner approves holds beyond its latest approval (per
//! "Does show print what a file holds past its latest approval?", 91f9652a
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
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

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
/// `Changed` or `Removed`) and its lines, marked. `baseline` is false for a
/// changed part whose approved text no commit holds: its lines are all
/// marked added, and `show` says why rather than pass them off as new.
#[derive(Debug, Clone)]
pub struct Change {
    pub status: FileStatus,
    pub lines: Vec<DiffLine>,
    pub baseline: bool,
}

impl Change {
    fn new(status: FileStatus, lines: Vec<DiffLine>) -> Change {
        Change { status, lines, baseline: true }
    }

    /// A changed part: its lines against `old`, the approved lines when a
    /// commit holds them.
    fn changed(old: Option<Vec<String>>, new: &[String]) -> Change {
        Change { status: FileStatus::Changed, baseline: old.is_some(), lines: changed(old, new) }
    }
}

/// The line diff of `old` against `new` (longest common subsequence): every
/// line of both, each marked by its side, in order.
pub fn diff(old: &[String], new: &[String]) -> Vec<DiffLine> {
    // The lines both start and end with are kept as they are; only the
    // stretch between is diffed, so one changed row of a long file costs
    // that row, not the whole file squared.
    let head = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let tail = old[head..].iter().rev().zip(new[head..].iter().rev()).take_while(|(a, b)| a == b).count();
    let both = |t: &String| DiffLine { side: Side::Both, text: t.clone() };
    let mut out: Vec<DiffLine> = old[..head].iter().map(both).collect();
    out.extend(diff_middle(&old[head..old.len() - tail], &new[head..new.len() - tail]));
    out.extend(new[new.len() - tail..].iter().map(both));
    out
}

fn diff_middle(old: &[String], new: &[String]) -> Vec<DiffLine> {
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

/// What the run has read of git history, kept while `HEAD` stays where it
/// was: the commits that touched a path, the text of a path at a commit,
/// and that text parsed as a topic file. A long-running `serve` drops all of
/// it when `HEAD` moves, so what it keeps is bounded by one `HEAD`'s history
/// and a new commit is read. A path a commit does not hold (the commit that
/// deleted it) is kept as absent, since a commit's content never changes; a
/// `git show` that fails for any other reason is not kept, and the next
/// reader asks git again.
#[derive(Default)]
struct History {
    /// The repository and the `HEAD` everything below was read at.
    at: Option<(PathBuf, String)>,
    /// Keyed by repository, `HEAD` and path, so a list read while another
    /// thread moved the cache on can never stand for another repository.
    commits: HashMap<(PathBuf, String, String), Arc<Vec<String>>>,
    /// Keyed by repository, commit and path: what a commit holds never
    /// changes, whatever `HEAD` is. `None` where the commit does not hold
    /// the path.
    texts: HashMap<(PathBuf, String, String), Option<Arc<String>>>,
    topics: HashMap<(PathBuf, String, String), Arc<RecordFile>>,
}

fn history() -> std::sync::MutexGuard<'static, History> {
    static HISTORY: OnceLock<Mutex<History>> = OnceLock::new();
    HISTORY.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

/// The commits that touched `rel`, newest first. Reads `HEAD` first and
/// drops what was kept of another repository or another `HEAD`.
fn commits_of(root: &Path, rel: &str) -> Arc<Vec<String>> {
    let head = repo::git(root, &["rev-parse", "HEAD"]).unwrap_or_default().trim().to_string();
    let key = (root.to_path_buf(), head.clone(), rel.to_string());
    let at = Some((root.to_path_buf(), head));
    {
        let mut h = history();
        if h.at != at {
            *h = History { at, ..Default::default() };
        }
        if let Some(found) = h.commits.get(&key) {
            return found.clone();
        }
    }
    // Every commit that touched the path, the side of a merge the merge did
    // not take included (`--full-history`), each line a hash and nothing
    // else (`--no-show-signature` over a user's `log.showSignature`).
    let Some(log) = repo::git(root, &["log", "--no-show-signature", "--full-history", "--format=%H", "--", rel]) else {
        // Not kept: the next reader asks git again.
        return Arc::new(Vec::new());
    };
    let commits: Vec<String> = log
        .lines()
        .map(str::trim)
        .filter(|l| crate::blame::is_hex_oid(l))
        .map(str::to_string)
        .collect();
    let commits = Arc::new(commits);
    history().commits.insert(key, commits.clone());
    commits
}

/// The text of `rel` at `commit`, `None` where the commit deleted it or git
/// could not read it.
fn text_at(root: &Path, commit: &str, rel: &str) -> Option<Arc<String>> {
    let key = (root.to_path_buf(), commit.to_string(), rel.to_string());
    if let Some(found) = history().texts.get(&key) {
        return found.clone();
    }
    let text = match repo::git(root, &["show", &format!("{}:{}", commit, rel)]) {
        Some(text) => Some(Arc::new(text)),
        // Kept as absent only when the commit's tree holds no such path (the
        // commit that deleted it); any other failure is asked again.
        None if commit_lacks(root, commit, rel) => None,
        None => return None,
    };
    history().texts.insert(key, text.clone());
    text
}

/// Whether `commit`'s tree holds nothing at `rel`: git lists the tree and
/// names no entry.
fn commit_lacks(root: &Path, commit: &str, rel: &str) -> bool {
    repo::git(root, &["ls-tree", "--name-only", commit, "--", rel]).is_some_and(|out| out.trim().is_empty())
}

/// The text of `rel` at `commit` parsed as a topic file. The parsed file
/// carries `root.join(rel)` as its path, and the key holds `root`, so a
/// version kept for one repository never stands for another.
fn topic_at(root: &Path, commit: &str, rel: &str) -> Option<Arc<RecordFile>> {
    let key = (root.to_path_buf(), commit.to_string(), rel.to_string());
    if let Some(found) = history().topics.get(&key) {
        return Some(found.clone());
    }
    let text = text_at(root, commit, rel)?;
    let rf = Arc::new(record::build(root.join(rel), rel.to_string(), record::split_lines(&text)));
    history().topics.insert(key, rf.clone());
    Some(rf)
}

/// The committed texts of `rel`, newest first, read one at a time: one for
/// each commit that touched the path. The one walk of git history every
/// reader of an approved text goes through; a reader that stops at its match
/// reads no older version, and a version read once is not read again in the
/// same run.
pub fn committed_texts(root: &Path, rel: &str) -> impl Iterator<Item = Arc<String>> {
    let (root, rel) = (root.to_path_buf(), rel.to_string());
    let commits = commits_of(&root, &rel);
    (0..commits.len()).filter_map(move |i| text_at(&root, &commits[i], &rel))
}

/// `committed_texts` of a topic file, each parsed.
pub fn committed_topics(root: &Path, rel: &str) -> impl Iterator<Item = Arc<RecordFile>> {
    let (root, rel) = (root.to_path_buf(), rel.to_string());
    let commits = commits_of(&root, &rel);
    (0..commits.len()).filter_map(move |i| topic_at(&root, &commits[i], &rel))
}

/// The newest committed text of `rel` that `matches` takes.
pub fn approved_text(root: &Path, rel: &str, matches: &dyn Fn(&str) -> bool) -> Option<Arc<String>> {
    committed_texts(root, rel).find(|text| matches(text))
}

/// The newest committed version of the topic file `rel` that `matches`
/// takes.
fn approved_topic(root: &Path, rel: &str, matches: &dyn Fn(&RecordFile) -> bool) -> Option<Arc<RecordFile>> {
    committed_topics(root, rel).find(|rf| matches(rf))
}

/// A question's lines as approve hashes them (`record::question_hash`): the
/// heading, the reference lines, the `id:` line and the body, so what show
/// prints of a change is what approve signs.
pub fn question_lines(rf: &RecordFile, sec: &Section) -> Vec<String> {
    let (s, e) = record::hash_range(&rf.lines, sec);
    rf.lines[s..e]
        .iter()
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.trim().is_empty() && !l.starts_with("terms: ") && !l.starts_with("**Answer:**"))
        .collect()
}

/// The prose above the first question as `record::context_hash` signs it:
/// its blank lines at either end left out, those between kept, so an edit
/// of blank lines alone shows as one.
fn prose_lines(rf: &RecordFile) -> Vec<String> {
    let (s, e) = rf.context_range;
    let e = e.min(rf.lines.len());
    inner_lines(&rf.lines[s.min(e)..e])
}

/// Whether a question of `rf` carries the identifier `id`.
fn holds(rf: &RecordFile, id: &str) -> bool {
    rf.sections.iter().any(|s| s.id.as_deref() == Some(id))
}

/// `lines` as a file hash signs them: each trimmed at its end, every blank
/// line kept, those at either end too.
fn signed_lines(lines: &[String]) -> Vec<String> {
    lines.iter().map(|l| l.trim_end().to_string()).collect()
}

/// The change of a whole file whose approved text is `old` (`None` when no
/// commit holds it): read on its lines less the blank ones at either end,
/// and, when that marks nothing although the file differs from its
/// approval (a blank line added or taken at an end), on every line as the
/// file hash signs it, so a changed file never prints with nothing marked.
fn file_change(old: Option<Vec<String>>, new: &[String]) -> Change {
    let trimmed = Change::changed(old.as_deref().map(inner_lines), &inner_lines(new));
    if trimmed.lines.iter().any(|d| d.side != Side::Both) {
        return trimmed;
    }
    Change::changed(old.as_deref().map(signed_lines), &signed_lines(new))
}

/// `lines`, each trimmed at its end, less the blank lines at either end.
fn inner_lines(lines: &[String]) -> Vec<String> {
    let lines: Vec<String> = lines.iter().map(|l| l.trim_end().to_string()).collect();
    let from = lines.iter().position(|l| !l.is_empty()).unwrap_or(lines.len());
    let to = lines.iter().rposition(|l| !l.is_empty()).map_or(from, |i| i + 1);
    lines[from..to].to_vec()
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
    let rf = approved_topic(root, rel, &|rf| lines_in(rf).is_some())?;
    lines_in(&rf)
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
    /// True when no commit holds the approved plan and the ledger alone
    /// cannot tell which of the questions approved for this file and gone
    /// from it the latest plan held: `struck` then names every one of them,
    /// by identifier, and some may have left by an earlier approval.
    pub struck_unsure: bool,
}

/// What the topic file `rf` (as loaded) holds beyond its latest approval.
pub fn topic_file(root: &Path, rf: &RecordFile, led: &Ledger) -> TopicChanges {
    let rel = rf.rel.as_str();
    let plan_ok = state::plan_approved(led, rf, rel);
    let prose = if state::context_approved(led, rf, rel) {
        None
    } else {
        let new = prose_lines(rf);
        match led.latest_for_path("context", rel) {
            None if new.is_empty() => None,
            None => Some(Change::new(FileStatus::New, added(&new))),
            Some(row) => {
                let hash = row.hash.clone();
                let old = approved_topic(root, rel, &|rf| record::context_hash(rf) == hash).map(|rf| prose_lines(&rf));
                Some(Change::changed(old, &new))
            }
        }
    };
    if plan_ok {
        return TopicChanges { prose, questions: vec![None; rf.sections.len()], struck: Vec::new(), struck_unsure: false };
    }
    // The committed version of the latest approved plan, when one holds it
    // exactly: the source of the approved text of every question.
    let plan = latest_plan(root, rel, led);
    // No commit holds it (approved and not yet committed): the ledger alone
    // tells which questions it held.
    let from_ledger = plan.is_none().then(|| ledger_plan(rf, led));
    let questions = rf
        .sections
        .iter()
        .map(|sec| {
            let new = question_lines(rf, sec);
            let hash = record::question_hash(rf, sec);
            // A question the latest approved plan does not hold is new
            // beyond it, whatever an earlier approval said of its text (one
            // struck by a plan change and put back reads as added).
            let outside_plan = match (&plan, &from_ledger) {
                (Some(p), _) => !p.sections.iter().any(|ps| ps.id.is_some() && ps.id == sec.id),
                (None, Some(lp)) => sec.id.as_ref().is_some_and(|id| lp.outside.contains(id)),
                (None, None) => false,
            };
            match sec.id.as_deref().and_then(|id| approved_hash(led, rel, id).map(|h| (id, h))) {
                None => Some(Change::new(FileStatus::New, added(&new))),
                Some(_) if outside_plan => Some(Change::new(FileStatus::New, added(&new))),
                Some((_, h)) if h == hash => None,
                Some((id, h)) => {
                    let old = approved_question(root, rel, id, h, plan.as_deref());
                    Some(Change::changed(old, &new))
                }
            }
        })
        .collect();
    let in_file = |id: &str| holds(rf, id);
    let (struck, struck_unsure) = match &plan {
        Some(old) => (
            old.sections
                .iter()
                .filter(|s| s.id.as_deref().is_some_and(|id| !in_file(id) && !state::is_dropped(led, id) && !state::is_superseded(led, id)))
                .map(|s| (s.heading_text.clone(), Change::new(FileStatus::Removed, removed(&question_lines(old, s)))))
                .collect(),
            false,
        ),
        // No committed text holds the approved plan (approved and not yet
        // committed): a struck question is still never silent. Its
        // identifier stands for its heading, which only that text held.
        None => {
            let lp = from_ledger.unwrap_or_else(|| ledger_plan(rf, led));
            (lp.struck.into_iter().map(|id| (id, Change::new(FileStatus::Removed, Vec::new()))).collect(), !lp.sure)
        }
    };
    TopicChanges { prose, questions, struck, struck_unsure }
}

/// The most identifiers `ledger_plan` tries in every combination.
const STRUCK_CANDIDATES: usize = 12;

/// What the ledger alone tells of the latest approved plan of `rf` when no
/// commit holds it: the identifiers it held that the file no longer holds
/// (`struck`), the questions of the file it did not hold (`outside`: put
/// back after a plan change struck them), and whether that is known for
/// sure. The plan is the set of approved questions whose approved hashes
/// complete the file's approved hash: first the questions still in the file
/// plus a set of those gone, then, when none completes it, any set of both.
/// When no set does, or too many identifiers are in play to try every set,
/// every identifier gone is named, not sure: a struck question is never
/// silent.
fn ledger_plan(rf: &RecordFile, led: &Ledger) -> LedgerPlan {
    let rel = rf.rel.as_str();
    let in_file = |id: &str| holds(rf, id);
    let kept: Vec<(String, String)> = rf
        .sections
        .iter()
        .filter_map(|s| {
            let id = s.id.as_deref()?;
            approved_hash(led, rel, id).map(|h| (id.to_string(), h.to_string()))
        })
        .collect();
    let mut gone: Vec<(String, String)> = Vec::new();
    for r in led.rows.iter().filter(|r| r.event == "approved" && r.detail == rel && r.id != "-") {
        if !in_file(&r.id) && !state::is_dropped(led, &r.id) && !state::is_superseded(led, &r.id) && !gone.iter().any(|(id, _)| *id == r.id) {
            if let Some(h) = approved_hash(led, rel, &r.id) {
                gone.push((r.id.clone(), h.to_string()));
            }
        }
    }
    let hashes_of = |v: &[&(String, String)]| v.iter().map(|(_, h)| h.clone()).collect::<Vec<_>>();
    let ids_of = |v: &[&(String, String)]| v.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();
    // With nothing gone a set the search cannot find leaves nothing struck
    // unnamed: the plan may be one approved before (or no plan at all).
    let unsure = || LedgerPlan { struck: ids_of(&gone.iter().collect::<Vec<_>>()), outside: Vec::new(), sure: gone.is_empty() };
    // The questions still in the file make the approved plan alone.
    if state::plan_approved_hashes(led, rel, &hashes_of(&kept.iter().collect::<Vec<_>>())) {
        return LedgerPlan { struck: Vec::new(), outside: Vec::new(), sure: true };
    }
    // Every question still in the file in the plan, and a set of those gone.
    if !gone.is_empty() && gone.len() <= STRUCK_CANDIDATES {
        if let Some(mask) = completing_set(gone.len(), |mask| {
            let mut set: Vec<&(String, String)> = kept.iter().collect();
            set.extend(gone.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, g)| g));
            state::plan_approved_hashes(led, rel, &hashes_of(&set))
        }) {
            let struck = gone.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, (id, _))| id.clone()).collect();
            return LedgerPlan { struck, outside: Vec::new(), sure: true };
        }
    }
    // Any set of both: a question of the file the plan did not hold.
    let both: Vec<&(String, String)> = kept.iter().chain(gone.iter()).collect();
    if both.len() > STRUCK_CANDIDATES {
        return unsure();
    }
    match completing_set(both.len(), |mask| {
        let set: Vec<&(String, String)> = both.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, b)| *b).collect();
        state::plan_approved_hashes(led, rel, &hashes_of(&set))
    }) {
        Some(mask) => {
            let held = |i: usize| mask & (1 << i) != 0;
            let outside = (0..kept.len()).filter(|&i| !held(i)).map(|i| kept[i].0.clone()).collect();
            let struck = (0..gone.len()).filter(|&i| held(kept.len() + i)).map(|i| gone[i].0.clone()).collect();
            LedgerPlan { struck, outside, sure: true }
        }
        None => unsure(),
    }
}

/// The first set of `n` items, the largest first, that `completes`; a set
/// is a bit mask over the items.
fn completing_set(n: usize, completes: impl Fn(u32) -> bool) -> Option<u32> {
    let mut sets: Vec<u32> = (1..(1u32 << n)).collect();
    sets.sort_by_key(|m| std::cmp::Reverse(m.count_ones()));
    sets.into_iter().find(|&m| completes(m))
}

/// The latest approved plan of a file as the ledger alone tells it
/// (`ledger_plan`).
struct LedgerPlan {
    struck: Vec<String>,
    outside: Vec<String>,
    sure: bool,
}

/// The newest committed version of `rel` whose plan is exactly the latest
/// approved one.
fn latest_plan(root: &Path, rel: &str, led: &Ledger) -> Option<Arc<RecordFile>> {
    led.latest_for_path("approved", rel)?;
    approved_topic(root, rel, &|rf| state::plan_approved_hashes(led, rel, &record::question_hashes(rf)))
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
/// approved as it stands, by `status::term_status`), then each entry
/// approved and gone from the file by name, then the change of the file as
/// a whole when it differs from its approval and no entry does (entries
/// reordered, blank lines edited): never a changed file with nothing marked.
pub fn terms_file(root: &Path, tf: &TermsFile, led: &Ledger) -> TermsChanges {
    use crate::status::TermStatus;
    let entries: Vec<Option<Change>> = tf
        .terms
        .iter()
        .map(|t| {
            let new = entry_lines(tf, t);
            match crate::status::term_status(led, tf, t) {
                TermStatus::New => Some(Change::new(FileStatus::New, added(&new))),
                TermStatus::Changed => {
                    let id = t.id.as_deref().unwrap_or_default();
                    let hash = led.latest_for_id("term", id).map(|r| r.hash.clone()).unwrap_or_default();
                    Some(Change::changed(approved_entry(root, id, &hash), &new))
                }
                TermStatus::Approved | TermStatus::Retired | TermStatus::Removed => None,
            }
        })
        .collect();
    let gone: Vec<(String, Change)> = state::removed_terms(led, tf)
        .into_iter()
        .map(|(id, name)| {
            let hash = led.latest_for_id("term", &id).map(|r| r.hash.clone()).unwrap_or_default();
            let old = approved_entry(root, &id, &hash).unwrap_or_else(|| vec![format!("## {}", name)]);
            (name, Change::new(FileStatus::Removed, removed(&old)))
        })
        .collect();
    let whole = (entries.iter().all(Option::is_none) && gone.is_empty() && !state::terms_approved(led, tf))
        .then(|| {
            let hash = led.latest_for_path("terms", repo::TERMS_PATH)?.hash.clone();
            let old = approved_text(root, repo::TERMS_PATH, &|t| terms::file_hash(&terms::parse(&record::split_lines(t))) == hash)
                .map(|t| record::split_lines(&t));
            Some(file_change(old, &tf.lines))
        })
        .flatten();
    TermsChanges { entries, gone, whole }
}

/// What the terms file holds beyond its latest approval (`terms_file`).
#[derive(Debug, Clone, Default)]
pub struct TermsChanges {
    pub entries: Vec<Option<Change>>,
    pub gone: Vec<(String, Change)>,
    pub whole: Option<Change>,
}

/// What the blame file `rel` holds beyond its latest approval: its lines
/// against the approved ones, `None` while it equals its latest approval.
/// Blank lines between rows are kept, as `blame::file_hash` signs them.
pub fn blame_file(root: &Path, rel: &str, lines: &[String], led: &Ledger) -> Option<Change> {
    let content = record::join_lines(lines);
    if state::blame_approved(led, rel, &content) {
        return None;
    }
    let Some(row) = led.latest_blame(rel) else {
        return Some(Change::new(FileStatus::New, added(&inner_lines(lines))));
    };
    let hash = row.hash.clone();
    let old = approved_text(root, rel, &|t| crate::blame::file_hash(&record::split_lines(t)) == hash)
        .map(|t| record::split_lines(&t));
    Some(file_change(old, lines))
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
    fn a_part_with_no_approved_text_reads_as_added_and_says_so() {
        let c = Change::changed(None, &l(&["a"]));
        assert_eq!(c.lines[0].side, Side::Added);
        assert!(!c.baseline);
        assert!(Change::changed(Some(l(&["a"])), &l(&["b"])).baseline);
    }

    #[test]
    fn a_diff_keeps_the_lines_both_ends_share() {
        let old: Vec<String> = (0..2000).map(|i| format!("row {}", i)).collect();
        let mut new = old.clone();
        new[1000] = "row changed".to_string();
        let d = diff(&old, &new);
        assert_eq!(d.iter().filter(|x| x.side != Side::Both).count(), 2);
        assert_eq!(d.len(), 2001);
    }

    fn git_in(dir: &Path, args: &[&str]) {
        let ok = std::process::Command::new("git").current_dir(dir).args(args).output().unwrap().status.success();
        assert!(ok, "git {:?}", args);
    }

    #[test]
    fn a_commit_that_deleted_the_path_is_kept_as_absent() {
        let dir = std::env::temp_dir().join(format!("adr-approval-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        git_in(&dir, &["init", "-q"]);
        git_in(&dir, &["config", "user.email", "t@example.com"]);
        git_in(&dir, &["config", "user.name", "T"]);
        git_in(&dir, &["config", "commit.gpgsign", "false"]);
        std::fs::write(dir.join("f.md"), "one\n").unwrap();
        git_in(&dir, &["add", "f.md"]);
        git_in(&dir, &["commit", "-q", "-m", "add"]);
        git_in(&dir, &["rm", "-q", "f.md"]);
        git_in(&dir, &["commit", "-q", "-m", "delete"]);
        let commits = commits_of(&dir, "f.md");
        assert_eq!(commits.len(), 2);
        // The newest commit deleted the path: absent, and kept so.
        assert!(text_at(&dir, &commits[0], "f.md").is_none());
        let key = (dir.clone(), commits[0].clone(), "f.md".to_string());
        assert_eq!(history().texts.get(&key), Some(&None));
        // The older one holds it.
        assert_eq!(text_at(&dir, &commits[1], "f.md").as_deref().map(String::as_str), Some("one\n"));
        // A commit git cannot read is not kept at all.
        assert!(text_at(&dir, "0000000000000000000000000000000000000000", "f.md").is_none());
        let unknown = (dir.clone(), "0000000000000000000000000000000000000000".to_string(), "f.md".to_string());
        assert!(!history().texts.contains_key(&unknown));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
