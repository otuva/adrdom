//! The reader: a model asked how a new text relates to a neighbour, a
//! decision, a term or another question of the same plan.
//!
//! "Does the reader read a neighbour of a heading?"
//! (60341be0-fd05-4281-92d7-668584a8539a A): no. A heading is never given to
//! the reader; its neighbours are recorded without a word
//! (`neighbours::record`). "What word does the reader give a neighbour of a
//! question with its body?" (d10dab89-1d42-4764-9d72-22d6fd6a9260 A): one of
//! same, consistent, inconsistent, unrelated, or `closes` with the letter of
//! the option it leaves standing, after one sentence of reason. "What word
//! does the reader give two current decisions?"
//! (a116a1c5-d6ea-4123-be25-3a222ac661e0 A): one of same, consistent,
//! inconsistent, unrelated, after one sentence of reason. A reply is the
//! reason, then on its last non-empty line exactly one word of the prompt's
//! set; any other reply is an error for that pair, never coerced. A question
//! of the plan against another gets same or unrelated. The stored word of
//! `closes B` is `closes B`.
//!
//! "What does the tool keep of a reader's word?"
//! (55a6a2d8-03af-4db9-a4c7-e2790e38357e A): a row in its own file,
//! `adrdom/state/reader.tsv`, with the columns time, text hash, neighbour,
//! word, reason, prompt hash, model and version. The text hash is the
//! sha256 of the new side's text as it was given to the reader. The
//! neighbour is the identifier of a decision or a term, or, for another
//! question of the same plan that has no identifier yet, `text:` and the
//! sha256 of that question's heading and body.
//!
//! "Does planning stop with no reader on the machine?"
//! (bd94a89f-7142-4dab-b757-6482a718a0a0 A): yes. `load` refuses when the
//! `claude` program is not on PATH; planning loads it only at the body
//! step. `adr read` loads the reader too, for a
//! `Set` pair: a current decision against another, the first text the one
//! with the lower identifier.
//!
//! `Claude` runs the `claude` program with a fixed model. `Fake`, built only
//! with the `fake-model` feature and used only when `ADR_FAKE_MODEL=1`,
//! needs no network: it answers with the word of the first marker
//! `[word:<w>]` in the new text, or, for a marker `[word:<w>@<key>]`, only
//! for a neighbour whose key contains `<key>`; a marker with a key comes
//! before one without. `<w>` may be `closes B`. A text with no fitting
//! marker gets `unrelated`. A word outside the prompt's set is given as it
//! is, so a test can see the error. With the feature on, `ADR_NO_READER=1` is
//! a machine with no `claude` program.
use crate::ledger;
use crate::msg;
use crate::record::sha256_hex;
use crate::repo;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const MODEL: &str = "claude-sonnet-5-5";
const TIMEOUT: Duration = Duration::from_secs(120);
/// How many pairs are read at once; `adr read` hands the reader this many.
pub const AT_ONCE: usize = 4;

/// The words of a question with its body against a decision or a term,
/// besides `closes` and a letter.
pub const BODY_WORDS: [&str; 4] = ["same", "consistent", "inconsistent", "unrelated"];
/// The words of a question against another question of the same plan.
pub const PLAN_WORDS: [&str; 2] = ["same", "unrelated"];
/// The words of a current decision against another current decision.
pub const SET_WORDS: [&str; 4] = ["same", "consistent", "inconsistent", "unrelated"];
/// The word that opens a stored `closes <letter>`.
pub const CLOSES: &str = "closes";
/// The words a stored row may hold besides `closes <letter>`: the words of
/// every kind, and `depends`, which rows written under an earlier prompt hold.
const STORED_WORDS: [&str; 5] = ["same", "consistent", "inconsistent", "unrelated", "depends"];

/// Whether `word` is a word a stored row may hold: one of `STORED_WORDS`, or
/// `closes` and one letter A to Z.
pub fn is_stored_word(word: &str) -> bool {
    STORED_WORDS.contains(&word)
        || word
            .strip_prefix(CLOSES)
            .and_then(|r| r.strip_prefix(' '))
            .is_some_and(|l| l.len() == 1 && l.chars().all(|c| c.is_ascii_uppercase()))
}

/// What the reader is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A question with its body against a decision or a term.
    Body,
    /// A question with its body against another question of the same plan.
    Plan,
    /// A current decision against another current decision.
    Set,
}

