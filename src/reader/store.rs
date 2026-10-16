//! The reader file: its rows, their line form, the cached read of the file,
//! the live sets a row may still name, and the locked append.
use super::*;

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
    pub(super) fn key(&self) -> (String, String, String, String) {
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
    if !is_time(p[0])
        || !is_hash(p[1])
        || !neighbour_ok
        || !word_ok
        || !is_hash(p[5])
        || p[6].is_empty()
    {
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
    let key: Key = (
        path.clone(),
        meta.as_ref().map_or(0, |m| m.len()),
        meta.and_then(|m| m.modified().ok()),
    );
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
        repo::Source::Index => std::sync::Arc::new(
            parse(&source.read(root, repo::READER_PATH).unwrap_or_default()).rows,
        ),
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
        Live {
            root: root.to_path_buf(),
            sets: std::sync::OnceLock::new(),
        }
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
        LiveSets {
            texts,
            neighbours,
            complete,
        }
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
    let lead = if !old.is_empty() && !old.ends_with('\n') {
        "\n"
    } else {
        ""
    };
    let kept: Vec<&str> = old
        .lines()
        .filter(|l| parse_row(l).map_or(true, |r| live.holds(&r)))
        .collect();
    if kept.len() == old.lines().count() {
        let text = format!("{}{}", lead, text);
        return handle
            .write_all(text.as_bytes())
            .map_err(msg::reader_write_failed);
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
        let h = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)?;
        h.lock()?;
        if same_file(&h, path) {
            return Ok(h);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "the file kept changing under the lock",
    ))
}

/// Removes every `<name>.*.tmp` beside the file `path`, `<name>` its file
/// name.
pub(crate) fn remove_stale_tmp(path: &Path) {
    let (Some(dir), Some(own)) = (path.parent(), path.file_name()) else {
        return;
    };
    let prefix = format!("{}.", own.to_string_lossy());
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_stored_word_is_a_word_of_a_set_depends_or_closes_with_a_capital_letter() {
        for w in [
            "same",
            "consistent",
            "inconsistent",
            "unrelated",
            "depends",
            "closes A",
            "closes Z",
        ] {
            assert!(is_stored_word(w), "{}", w);
        }
        for w in [
            "closes",
            "closes a",
            "closes AB",
            "closes  A",
            "close A",
            "maybe",
            "",
        ] {
            assert!(!is_stored_word(w), "{}", w);
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
        assert!(
            parse_row(&good.replace("d10dab89-1d42-4764-9d72-22d6fd6a9260", "text:abc")).is_none()
        );
        assert!(parse_row(&good.replace("d10dab89-1d42-4764-9d72-22d6fd6a9260", "")).is_none());
        assert!(parse_row(&good.replace("depends", "closes B")).is_some());
        assert!(parse_row(&good.replace("depends", "closes")).is_none());
        let f = parse(&format!("{}\n\nbad\n", good));
        assert_eq!(f.rows.len(), 1);
        assert_eq!(f.malformed, vec![(3, "bad".to_string())]);
    }
}
