//! Planning a topic file: the neighbours of each question not yet asked,
//! what the reader said about those of its body, which neighbour is on a
//! `depends:` or `supersedes:` line, and what is missing. `adr plan`,
//! `adr approve` and show all read it.
//!
//! "What does the agent do to plan a topic file?"
//! (46fc51ff-66b9-4841-a953-ae0d4c2fc424 A): write headings; get neighbours;
//! write bodies; get neighbours of the full questions; ask
//! approval. "Does a planning step run without the record of the step
//! before?" (d3be71e4-ce7b-43b0-86a8-e86868521f99 A): no; the records are
//! the neighbours file, the reader file's rows and the lines the agent wrote.
//! "Does the agent write an unrelated line?"
//! (a065dced-a3b3-467c-b299-17bb70742046 A): no; a line is a `depends:` or
//! `supersedes:` entry, and a neighbour on neither needs none.
//!
//! The records of a question are two. The neighbours file holds, at the
//! heading hash (`record::heading_hash`), the current decisions and terms
//! brought for the heading, with no word. The reader file's rows at the
//! body hash (`record::body_hash`) hold the neighbours of the heading with
//! its body: current decisions and terms, among them each neighbour of the
//! heading still in the pool ("Does the body step read the neighbours
//! brought for the heading?", e1cbe055-5638-41ea-853e-13991b945ae1 A: yes),
//! and the other questions of the file, whose key is `text:` and their own
//! body hash. The body step is not done until each has its row. A
//! question whose body rows are on record has its heading step done, whether
//! or not the neighbours file holds its heading. The reader's word
//! never stops anything ("Does a reader's word stop a command?",
//! e78b8297-fd89-4e09-9c3e-9eeb0fd0f654 A).
//!
//! A neighbour that is no longer in the pool, a decision no longer current,
//! a retired term, a question whose body changed or left the file, needs no
//! line. A question has nothing to be near when the pool is empty (and, for
//! its body, no other question of the file is planned): its step needs no
//! rows.
use crate::depends::{self, DepTarget};
use crate::index;
use crate::ledger::Ledger;
use crate::msg;
use crate::reader::{self, Kind, Pair};
use crate::record::{self, RecordFile, Section};
use crate::state;
use crate::terms::{self, Term};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// What a neighbour is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NKind {
    Decision,
    Term,
    Question,
}

impl NKind {
    pub fn label(self) -> &'static str {
        match self {
            NKind::Decision => "decision",
            NKind::Term => "term",
            NKind::Question => "question",
        }
    }
}

/// Which step's record a neighbour is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Neighbours of the heading.
    Heading,
    /// Neighbours of the heading with its body.
    Body,
}

/// What kind of line the agent wrote a neighbour on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Depends,
    Supersedes,
}

impl LineKind {
    pub fn label(self) -> &'static str {
        match self {
            LineKind::Depends => "depends",
            LineKind::Supersedes => "supersedes",
        }
    }
}

/// One neighbour in the rows of a question, with what the agent wrote.
#[derive(Debug, Clone)]
pub struct Neighbour {
    pub kind: NKind,
    /// The decision's heading, the term's name, or the question's heading.
    pub name: String,
    /// The neighbour's key in the rows: the pool identifier, or `text:` and
    /// the body hash of a question of the file.
    pub key: String,
    /// The reader's word, `None` for a neighbour of a heading: it has none
    /// until the body step reads it.
    pub word: Option<String>,
    /// The reader's sentence of reason, `None` with no word.
    pub reason: Option<String>,
    /// The line the neighbour is on, `None` when it is on no line.
    pub line: Option<LineKind>,
}

/// A question not yet asked (or asked and waiting), and its records.
#[derive(Debug, Clone)]
pub struct Question {
    pub idx: usize,
    pub heading: String,
    /// The question is asked and waits for its answer.
    pub asked: bool,
    pub heading_hash: String,
    pub has_body: bool,
    /// The hash of the heading and body, when it has a body.
    pub body_hash: Option<String>,
    /// The heading step has its neighbours on record, or needs none.
    pub heading_done: bool,
    pub heading_neighbours: Vec<Neighbour>,
    /// The body step has its rows, or needs none (or the question has no
    /// body yet, and the step has not come).
    pub body_done: bool,
    pub body_neighbours: Vec<Neighbour>,
}

