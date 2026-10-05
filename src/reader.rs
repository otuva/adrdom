//! The reader: a model asked how a new text relates to a neighbour, a
//! decision, a term or another question of the same plan.
//!
//! "What does the reader say about a neighbour of a heading?"
//! (62e50382-db1b-4649-9f26-7f44c379d6fe A): one of same, depends,
//! unrelated, after one sentence of reason. "What does the reader say about
//! a neighbour of a question with its body?"
//! (c6e0b7d6-cdec-46da-b2bc-cda218673868 A): one of same, consistent,
//! inconsistent, depends, unrelated, after one sentence of reason. A reply
//! is the reason, then on its last non-empty line exactly one word of the
//! prompt's set; any other reply is an error for that pair, never coerced.
//!
//! "What does the tool keep of the reader's word?"
//! (08a1b488-32fc-4dc7-b4b3-93a6e768725f A): a row in its own file,
//! `adrdom/state/reader.tsv`, with the columns time, text hash, neighbour,
//! word, reason, prompt hash, model and version. The text hash is the
//! sha256 of the new side's text as it was given to the reader. The
//! neighbour is the identifier of a decision or a term, or, for another
//! question of the same plan that has no identifier yet, `text:` and the
//! sha256 of that question's heading and body.
//!
//! "Does planning stop without the reader?"
//! (3dba486c-a399-4ce3-a794-e38c4f733403 A): yes. `load` refuses when the
//! `claude` program is not on PATH. No other command loads the reader.
//!
//! `Claude` runs the `claude` program with a fixed model. `Fake`, built only
//! with the `fake-model` feature and used only when `ADR_FAKE_MODEL=1`,
//! needs no network: it answers with the word of the first marker
//! `[word:<w>]` in the new text, or, for a marker `[word:<w>@<key>]`, only
//! for a neighbour whose key contains `<key>`; a marker with a key comes
//! before one without. A text with no fitting marker gets `unrelated`. A
//! word outside the prompt's set is given as it is, so a test can see the
//! error. With the feature on, `ADR_NO_READER=1` is a machine with no
//! `claude` program.
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
const AT_ONCE: usize = 4;

/// The words of a heading against a decision or a term, and of a heading
/// against another question of the plan.
pub const HEADING_WORDS: [&str; 3] = ["same", "depends", "unrelated"];
/// The words of a question with its body against a decision or a term.
pub const BODY_WORDS: [&str; 5] = ["same", "consistent", "inconsistent", "depends", "unrelated"];

/// What the reader is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A heading against a decision or a term.
    Heading,
    /// A question with its body against a decision or a term.
    Body,
    /// A question with its body against another question of the same plan.
    Plan,
}

