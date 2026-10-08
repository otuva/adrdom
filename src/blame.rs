//! `adrdom/blame/<stem>.md`: for one origin, its line ranges and what each
//! range went to in the records (per "What does a blame file hold?",
//! 34f0202a A; "blame", 108053a8-c2c3-4561-874c-2f573a5409f2). Parsing,
//! coverage, and destination resolution live here; `actions::do_approve`
//! dispatches a path under `repo::BLAME_DIR` to `do_approve_blame` below,
//! the same way it dispatches `repo::TERMS_PATH` to `do_approve_terms`.
//!
//! `check` holds a blame file to its origin's blob and to the open
//! decision rows an about-to-vanish origin still carries (5fddeeb5 A,
//! 4bb5ef1b A -- `commands::check::blame`); `show` prints a blame file in
//! the form every file approve takes is printed (6addc2f4 A, bceceda8 A --
//! `commands::show`).
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::terms::Term;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// One row of a blame file, as written: the origin's 1-based inclusive
/// line range and the destination text exactly as it stands (before
/// `approve` rewrites a heading/term name to its id). `line_idx` is the
/// 0-based index into `BlameFile::lines` this row came from, so the
/// rewrite pass can replace it in place.
#[derive(Debug, Clone)]
pub struct Row {
    pub start: usize,
    pub end: usize,
    pub dest: String,
    pub line_idx: usize,
}

#[derive(Debug, Clone)]
pub struct BlameFile {
    pub path: PathBuf,
    pub rel: String,
    pub lines: Vec<String>,
    pub origin: Option<String>,
    pub blob: Option<String>,
    pub rows: Vec<Row>,
    pub shape_errors: Vec<String>,
}

/// A git object id: 40 hex characters (sha1) or 64 (sha256, per a
/// `--object-format=sha256` repository).
pub(crate) fn is_hex_oid(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64) && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// `"<start>-<end>\t<destination>"`: `None` when the line doesn't have
/// that shape at all (no tab, no dash, non-numeric range, empty
/// destination, or `end < start`).
fn parse_row(line: &str) -> Option<(usize, usize, String)> {
    let (range, dest) = line.split_once('\t')?;
    if dest.trim().is_empty() {
        return None;
    }
    let (a, b) = range.split_once('-')?;
    let start: usize = a.trim().parse().ok()?;
    let end: usize = b.trim().parse().ok()?;
    if start == 0 || end < start {
        return None;
    }
    Some((start, end, dest.to_string()))
}

/// Parses a blame file's fixed shape (per "What does a blame file hold?"):
/// `origin: <path>` on line 1, `blob: <40 or 64 hex>` on line 2, a blank line on
/// line 3, then one row per non-blank line after that. Anything else is a
/// shape error, collected here rather than returned at the first, the same
/// way `terms::parse`/`record::build` collect every shape error at once.
pub fn parse(path: PathBuf, rel: String, lines: Vec<String>) -> BlameFile {
    let mut errors = Vec::new();

    let origin = match lines.first().and_then(|l| l.strip_prefix("origin: ")) {
        Some(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
        _ => {
            errors.push("line 1: missing \"origin: <path>\"".to_string());
            None
        }
    };

    let blob = match lines.get(1).and_then(|l| l.strip_prefix("blob: ")) {
        Some(v) if is_hex_oid(v.trim()) => Some(v.trim().to_string()),
        _ => {
            errors.push("line 2: missing \"blob: <40 or 64 hex>\"".to_string());
            None
        }
    };

    if !lines.get(2).map(|l| l.trim().is_empty()).unwrap_or(false) {
        errors.push("line 3: expected a blank line after the header".to_string());
    }

    let mut rows = Vec::new();
    for (i, l) in lines.iter().enumerate().skip(3) {
        if l.trim().is_empty() {
            continue;
        }
        match parse_row(l) {
            Some((start, end, dest)) => rows.push(Row { start, end, dest, line_idx: i }),
            None => errors.push(format!(
                "line {}: not a row (\"<start>-<end>\", a tab, then the destination)",
                i + 1
            )),
        }
    }

    BlameFile { path, rel, lines, origin, blob, rows, shape_errors: errors }
}

pub fn load(path: &Path, rel: &str) -> std::io::Result<BlameFile> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse(path.to_path_buf(), rel.to_string(), record::split_lines(&content)))
}