#[derive(Debug, Clone)]
pub struct Planning {
    pub rel: String,
    pub questions: Vec<Question>,
}

/// What the agent or the owner does next, in the order of the steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextMove {
    /// Some question has no record at a step: `adr plan`.
    Plan,
    /// Some headings have no body: the agent writes them (their headings).
    WriteBodies(Vec<String>),
    /// Every record is there: the owner approves.
    Ready,
}

/// What an entry of a line names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    Decision(String),
    Term(String),
    /// A question of the file, by its index in `rf.sections`.
    Question(usize),
    /// An identifier that has a `dropped` row.
    Dropped(String),
}

/// Why an `unrelated:` entry names nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefError {
    NoName,
    Ambiguous(Vec<String>),
}

/// What a line holds for question `i` of the file: a question of the plan,
/// or the decision it is once it is current.
fn question_target(rf: &RecordFile, i: usize, led: &Ledger) -> Target {
    match &rf.sections[i].id {
        Some(id) if state::is_current(led, id) => Target::Decision(id.clone()),
        _ => Target::Question(i),
    }
}

/// What an `unrelated:` entry names: a question of `rf` by heading or id
/// (never `self_idx`, the question the line is under), a current decision
/// by id or by heading, a term by id, name or alias (a retired one too: the line says it is
/// unrelated, and that stays true), or
/// the identifier of a dropped or superseded question.
pub fn resolve_ref(
    entry: &str,
    rf: &RecordFile,
    self_idx: usize,
    terms_list: &[Term],
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> Result<Target, RefError> {
    let entry = entry.trim();
    if let Some(i) = rf
        .sections
        .iter()
        .position(|s| s.id.as_deref() == Some(entry) || s.heading_text.trim() == entry)
    {
        return if i == self_idx {
            Err(RefError::NoName)
        } else {
            Ok(question_target(rf, i, led))
        };
    }
    if state::is_current(led, entry) {
        return Ok(Target::Decision(entry.to_string()));
    }
    let lower = entry.to_lowercase();
    for t in terms_list {
        let key = t.id.clone().unwrap_or_else(|| t.name.clone());
        if key == entry
            || t.name.trim().to_lowercase() == lower
            || t.aliases.iter().any(|a| a.trim().to_lowercase() == lower)
        {
            return Ok(Target::Term(key));
        }
    }
    // A superseded question's section is struck from every file (adr-code:
    // b5ca161f-c88c-4f02-a9d9-1d406cf3a252): the ledger alone holds it, as
    // it holds a dropped one.
    if state::is_dropped(led, entry) || state::is_superseded(led, entry) {
        return Ok(Target::Dropped(entry.to_string()));
    }
    match cross_file_headings
        .get(entry)
        .map(|v| v.as_slice())
        .unwrap_or(&[])
    {
        [] => Err(RefError::NoName),
        [(_, id)] => Ok(Target::Decision(id.clone())),
        many => {
            let mut files: Vec<String> = many.iter().map(|(rel, _)| rel.clone()).collect();
            files.sort();
            files.dedup();
            Err(RefError::Ambiguous(files))
        }
    }
}

/// The entries of an `unrelated:` line from the pieces a split at every
/// comma gave: a heading may hold a comma, so pieces are joined again, the
/// longest run first, while the joined text names something.
pub fn unrelated_entries(pieces: &[String], names_something: impl Fn(&str) -> bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < pieces.len() {
        let mut chosen = i + 1;
        // An identifier holds no comma: it is an entry of its own, and no
        // longer join is tried (a line approve wrote holds identifiers
        // alone, so it costs one look per entry, not one per pair).
        if record::is_uuid(pieces[i].trim()) {
            out.push(pieces[i].clone());
            i = chosen;
            continue;
        }
        for j in (i + 2..=pieces.len()).rev() {
            if names_something(&pieces[i..j].join(", ")) {
                chosen = j;
                break;
            }
        }
        out.push(pieces[i..chosen].join(", "));
        i = chosen;
    }
    out
}

/// The entries of `sec`'s `unrelated:` line, joined.
pub fn section_unrelated_entries(
    idx: usize,
    rf: &RecordFile,
    terms_list: &[Term],
    cross: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> Vec<String> {
    unrelated_entries(&rf.sections[idx].unrelated, |s| {
        !matches!(
            resolve_ref(s, rf, idx, terms_list, cross, led),
            Err(RefError::NoName)
        )
    })
}

/// Every refusal the `unrelated:` lines of `rf` earn: an entry that names
/// no current decision, no term, no other question of the file and no
/// dropped question, or whose heading is the heading of several current
/// decisions. Judged for each question not yet asked.
pub fn unrelated_refusals(
    rel: &str,
    rf: &RecordFile,
    terms_list: &[Term],
    cross: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> Vec<msg::Said> {
    let mut out = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if sec.unrelated.is_empty() || !crate::status::question_status(led, sec).is_unasked() {
            continue;
        }
        for e in section_unrelated_entries(i, rf, terms_list, cross, led) {
            match resolve_ref(&e, rf, i, terms_list, cross, led) {
                Ok(_) => {}
                Err(RefError::NoName) => {
                    out.push(msg::unrelated_names_nothing(rel, &sec.heading_text, &e))
                }
                Err(RefError::Ambiguous(files)) => {
                    out.push(msg::unrelated_ambiguous(rel, &sec.heading_text, &e, &files))
                }
            }
        }
    }
    out
}

/// What every line of question `idx` names, from its `depends:` and
/// `supersedes:` lines.
fn line_targets(
    idx: usize,
    rf: &RecordFile,
    terms_list: &[Term],
    phrase_lookup: &HashMap<String, String>,
    cross: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> HashMap<Target, LineKind> {
    let sec = &rf.sections[idx];
    let mut out = HashMap::new();
    for d in &sec.depends {
        if let Ok(Some(t)) = depends::resolve_dep(d, sec, rf, terms_list, phrase_lookup, led) {
            out.entry(match t {
                DepTarget::Heading(i) => question_target(rf, i, led),
                DepTarget::Term(id) => Target::Term(id),
                DepTarget::Decision(id) => Target::Decision(id),
                DepTarget::Struck(id) => Target::Dropped(id),
            })
            .or_insert(LineKind::Depends);
        }
    }
    for e in depends::supersedes_entries(&sec.supersedes, cross, led) {
        if let Ok(id) = depends::resolve_supersedes(&e, cross, led) {
            out.entry(Target::Decision(id))
                .or_insert(LineKind::Supersedes);
        }
    }
    out
}

/// The heading of every question, by id, of every topic file.
fn decision_headings(all: &[RecordFile]) -> HashMap<String, String> {
    crate::view::heading_texts(all)
}

/// What a neighbour key names now: its kind, its name, and the target a
/// line would hold for it. `None` when it is no longer in the pool, so it
/// needs no line.
fn describe(
    key: &str,
    rf: &RecordFile,
    by_body: &dyn Fn(&str) -> Option<usize>,
    terms_list: &[Term],
    headings: &HashMap<String, String>,
    led: &Ledger,
) -> Option<(NKind, String, Target)> {
    if let Some(h) = key.strip_prefix("text:") {
        let i = by_body(h)?;
        return Some((
            NKind::Question,
            rf.sections[i].heading_text.clone(),
            Target::Question(i),
        ));
    }
    if let Some(t) = terms_list
        .iter()
        .find(|t| t.id.as_deref().unwrap_or(&t.name) == key)
    {
        if t.retired {
            return None;
        }
        return Some((NKind::Term, t.name.clone(), Target::Term(key.to_string())));
    }
    if !state::is_current(led, key) {
        return None;
    }
    let name = headings
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string());
    Some((NKind::Decision, name, Target::Decision(key.to_string())))
}

/// Reads the planning state of the questions of `rf` that are not yet
/// asked. `all` is every topic file, `terms_list` the terms, `rows` the
/// reader file's rows.
pub fn load(
    root: &Path,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[Term],
    led: &Ledger,
) -> Planning {
    load_with(
        rf,
        terms_list,
        led,
        &Shared::new(root, all, terms_list, led),
    )
}

/// `load` with what every file of a run shares already read (`Shared`):
/// what a command that plans or judges many files calls once per file.
pub fn load_with(rf: &RecordFile, terms_list: &[Term], led: &Ledger, shared: &Shared) -> Planning {
    load_inner(rf, terms_list, led, shared, false)
}

/// The planning state shown under `show --neighbours`: `load_with`, the
/// questions that wait for the owner's answer included, with what the run
/// shares already read (`Shared`).
pub fn load_shown_with(
    rf: &RecordFile,
    terms_list: &[Term],
    led: &Ledger,
    shared: &Shared,
) -> Planning {
    load_inner(rf, terms_list, led, shared, true)
}

fn is_waiting(led: &Ledger, sec: &Section) -> bool {
    sec.id
        .as_deref()
        .and_then(|id| led.latest_state(id))
        .is_some_and(|r| r.event == "asked")
}

/// What the planning of every topic file of a run reads of the whole
/// repository, read once: the term phrases, the decision headings of every
/// topic file, the neighbours on record for each heading, the reader's rows
/// by the hash of the text they read, and
/// whether the pool of neighbours is empty.
pub struct Shared {
    pub(crate) phrase_lookup: HashMap<String, String>,
    pub(crate) cross: HashMap<String, Vec<(String, String)>>,
    pub(crate) headings: HashMap<String, String>,
    rows: std::sync::Arc<Vec<reader::Row>>,
    /// The neighbour identifiers on record for each heading hash.
    heading_ids: HashMap<String, Vec<String>>,
    /// Each text hash's rows written with the prompt and model that would
    /// read the pair now, by index into `rows`.
    by_hash: HashMap<String, Vec<usize>>,
    /// The kinds read at a hash by a row of another prompt or model.
    stale: HashMap<Kind, HashSet<String>>,
    current: HashMap<Kind, String>,
    /// The identifiers of the pool (`index::pool`): every current decision
    /// and every term that is not retired. Empty when there is nothing to be
    /// near.
    pool: HashSet<String>,
}

impl Shared {
    pub fn new(root: &Path, all: &[RecordFile], terms_list: &[Term], led: &Ledger) -> Shared {
        Shared::new_via(
            root,
            all,
            terms_list,
            led,
            &crate::repo::Source::WorkingTree,
        )
    }

    /// `new` with the reader's rows as `source` holds them.
    pub fn new_via(
        root: &Path,
        all: &[RecordFile],
        terms_list: &[Term],
        led: &Ledger,
        source: &crate::repo::Source,
    ) -> Shared {
        let rows = reader::rows_via(root, source);
        let model = reader::model_of(root);
        // A row counts only when it was written with the prompt and the
        // model that would read its pair now; a row from another prompt or
        // model is kept apart, and its kind at that hash is read again.
        let mut by_hash: HashMap<String, Vec<usize>> = HashMap::new();
        let mut stale: HashMap<Kind, HashSet<String>> = HashMap::new();
        let current: HashMap<Kind, String> = [Kind::Body, Kind::Plan]
            .into_iter()
            .map(|k| (k, k.prompt_hash()))
            .collect();
        for (i, r) in rows.iter().enumerate() {
            let kind = row_kind(&r.neighbour);
            if r.model == model && current[&kind] == r.prompt_hash {
                by_hash.entry(r.text_hash.clone()).or_default().push(i);
            } else {
                stale.entry(kind).or_default().insert(r.text_hash.clone());
            }
        }
        Shared {
            phrase_lookup: terms::phrase_lookup(terms_list),
            cross: depends::cross_file_heading_index(all, led),
            headings: decision_headings(all),
            rows,
            heading_ids: crate::neighbours::read(root),
            by_hash,
            stale,
            current,
            pool: index::pool(all, led, terms_list)
                .into_iter()
                .map(|r| r.id)
                .collect(),
        }
    }
}

/// One word the reader gave on a neighbour of a question's current text,
/// as `show` prints it.
#[derive(Debug, Clone)]
pub struct ReaderWord {
    /// What the neighbour is called: a decision's heading, a term's name or
    /// a sibling question's heading.
    pub name: String,
    pub word: String,
    pub reason: String,
    /// The question's `supersedes:` line names the neighbour.
    pub on_supersedes: bool,
}

/// The sections of one file with a body, by the hash of the body, in file
/// order. Built once per file for `Shared::reader_words`.
pub struct BodyIndex(HashMap<String, Vec<usize>>);

impl BodyIndex {
    pub fn of(rf: &RecordFile) -> BodyIndex {
        let mut m: HashMap<String, Vec<usize>> = HashMap::new();
        for (j, sec) in rf
            .sections
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.is_skeleton)
        {
            m.entry(record::body_hash(rf, sec)).or_default().push(j);
        }
        BodyIndex(m)
    }
}

impl Shared {
    /// Every word the reader gave on the question at `idx` of `rf`, whatever
    /// its status: its rows are those at the hash of its body, written with
    /// the prompt and model that would read the pair now, the latest row for
    /// each neighbour; a question with no body has none. A neighbour that
    /// names nothing now is left out.
    pub fn reader_words(
        &self,
        rf: &RecordFile,
        idx: usize,
        terms_list: &[Term],
        led: &Ledger,
        by_body: &BodyIndex,
    ) -> Vec<ReaderWord> {
        let sec = &rf.sections[idx];
        if sec.is_skeleton {
            return Vec::new();
        }
        let hash = record::body_hash(rf, sec);
        let Some(at) = self.by_hash.get(&hash) else {
            return Vec::new();
        };
        let mut order: Vec<&str> = Vec::new();
        let mut last: HashMap<&str, &reader::Row> = HashMap::new();
        for &i in at {
            let r = &self.rows[i];
            if last.insert(r.neighbour.as_str(), r).is_none() {
                order.push(r.neighbour.as_str());
            }
        }
        let lookup = |h: &str| {
            by_body
                .0
                .get(h)
                .and_then(|js| js.iter().copied().find(|&j| j != idx))
        };
        let targets = line_targets(idx, rf, terms_list, &self.phrase_lookup, &self.cross, led);
        let mut out = Vec::new();
        for key in order {
            let r = last[key];
            let Some((_, name, target)) =
                describe(key, rf, &lookup, terms_list, &self.headings, led)
            else {
                continue;
            };
            out.push(ReaderWord {
                name,
                word: r.word.clone(),
                reason: r.reason.clone(),
                on_supersedes: matches!(targets.get(&target), Some(LineKind::Supersedes)),
            });
        }
        out
    }
}

fn load_inner(
    rf: &RecordFile,
    terms_list: &[Term],
    led: &Ledger,
    shared: &Shared,
    with_asked: bool,
) -> Planning {
    let Shared {
        phrase_lookup,
        cross,
        headings,
        rows,
        heading_ids,
        by_hash,
        stale,
        current,
        pool,
    } = shared;
    let pool_empty = pool.is_empty();
    // Borrowed from the shared index: no copy per lookup.
    let rows_at = |hash: &str| by_hash.get(hash).map(|is| is.iter().map(|&i| &rows[i]));
    // Rows of a kind at a hash, none of them current.
    let is_stale = |hash: &str, kind: Kind| -> bool {
        stale.get(&kind).is_some_and(|hs| hs.contains(hash))
            && !rows_at(hash).is_some_and(|mut rs| rs.any(|r| current[&kind] == r.prompt_hash))
    };
    let build = |with_asked: bool| -> Planning {
        let unasked: Vec<usize> = (0..rf.sections.len())
            .filter(|&i| {
                crate::status::question_status(led, &rf.sections[i]).is_unasked()
                    || (with_asked && is_waiting(led, &rf.sections[i]))
            })
            .collect();
        let mut by_body: HashMap<String, usize> = HashMap::new();
        for &i in &unasked {
            if !rf.sections[i].is_skeleton {
                by_body
                    .entry(record::body_hash(rf, &rf.sections[i]))
                    .or_insert(i);
            }
        }
        let neighbours =
            |hash: &str, targets: &HashMap<Target, LineKind>| -> Option<Vec<Neighbour>> {
                let rs = rows_at(hash)?;
                // The latest row for a neighbour is the one that counts.
                let mut order: Vec<&str> = Vec::new();
                let mut last: HashMap<&str, &reader::Row> = HashMap::new();
                for r in rs {
                    if last.insert(r.neighbour.as_str(), r).is_none() {
                        order.push(r.neighbour.as_str());
                    }
                }
                let mut out = Vec::new();
                for key in order {
                    let r = last[key];
                    let Some((kind, name, target)) = describe(
                        key,
                        rf,
                        &|h| by_body.get(h).copied(),
                        terms_list,
                        headings,
                        led,
                    ) else {
                        continue;
                    };
                    out.push(Neighbour {
                        kind,
                        name,
                        key: key.to_string(),
                        word: Some(r.word.clone()),
                        reason: Some(r.reason.clone()),
                        line: targets.get(&target).copied(),
                    });
                }
                Some(out)
            };
        // The neighbours on record for a heading hash: no word, no reason.
        let heading_neighbours =
            |hash: &str, targets: &HashMap<Target, LineKind>| -> Option<Vec<Neighbour>> {
                let ids = heading_ids.get(hash)?;
                let mut out = Vec::new();
                for key in ids {
                    let Some((kind, name, target)) = describe(
                        key,
                        rf,
                        &|h| by_body.get(h).copied(),
                        terms_list,
                        headings,
                        led,
                    ) else {
                        continue;
                    };
                    out.push(Neighbour {
                        kind,
                        name,
                        key: key.clone(),
                        word: None,
                        reason: None,
                        line: targets.get(&target).copied(),
                    });
                }
                Some(out)
            };
        let mut questions = Vec::new();
        for &i in &unasked {
            let sec = &rf.sections[i];
            let targets = line_targets(i, rf, terms_list, phrase_lookup, cross, led);
            let heading_hash = record::heading_hash(sec);
            let has_body = !sec.is_skeleton;
            let body_hash = has_body.then(|| record::body_hash(rf, sec));
            let others = unasked
                .iter()
                .filter(|&&j| j != i && !rf.sections[j].is_skeleton)
                .count();
            let b_rows = body_hash.as_deref().and_then(|h| neighbours(h, &targets));
            let body_stale = body_hash.as_deref().is_some_and(|h| {
                (!pool_empty && is_stale(h, Kind::Body)) || (others > 0 && is_stale(h, Kind::Plan))
            });
            let h_rows = heading_neighbours(&heading_hash, &targets);
            // Each neighbour on record for the heading and still in the pool
            // has its row at the body hash: the pool `adr plan` reads the
            // pairs from, so a neighbour it cannot read is never waited on.
            let heading_read = h_rows
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter(|h| pool.contains(&h.key))
                .all(|h| {
                    b_rows
                        .as_deref()
                        .unwrap_or_default()
                        .iter()
                        .any(|b| b.key == h.key)
                });
            let body_done = has_body
                && !body_stale
                && heading_read
                && (b_rows.is_some() || (pool_empty && others == 0));
            // A question whose body step is done has its heading step done, with
            // no neighbours of its own when none is on record.
            let heading_done = h_rows.is_some() || pool_empty || body_done;
            questions.push(Question {
                idx: i,
                heading: sec.heading_text.clone(),
                asked: !crate::status::question_status(led, sec).is_unasked(),
                heading_hash,
                has_body,
                body_hash,
                heading_done,
                heading_neighbours: h_rows.unwrap_or_default(),
                body_done,
                body_neighbours: b_rows.unwrap_or_default(),
            });
        }
        Planning {
            rel: rf.rel.clone(),
            questions,
        }
    };
    build(with_asked)
}

impl Planning {
    /// Questions whose heading has no neighbours on record.
    pub fn missing_heading(&self) -> Vec<&Question> {
        self.questions.iter().filter(|q| !q.heading_done).collect()
    }

    /// Questions with a body whose body has no rows.
    pub fn missing_body(&self) -> Vec<&Question> {
        self.questions
            .iter()
            .filter(|q| q.has_body && !q.body_done)
            .collect()
    }

    /// Questions with no body.
    pub fn without_body(&self) -> Vec<&Question> {
        self.questions.iter().filter(|q| !q.has_body).collect()
    }

    /// The next move, in the order of the steps.
    pub fn next(&self) -> NextMove {
        if !self.missing_heading().is_empty() {
            return NextMove::Plan;
        }
        let bare = self.without_body();
        if !bare.is_empty() {
            return NextMove::WriteBodies(bare.iter().map(|q| q.heading.clone()).collect());
        }
        if !self.missing_body().is_empty() {
            return NextMove::Plan;
        }
        NextMove::Ready
    }

    /// The records missing at the step `adr plan` takes next: every heading's
    /// neighbours while one heading lacks them, else every body's rows. A
    /// body's rows wait on the heading step, as `next` reads the steps in
    /// order.
    pub fn rows_missing_at_step(&self) -> Vec<msg::Said> {
        if !self.missing_heading().is_empty() {
            return self
                .questions
                .iter()
                .filter(|q| !q.heading_done)
                .map(|q| msg::plan_neighbours_missing(&self.rel, &q.heading))
                .collect();
        }
        self.questions
            .iter()
            .filter(|q| q.has_body && !q.body_done)
            .map(|q| msg::plan_rows_missing(&self.rel, &q.heading))
            .collect()
    }
}

/// The lines `show --neighbours` prints under an open question: each
/// neighbour on record at the heading step, then at the body step, each
/// with the agent's line and, at the body step, the reader's word and reason; and,
/// for a question not yet asked that has no records, one line saying so. Nothing for an
/// asked question with no records (planned before the records existed).
pub fn neighbour_lines(q: &Question) -> Vec<String> {
    let mut out = neighbour_lines_at(q, Step::Heading);
    out.extend(neighbour_lines_at(q, Step::Body));
    if out.is_empty() && !q.asked {
        out.push(msg::show_no_records());
    }
    out
}

/// The neighbours of `q` on record at `step`, one line each, by the one
/// renderer of a neighbour (`msg::neighbour`): what `adr plan` prints for
/// the step it read, and `adr show --neighbours` for both steps.
pub fn neighbour_lines_at(q: &Question, step: Step) -> Vec<String> {
    let (label, ns) = match step {
        Step::Heading => ("heading", &q.heading_neighbours),
        Step::Body => ("body", &q.body_neighbours),
    };
    ns.iter()
        .map(|n| {
            msg::neighbour(
                n.kind.label(),
                &n.name,
                label,
                n.word.as_deref(),
                n.line.map(LineKind::label),
                &reader::flatten(n.reason.as_deref().unwrap_or("")),
            )
        })
        .collect()
}

/// The text a heading is read as: the heading alone.
pub fn heading_text(sec: &Section) -> String {
    sec.heading_text.clone()
}

/// The kind of prompt a row at a body hash was read with: `Plan` for
/// another question of the file, `Body` for the rest.
fn row_kind(neighbour: &str) -> Kind {
    if neighbour.starts_with("text:") {
        Kind::Plan
    } else {
        Kind::Body
    }
}

/// The reader's pairs for the question at `idx` with its body: each
/// neighbour of the pool (`near`, identifier and text, the body's nearest
/// then the heading's), then each of the other questions of the file given
/// in `own` nearest first. A pair carries the letters of the question's
/// options.
pub fn body_pairs(
    rf: &RecordFile,
    idx: usize,
    near: &[(String, String)],
    own: &[usize],
) -> Vec<Pair> {
    let new_text = record::body_text(rf, &rf.sections[idx]);
    let letters: Vec<char> = rf.sections[idx].options.iter().map(|o| o.letter).collect();
    let mut out: Vec<Pair> = near
        .iter()
        .map(|(id, text)| Pair {
            kind: Kind::Body,
            new_text: new_text.clone(),
            neighbour: id.clone(),
            neighbour_text: text.clone(),
            letters: letters.clone(),
        })
        .collect();
    for &j in own {
        let other = record::body_text(rf, &rf.sections[j]);
        out.push(Pair {
            kind: Kind::Plan,
            new_text: new_text.clone(),
            neighbour: reader::neighbour_key_of_text(&other),
            neighbour_text: other,
            letters: Vec::new(),
        });
    }
    out
}
