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
//! "What does the reader run?" (5418b256-dfae-439a-96c1-57d82380cd1e A):
//! the program of the agent named at install, found by the files install
//! wrote for it: `CLAUDE.md` importing `AGENTS.md` for `claude`, else
//! `.codex/rules/adrdom.rules` for `codex`. A repository installed for no
//! agent has no reader, and with both the tool picks `claude`.
//!
//! "Does planning stop with no reader on the machine?"
//! (bd94a89f-7142-4dab-b757-6482a718a0a0 A): yes. `load` refuses when that
//! program is not on PATH; planning loads it only at the body
//! step. `adr read` loads the reader too, for a
//! `Set` pair: a current decision against another, the first text the one
//! with the lower identifier.
//!
//! `Claude` and `Codex` each run their program with a fixed model, the
//! prompt on its standard input. `Fake`, built only
//! with the `fake-model` feature and used only when `ADR_FAKE_MODEL=1`,
//! needs no network: it answers with the word of the first marker
//! `[word:<w>]` in the new text, or, for a marker `[word:<w>@<key>]`, only
//! for a neighbour whose key contains `<key>`; a marker with a key comes
//! before one without. `<w>` may be `closes B`. A text with no fitting
//! marker gets `unrelated`. A word outside the prompt's set is given as it
//! is, so a test can see the error. With the feature on, `ADR_NO_READER=1` is
//! a machine with no reader program.
mod program;
mod store;

use program::SUFFIXES;

#[cfg(feature = "fake-model")]
pub use program::Fake;
pub use program::{find_program, model_of, program, Claude, Codex, Program};
pub use store::{
    append, append_live, flatten, is_stored_word, parse, parse_row, read_file, row_line, rows,
    rows_via, File, Live, Row,
};
pub(crate) use store::{lock_file, remove_stale_tmp};

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
/// The model `codex` is asked for; the key of the rows it reads.
pub const CODEX_MODEL: &str = "gpt-5.6-terra";
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
            Kind::Body => include_str!("../../assets/reader-body.md"),
            Kind::Plan => include_str!("../../assets/reader-plan.md"),
            Kind::Set => include_str!("../../assets/reader-set.md"),
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

    pub(super) fn labels(self) -> (&'static str, &'static str) {
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
            return Err(format!(
                "the last line is not one of {}: {:?}",
                set,
                lines[last].trim()
            ));
        }
    };
    let reason = lines[..last]
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Ok((reason, word))
}

/// The upper-case letter of `closes <letter>` in `word` (lowercased) when
/// `kind` is a body and the letter is one of `letters`.
fn closes_letter(kind: Kind, letters: &[char], word: &str) -> Option<char> {
    if kind != Kind::Body {
        return None;
    }
    // `closes: b` and `closes (b)` name the letter as `closes b` does.
    let mut parts = word
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty());
    let (Some(CLOSES), Some(l), None) = (parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    let mut cs = l.chars();
    let (Some(c), None) = (cs.next(), cs.next()) else {
        return None;
    };
    let c = c.to_ascii_uppercase();
    letters.contains(&c).then_some(c)
}

pub trait Reader: Sync {
    /// The raw reply of the reader to `pair`.
    fn ask(&self, pair: &Pair) -> Result<String, String>;
    /// The model it reads with: the key of the rows it writes.
    fn model(&self) -> &'static str;
}

/// The fake reader when the feature is on and `ADR_FAKE_MODEL=1`, else
/// the program of the agent installed in `root`. Refuses when no agent is
/// installed or its program is not on PATH.
pub fn load(root: &Path) -> Result<Box<dyn Reader>, String> {
    #[cfg(feature = "fake-model")]
    if std::env::var("ADR_NO_READER").as_deref() == Ok("1") {
        return Err(match program(root) {
            Some(p) => msg::reader_missing(p.name()),
            None => msg::reader_no_agent(),
        });
    }
    #[cfg(feature = "fake-model")]
    if std::env::var("ADR_FAKE_MODEL").as_deref() == Ok("1") {
        return Ok(Box::new(Fake));
    }
    load_in(root, &std::env::var_os("PATH").unwrap_or_default())
}

