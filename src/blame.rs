//! `adrdom/blame/<name>.md`: for one origin, its line ranges and whether each
//! range holds a decision or only detail (per "What does a blame file hold
//! of its origin?", d2851ea1 A; "blame", 108053a8-c2c3-4561-874c-2f573a5409f2).
//! Parsing, coverage, what one file waits for and the `Index` of every blame
//! file live here; `actions::do_approve` dispatches a path under
//! `repo::BLAME_DIR` to `actions::do_approve_blame`, the same way it
//! dispatches `repo::TERMS_PATH` to `do_approve_terms`.
//!
//! `check` holds a blame file to its origin's blob (6f6e7f4a A) and to its
//! origin staying until the file is approved (0c3d498a A) --
//! `commands::check::blame`; `show` prints a blame file in the form every
//! file approve takes is printed (91f9652a A, 1af76f91 A -- `commands::show`).
use crate::ledger::Ledger;
use crate::msg;
use crate::record;
use crate::repo;
use crate::rules::{Class, Finding, Subject};
use crate::status::{self, FileStatus};
use std::path::{Path, PathBuf};

/// What a row says of its range when the range is a decision the owner
/// settles by picking an option (d2851ea1 A).
const DECISION: &str = "decision";
/// What a row says of a range that only the code or the origin records.
const DETAIL: &str = "detail";
/// Opens the id line of a blame file, on line 3.
const ID_PREFIX: &str = "id: ";

/// One row of a blame file, as written: the origin's 1-based inclusive line
/// range and what it holds, `decision` or `detail` (d2851ea1 A), trimmed. A
/// row with any other value is a shape fault of the file and stays here, so
/// coverage still reads its range.
#[derive(Debug, Clone)]
pub struct Row {
    pub start: usize,
    pub end: usize,
    pub value: String,
}

impl Row {
    pub fn is_decision(&self) -> bool {
        self.value == DECISION
    }
}

/// Which rule a fault of a blame file's shape breaks, carried beside its
/// text so that no rule is ever read off the wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    /// The structure of the file: a header line missing, the blank line
    /// after the header, a line that is no row.
    File,
    /// A row whose value is neither `decision` nor `detail`.
    RowValue,
    /// An id line that holds no identifier, or sits below the header: the
    /// tool writes the identifier, at approval (d2851ea1 A).
    Id,
}

/// One fault of a blame file's shape: `text` is what is found, naming its
/// line, and `kind` the rule it breaks.
#[derive(Debug, Clone)]
pub struct ShapeError {
    pub kind: ShapeKind,
    pub text: String,
}