impl Kind {
    pub fn prompt(self) -> &'static str {
        match self {
            Kind::Body => include_str!("../assets/reader-body.md"),
            Kind::Plan => include_str!("../assets/reader-plan.md"),
            Kind::Set => include_str!("../assets/reader-set.md"),
        }
    }

    pub fn prompt_hash(self) -> String {
        sha256_hex(self.prompt())
    }

    pub fn words(self) -> &'static [&'static str] {
        match self {
            Kind::Body => &BODY_WORDS,
            Kind::Plan => &PLAN_WORDS,
            Kind::Set => &SET_WORDS,
        }
    }

    fn labels(self) -> (&'static str, &'static str) {
        match self {
            Kind::Body => ("Existing record", "New question"),
            Kind::Plan => ("First question", "Second question"),
            Kind::Set => ("First decision", "Second decision"),
        }
    }
}

/// One new text and one neighbour to read it against.
#[derive(Debug, Clone)]
pub struct Pair {
    pub kind: Kind,
    pub new_text: String,
    /// The identifier of the neighbour, or `neighbour_key_of_text`.
    pub neighbour: String,
    pub neighbour_text: String,
    /// The letters of the new question's options: the letters `closes` may
    /// name. Empty for a kind that has no `closes`.
    pub letters: Vec<char>,
}

/// The neighbour key of a question of the plan that has no identifier yet.
pub fn neighbour_key_of_text(text: &str) -> String {
    format!("text:{}", sha256_hex(text))
}

/// One row of the reader file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub time: String,
    pub text_hash: String,
    pub neighbour: String,
    pub word: String,
    pub reason: String,
    pub prompt_hash: String,
    pub model: String,
    pub version: String,
}

impl Row {
    fn key(&self) -> (String, String, String, String) {
        (
            self.text_hash.clone(),
            self.neighbour.clone(),
            self.prompt_hash.clone(),
            self.model.clone(),
        )
    }
}

/// A column as the reader file keeps it: tabs and newlines become a space.
pub fn flatten(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

/// The reader file's line for a row: eight tab-separated columns, tabs and
/// newlines in a column flattened to a space the way the ledger does.
pub fn row_line(r: &Row) -> String {
    let f = flatten;
    [
        &r.time,
        &r.text_hash,
        &r.neighbour,
        &r.word,
        &r.reason,
        &r.prompt_hash,
        &r.model,
        &r.version,
    ]
    .iter()
    .map(|c| f(c))
    .collect::<Vec<_>>()
    .join("\t")
}

fn is_hash(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Whether `s` reads as a row's time, an RFC3339 time.
fn is_time(s: &str) -> bool {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::parse(s, &Rfc3339).is_ok()
}

/// A row of the reader file from `line`, or `None` when it is malformed:
/// eight columns, an RFC3339 time, a sha256 text hash, a neighbour that is
/// a name without spaces (an identifier) or `text:` and a sha256, a word
/// `is_stored_word` holds, a reason, a sha256 prompt hash and a model.
pub fn parse_row(line: &str) -> Option<Row> {
    let p: Vec<&str> = line.split('\t').collect();
    if p.len() != 8 {
        return None;
    }
    let neighbour_ok = match p[2].strip_prefix("text:") {
        Some(h) => is_hash(h),
        None => !p[2].is_empty() && !p[2].contains(char::is_whitespace),
    };
    let word_ok = is_stored_word(p[3]);
    if !is_time(p[0]) || !is_hash(p[1]) || !neighbour_ok || !word_ok || !is_hash(p[5]) || p[6].is_empty() {
        return None;
    }
    Some(Row {
        time: p[0].to_string(),
        text_hash: p[1].to_string(),
        neighbour: p[2].to_string(),
        word: p[3].to_string(),
        reason: p[4].to_string(),
        prompt_hash: p[5].to_string(),
        model: p[6].to_string(),
        version: p[7].to_string(),
    })
}

/// The rows of the reader file's `content`, and the malformed lines with
/// their line numbers. A row's place in the file is free.
pub struct File {
    pub rows: Vec<Row>,
    pub malformed: Vec<(usize, String)>,
}

pub fn parse(content: &str) -> File {
    let mut rows = Vec::new();
    let mut malformed = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        match parse_row(line) {
            Some(r) => rows.push(r),
            None => malformed.push((i + 1, line.to_string())),
        }
    }
    File { rows, malformed }
}

pub fn read_file(root: &Path) -> File {
    parse(&std::fs::read_to_string(root.join(repo::READER_PATH)).unwrap_or_default())
}

/// The rows of the reader file, parsed once for as long as the file stands
/// as it was read: every planning record of every topic file is read off
/// the one file, and a command reads it for each file it judges. Keyed by
/// the file's path, length and time of change, so a row appended is read.
pub fn rows(root: &Path) -> std::sync::Arc<Vec<Row>> {
    use std::sync::{Arc, Mutex, OnceLock};
    type Key = (std::path::PathBuf, u64, Option<std::time::SystemTime>);
    static CACHE: OnceLock<Mutex<Option<(Key, Arc<Vec<Row>>)>>> = OnceLock::new();
    let path = root.join(repo::READER_PATH);
    let meta = std::fs::metadata(&path).ok();
    let key: Key = (path.clone(), meta.as_ref().map_or(0, |m| m.len()), meta.and_then(|m| m.modified().ok()));
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    let mut held = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, rows)) = held.as_ref() {
        if *k == key {
            return Arc::clone(rows);
        }
    }
    // A file that exists and cannot be read is no file of zero rows: what
    // it read is not kept, so the next call reads it again.
    let text = std::fs::read_to_string(&path);
    let failed = text.is_err() && path.exists();
    let rows = Arc::new(parse(&text.unwrap_or_default()).rows);
    if !failed {
        *held = Some((key, Arc::clone(&rows)));
    }
    rows
}