/// `load` with the directories of `path` searched, and no fake reader.
pub fn load_in(root: &Path, path: &std::ffi::OsStr) -> Result<Box<dyn Reader>, String> {
    let p = program(root).ok_or_else(msg::reader_no_agent)?;
    let exe =
        find_program(p.name(), path, SUFFIXES).ok_or_else(|| msg::reader_missing(p.name()))?;
    Ok(match p {
        Program::Claude => Box::new(Claude { exe }),
        Program::Codex => Box::new(Codex { exe }),
    })
}

/// What one call of the reader gave. `rows` holds, for each pair in order,
/// its row: the recorded one for a pair on record, and the fresh one when
/// every fresh pair of the call got a word; a fresh pair of a call that
/// did not is `None`. `failed` holds each pair the reader gave no word for,
/// with why.
#[derive(Debug, Default)]
pub struct Outcome {
    pub rows: Vec<Option<Row>>,
    pub failed: Vec<Failure>,
}

impl Outcome {
    /// Every row, when every pair has one.
    pub fn all(self) -> Option<Vec<Row>> {
        self.rows.into_iter().collect()
    }
}

/// A pair the reader gave no word for, asked twice: the program gave no
/// reply, or the reply was not a reason and one word of the set.
#[derive(Debug, Clone)]
pub struct Failure {
    /// The pair's position in the call.
    pub index: usize,
    pub neighbour: String,
    pub error: String,
}

/// The reader's answer to every pair, in order. A pair whose text hash,
/// neighbour, prompt hash and model are on record keeps its recorded row
/// and is not read again; the rest are read four at a time, and a pair
/// that gets no word is asked once more. The new rows are appended to the
/// reader file, under one time, only when every pair of the call has a
/// word: a call with a pair the reader gave no word for keeps nothing of
/// what it read, and names that pair in `failed`. `Err` is the reader
/// missing or the file unwritable, never a reply.
pub fn read(root: &Path, pairs: &[Pair]) -> Result<Outcome, String> {
    read_live(root, pairs, &Live::of(root))
}

/// Like `read`, against the live sets a command read once for its run.
pub fn read_live(root: &Path, pairs: &[Pair], live: &Live) -> Result<Outcome, String> {
    let reader = load(root)?;
    read_with_live(root, reader.as_ref(), pairs, live)
}

pub fn read_with(root: &Path, reader: &dyn Reader, pairs: &[Pair]) -> Result<Outcome, String> {
    read_with_live(root, reader, pairs, &Live::of(root))
}

/// One answer of the reader, by the index of the pair it answers.
type Answered = (usize, Result<(String, String), String>);

/// The reader asked `idxs` of `pairs`, `AT_ONCE` at a time.
fn ask_all(reader: &dyn Reader, pairs: &[Pair], idxs: &[usize]) -> Vec<Answered> {
    let mut out = Vec::new();
    for chunk in idxs.chunks(AT_ONCE) {
        let got: Vec<Answered> = std::thread::scope(|s| {
            let hs: Vec<_> = chunk
                .iter()
                .map(|&i| {
                    s.spawn(move || {
                        let p = &pairs[i];
                        (
                            i,
                            reader
                                .ask(p)
                                .and_then(|reply| parse_reply(p.kind, &p.letters, &reply)),
                        )
                    })
                })
                .collect();
            hs.into_iter()
                .zip(chunk)
                .map(|(h, &i)| {
                    h.join()
                        .unwrap_or((i, Err("the reader stopped".to_string())))
                })
                .collect()
        });
        out.extend(got);
    }
    out
}