impl ShapeError {
    /// The message of this fault on the blame file `rel`.
    pub fn said(&self, rel: &str, approved: Option<&str>) -> msg::Said {
        match self.kind {
            ShapeKind::File => msg::blame_shape_error(rel, &self.text),
            ShapeKind::RowValue => msg::blame_row_value_error(rel, &self.text),
            ShapeKind::Id => msg::blame_id_error(rel, &self.text, approved),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlameFile {
    pub path: PathBuf,
    pub rel: String,
    pub lines: Vec<String>,
    pub origin: Option<String>,
    pub blob: Option<String>,
    /// The identifier on line 3, when the file has one and it is one.
    pub id: Option<String>,
    pub rows: Vec<Row>,
    pub shape_errors: Vec<ShapeError>,
}

/// A git object id: 40 hex characters (sha1) or 64 (sha256, per a
/// `--object-format=sha256` repository).
pub(crate) fn is_hex_oid(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64) && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// `"<start>-<end>\t<value>"`: `None` when the line doesn't have that shape
/// at all (no tab, a range that is not digits, a dash and digits as a blame
/// line writes it -- `record::range_token` -- empty value, or `end < start`).
/// The value is trimmed and not judged here.
fn parse_row(line: &str) -> Option<(usize, usize, String)> {
    let (range, value) = line.split_once('\t')?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let (start, end) = record::range_token(range.trim())?;
    Some((start, end, value.to_string()))
}

/// Parses a blame file's fixed shape (per "What does a blame file hold of
/// its origin?"): `origin: <path>` on line 1, `blob: <40 or 64 hex>` on line
/// 2, then a blank line, or an `id: <identifier>` line and then the blank
/// line, then one row per non-blank line after that. Anything else is a
/// shape error, collected here rather than returned at the first, the same
/// way `terms::parse`/`record::build` collect every shape error at once.
pub fn parse(path: PathBuf, rel: String, lines: Vec<String>) -> BlameFile {
    let mut errors = Vec::new();
    let mut fault = |kind: ShapeKind, text: String| errors.push(ShapeError { kind, text });

    let origin = match lines.first().and_then(|l| l.strip_prefix("origin: ")) {
        Some(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
        _ => {
            fault(ShapeKind::File, "line 1: missing \"origin: <path>\"".to_string());
            None
        }
    };

    let blob = match lines.get(1).and_then(|l| l.strip_prefix("blob: ")) {
        Some(v) if is_hex_oid(v.trim()) => Some(v.trim().to_string()),
        _ => {
            fault(ShapeKind::File, "line 2: missing \"blob: <40 or 64 hex>\"".to_string());
            None
        }
    };

    // `approve` writes the id line (d2851ea1 A) into the blank line's place:
    // the blank line then follows it.
    let mut id = None;
    let mut blank = 2;
    if let Some(v) = lines.get(2).and_then(|l| l.strip_prefix(ID_PREFIX)) {
        let v = v.trim();
        if record::is_uuid(v) {
            id = Some(v.to_string());
        } else {
            fault(
                ShapeKind::Id,
                format!("line 3: id line holds \"{}\", which is no identifier the tool assigns", v),
            );
        }
        blank = 3;
    }
    if !lines.get(blank).is_some_and(|l| l.trim().is_empty()) {
        fault(ShapeKind::File, format!("line {}: expected a blank line after the header", blank + 1));
    }

    let mut rows = Vec::new();
    for (i, l) in lines.iter().enumerate().skip(blank + 1) {
        if l.trim().is_empty() {
            continue;
        }
        if l.starts_with(ID_PREFIX) {
            fault(ShapeKind::Id, format!("line {}: id line below the header", i + 1));
            continue;
        }
        match parse_row(l) {
            Some((start, end, value)) => {
                if value != DECISION && value != DETAIL {
                    fault(
                        ShapeKind::RowValue,
                        format!("line {}: row value \"{}\" is neither decision nor detail", i + 1, value),
                    );
                }
                rows.push(Row { start, end, value });
            }
            None => fault(
                ShapeKind::File,
                format!("line {}: not a row (\"<start>-<end>\", a tab, then decision or detail)", i + 1),
            ),
        }
    }

    BlameFile { path, rel, lines, origin, blob, id, rows, shape_errors: errors }
}

/// The hash `state::blame_approved`/`do_approve_blame` compare against the
/// ledger's latest `blame` row for a path: the same normalization
/// `terms::file_hash` applies (trailing whitespace on a line never moves
/// the hash), over `lines` rather than a whole file's text so a caller that
/// already has the split lines (the copy with the id line in it, or a raw
/// file's content split the same way) never has to re-join and re-split.
pub fn file_hash(lines: &[String]) -> String {
    let text = lines
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    record::sha256_hex(&record::normalize(&text))
}

// --- coverage --------------------------------------------------------------

/// A row whose range fails to fit inside the origin's `1..=n` (too far, or
/// -- since parsing already rejects `end < start` -- never inverted): named
/// separately from a gap so `approve` can name the excess distinctly.
pub struct BeyondError {
    pub start: usize,
    pub end: usize,
}

/// One run of consecutive lines (1-based, inclusive) covered by no row.
pub struct GapError {
    pub start: usize,
    pub end: usize,
}

/// Checks every row's range against the origin's line count `n`
/// (`git cat-file -p <blob> | wc -l`, trailing newline not counted, per
/// "Does a blame file cover its whole origin?"). A range wholly or partly
/// beyond `n` is reported by `beyond`; the rest are used to mark
/// `1..=n` covered, and every run left uncovered is reported by `gaps`.
/// Overlap is never an error (5c399a14 A: "a range may appear in more than
/// one row").
pub fn coverage_errors(rows: &[Row], n: usize) -> (Vec<BeyondError>, Vec<GapError>) {
    let mut beyond = Vec::new();
    let mut covered = vec![false; n + 1]; // 1-based; index 0 unused.
    for r in rows {
        if r.start > n || r.end > n {
            beyond.push(BeyondError { start: r.start, end: r.end });
        }
        // Only the part inside the origin marks coverage.
        for i in r.start..=r.end.min(n) {
            covered[i] = true;
        }
    }

    let mut gaps = Vec::new();
    let mut i = 1;
    while i <= n {
        if covered[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i <= n && !covered[i] {
            i += 1;
        }
        gaps.push(GapError { start, end: i - 1 });
    }
    (beyond, gaps)
}

// --- the origin -------------------------------------------------------------

/// Whether `path` (repo-root-relative, forward slashes) may name an origin:
/// not under `repo::ADRDOM_DIR`, not a directory in `source` and not a
/// symbolic link (per the "origin" term: "a prose file outside adrdom whose
/// text the records take in" -- `adrdom/blame`'s own tree is never outside
/// adrdom, and neither a directory nor a link is a file; `Source::is_symlink`
/// sees a link in the working tree only). A path that simply does not exist
/// at all is not this rule's problem: that is the missing-origin case
/// (0c3d498a A), checked on its own. Read by `file_state`, so `approve`,
/// `check` and `show` hold the rule in exactly one place.
pub fn valid_origin_path(root: &Path, source: &repo::Source, path: &str) -> bool {
    if leaves_repository(path) {
        return false;
    }
    let plain = plain_path(path);
    let under_adrdom =
        plain == repo::ADRDOM_DIR || plain.starts_with(&format!("{}/", repo::ADRDOM_DIR));
    !under_adrdom && !source.is_dir(root, &plain) && !source.is_symlink(root, &plain)
}

/// Whether the text of `path` cannot name a file inside the repository:
/// absolute, ending in `/`, or with a `..` component that climbs above the
/// repository root. Text only: nothing is read from disk.
fn leaves_repository(path: &str) -> bool {
    let path = path.trim();
    if path.starts_with('/') || path.ends_with('/') {
        return true;
    }
    let mut depth = 0usize;
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
            }
            _ => depth += 1,
        }
    }
    false
}

/// `path` with every empty and `.` component dropped and every `..`
/// resolved against the component before it, so two spellings of one
/// path (`docs/x.md`, `./docs/x.md`, `docs/../docs/x.md`) compare equal.
/// Text only: nothing is read from disk.
pub fn plain_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.trim().split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    parts.join("/")
}

// --- every blame file -------------------------------------------------------

/// One blame file of the `Index`: parsed, with its name and its status.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Its path under `repo::BLAME_DIR` without `.md`: `pass/notes` for
    /// `adrdom/blame/pass/notes.md`.
    pub name: String,
    /// Against its latest approval (`status::blame_file_status`).
    pub status: FileStatus,
    pub file: BlameFile,
}