/// The rows of the reader file as `source` holds it: the working tree's
/// through `rows`, the staged copy at a commit (per "Does check at a commit
/// read the staged files?"), so the planning records judged at a commit are
/// the ones the commit carries.
pub fn rows_via(root: &Path, source: &repo::Source) -> std::sync::Arc<Vec<Row>> {
    match source {
        repo::Source::WorkingTree => rows(root),
        repo::Source::Index => std::sync::Arc::new(parse(&source.read(root, repo::READER_PATH).unwrap_or_default()).rows),
    }
}

/// The texts and the neighbours the records hold, the ones a row of the
/// reader file may still name.
pub struct Live {
    root: PathBuf,
    sets: std::sync::OnceLock<LiveSets>,
}

struct LiveSets {
    texts: HashSet<String>,
    neighbours: HashSet<String>,
    /// Whether every topic file and the terms file loaded. A text is not
    /// gone from records that could not be read, so nothing is dropped
    /// against sets that are not complete.
    complete: bool,
}

impl Live {
    /// Read at most once for a command run, and only when an append has
    /// rows to write: the working tree's topic files and terms file. A text is the hash of a heading, of a heading with its
    /// body, or of an answered question's heading and picked option (what
    /// `adr read` gives the reader). A neighbour is the id of a question,
    /// the id (or name) of a term, retired or not, or `text:` and the hash
    /// of a question's heading and body.
    pub fn of(root: &Path) -> Live {
        Live { root: root.to_path_buf(), sets: std::sync::OnceLock::new() }
    }

    fn load(root: &Path) -> LiveSets {
        let mut texts = HashSet::new();
        let mut neighbours = HashSet::new();
        let (records, load_errors) = crate::record::load_all_via(root, &repo::Source::WorkingTree);
        let mut complete = load_errors.is_empty();
        for rf in records {
            for sec in &rf.sections {
                texts.insert(crate::record::heading_hash(sec));
                let body = crate::record::body_hash(&rf, sec);
                neighbours.insert(format!("text:{}", body));
                texts.insert(body);
                if let Some(id) = &sec.id {
                    neighbours.insert(id.clone());
                    if let Some(s) = sec.chosen_sentence() {
                        texts.insert(sha256_hex(&format!("{}\n{}", sec.heading_text.trim(), s)));
                    }
                }
            }
        }
        match crate::state::load_terms_checked(root) {
            crate::state::TermsLoad::Ok(tf) => {
                for t in &tf.terms {
                    neighbours.insert(t.id.clone().unwrap_or_else(|| t.name.clone()));
                }
            }
            crate::state::TermsLoad::Missing => {}
            crate::state::TermsLoad::LoadError(_) => complete = false,
        }
        LiveSets { texts, neighbours, complete }
    }

    /// Reads the records now, so that a caller about to take a lock does not
    /// read them under it.
    pub fn preload(&self) {
        self.sets.get_or_init(|| Live::load(&self.root));
    }

    /// Whether a heading's or a question's text of this hash is in a topic
    /// file. True when the records could not all be read.
    pub fn has_text(&self, hash: &str) -> bool {
        let s = self.sets.get_or_init(|| Live::load(&self.root));
        !s.complete || s.texts.contains(hash)
    }

    fn holds(&self, r: &Row) -> bool {
        let s = self.sets.get_or_init(|| Live::load(&self.root));
        !s.complete || s.texts.contains(&r.text_hash) && s.neighbours.contains(&r.neighbour)
    }
}