pub fn save(bf: &BlameFile) -> std::io::Result<()> {
    std::fs::write(&bf.path, record::join_lines(&bf.lines))
}

/// The hash `state::blame_approved`/`do_approve_blame` compare against the
/// ledger's latest `blame` row for a path: the same normalization
/// `terms::file_hash` applies (trailing whitespace on a line never moves
/// the hash), over `lines` rather than a whole file's text so a caller that
/// already has the split lines (the rewritten in-memory copy, or a raw
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
            continue;
        }
        for i in r.start..=r.end {
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

// --- destination resolution -------------------------------------------------

// The closed set's own prefixes (per "What does a blame row name?",
// 38216d39 A): the one table `resolve_destination` and the row readers
// (`decision_target`, `copy_target`) read from.
const DECISION_PREFIX: &str = "decision: ";
const TERM_PREFIX: &str = "term: ";
const PROSE_PREFIX: &str = "prose: ";
const DROPPED_PREFIX: &str = "dropped: ";
const DROPPED_DEAD: &str = "dead";
const DROPPED_DETAIL: &str = "detail";
const COPY_OF_PREFIX: &str = "copy of ";

/// What one row's destination resolves to (per "What does a blame row
/// name?", 38216d39 A): the canonical form `approve` writes back, holding
/// an id wherever the closed set names one.
pub enum Resolved {
    Decision(String),
    Term(String),
    Prose(String),
    DroppedDead,
    DroppedDetail,
    DroppedCopyDecision(String),
    DroppedCopyOrigin(String),
}

impl Resolved {
    /// The canonical text `approve` writes back after the tab.
    pub fn canonical(&self) -> String {
        match self {
            Resolved::Decision(id) => format!("{}{}", DECISION_PREFIX, id),
            Resolved::Term(id) => format!("{}{}", TERM_PREFIX, id),
            Resolved::Prose(stem) => format!("{}{}", PROSE_PREFIX, stem),
            Resolved::DroppedDead => format!("{}{}", DROPPED_PREFIX, DROPPED_DEAD),
            Resolved::DroppedDetail => format!("{}{}", DROPPED_PREFIX, DROPPED_DETAIL),
            Resolved::DroppedCopyDecision(id) => format!("{}{}{}", DROPPED_PREFIX, COPY_OF_PREFIX, id),
            Resolved::DroppedCopyOrigin(path) => format!("{}{}{}", DROPPED_PREFIX, COPY_OF_PREFIX, path),
        }
    }
}

/// Why a row's destination did not resolve (per "What does a blame row
/// name?", 38216d39 A): every case that closed set enumerates, plus the
/// catch-all for text naming none of it.
pub enum DestError {
    UnknownHeading(String),
    AmbiguousHeading(String, Vec<String>),
    UnknownTerm(String),
    RetiredTerm(String),
    MissingProseStem(String),
    CopyPathMissing(String),
    /// A `dropped: copy of <path>` target that is under `repo::ADRDOM_DIR`,
    /// or that exists but is not a regular file -- the same rule the
    /// origin line itself is held to (`valid_origin_path`), per the
    /// "origin" term: "a prose file outside adrdom".
    BadPath(String),
    /// A `dropped: copy of <path>` target that is this blame file's own
    /// origin.
    CopyOfOwnOrigin(String),
    Unrecognized(String),
}

/// Every set `resolve_destination` needs, built once per `do_approve_blame`
/// run (like `terms_list`/`phrase_lookup`/`cross_file_headings` already are
/// for a topic file) rather than rebuilt per row.
pub struct ResolveCtx<'a> {
    /// Every heading text that carries an id anywhere, any ledger state
    /// (asked, answered, or a skeleton kept from an earlier approval),
    /// mapped to the file and id of each section that carries it -- what
    /// `decision: <heading text>` resolves against (38216d39 A: "a question
    /// by heading", not only a *current* one).
    pub any_headings: &'a HashMap<String, Vec<(String, String)>>,
    /// Every id that names a section anywhere with an id -- what
    /// `decision: <uuid>` (already in uuid form) is checked against, and
    /// (per the owner's fix to 38216d39 A) what `dropped: copy of <heading
    /// text>` resolves against too, the same way a `decision:` row does --
    /// a copy target is any question with an id, not only a current one.
    pub known_heading_ids: &'a HashSet<String>,
    pub terms_list: &'a [Term],
    pub phrase_lookup: &'a HashMap<String, String>,
    /// The tree the rows are judged in: a copy target's path is looked up
    /// there (`check --staged` judges the staged files).
    pub source: repo::Source,
    /// Every topic file's stem (its file name without `.md`): what
    /// `prose: <stem>` is checked against.
    pub decision_stems: &'a HashSet<String>,
    /// Every other origin already named by some blame file's own `origin:`
    /// line (any blame file in `repo::BLAME_DIR`, including this one):
    /// alongside a path that exists in the working tree, what
    /// `dropped: copy of <path>` is checked against (38216d39 A: "a copy of
    /// a passage whose decision lives in an origin not yet mapped still
    /// has a target").
    pub known_origins: &'a HashSet<String>,
    /// The `origin:` path of the blame file being resolved: what a
    /// `dropped: copy of <path>` row may never name (38216d39 A: "another
    /// origin").
    pub own_origin: String,
    pub root: PathBuf,
}