impl Entry {
    /// Its path from the repository root.
    pub fn rel(&self) -> &str {
        &self.file.rel
    }

    /// The identifier on its id line, when it has one.
    pub fn id(&self) -> Option<&str> {
        self.file.id.as_deref()
    }

    pub fn rows(&self) -> &[Row] {
        &self.file.rows
    }
}

/// Every blame file under `repo::BLAME_DIR`, nested ones too, sorted by
/// path: the one list that anything resolving a blame file by its name or
/// its identifier reads (per "What does a question carry of the blame file
/// it came from?", 672201dd A). `check` at a commit reads it through the
/// staged files, every other command through the working tree.
#[derive(Debug, Clone)]
pub struct Index {
    entries: Vec<Entry>,
    /// The path and identifier of every `blame` ledger row, in ledger order.
    blame_rows: Vec<(String, String)>,
}

impl Index {
    /// The index of `files`, already parsed, each with its status read from
    /// the ledger `led`.
    pub fn of(mut files: Vec<BlameFile>, led: &Ledger) -> Index {
        files.sort_by(|a, b| a.rel.cmp(&b.rel));
        let entries = files
            .into_iter()
            .map(|file| Entry {
                name: name_of(&file.rel),
                status: status::blame_file_status(led, &file.rel, &record::join_lines(&file.lines)),
                file,
            })
            .collect();
        let blame_rows =
            led.rows.iter().filter(|r| r.event == "blame").map(|r| (r.detail.clone(), r.id.clone())).collect();
        Index { entries, blame_rows }
    }

    /// The identifier of the latest `blame` row for `rel`; `None` when the
    /// file has no approval, or when its latest row is one of an older ledger
    /// (identifier `-`): approved, with no identifier assigned yet.
    pub fn approved_id(&self, rel: &str) -> Option<&str> {
        self.blame_rows.iter().rfind(|(path, _)| path == rel).map(|(_, id)| id.as_str()).filter(|id| *id != "-")
    }

    /// True when some `blame` row, for any path, holds `id`.
    pub fn row_holds(&self, id: &str) -> bool {
        self.blame_rows.iter().any(|(_, held)| held.eq_ignore_ascii_case(id))
    }