/// Appends `rows` to the reader file. Per "Does the reader's file keep a row
/// whose text left the records?" (a8034401-420c-45d7-8f43-60c1b9f3baf9
/// A): when it appends, it first drops each row whose text or neighbour is
/// gone from the records; a line that is no row stays as it is. With no row
/// to append nothing is dropped. Prints nothing: a dropped row is no ledger
/// event.
pub fn append(root: &Path, rows: &[Row]) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    append_live(root, rows, &Live::of(root))
}

/// Like `append`, against the live sets a command read once for its run.
/// The whole read-filter-write runs under an exclusive lock on the reader
/// file, so a session that appends or rewrites at the same time loses no row.
pub fn append_live(root: &Path, rows: &[Row], live: &Live) -> Result<(), String> {
    use std::io::{Read, Write};
    if rows.is_empty() {
        return Ok(());
    }
    let path = root.join(repo::READER_PATH);
    let mut text = String::new();
    for r in rows {
        text.push_str(&row_line(r));
        text.push('\n');
    }
    let mut handle = lock_file(&path).map_err(msg::reader_write_failed)?;
    // Under the lock no other writer is mid-rewrite, so a temporary file a
    // killed process left is stale.
    remove_stale_tmp(&path);
    let mut old = String::new();
    // Bytes that are no text read as empty and so are never dropped.
    if handle.read_to_string(&mut old).is_err() {
        old.clear();
    }
    // A file that does not end in a newline would glue its last line to the
    // first new row.
    let lead = if !old.is_empty() && !old.ends_with('\n') { "\n" } else { "" };
    let kept: Vec<&str> = old.lines().filter(|l| parse_row(l).map_or(true, |r| live.holds(&r))).collect();
    if kept.len() == old.lines().count() {
        let text = format!("{}{}", lead, text);
        return handle.write_all(text.as_bytes()).map_err(msg::reader_write_failed);
    }
    let mut all = String::new();
    for l in kept {
        all.push_str(l);
        all.push('\n');
    }
    all.push_str(&text);
    let tmp = path.with_file_name(format!("reader.tsv.{}.tmp", uuid::Uuid::new_v4()));
    let done = std::fs::write(&tmp, all).and_then(|_| std::fs::rename(&tmp, &path));
    if let Err(e) = done {
        let _ = std::fs::remove_file(&tmp);
        return Err(msg::reader_write_failed(e));
    }
    Ok(())
}

/// How many times `append_live` reopens the reader file when a rewrite by
/// another session renames a new file over it, before it gives up.
const LOCK_TRIES: usize = 50;

/// Opens `path`, making it when it is missing, and locks it for one
/// writer. A rewrite by another session renames a new file over the path, so
/// the lock counts only when the path still names the locked file.
pub(crate) fn lock_file(path: &Path) -> std::io::Result<std::fs::File> {
    for _ in 0..LOCK_TRIES {
        let h = std::fs::OpenOptions::new().create(true).read(true).append(true).open(path)?;
        h.lock()?;
        if same_file(&h, path) {
            return Ok(h);
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::Other, "the file kept changing under the lock"))
}