impl Kind {
    pub fn prompt(self) -> &'static str {
        match self {
            Kind::Heading => include_str!("../assets/reader-heading.md"),
            Kind::Body => include_str!("../assets/reader-body.md"),
            Kind::Plan => include_str!("../assets/reader-plan.md"),
        }
    }

    pub fn prompt_hash(self) -> String {
        sha256_hex(self.prompt())
    }

    pub fn words(self) -> &'static [&'static str] {
        match self {
            Kind::Heading | Kind::Plan => &HEADING_WORDS,
            Kind::Body => &BODY_WORDS,
        }
    }

    fn labels(self) -> (&'static str, &'static str) {
        match self {
            Kind::Heading | Kind::Body => ("Existing record", "New question"),
            Kind::Plan => ("First question", "Second question"),
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

/// The reader file's line for a row: eight tab-separated columns, tabs and
/// newlines in a column flattened to a space the way the ledger does.
pub fn row_line(r: &Row) -> String {
    let f = |s: &str| s.replace(['\t', '\n', '\r'], " ");
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
/// a name without spaces (an identifier) or `text:` and a sha256, a word of one of the two sets, a
/// reason, a sha256 prompt hash and a model.
pub fn parse_row(line: &str) -> Option<Row> {
    let p: Vec<&str> = line.split('\t').collect();
    if p.len() != 8 {
        return None;
    }
    let neighbour_ok = match p[2].strip_prefix("text:") {
        Some(h) => is_hash(h),
        None => !p[2].is_empty() && !p[2].contains(char::is_whitespace),
    };
    let word_ok = HEADING_WORDS.contains(&p[3]) || BODY_WORDS.contains(&p[3]);
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

/// Appends `rows` to the reader file.
pub fn append(root: &Path, rows: &[Row]) -> Result<(), String> {
    use std::io::Write;
    if rows.is_empty() {
        return Ok(());
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join(repo::READER_PATH))
        .map_err(msg::reader_write_failed)?;
    let mut text = String::new();
    for r in rows {
        text.push_str(&row_line(r));
        text.push('\n');
    }
    f.write_all(text.as_bytes()).map_err(msg::reader_write_failed)
}

/// The reason and the word of a reply: the last non-empty line is the word,
/// lowercased and trimmed of punctuation, and must be one word of the
/// prompt's set; the text before it is the reason.
pub fn parse_reply(kind: Kind, reply: &str) -> Result<(String, String), String> {
    let lines: Vec<&str> = reply.lines().collect();
    let Some(last) = lines.iter().rposition(|l| !l.trim().is_empty()) else {
        return Err("the reply is empty".to_string());
    };
    let word = lines[last]
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    if !kind.words().contains(&word.as_str()) {
        return Err(format!(
            "the last line is not one of {}: {:?}",
            kind.words().join(", "),
            lines[last].trim()
        ));
    }
    let reason = lines[..last].iter().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
    Ok((reason, word))
}

pub trait Reader: Sync {
    /// The raw reply of the reader to `pair`.
    fn ask(&self, pair: &Pair) -> Result<String, String>;
}

pub struct Claude;

impl Reader for Claude {
    fn ask(&self, pair: &Pair) -> Result<String, String> {
        let (existing, new) = pair.kind.labels();
        let user = format!("{}:\n{}\n\n{}:\n{}", existing, pair.neighbour_text, new, pair.new_text);
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
    let reader = load()?;
    read_with(root, reader.as_ref(), pairs)
}

pub fn read_with(root: &Path, reader: &dyn Reader, pairs: &[Pair]) -> Result<Vec<Row>, String> {
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
                        (i, reader.ask(p).and_then(|reply| parse_reply(p.kind, &reply)))
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
    append(root, &fresh)?;
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
            neighbour: "62e50382-db1b-4649-9f26-7f44c379d6fe".to_string(),
            word: "depends".to_string(),
            reason: "It stands on it.".to_string(),
            prompt_hash: Kind::Heading.prompt_hash(),
            model: MODEL.to_string(),
            version: "0.1.0".to_string(),
        }
    }

    // adr: 62e50382-db1b-4649-9f26-7f44c379d6fe
    #[test]
    fn heading_reply_is_a_reason_then_one_word_of_the_heading_set() {
        let (reason, word) = parse_reply(Kind::Heading, "It asks the same.\n\nSame.\n").unwrap();
        assert_eq!((reason.as_str(), word.as_str()), ("It asks the same.", "same"));
        for w in HEADING_WORDS {
            assert_eq!(parse_reply(Kind::Heading, &format!("r\n{}", w.to_uppercase())).unwrap().1, w);
            assert_eq!(parse_reply(Kind::Plan, &format!("r\n{}.", w)).unwrap().1, w);
        }
        // Words of the body set are not words of the heading set.
        assert!(parse_reply(Kind::Heading, "r\nconsistent").is_err());
        assert!(parse_reply(Kind::Heading, "r\ninconsistent").is_err());
        assert!(parse_reply(Kind::Plan, "r\nconsistent").is_err());
        // Never coerced: a word inside a sentence, or a word before the last line.
        assert!(parse_reply(Kind::Heading, "same\nI think so").is_err());
        assert!(parse_reply(Kind::Heading, "r\nsame or depends").is_err());
        assert!(parse_reply(Kind::Heading, "").is_err());
    }

    // adr: c6e0b7d6-cdec-46da-b2bc-cda218673868
    #[test]
    fn body_reply_is_a_reason_then_one_word_of_the_body_set() {
        for w in BODY_WORDS {
            let (reason, got) = parse_reply(Kind::Body, &format!("One sentence.\n{}", w)).unwrap();
            assert_eq!((reason.as_str(), got.as_str()), ("One sentence.", w));
        }
        assert_eq!(BODY_WORDS.len(), 5);
        assert!(parse_reply(Kind::Body, "r\nmaybe").is_err());
        assert!(parse_reply(Kind::Body, "r\nconsistently").is_err());
        assert!(parse_reply(Kind::Body, "inconsistent").is_ok());
    }

    #[test]
    fn the_three_prompts_differ_and_name_their_words() {
        let hashes: HashSet<String> = [Kind::Heading, Kind::Body, Kind::Plan].iter().map(|k| k.prompt_hash()).collect();
        assert_eq!(hashes.len(), 3);
        assert!(Kind::Body.prompt().contains("inconsistent"));
        assert!(!Kind::Heading.prompt().contains("inconsistent"));
    }

    // adr: 08a1b488-32fc-4dc7-b4b3-93a6e768725f
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
        assert!(parse_row(&good.replace("62e50382-db1b-4649-9f26-7f44c379d6fe", "text:abc")).is_none());
        assert!(parse_row(&good.replace("62e50382-db1b-4649-9f26-7f44c379d6fe", "")).is_none());
        let f = parse(&format!("{}\n\nbad\n", good));
        assert_eq!(f.rows.len(), 1);
        assert_eq!(f.malformed, vec![(3, "bad".to_string())]);
    }

    #[cfg(feature = "fake-model")]
    #[test]
    fn the_fake_reader_follows_the_marker_for_the_neighbour() {
        let p = |text: &str, n: &str| Pair {
            kind: Kind::Heading,
            new_text: text.to_string(),
            neighbour: n.to_string(),
            neighbour_text: String::new(),
        };
        let word = |p: &Pair| parse_reply(p.kind, &Fake.ask(p).unwrap()).unwrap().1;
        assert_eq!(word(&p("Does X? [word:same]", "id-1")), "same");
        assert_eq!(word(&p("Does X?", "id-1")), "unrelated");
        let t = "Does X? [word:same] [word:depends@id-2]";
        assert_eq!(word(&p(t, "id-1")), "same");
        assert_eq!(word(&p(t, "id-2")), "depends");
    }
}