    /// The index of every blame file in `source`, and the path of each one
    /// that could not be read as text (left out of the index).
    pub fn read(root: &Path, source: &repo::Source, led: &Ledger) -> (Index, Vec<String>) {
        let mut files = Vec::new();
        let mut unreadable = Vec::new();
        for rel in rels(root, source) {
            match source.read(root, &rel) {
                Some(content) => files.push(parse(root.join(&rel), rel, record::split_lines(&content))),
                None => unreadable.push(rel),
            }
        }
        (Index::of(files, led), unreadable)
    }

    /// Every blame file, by path.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The blame file called `name` (`Entry::name`).
    pub fn by_name(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// The blame file a `blame:` entry names: the one that keeps the
    /// identifier `written` when it is one, else the one called `written`.
    pub fn named(&self, written: &str) -> Option<&Entry> {
        if record::is_uuid(written) {
            self.keeper_of(written)
        } else {
            self.by_name(written)
        }
    }

    /// The blame file that keeps `id` when several hold it: the first
    /// approved one, else the first whose path the ledger holds an approval
    /// for (changed), else the first by path. A file approved once never
    /// loses its identifier to a copy made of it, edited or not.
    pub fn keeper_of(&self, id: &str) -> Option<&Entry> {
        let holds = |e: &&Entry| e.id().is_some_and(|held| held.eq_ignore_ascii_case(id));
        [FileStatus::Approved, FileStatus::Changed]
            .into_iter()
            .find_map(|status| self.entries.iter().filter(holds).find(|e| e.status == status))
            .or_else(|| self.entries.iter().find(holds))
    }

    /// The blame file whose identifier `bf` holds, when another file keeps
    /// it (`keeper_of`); `None` when `bf` has no id line or keeps its id.
    pub fn id_taken_from(&self, bf: &BlameFile) -> Option<&Entry> {
        let keeper = self.keeper_of(bf.id.as_deref()?)?;
        (keeper.rel() != bf.rel).then_some(keeper)
    }
}

/// The name of the blame file at `rel`: `Entry::name`.
fn name_of(rel: &str) -> String {
    let under = rel.strip_prefix(repo::BLAME_DIR).map_or(rel, |r| r.trim_start_matches('/'));
    under.strip_suffix(".md").unwrap_or(under).to_string()
}

/// Every `*.md` file under `repo::BLAME_DIR` in `source`, nested ones too,
/// sorted: the listing `check` judges and approve takes.
fn rels(root: &Path, source: &repo::Source) -> Vec<String> {
    let mut rels: Vec<String> =
        source.list_under(root, repo::BLAME_DIR).into_iter().filter(|p| p.ends_with(".md")).collect();
    rels.sort();
    rels
}

/// Every `*.md` file under `repo::BLAME_DIR`, parsed, off the working tree,
/// and every file that failed to read at all (`(rel, io error text)`), so a
/// caller (`show`) can print a line for it instead of silently omitting it
/// -- fail-open would otherwise hide a blame file's own read failure from
/// the one command that has no `check` category to catch it.
pub fn load_all_with_errors(root: &Path) -> (Vec<BlameFile>, Vec<(String, String)>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    for rel in rels(root, &repo::Source::WorkingTree) {
        let p = root.join(&rel);
        match std::fs::read_to_string(&p) {
            Ok(content) => out.push(parse(p.clone(), rel, record::split_lines(&content))),
            Err(e) => errors.push((rel, e.to_string())),
        }
    }
    (out, errors)
}

// --- the rows a question names -----------------------------------------------

/// The `decision` rows of the blame file `entry` in file order, each with
/// whether a question names it (per "What does check do with a decision row
/// that no question names?", ee27fd4c A). A question names a row when a
/// `blame:` entry of some question of `topic_files`, whatever its status,
/// resolves to `entry` (`Index::named`: its name, else its identifier) and
/// holds exactly the row's range. The one test `show --questions` counts
/// the rows by.
pub fn decision_rows<'a>(entry: &'a Entry, index: &Index, topic_files: &[record::RecordFile]) -> Vec<(&'a Row, bool)> {
    let named: Vec<(usize, usize)> = topic_files
        .iter()
        .flat_map(|rf| &rf.sections)
        .flat_map(|sec| &sec.blame)
        .filter(|b| index.named(&b.file).is_some_and(|f| f.rel() == entry.rel()))
        .flat_map(|b| b.ranges.iter().copied())
        .collect();
    entry.rows().iter().filter(|r| r.is_decision()).map(|r| (r, named.contains(&(r.start, r.end)))).collect()
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
    let mut st = FileState { findings: Vec::new(), stops: Vec::new(), origin_content: None };
    let about = |said: msg::Said| Finding::said(Class::Blame, said).about(Subject::BlameFile(rel.clone()));
    st.findings = bf.shape_errors.iter().map(|e| about(e.said(rel, index.approved_id(rel)))).collect();
    if let Some(keeper) = index.id_taken_from(bf) {
        st.findings.push(about(msg::blame_id_shared(rel, keeper.rel())));
    }
    // An approved file keeps the identifier its approval gave: approve writes
    // one only for a file never approved (d2851ea1 A). A faulty id line is
    // an id line, and its own finding says what is wrong with it.
    if let (FileStatus::Changed, None, Some(approved)) = (status, &bf.id, index.approved_id(rel)) {
        if !bf.shape_errors.iter().any(|e| e.kind == ShapeKind::Id) {
            st.findings.push(about(msg::blame_id_line_gone(rel, approved)));
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
                st.findings.push(about(msg::blame_id_edited(rel, written)))
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
        st.findings.push(about(msg::blame_bad_origin_path(rel, &origin)));
        return st;
    }
    if !source.exists(root, &plain) {
        if status.awaits_approval() {
            st.findings.push(about(msg::blame_origin_missing(rel, &origin, status)));
        }
        return st;
    }
    // A fault of the structure leaves nothing to compare the origin with; a
    // row value or an id line leaves the ranges and the blob as they are.
    if bf.shape_errors.iter().any(|e| e.kind == ShapeKind::File) {
        return st;
    }
    let blob = bf.blob.clone().expect("no structure fault means blob is set");
    match source.read(root, &plain) {
        None => st.stops.push(msg::blame_origin_unreadable(rel, &origin)),
        Some(origin_content) => {
            // The working tree file is hashed the way git stores it (its
            // filters applied); the index holds the stored blob already.
            let actual = match source {
                repo::Source::WorkingTree => repo::git_hash_object_filtered(root, &origin_content, &plain),
                repo::Source::Index => repo::git_hash_object_stdin(root, &origin_content),
            };
            match actual {
                // The `blob:` line may hold upper case hex; git prints lower.
                Some(actual) if !actual.eq_ignore_ascii_case(&blob) => {
                    st.findings.push(about(msg::blame_blob_mismatch(rel, &origin)))
                }
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
    use super::*;
    use crate::ledger::Row as LedgerRow;

    const ZERO_BLOB: &str = "0000000000000000000000000000000000000000";
    const ID_A: &str = "11111111-1111-4111-8111-111111111111";

    fn lines_of(s: &str) -> Vec<String> {
        record::split_lines(s)
    }

    fn sample() -> String {
        format!("origin: docs/x.md\nblob: {}\n\n1-3\tdecision\n4-4\tdetail\n", ZERO_BLOB)
    }

    fn with_id(id: &str) -> String {
        sample().replacen("\n\n", &format!("\nid: {}\n\n", id), 1)
    }

    fn parsed(rel: &str, s: &str) -> BlameFile {
        parse(PathBuf::from(rel), rel.to_string(), lines_of(s))
    }

    fn texts(bf: &BlameFile) -> Vec<&str> {
        bf.shape_errors.iter().map(|e| e.text.as_str()).collect()
    }

    fn ledger_approving(files: &[&BlameFile]) -> Ledger {
        Ledger {
            rows: files
                .iter()
                .map(|bf| LedgerRow {
                    time: "2024-01-01T00:00:00Z".to_string(),
                    event: "blame".to_string(),
                    id: bf.id.clone().unwrap_or_else(|| "-".to_string()),
                    hash: file_hash(&bf.lines),
                    detail: bf.rel.clone(),
                    version: "0.1.0".to_string(),
                })
                .collect(),
            malformed: vec![],
        }
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn well_formed_file_parses_with_no_shape_errors() {
        let bf = parse(PathBuf::from("x.md"), "adrdom/blame/x.md".to_string(), lines_of(&sample()));
        assert!(bf.shape_errors.is_empty(), "{:?}", bf.shape_errors);
        assert_eq!(bf.origin.as_deref(), Some("docs/x.md"));
        assert_eq!(bf.blob.as_deref(), Some(ZERO_BLOB));
        assert_eq!(bf.id, None);
        assert_eq!(bf.rows.len(), 2);
        assert_eq!((bf.rows[0].start, bf.rows[0].end), (1, 3));
        assert!(bf.rows[0].is_decision());
        assert!(!bf.rows[1].is_decision());
        assert_eq!(bf.rows[1].value, DETAIL);
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn a_row_value_is_trimmed() {
        let s = sample().replacen("1-3\tdecision", "1-3\t  decision  ", 1);
        let bf = parsed("x.md", &s);
        assert!(bf.shape_errors.is_empty(), "{:?}", bf.shape_errors);
        assert_eq!(bf.rows[0].value, "decision");
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn missing_origin_is_a_shape_error() {
        let s = sample().replacen("origin: docs/x.md\n", "not an origin line\n", 1);
        let bf = parsed("x.md", &s);
        assert!(bf.shape_errors.iter().any(|e| e.text.contains("origin") && e.kind == ShapeKind::File));
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn a_short_blob_is_a_shape_error() {
        let s = sample().replacen(&format!("blob: {}\n", ZERO_BLOB), "blob: abc\n", 1);
        let bf = parsed("x.md", &s);
        assert!(bf.shape_errors.iter().any(|e| e.text.contains("blob") && e.kind == ShapeKind::File));
    }

    // A sha256-object-format repo's blob ids are 64 hex characters, not 40.
    #[test]
    fn a_64_hex_blob_is_accepted() {
        let s = sample().replacen(&format!("blob: {}\n", ZERO_BLOB), &format!("blob: {}\n", "0".repeat(64)), 1);
        let bf = parsed("x.md", &s);
        assert!(bf.shape_errors.is_empty(), "{:?}", bf.shape_errors);
        assert_eq!(bf.blob.as_deref(), Some("0".repeat(64).as_str()));
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn a_row_with_no_tab_is_a_shape_error() {
        let s = sample().replacen("1-3\tdecision\n", "1-3 decision\n", 1);
        let bf = parsed("x.md", &s);
        assert_eq!(texts(&bf).len(), 1);
        assert!(texts(&bf)[0].contains("line 4: not a row"), "{:?}", texts(&bf));
        assert_eq!(bf.shape_errors[0].kind, ShapeKind::File);
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn a_missing_blank_line_after_the_header_is_a_shape_error() {
        let s = sample().replacen("\n\n1-3", "\n1-3", 1);
        let bf = parsed("x.md", &s);
        assert!(texts(&bf).contains(&"line 3: expected a blank line after the header"), "{:?}", texts(&bf));
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    // A row whose value is neither word keeps its range, so coverage still
    // reads it, and its fault carries the kind of its own rule.
    #[test]
    fn a_row_value_that_is_neither_decision_nor_detail_keeps_its_range() {
        for old in ["term: pass", "dropped: dead", "prose: example", "free text", "Decision"] {
            let s = sample().replacen("1-3\tdecision", &format!("1-3\t{}", old), 1);
            let bf = parsed("x.md", &s);
            assert_eq!(bf.rows.len(), 2, "{}", old);
            assert_eq!((bf.rows[0].start, bf.rows[0].end), (1, 3));
            assert!(!bf.rows[0].is_decision());
            assert_eq!(bf.shape_errors.len(), 1, "{}: {:?}", old, bf.shape_errors);
            assert_eq!(bf.shape_errors[0].kind, ShapeKind::RowValue);
            assert_eq!(
                bf.shape_errors[0].text,
                format!("line 4: row value \"{}\" is neither decision nor detail", old)
            );
        }
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn an_id_line_on_line_3_is_read_and_the_blank_line_follows_it() {
        let bf = parsed("x.md", &with_id(ID_A));
        assert!(bf.shape_errors.is_empty(), "{:?}", bf.shape_errors);
        assert_eq!(bf.id.as_deref(), Some(ID_A));
        assert_eq!(bf.rows.len(), 2);
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn an_id_line_with_no_blank_line_after_it_is_a_shape_error() {
        let s = with_id(ID_A).replacen("\n\n1-3", "\n1-3", 1);
        let bf = parsed("x.md", &s);
        assert_eq!(texts(&bf), vec!["line 4: expected a blank line after the header"]);
        assert_eq!(bf.shape_errors[0].kind, ShapeKind::File);
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn an_id_line_that_is_no_identifier_is_refused_with_the_id_kind() {
        let bf = parsed("x.md", &with_id("not-an-id"));
        assert_eq!(
            texts(&bf),
            vec!["line 3: id line holds \"not-an-id\", which is no identifier the tool assigns"]
        );
        assert_eq!(bf.shape_errors[0].kind, ShapeKind::Id);
        assert_eq!(bf.id, None);
        assert_eq!(bf.rows.len(), 2, "the rows after it are still read");
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn an_id_line_below_the_header_is_refused_with_the_id_kind() {
        let s = sample() + &format!("id: {}\n", ID_A);
        let bf = parsed("x.md", &s);
        assert_eq!(texts(&bf), vec!["line 6: id line below the header"]);
        assert_eq!(bf.shape_errors[0].kind, ShapeKind::Id);
        assert_eq!(bf.id, None);
        assert_eq!(bf.rows.len(), 2);
    }

    // adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
    #[test]
    fn coverage_errors_finds_a_gap_and_an_overlap_passes() {
        let row = |start, end| Row { start, end, value: "detail".into() };
        let rows = vec![
            row(1, 2),
            row(2, 4),
            // line 5 is a gap.
        ];
        let (beyond, gaps) = coverage_errors(&rows, 6);
        assert!(beyond.is_empty());
        assert_eq!(gaps.len(), 1);
        assert_eq!((gaps[0].start, gaps[0].end), (5, 6));
    }

    // adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
    #[test]
    fn coverage_errors_finds_a_range_beyond_n() {
        let rows = vec![Row { start: 1, end: 10, value: "decision".into() }];
        let (beyond, gaps) = coverage_errors(&rows, 5);
        assert_eq!(beyond.len(), 1);
        assert_eq!((beyond[0].start, beyond[0].end), (1, 10));
        assert!(gaps.is_empty(), "the part inside the origin counts as covered");
    }

    #[test]
    fn parse_row_rejects_an_inverted_range() {
        assert!(parse_row("5-3\tdetail").is_none());
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    // A range is digits, a dash and digits, as on a blame line: a sign or a
    // space inside it makes the line no row.
    #[test]
    fn a_row_range_is_digits_a_dash_and_digits() {
        assert_eq!(parse_row("1-3\tdetail"), Some((1, 3, "detail".to_string())));
        for bad in ["+1-+3", "+1-3", "1-+3", "-1-3", "1 - 3", "0-2", "1-x", "1-2-3"] {
            assert!(parse_row(&format!("{}\tdetail", bad)).is_none(), "{}", bad);
        }
        let bf = parsed("x.md", &sample().replacen("1-3\tdecision", "+1-+3\tdecision", 1));
        assert_eq!(texts(&bf).len(), 1, "{:?}", texts(&bf));
        assert!(texts(&bf)[0].contains("line 4: not a row"), "{:?}", texts(&bf));
        assert_eq!(bf.rows.len(), 1);
    }

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
            file_state(&root, &repo::Source::WorkingTree, &bf, FileStatus::New, &index).findings.len()
        };
        assert_eq!(findings(&blob), 0);
        assert_eq!(findings(&blob.to_uppercase()), 0);
        assert_eq!(findings(ZERO_BLOB), 1, "a blob of another content still mismatches");
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
            parsed(rel, &format!("origin: docs/x.md\nblob: {}\n{}\n{}", ZERO_BLOB, id_line, rows))
        };
        let index = Index::of(
            vec![file("adrdom/blame/x.md", Some(ID_A)), file("adrdom/blame/o.md", None)],
            &ledger_approving(&[]),
        );
        let topic = |text: &str| record::build(PathBuf::from("t.md"), "adrdom/decisions/t.md".to_string(), lines_of(text));
        let topic_files = vec![
            topic("# T\n\n### Q?\nblame: x 1-1 2-2 3-3, o 5-5\n"),
            topic(&format!("# U\n\n### R?\nblame: {} 5-5\n", ID_A)),
        ];
        let listed = |name: &str| -> Vec<(usize, usize, bool)> {
            let entry = index.by_name(name).unwrap();
            decision_rows(entry, &index, &topic_files).into_iter().map(|(r, named)| (r.start, r.end, named)).collect()
        };
        assert_eq!(listed("x"), vec![(1, 1, true), (3, 4, false), (5, 5, true), (6, 6, false)]);
        assert_eq!(listed("o"), vec![(1, 1, false), (3, 4, false), (5, 5, true), (6, 6, false)]);
        assert!(decision_rows(index.by_name("x").unwrap(), &index, &[]).iter().all(|&(_, named)| !named));
    }

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    // The index names each file by its path under adrdom/blame without
    // `.md`, nested files too, and lists them by path.
    #[test]
    fn the_index_names_each_file_and_lists_them_by_path() {
        let nested = parsed("adrdom/blame/pass/notes.md", &sample());
        let b = parsed("adrdom/blame/b.md", &sample());
        let a = parsed("adrdom/blame/a.md", &sample());
        let index = Index::of(vec![nested, b, a], &ledger_approving(&[]));
        let rels: Vec<&str> = index.entries().iter().map(|e| e.rel()).collect();
        assert_eq!(rels, vec!["adrdom/blame/a.md", "adrdom/blame/b.md", "adrdom/blame/pass/notes.md"]);
        assert_eq!(index.by_name("pass/notes").map(|e| e.rel()), Some("adrdom/blame/pass/notes.md"));
        assert!(index.by_name("adrdom/blame/a").is_none());
        assert_eq!(index.by_name("a").map(|e| e.rows().len()), Some(2));
        assert!(index.entries().iter().all(|e| e.status == FileStatus::New));
    }

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    // Text that is an identifier names a blame file by its id line alone, and
    // any other text by its name alone: a file whose name is another file's
    // identifier is not found by it.
    #[test]
    fn an_identifier_on_a_blame_line_resolves_as_an_identifier() {
        const ID_B: &str = "22222222-2222-4222-8222-222222222222";
        let named_like_an_id = parsed(&format!("adrdom/blame/{}.md", ID_A), &with_id(ID_B));
        let x = parsed("adrdom/blame/x.md", &with_id(ID_A));
        let index = Index::of(vec![named_like_an_id, x], &ledger_approving(&[]));
        let rel = |written: &str| index.named(written).map(|e| e.rel().to_string());
        assert_eq!(rel(ID_A).as_deref(), Some("adrdom/blame/x.md"), "the identifier, not the file called so");
        assert_eq!(rel(ID_B), Some(format!("adrdom/blame/{}.md", ID_A)));
        assert_eq!(rel("x").as_deref(), Some("adrdom/blame/x.md"));
        assert_eq!(rel("22222222-2222-4222-8222-333333333333"), None);
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    // One identifier names one file: the first approved holder keeps it,
    // else the first changed one (approved once), else the first by path,
    // and every other holder is the copy.
    #[test]
    fn the_first_approved_holder_keeps_an_identifier_else_the_first_by_path() {
        let a = parsed("adrdom/blame/a.md", &with_id(ID_A));
        let b = parsed("adrdom/blame/b.md", &with_id(ID_A));
        let c = parsed("adrdom/blame/c.md", &sample());

        let drafts = Index::of(vec![c.clone(), b.clone(), a.clone()], &ledger_approving(&[]));
        assert_eq!(drafts.keeper_of(ID_A).map(|e| e.rel()), Some("adrdom/blame/a.md"));
        assert!(drafts.id_taken_from(&a).is_none());
        assert_eq!(drafts.id_taken_from(&b).map(|e| e.rel()), Some("adrdom/blame/a.md"));
        assert!(drafts.id_taken_from(&c).is_none(), "a file with no id line took none");

        let original_is_b = Index::of(vec![c, b.clone(), a.clone()], &ledger_approving(&[&b]));
        assert_eq!(original_is_b.keeper_of(ID_A).map(|e| e.rel()), Some("adrdom/blame/b.md"));
        assert!(original_is_b.id_taken_from(&b).is_none(), "an approved original never gets a finding");
        assert_eq!(original_is_b.id_taken_from(&a).map(|e| e.rel()), Some("adrdom/blame/b.md"));
        assert!(original_is_b.keeper_of("22222222-2222-4222-8222-222222222222").is_none());

        // An approved original that was edited since is changed, and still
        // keeps the identifier over a copy never approved that sorts first.
        let edited_b = parsed("adrdom/blame/b.md", &with_id(ID_A).replacen("4-4\tdetail", "4-4\tdecision", 1));
        let edited = Index::of(vec![edited_b.clone(), a.clone()], &ledger_approving(&[&b]));
        assert_eq!(edited.by_name("b").map(|e| e.status), Some(FileStatus::Changed));
        assert_eq!(edited.keeper_of(ID_A).map(|e| e.rel()), Some("adrdom/blame/b.md"));
        assert!(edited.id_taken_from(&edited_b).is_none());
        assert_eq!(edited.id_taken_from(&a).map(|e| e.rel()), Some("adrdom/blame/b.md"));
    }
}