/// Removes every `<name>.*.tmp` beside the file `path`, `<name>` its file
/// name.
pub(crate) fn remove_stale_tmp(path: &Path) {
    let (Some(dir), Some(own)) = (path.parent(), path.file_name()) else { return };
    let prefix = format!("{}.", own.to_string_lossy());
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&prefix) && name.ends_with(".tmp") {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Whether `path` still names the file `handle` holds open.
#[cfg(unix)]
fn same_file(handle: &std::fs::File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (handle.metadata(), std::fs::metadata(path)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_file(_handle: &std::fs::File, _path: &Path) -> bool {
    true
}

/// The reason and the word of a reply: the last non-empty line is the word,
/// lowercased and trimmed of punctuation, and must be one word of the
/// prompt's set; the text before it is the reason. For a body, `closes` and
/// one of `letters` is a word too, kept as `closes B`.
pub fn parse_reply(kind: Kind, letters: &[char], reply: &str) -> Result<(String, String), String> {
    let lines: Vec<&str> = reply.lines().collect();
    let Some(last) = lines.iter().rposition(|l| !l.trim().is_empty()) else {
        return Err("the reply is empty".to_string());
    };
    let word = lines[last]
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    let word = match closes_letter(kind, letters, &word) {
        Some(l) => format!("{} {}", CLOSES, l),
        None if kind.words().contains(&word.as_str()) => word,
        None => {
            let mut set = kind.words().join(", ");
            if kind == Kind::Body {
                set.push_str(", closes and the letter of an option");
            }
            return Err(format!("the last line is not one of {}: {:?}", set, lines[last].trim()));
        }
    };
    let reason = lines[..last].iter().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
    Ok((reason, word))
}

/// The upper-case letter of `closes <letter>` in `word` (lowercased) when
/// `kind` is a body and the letter is one of `letters`.
fn closes_letter(kind: Kind, letters: &[char], word: &str) -> Option<char> {
    if kind != Kind::Body {
        return None;
    }
    let mut parts = word.split_whitespace();
    let (Some(CLOSES), Some(l), None) = (parts.next(), parts.next(), parts.next()) else { return None };
    let mut cs = l.chars();
    let (Some(c), None) = (cs.next(), cs.next()) else { return None };
    let c = c.to_ascii_uppercase();
    letters.contains(&c).then_some(c)
}

pub trait Reader: Sync {
    /// The raw reply of the reader to `pair`.
    fn ask(&self, pair: &Pair) -> Result<String, String>;
}

pub struct Claude;

/// The message the reader is given for `pair`. Two decisions are given in
/// the pair's fixed order: the new side, the lower identifier, first. Every
/// other kind gives the record first and the new question second.
fn user_message(pair: &Pair) -> String {
    let (first, second) = pair.kind.labels();
    let (a, b) = match pair.kind {
        Kind::Set => (&pair.new_text, &pair.neighbour_text),
        _ => (&pair.neighbour_text, &pair.new_text),
    };
    format!("{}:\n{}\n\n{}:\n{}", first, a, second, b)
}

impl Reader for Claude {
    fn ask(&self, pair: &Pair) -> Result<String, String> {
        let user = user_message(pair);
        // A private directory made for this run: another local user cannot
        // plant settings or a CLAUDE.md in it.
        let cwd = PrivateDir::create()?;
        let mut child = Command::new("claude")
            .args([
                "-p",
                "--model",
                MODEL,
                "--output-format",
                "json",
                "--no-session-persistence",
                // No MCP server of the user's config loads into a reader call.
                "--strict-mcp-config",
                // Not the user's global settings or CLAUDE.md (cwd is a private temp dir).
                "--setting-sources",
                "project",
                "--tools",
                "",
                "--system-prompt",
                pair.kind.prompt(),
            ])
            .arg(&user)
            .current_dir(&cwd.0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    msg::reader_missing()
                } else {
                    format!("cannot run claude: {}", e)
                }
            })?;
        let mut out = child.stdout.take().ok_or("no stdout")?;
        let mut err = child.stderr.take().ok_or("no stderr")?;
        let t_out = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = out.read_to_string(&mut s);
            s
        });
        let t_err = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            s
        });
        let start = Instant::now();
        let status = loop {
            match child.try_wait().map_err(|e| e.to_string())? {
                Some(s) => break s,
                None if start.elapsed() > TIMEOUT => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("no answer in {} seconds", TIMEOUT.as_secs()));
                }
                None => std::thread::sleep(Duration::from_millis(50)),
            }
        };
        let stdout = t_out.join().unwrap_or_default();
        let stderr = t_err.join().unwrap_or_default();
        if !status.success() {
            return Err(format!("claude exited with {}: {}", status, stderr.trim()));
        }
        let v: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| format!("reply is not JSON: {}", e))?;
        v.get("result")
            .and_then(|r| r.as_str())
            .map(String::from)
            .ok_or_else(|| "reply has no result field".to_string())
    }
}

#[cfg(feature = "fake-model")]
pub struct Fake;

/// The word a marker in `text` picks for `neighbour`, if any.
#[cfg(feature = "fake-model")]
fn marked_word(text: &str, neighbour: &str) -> Option<String> {
    let mut general: Option<String> = None;
    let mut rest = text;
    while let Some(i) = rest.find("[word:") {
        rest = &rest[i + "[word:".len()..];
        let Some(end) = rest.find(']') else { break };
        let body = &rest[..end];
        rest = &rest[end..];
        match body.split_once('@') {
            Some((w, key)) if neighbour.contains(key) => return Some(w.to_string()),
            Some(_) => {}
            None if general.is_none() => general = Some(body.to_string()),
            None => {}
        }
    }
    general
}

#[cfg(feature = "fake-model")]
impl Reader for Fake {
    fn ask(&self, pair: &Pair) -> Result<String, String> {
        let word = marked_word(&pair.new_text, &pair.neighbour).unwrap_or_else(|| "unrelated".to_string());
        Ok(format!("The fake reader's reason.\n{}", word))
    }
}