pub fn read_with_live(
    root: &Path,
    reader: &dyn Reader,
    pairs: &[Pair],
    live: &Live,
) -> Result<Outcome, String> {
    let model = reader.model();
    let mut have: HashMap<(String, String, String, String), Row> = HashMap::new();
    for r in read_file(root).rows {
        if r.model == model {
            have.insert(r.key(), r);
        }
    }
    let key = |p: &Pair| {
        (
            sha256_hex(&p.new_text),
            p.neighbour.clone(),
            p.kind.prompt_hash(),
            model.to_string(),
        )
    };
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
    let mut failed: Vec<Failure> = Vec::new();
    // A pair that gets no word is asked once more: a reader is a model, and
    // one reply off its set is not the pair's answer.
    let mut got = ask_all(reader, pairs, &todo);
    let again: Vec<usize> = got
        .iter()
        .filter(|(_, r)| r.is_err())
        .map(|&(i, _)| i)
        .collect();
    if !again.is_empty() {
        got.retain(|(_, r)| r.is_ok());
        got.extend(ask_all(reader, pairs, &again));
    }
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
            Err(e) => failed.push(Failure {
                index: i,
                neighbour: p.neighbour.clone(),
                error: flatten(&e),
            }),
        }
    }
    failed.sort_by_key(|f| f.index);
    if failed.is_empty() {
        append_live(root, &fresh, live)?;
        for r in fresh {
            have.insert(r.key(), r);
        }
    }
    Ok(Outcome {
        rows: pairs.iter().map(|p| have.get(&key(p)).cloned()).collect(),
        failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_and_set_replies_are_a_reason_then_one_word_of_their_set() {
        let (reason, word) = parse_reply(Kind::Plan, &[], "It asks the same.\n\nSame.\n").unwrap();
        assert_eq!(
            (reason.as_str(), word.as_str()),
            ("It asks the same.", "same")
        );
        for w in PLAN_WORDS {
            assert_eq!(
                parse_reply(Kind::Plan, &[], &format!("r\n{}.", w))
                    .unwrap()
                    .1,
                w
            );
        }
        for w in SET_WORDS {
            assert_eq!(
                parse_reply(Kind::Set, &[], &format!("r\n{}", w.to_uppercase()))
                    .unwrap()
                    .1,
                w
            );
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
            let (reason, got) =
                parse_reply(Kind::Body, &['A', 'B'], &format!("One sentence.\n{}", w)).unwrap();
            assert_eq!((reason.as_str(), got.as_str()), ("One sentence.", w));
        }
        assert_eq!(BODY_WORDS.len(), 4);
        assert!(parse_reply(Kind::Body, &['A', 'B'], "r\nmaybe").is_err());
        assert!(parse_reply(Kind::Body, &['A', 'B'], "r\ndepends").is_err());
        assert!(parse_reply(Kind::Body, &['A', 'B'], "r\nconsistently").is_err());
        assert!(parse_reply(Kind::Body, &[], "inconsistent").is_ok());
        for reply in [
            "r\ncloses B",
            "r\nCloses b.",
            "r\n  closes   B  ",
            "r\nCloses: B",
            "r\ncloses (B)",
            "r\n**closes B**",
        ] {
            assert_eq!(
                parse_reply(Kind::Body, &['A', 'B'], reply).unwrap().1,
                "closes B"
            );
        }
        // No letter, a letter the question lacks, two letters, or another kind.
        for reply in [
            "r\ncloses",
            "r\ncloses C",
            "r\ncloses AB",
            "r\ncloses 1",
            "r\ncloses B or A",
            "r\ncloses option B",
        ] {
            assert!(
                parse_reply(Kind::Body, &['A', 'B'], reply).is_err(),
                "{}",
                reply
            );
        }
        assert!(parse_reply(Kind::Body, &[], "r\ncloses A").is_err());
        assert!(parse_reply(Kind::Plan, &['A'], "r\ncloses A").is_err());
    }

    #[test]
    fn the_three_prompts_differ_and_name_their_words() {
        let hashes: HashSet<String> = [Kind::Body, Kind::Plan, Kind::Set]
            .iter()
            .map(|k| k.prompt_hash())
            .collect();
        assert_eq!(hashes.len(), 3);
        assert!(Kind::Body.prompt().contains("inconsistent"));
        assert!(Kind::Body.prompt().contains("closes"));
        assert!(!Kind::Plan.prompt().contains("inconsistent"));
        for k in [Kind::Body, Kind::Plan, Kind::Set] {
            assert!(!k.prompt().contains("depends"));
        }
    }
}