/// Every heading text carrying an id, anywhere, any state -- `decision:`'s
/// own index (wider than `depends::cross_file_heading_index`, which only
/// counts current decisions): the same traversal, `None` filter.
pub fn any_headings_with_id(records: &[RecordFile]) -> HashMap<String, Vec<(String, String)>> {
    crate::depends::heading_index(records, None)
}

/// Every id from `any_headings_with_id`'s own values, flattened -- what a
/// `decision:` destination already in uuid form is checked against.
pub fn known_heading_ids(index: &HashMap<String, Vec<(String, String)>>) -> HashSet<String> {
    index.values().flatten().map(|(_, id)| id.clone()).collect()
}

/// `any_headings_with_id(records)` paired with `known_heading_ids` of it:
/// every caller that needs one needs the other too (`ResolveCtx`,
/// `open_decision_rows`, `show`'s rendering), so this is the one place
/// both are built from a records slice, rather than two separate calls
/// repeated at each site.
pub fn heading_ids(records: &[RecordFile]) -> (HashMap<String, Vec<(String, String)>>, HashSet<String>) {
    let any_headings = any_headings_with_id(records);
    let known_ids = known_heading_ids(&any_headings);
    (any_headings, known_ids)
}

/// Like `known_origins`, read through `source`: `check --staged` reads the
/// blame files as staged.
pub fn known_origins_via(root: &Path, source: &repo::Source) -> HashSet<String> {
    let mut out = HashSet::new();
    for rel in source.list_under(root, repo::BLAME_DIR) {
        if !rel.ends_with(".md") {
            continue;
        }
        let Some(content) = source.read(root, &rel) else { continue };
        let lines = record::split_lines(&content);
        if let Some(origin) = lines.first().and_then(|l| l.strip_prefix("origin: ")) {
            let origin = origin.trim();
            if !origin.is_empty() {
                out.insert(origin.to_string());
            }
        }
    }
    out
}