/// Whether a program named `claude` is in a directory of PATH.
fn claude_on_path() -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join("claude").is_file()))
        .unwrap_or(false)
}

/// The fake reader when the feature is on and `ADR_FAKE_MODEL=1`, else
/// Claude. Refuses when there is no `claude` program to run.
pub fn load() -> Result<Box<dyn Reader>, String> {
    #[cfg(feature = "fake-model")]
    if std::env::var("ADR_NO_READER").as_deref() == Ok("1") {
        return Err(msg::reader_missing());
    }
    #[cfg(feature = "fake-model")]
    if std::env::var("ADR_FAKE_MODEL").as_deref() == Ok("1") {
        return Ok(Box::new(Fake));
    }
    if !claude_on_path() {
        return Err(msg::reader_missing());
    }
    Ok(Box::new(Claude))
}

/// A row for every pair, in order. A pair whose text hash, neighbour,
/// prompt hash and model are on record keeps its recorded row and is not
/// read again; the rest are read four at a time. The new rows are appended
/// to the reader file, under one time, only when every pair of the call has
/// a word: a call that fails keeps nothing.
pub fn read(root: &Path, pairs: &[Pair]) -> Result<Vec<Row>, String> {
    read_live(root, pairs, &Live::of(root))
}

/// Like `read`, against the live sets a command read once for its run.
pub fn read_live(root: &Path, pairs: &[Pair], live: &Live) -> Result<Vec<Row>, String> {
    let reader = load()?;
    read_with_live(root, reader.as_ref(), pairs, live)
}

pub fn read_with(root: &Path, reader: &dyn Reader, pairs: &[Pair]) -> Result<Vec<Row>, String> {
    read_with_live(root, reader, pairs, &Live::of(root))
}

pub fn read_with_live(root: &Path, reader: &dyn Reader, pairs: &[Pair], live: &Live) -> Result<Vec<Row>, String> {
    let mut have: HashMap<(String, String, String, String), Row> = HashMap::new();
    for r in read_file(root).rows {
        if r.model == MODEL {
            have.insert(r.key(), r);
        }
    }
    let key = |p: &Pair| (sha256_hex(&p.new_text), p.neighbour.clone(), p.kind.prompt_hash(), MODEL.to_string());
    let mut todo: Vec<usize> = Vec::new();
    let mut seen: HashSet<(String, String, String, String)> = HashSet::new();
    for (i, p) in pairs.iter().enumerate() {
        let k = key(p);
        if !have.contains_key(&k) && seen.insert(k) {
            todo.push(i);
        }
    }
    let time = ledger::now_rfc3339();
    let mut fresh: Vec<Row> = Vec::new();
    for chunk in todo.chunks(AT_ONCE) {
        let got: Vec<(usize, Result<(String, String), String>)> = std::thread::scope(|s| {
            let hs: Vec<_> = chunk
                .iter()
                .map(|&i| {
                    s.spawn(move || {
                        let p = &pairs[i];
                        (i, reader.ask(p).and_then(|reply| parse_reply(p.kind, &p.letters, &reply)))
                    })
                })
                .collect();
            hs.into_iter()
                .zip(chunk)
                .map(|(h, &i)| h.join().unwrap_or((i, Err("the reader stopped".to_string()))))
                .collect()
        });
        for (i, r) in got {
            let p = &pairs[i];
            match r {
                Ok((reason, word)) => {
                    let (text_hash, neighbour, prompt_hash, model) = key(p);
                    fresh.push(Row {
                        time: time.clone(),
                        text_hash,
                        neighbour,
                        word,
                        reason,
                        prompt_hash,
                        model,
                        version: crate::version::VERSION.to_string(),
                    });
                }
                Err(e) => {
                    let e = if e.starts_with("reader: ") { e } else { msg::reader_failed(&p.neighbour, e) };
                    return Err(e);
                }
            }
        }
    }
    append_live(root, &fresh, live)?;
    for r in fresh {
        have.insert(r.key(), r);
    }
    Ok(pairs.iter().map(|p| have[&key(p)].clone()).collect())
}

/// A fresh directory under the temp dir, named at random and open to its
/// owner only; removed when dropped. `create_dir` fails when the name exists.
struct PrivateDir(PathBuf);

impl PrivateDir {
    fn create() -> Result<PrivateDir, String> {
        use std::os::unix::fs::DirBuilderExt;
        let path = std::env::temp_dir().join(format!("adr-reader-{}", uuid::Uuid::new_v4()));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|e| format!("cannot make a directory for claude: {}", e))?;
        Ok(PrivateDir(path))
    }
}