/// Whether `path` (repo-root-relative, forward slashes) may name an
/// origin, or a `dropped: copy of <path>` target: not under
/// `repo::ADRDOM_DIR`, and not a directory in `source` (per the "origin"
/// term: "a prose file outside adrdom whose text the records take in" --
/// `adrdom/blame`'s own tree is never outside adrdom, and a directory is
/// never a file). A path that simply does not exist at all is not this
/// rule's problem: that is the missing-origin case (4bb5ef1b A) or the
/// copy-path-missing case (38216d39 A), each checked on its own. Shared by
/// `actions::do_approve_blame`, `commands::check::blame`, and
/// `resolve_copy_target` below, so the rule lives in exactly one place.
pub fn valid_origin_path(root: &Path, source: &repo::Source, path: &str) -> bool {
    let plain = plain_path(path);
    let under_adrdom =
        plain == repo::ADRDOM_DIR || plain.starts_with(&format!("{}/", repo::ADRDOM_DIR));
    !under_adrdom && !source.is_dir(root, path)
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

fn resolve_decision(value: &str, ctx: &ResolveCtx) -> Result<String, DestError> {
    if record::is_uuid(value) {
        return if ctx.known_heading_ids.contains(value) {
            Ok(value.to_string())
        } else {
            Err(DestError::UnknownHeading(value.to_string()))
        };
    }
    match ctx.any_headings.get(value).map(|v| v.as_slice()) {
        None | Some([]) => Err(DestError::UnknownHeading(value.to_string())),
        Some([(_, id)]) => Ok(id.clone()),
        Some(many) => {
            let mut files: Vec<String> = many.iter().map(|(rel, _)| rel.clone()).collect();
            files.sort();
            files.dedup();
            Err(DestError::AmbiguousHeading(value.to_string(), files))
        }
    }
}

fn resolve_term(value: &str, ctx: &ResolveCtx) -> Result<String, DestError> {
    if record::is_uuid(value) {
        return match ctx.terms_list.iter().find(|t| t.id.as_deref() == Some(value)) {
            None => Err(DestError::UnknownTerm(value.to_string())),
            Some(t) if t.retired => Err(DestError::RetiredTerm(t.name.clone())),
            Some(t) => Ok(t.id.clone().unwrap_or_default()),
        };
    }
    let lower = value.to_lowercase();
    if let Some(id) = ctx.phrase_lookup.get(&lower) {
        return Ok(id.clone());
    }
    // Not resolved as a current term: check whether it names a retired one,
    // for a more useful message than a bare "unknown".
    let named = ctx.terms_list.iter().find(|t| {
        t.name.trim().to_lowercase() == lower
            || t.aliases.iter().any(|a| a.trim().to_lowercase() == lower)
    });
    match named {
        Some(t) if t.retired => Err(DestError::RetiredTerm(t.name.clone())),
        _ => Err(DestError::UnknownTerm(value.to_string())),
    }
}

/// `dropped: copy of <heading or uuid>` resolves exactly like a
/// `decision:` row (per the owner's fix to 38216d39 A: a copy target is
/// "a question or of another origin" -- any question with an id, in any
/// state, not only a current one; an approved copy row that is later
/// superseded elsewhere must not stop this file's own re-approval). A path
/// target is handled separately, below.
fn resolve_copy_target(value: &str, ctx: &ResolveCtx) -> Result<Resolved, DestError> {
    if record::is_uuid(value) || value.trim_end().ends_with('?') {
        return resolve_decision(value, ctx).map(Resolved::DroppedCopyDecision);
    }
    // A path to another origin, not yet (or already) mapped by its own
    // blame file (38216d39 A) -- itself held to the same rule an origin's
    // own path is (`valid_origin_path`): not under adrdom, not a
    // directory.
    // Looked up by its plain spelling, as the working tree resolves it, so
    // `docs//x.md` or `docs/./x.md` reads the same at a commit as on disk.
    let path = plain_path(value);
    if !valid_origin_path(&ctx.root, &ctx.source, &path) {
        return Err(DestError::BadPath(value.to_string()));
    }
    // 38216d39 A: "copy of ... another origin" -- never the origin this
    // blame file itself maps.
    if path == plain_path(&ctx.own_origin) {
        return Err(DestError::CopyOfOwnOrigin(value.to_string()));
    }
    if ctx.source.exists(&ctx.root, &path) || ctx.known_origins.contains(value) || ctx.known_origins.contains(&path) {
        Ok(Resolved::DroppedCopyOrigin(value.to_string()))
    } else {
        Err(DestError::CopyPathMissing(value.to_string()))
    }
}

/// Resolves one row's destination text against the closed set (per "What
/// does a blame row name?", 38216d39 A). `text` is the raw text after the
/// row's tab, exactly as written (or already in canonical/uuid form, on a
/// second approve of an unchanged row).
pub fn resolve_destination(text: &str, ctx: &ResolveCtx) -> Result<Resolved, DestError> {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix(DECISION_PREFIX) {
        return resolve_decision(rest.trim(), ctx).map(Resolved::Decision);
    }
    if let Some(rest) = t.strip_prefix(TERM_PREFIX) {
        return resolve_term(rest.trim(), ctx).map(Resolved::Term);
    }
    if let Some(rest) = t.strip_prefix(PROSE_PREFIX) {
        let stem = rest.trim();
        return if ctx.decision_stems.contains(stem) {
            Ok(Resolved::Prose(stem.to_string()))
        } else {
            Err(DestError::MissingProseStem(stem.to_string()))
        };
    }
    if let Some(rest) = t.strip_prefix(DROPPED_PREFIX) {
        let rest = rest.trim();
        if rest == DROPPED_DEAD {
            return Ok(Resolved::DroppedDead);
        }
        if rest == DROPPED_DETAIL {
            return Ok(Resolved::DroppedDetail);
        }
        if let Some(target) = rest.strip_prefix(COPY_OF_PREFIX) {
            return resolve_copy_target(target.trim(), ctx);
        }
    }
    Err(DestError::Unrecognized(t.to_string()))
}

/// The refusal text for a `resolve_destination` failure on `row`, the
/// `msg::blame_row_*` wording matching every destination kind in the
/// closed set, plus the catch-all for text naming none of it.
pub fn dest_refusal_text(rel: &str, row: &Row, err: &DestError) -> String {
    match err {
        DestError::UnknownHeading(text) => msg::blame_row_unknown_heading(rel, text),
        DestError::AmbiguousHeading(text, files) => msg::blame_row_ambiguous_heading(rel, text, files),
        DestError::UnknownTerm(text) => msg::blame_row_unknown_term(rel, text),
        DestError::RetiredTerm(name) => msg::blame_row_retired_term(rel, name),
        DestError::MissingProseStem(stem) => msg::blame_row_missing_prose_stem(rel, stem),
        DestError::CopyPathMissing(path) => msg::blame_row_copy_path_missing(rel, path),
        DestError::BadPath(path) => msg::blame_bad_origin_path(rel, path),
        DestError::CopyOfOwnOrigin(path) => {
            msg::blame_row_copy_of_own_origin(rel, row.start, row.end, path)
        }
        DestError::Unrecognized(text) => msg::blame_row_unknown_destination(rel, row.start, row.end, text),
    }
}

// --- shared by `check`'s `blame` category and `show` -----------------------

/// Every `*.md` file under `repo::BLAME_DIR`, parsed, off the working tree:
/// `show` never reads a commit, so it has no `Source` to thread through. `check`'s own listing
/// goes through `Source` instead (a staged blame file must be judged too,
/// per "Does check at a commit read the staged files?"), so it is built in
/// `commands::check::blame` rather than here.
/// Like `load_all`, but also naming every file that failed to read at all
/// (`(rel, io error text)`), so a caller (`show`) can print a line for it
/// instead of silently omitting it -- fail-open would otherwise hide a
/// blame file's own read failure from the one command that has no `check`
/// category to catch it.
pub fn load_all_with_errors(root: &Path) -> (Vec<BlameFile>, Vec<(String, String)>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    // Every blame file under the directory, nested ones too: the listing
    // `check` judges (`Source::list_under`) and approve takes.
    let mut rels: Vec<String> = repo::Source::WorkingTree
        .list_under(root, repo::BLAME_DIR)
        .into_iter()
        .filter(|p| p.ends_with(".md"))
        .collect();
    rels.sort();
    for rel in rels {
        let p = root.join(&rel);
        match std::fs::read_to_string(&p) {
            Ok(content) => out.push(parse(p.clone(), rel, record::split_lines(&content))),
            Err(e) => errors.push((rel, e.to_string())),
        }
    }
    (out, errors)
}

/// Every parsed blame file, dropping any that failed to read entirely.
/// Kept for a caller that has no use for the error list.
pub fn load_all(root: &Path) -> Vec<BlameFile> {
    load_all_with_errors(root).0
}

/// The value after `decision: `, from either a draft row (still the
/// heading text) or a canonical one (the id `approve` rewrote it to).
pub fn decision_target(dest: &str) -> Option<&str> {
    dest.trim().strip_prefix(DECISION_PREFIX)
}

/// The value after `dropped: copy of `, likewise either form.
pub fn copy_target(dest: &str) -> Option<&str> {
    dest.trim()
        .strip_prefix(DROPPED_PREFIX)?
        .trim()
        .strip_prefix(COPY_OF_PREFIX)
        .map(str::trim)
}

/// Resolves a `decision:`/`dropped: copy of <heading>` value -- already a
/// uuid, or still the heading text a draft blame file carries -- to the id
/// of the section it names. `any_headings`/`known_ids` are
/// `any_headings_with_id`/`known_heading_ids`, built once by the caller.
/// `None` when the value names no section at all (a heading removed since
/// the row was written, or free text that never resolved).
pub fn resolve_heading_value(
    value: &str,
    any_headings: &HashMap<String, Vec<(String, String)>>,
    known_ids: &HashSet<String>,
) -> Option<String> {
    if record::is_uuid(value) {
        return if known_ids.contains(value) {
            Some(value.to_string())
        } else {
            None
        };
    }
    any_headings.get(value).and_then(|v| v.first()).map(|(_, id)| id.clone())
}

/// The heading text a section id carries, anywhere in `records`, any state
/// -- what turns an id in a canonical row back into words to print, the
/// same way `view::heading_texts` turns a depends id back into a name.
pub fn heading_text_for_id<'a>(records: &'a [RecordFile], id: &str) -> Option<&'a str> {
    records
        .iter()
        .flat_map(|rf| rf.sections.iter())
        .find(|s| s.id.as_deref() == Some(id))
        .map(|s| s.heading_text.as_str())
}

/// Every `decision:` row in `bf` whose target is closed (per the owner's
/// fix to "Does an origin leave the repository while its blame file has an
/// open row?", 4bb5ef1b A): closed means the id has an answered row in the
/// ledger at all -- latest state `answered` or `superseded` (a decision
/// that was answered and later superseded is not open again); open means
/// `asked`, `refused`, `withdrawn`, or no state row at all (a skeleton).
/// Named by its heading text (or the raw value when it resolves to no
/// section at all -- conservatively still open, never silently dropped).
/// Sorted and deduped: `check`'s own message lists each open row once, not
/// once per duplicate row naming the same question.
pub fn open_decision_rows(
    bf: &BlameFile,
    records: &[RecordFile],
    led: &Ledger,
    any_headings: &HashMap<String, Vec<(String, String)>>,
    known_ids: &HashSet<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for row in &bf.rows {
        let Some(value) = decision_target(&row.dest) else { continue };
        let name = match resolve_heading_value(value, any_headings, known_ids) {
            Some(id) => {
                // Answered: a current decision, or one a later decision
                // superseded once it was answered.
                use crate::status::QuestionStatus;
                let closed = matches!(
                    crate::status::question_status_of_id(led, &id),
                    Some(QuestionStatus::Answered | QuestionStatus::Superseded)
                );
                if closed {
                    continue;
                }
                heading_text_for_id(records, &id).map(str::to_string).unwrap_or(id)
            }
            None => value.to_string(),
        };
        out.push(name);
    }
    out.sort();
    out.dedup();
    out
}