impl Drop for PrivateDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: ed372cc6-c161-4b90-85bd-c73a6e788c5e
    #[test]
    fn a_set_pair_is_given_the_lower_identifier_first() {
        let mut p = Pair { kind: Kind::Set, new_text: "NEW".into(), neighbour: "n".into(), neighbour_text: "NEIGHBOUR".into(), letters: Vec::new() };
        assert_eq!(user_message(&p), "First decision:\nNEW\n\nSecond decision:\nNEIGHBOUR");
        p.kind = Kind::Body;
        assert!(user_message(&p).starts_with("Existing record:\nNEIGHBOUR"));
    }

    #[test]
    fn a_run_directory_is_private_fresh_and_removed_after() {
        use std::os::unix::fs::PermissionsExt;
        let a = PrivateDir::create().unwrap();
        let b = PrivateDir::create().unwrap();
        assert_ne!(a.0, b.0);
        assert_eq!(std::fs::metadata(&a.0).unwrap().permissions().mode() & 0o777, 0o700);
        let path = a.0.clone();
        drop(a);
        assert!(!path.exists());
    }

    fn row() -> Row {
        Row {
            time: "2026-10-05T10:00:00Z".to_string(),
            text_hash: sha256_hex("heading"),
            neighbour: "d10dab89-1d42-4764-9d72-22d6fd6a9260".to_string(),
            word: "depends".to_string(),
            reason: "It stands on it.".to_string(),
            prompt_hash: Kind::Body.prompt_hash(),
            model: MODEL.to_string(),
            version: "0.1.0".to_string(),
        }
    }

    #[test]
    fn plan_and_set_replies_are_a_reason_then_one_word_of_their_set() {
        let (reason, word) = parse_reply(Kind::Plan, &[], "It asks the same.\n\nSame.\n").unwrap();
        assert_eq!((reason.as_str(), word.as_str()), ("It asks the same.", "same"));
        for w in PLAN_WORDS {
            assert_eq!(parse_reply(Kind::Plan, &[], &format!("r\n{}.", w)).unwrap().1, w);
        }
        for w in SET_WORDS {
            assert_eq!(parse_reply(Kind::Set, &[], &format!("r\n{}", w.to_uppercase())).unwrap().1, w);
        }
        assert!(parse_reply(Kind::Plan, &[], "r\nconsistent").is_err());
        assert!(parse_reply(Kind::Plan, &[], "r\ndepends").is_err());
        assert!(parse_reply(Kind::Set, &[], "r\ndepends").is_err());
        assert!(parse_reply(Kind::Set, &['A', 'B'], "r\ncloses B").is_err());
        // Never coerced: a word inside a sentence, or a word before the last line.
        assert!(parse_reply(Kind::Plan, &[], "same\nI think so").is_err());
        assert!(parse_reply(Kind::Plan, &[], "r\nsame or unrelated").is_err());
        assert!(parse_reply(Kind::Plan, &[], "").is_err());
    }

    // adr: d10dab89-1d42-4764-9d72-22d6fd6a9260
    #[test]
    fn body_reply_is_a_reason_then_one_word_of_the_body_set_or_closes_with_a_letter() {
        for w in BODY_WORDS {
            let (reason, got) = parse_reply(Kind::Body, &['A', 'B'], &format!("One sentence.\n{}", w)).unwrap();
            assert_eq!((reason.as_str(), got.as_str()), ("One sentence.", w));
        }
        assert_eq!(BODY_WORDS.len(), 4);
        assert!(parse_reply(Kind::Body, &['A', 'B'], "r\nmaybe").is_err());
        assert!(parse_reply(Kind::Body, &['A', 'B'], "r\ndepends").is_err());
        assert!(parse_reply(Kind::Body, &['A', 'B'], "r\nconsistently").is_err());
        assert!(parse_reply(Kind::Body, &[], "inconsistent").is_ok());
        for reply in ["r\ncloses B", "r\nCloses b.", "r\n  closes   B  "] {
            assert_eq!(parse_reply(Kind::Body, &['A', 'B'], reply).unwrap().1, "closes B");
        }
        // No letter, a letter the question lacks, two letters, or another kind.
        for reply in ["r\ncloses", "r\ncloses C", "r\ncloses AB", "r\ncloses 1", "r\ncloses B or A"] {
            assert!(parse_reply(Kind::Body, &['A', 'B'], reply).is_err(), "{}", reply);
        }
        assert!(parse_reply(Kind::Body, &[], "r\ncloses A").is_err());
        assert!(parse_reply(Kind::Plan, &['A'], "r\ncloses A").is_err());
    }

    #[test]
    fn a_stored_word_is_a_word_of_a_set_depends_or_closes_with_a_capital_letter() {
        for w in ["same", "consistent", "inconsistent", "unrelated", "depends", "closes A", "closes Z"] {
            assert!(is_stored_word(w), "{}", w);
        }
        for w in ["closes", "closes a", "closes AB", "closes  A", "close A", "maybe", ""] {
            assert!(!is_stored_word(w), "{}", w);
        }
    }

    #[test]
    fn the_three_prompts_differ_and_name_their_words() {
        let hashes: HashSet<String> = [Kind::Body, Kind::Plan, Kind::Set].iter().map(|k| k.prompt_hash()).collect();
        assert_eq!(hashes.len(), 3);
        assert!(Kind::Body.prompt().contains("inconsistent"));
        assert!(Kind::Body.prompt().contains("closes"));
        assert!(!Kind::Plan.prompt().contains("inconsistent"));
        for k in [Kind::Body, Kind::Plan, Kind::Set] {
            assert!(!k.prompt().contains("depends"));
        }
    }

    // adr: 55a6a2d8-03af-4db9-a4c7-e2790e38357e
    #[test]
    fn a_row_has_eight_columns_and_round_trips() {
        let r = row();
        let line = row_line(&r);
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(cols.len(), 8);
        assert_eq!(cols[0], r.time);
        assert_eq!(cols[1], r.text_hash);
        assert_eq!(cols[2], r.neighbour);
        assert_eq!(cols[3], "depends");
        assert_eq!(cols[4], r.reason);
        assert_eq!(cols[5], r.prompt_hash);
        assert_eq!(cols[6], MODEL);
        assert_eq!(cols[7], "0.1.0");
        assert_eq!(parse_row(&line), Some(r.clone()));
        // A neighbour of the plan is `text:` and a hash.
        let mut plan = r.clone();
        plan.neighbour = neighbour_key_of_text("heading\nbody");
        assert!(plan.neighbour.starts_with("text:"));
        assert_eq!(parse_row(&row_line(&plan)), Some(plan));
    }

    #[test]
    fn a_reason_with_tabs_and_newlines_is_flattened_into_one_line() {
        let mut r = row();
        r.reason = "a\tb\nc".to_string();
        let line = row_line(&r);
        assert_eq!(line.lines().count(), 1);
        assert_eq!(parse_row(&line).unwrap().reason, "a b c");
    }

    #[test]
    fn a_malformed_row_is_not_a_row() {
        let good = row_line(&row());
        assert!(parse_row(&good).is_some());
        assert!(parse_row("a\tb").is_none());
        assert!(parse_row(&format!("{}\textra", good)).is_none());
        assert!(parse_row(&good.replace("depends", "maybe")).is_none());
        assert!(parse_row(&good.replace("2026-10-05T10:00:00Z", "yesterday")).is_none());
        assert!(parse_row(&good.replace("d10dab89-1d42-4764-9d72-22d6fd6a9260", "text:abc")).is_none());
        assert!(parse_row(&good.replace("d10dab89-1d42-4764-9d72-22d6fd6a9260", "")).is_none());
        assert!(parse_row(&good.replace("depends", "closes B")).is_some());
        assert!(parse_row(&good.replace("depends", "closes")).is_none());
        let f = parse(&format!("{}\n\nbad\n", good));
        assert_eq!(f.rows.len(), 1);
        assert_eq!(f.malformed, vec![(3, "bad".to_string())]);
    }

    #[cfg(feature = "fake-model")]
    #[test]
    fn the_fake_reader_follows_the_marker_for_the_neighbour() {
        let p = |text: &str, n: &str| Pair {
            kind: Kind::Body,
            new_text: text.to_string(),
            neighbour: n.to_string(),
            neighbour_text: String::new(),
            letters: vec!['A', 'B'],
        };
        let word = |p: &Pair| parse_reply(p.kind, &p.letters, &Fake.ask(p).unwrap()).unwrap().1;
        assert_eq!(word(&p("Does X? [word:same]", "id-1")), "same");
        assert_eq!(word(&p("Does X?", "id-1")), "unrelated");
        let t = "Does X? [word:same] [word:closes B@id-2]";
        assert_eq!(word(&p(t, "id-1")), "same");
        assert_eq!(word(&p(t, "id-2")), "closes B");
    }
}