// --- what one blame file waits for ------------------------------------------

/// What `check` and `show` both read off one blame file against its origin:
/// the refusals `check` prints for it (shape, origin path, origin missing
/// while a row is open, origin unreadable, blob mismatch, hash failure),
/// whether it awaits approval, and the origin's content when it could be
/// read. `awaiting` is a flag: a draft's shape faults are notes.
pub struct FileState {
    /// The shape errors of the file itself: a note while the file awaits
    /// approval, a failure once approved (per "What does check do with a
    /// fault on a file awaiting approval?").
    pub shape: Vec<String>,
    /// What fails whatever the file's approval: the origin against the file
    /// (per "Does check hold a blame file to its origin?" and "Does an
    /// origin leave the repository while its blame file has an open row?").
    pub failures: Vec<String>,
    pub awaiting: bool,
    pub origin_content: Option<String>,
}

pub fn file_state(
    root: &Path,
    source: &repo::Source,
    bf: &BlameFile,
    content: &str,
    records: &[RecordFile],
    led: &Ledger,
    any_headings: &HashMap<String, Vec<(String, String)>>,
    known_ids: &HashSet<String>,
) -> FileState {
    let rel = &bf.rel;
    let approved = crate::state::blame_approved(led, rel, content);
    let mut st = FileState { shape: Vec::new(), failures: Vec::new(), awaiting: !approved, origin_content: None };
    st.shape = bf.shape_errors.iter().map(|e| msg::blame_shape_error(rel, e)).collect();
    let Some(origin) = bf.origin.clone() else {
        return st;
    };
    // An origin path that cannot be one is a fault of the file's header, a
    // shape fault: a note while the file awaits approval, a failure once
    // approved (per "What does check do with a fault on a file awaiting
    // approval?").
    if !valid_origin_path(root, source, &origin) {
        let fault = msg::blame_bad_origin_path(rel, &origin);
        if approved {
            st.failures.push(fault);
        } else {
            st.shape.push(fault);
        }
        return st;
    }
    // The origin's own rule holds whatever else is wrong with the rows: a
    // row typo never hides an origin gone while a row is open.
    if !source.exists(root, &origin) {
        let mut open = open_decision_rows(bf, records, led, any_headings, known_ids);
        if !approved {
            open.push("the blame file's own approval".to_string());
            open.sort();
        }
        if !open.is_empty() {
            st.failures.push(msg::blame_origin_missing(rel, &origin, &open));
        }
        return st;
    }
    if !st.shape.is_empty() {
        return st;
    }
    let blob = bf.blob.clone().expect("no shape error means blob is set");
    match source.read(root, &origin) {
        None => st.failures.push(msg::blame_origin_unreadable(rel, &origin)),
        Some(origin_content) => {
            match repo::git_hash_object_stdin(root, &origin_content) {
                Some(actual) if actual != blob => st.failures.push(msg::blame_blob_mismatch(rel, &origin)),
                Some(_) => {}
                None => st.failures.push(msg::blame_hash_failed(rel, &origin)),
            }
            st.origin_content = Some(origin_content);
        }
    }
    st
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines_of(s: &str) -> Vec<String> {
        record::split_lines(s)
    }

    fn sample() -> String {
        "origin: docs/x.md\nblob: 0000000000000000000000000000000000000000\n\n1-3\tdropped: dead\n4-4\tdropped: detail\n"
            .to_string()
    }

    #[test]
    fn well_formed_file_parses_with_no_shape_errors() {
        let bf = parse(PathBuf::from("x.md"), "adrdom/blame/x.md".to_string(), lines_of(&sample()));
        assert!(bf.shape_errors.is_empty(), "{:?}", bf.shape_errors);
        assert_eq!(bf.origin.as_deref(), Some("docs/x.md"));
        assert_eq!(bf.blob.as_deref(), Some("0000000000000000000000000000000000000000"));
        assert_eq!(bf.rows.len(), 2);
        assert_eq!((bf.rows[0].start, bf.rows[0].end), (1, 3));
    }

    #[test]
    fn missing_origin_is_a_shape_error() {
        let s = sample().replacen("origin: docs/x.md\n", "not an origin line\n", 1);
        let bf = parse(PathBuf::from("x.md"), "x.md".to_string(), lines_of(&s));
        assert!(bf.shape_errors.iter().any(|e| e.contains("origin")));
    }

    #[test]
    fn a_short_blob_is_a_shape_error() {
        let s = sample().replacen(
            "blob: 0000000000000000000000000000000000000000\n",
            "blob: abc\n",
            1,
        );
        let bf = parse(PathBuf::from("x.md"), "x.md".to_string(), lines_of(&s));
        assert!(bf.shape_errors.iter().any(|e| e.contains("blob")));
    }

    // A sha256-object-format repo's blob ids are 64 hex characters, not 40.
    #[test]
    fn a_64_hex_blob_is_accepted() {
        let s = sample().replacen(
            "blob: 0000000000000000000000000000000000000000\n",
            &format!("blob: {}\n", "0".repeat(64)),
            1,
        );
        let bf = parse(PathBuf::from("x.md"), "x.md".to_string(), lines_of(&s));
        assert!(bf.shape_errors.is_empty(), "{:?}", bf.shape_errors);
        assert_eq!(bf.blob.as_deref(), Some("0".repeat(64).as_str()));
    }

    #[test]
    fn a_row_with_no_tab_is_a_shape_error() {
        let s = sample().replacen("1-3\tdropped: dead\n", "1-3 dropped: dead\n", 1);
        let bf = parse(PathBuf::from("x.md"), "x.md".to_string(), lines_of(&s));
        assert!(bf.shape_errors.iter().any(|e| e.contains("not a row")));
    }

    #[test]
    fn coverage_errors_finds_a_gap_and_an_overlap_passes() {
        let rows = vec![
            Row { start: 1, end: 2, dest: "dropped: dead".into(), line_idx: 0 },
            Row { start: 2, end: 4, dest: "dropped: dead".into(), line_idx: 1 },
            // line 5 is a gap.
        ];
        let (beyond, gaps) = coverage_errors(&rows, 6);
        assert!(beyond.is_empty());
        assert_eq!(gaps.len(), 1);
        assert_eq!((gaps[0].start, gaps[0].end), (5, 6));
    }

    #[test]
    fn coverage_errors_finds_a_range_beyond_n() {
        let rows = vec![Row { start: 1, end: 10, dest: "dropped: dead".into(), line_idx: 0 }];
        let (beyond, gaps) = coverage_errors(&rows, 5);
        assert_eq!(beyond.len(), 1);
        assert_eq!((beyond[0].start, beyond[0].end), (1, 10));
        assert_eq!(gaps.len(), 1, "the rest of the origin is still a gap");
    }

    #[test]
    fn parse_row_rejects_an_inverted_range() {
        assert!(parse_row("5-3\tdropped: dead").is_none());
    }
}
