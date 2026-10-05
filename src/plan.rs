//! Planning a topic file: what the reader said about the neighbours of each
//! question not yet asked, which neighbour is on a line, and what is
//! missing. `adr plan`, `adr approve` and show all read it.
//!
//! "What does the agent do to plan a topic file?"
//! (46fc51ff-66b9-4841-a953-ae0d4c2fc424 A): write headings; get neighbours;
//! write lines; write bodies; get neighbours of the full questions; ask
//! approval. "Does a planning step run without the record of the step
//! before?" (d3be71e4-ce7b-43b0-86a8-e86868521f99 A): no; the records are
//! the reader file's rows and the lines the agent wrote.
//!
//! The records of a question are the rows of the reader file at two hashes.
//! The heading hash (`record::heading_hash`) holds the neighbours of the
//! heading: current decisions and terms. The body hash (`record::body_hash`)
//! holds the neighbours of the heading with its body: current decisions and
//! terms, and the other questions of the file, whose key is `text:` and
//! their own body hash. A line is a `depends:`, `supersedes:` or
//! `unrelated:` entry ("Does every neighbour get a line under its
//! heading?", c6451ecc-17a9-447e-a9c9-fe9b35ec789c A). The reader's word
//! never stops anything ("Does the reader's word stop a command?",
//! 902bd279-e6c1-4a21-9637-a3a43f360bae A).
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

/// Which step's rows a neighbour is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Neighbours of the heading.
    Heading,
    /// Neighbours of the heading with its body.
    Body,
}

/// One neighbour in the rows of a question, with what the agent wrote.
#[derive(Debug, Clone)]
pub struct Neighbour {
    pub kind: NKind,
    /// The reader file's neighbour column: an identifier, or `text:<sha>`.
    pub key: String,
    /// The decision's heading, the term's name, or the question's heading.
    pub name: String,
    pub word: String,
    pub reason: String,
    pub on_line: bool,
}

/// A question not yet asked, and its records.
#[derive(Debug, Clone)]
pub struct Question {
    pub idx: usize,
    pub heading: String,
    pub heading_hash: String,
    pub has_body: bool,
    /// The hash of the heading and body, when it has a body.
    pub body_hash: Option<String>,
    /// The heading step has its rows, or needs none.
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
    /// Some question has no rows at a hash: `adr plan`.
    Plan,
    /// Some neighbour is on no line: the agent writes the lines.
    WriteLines,
    /// Some headings have no body: the agent writes them (their headings).
    WriteBodies(Vec<String>),
    /// Every record is there: the owner approves.
    Ready,
}

/// Whether `sec` is a question not yet asked: no `asked` row names it, and
/// no row of a later state (answered, refused, superseded, withdrawn) does.
pub fn is_unasked(led: &Ledger, sec: &Section) -> bool {
    sec.id.as_deref().map_or(true, |id| led.latest_asked(id).is_none() && led.latest_state(id).is_none())
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
/// the identifier of a dropped question.
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
        return if i == self_idx { Err(RefError::NoName) } else { Ok(question_target(rf, i, led)) };
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
    if state::is_dropped(led, entry) {
        return Ok(Target::Dropped(entry.to_string()));
    }
    match cross_file_headings.get(entry).map(|v| v.as_slice()).unwrap_or(&[]) {
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
pub fn unrelated_entries(
    pieces: &[String],
    names_something: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < pieces.len() {
        let mut chosen = i + 1;
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
        !matches!(resolve_ref(s, rf, idx, terms_list, cross, led), Err(RefError::NoName))
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
) -> Vec<String> {
    let mut out = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if sec.unrelated.is_empty() || !is_unasked(led, sec) {
            continue;
        }
        for e in section_unrelated_entries(i, rf, terms_list, cross, led) {
            match resolve_ref(&e, rf, i, terms_list, cross, led) {
                Ok(_) => {}
                Err(RefError::NoName) => out.push(msg::unrelated_names_nothing(rel, &sec.heading_text, &e)),
                Err(RefError::Ambiguous(files)) => {
                    out.push(msg::unrelated_ambiguous(rel, &sec.heading_text, &e, &files))
                }
            }
        }
    }
    out
}

/// What every line of question `idx` names, from its `depends:`,
/// `supersedes:` and `unrelated:` lines.
fn line_targets(
    idx: usize,
    rf: &RecordFile,
    terms_list: &[Term],
    phrase_lookup: &HashMap<String, String>,
    cross: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> HashSet<Target> {
    let sec = &rf.sections[idx];
    let mut out = HashSet::new();
    for d in &sec.depends {
        if let Ok(Some(t)) = depends::resolve_dep(d, sec, rf, terms_list, phrase_lookup, cross, led) {
            out.insert(match t {
                DepTarget::Heading(i) => question_target(rf, i, led),
                DepTarget::Term(id) => Target::Term(id),
                DepTarget::Decision(id) => Target::Decision(id),
            });
        }
    }
    for e in depends::supersedes_entries(&sec.supersedes, cross, led) {
        if let Ok(id) = depends::resolve_supersedes(&e, cross, led) {
            out.insert(Target::Decision(id));
        }
    }
    for e in section_unrelated_entries(idx, rf, terms_list, cross, led) {
        if let Ok(t) = resolve_ref(&e, rf, idx, terms_list, cross, led) {
            out.insert(t);
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
    by_body: &HashMap<String, usize>,
    terms_list: &[Term],
    headings: &HashMap<String, String>,
    led: &Ledger,
) -> Option<(NKind, String, Target)> {
    if let Some(h) = key.strip_prefix("text:") {
        let i = *by_body.get(h)?;
        return Some((NKind::Question, rf.sections[i].heading_text.clone(), Target::Question(i)));
    }
    if let Some(t) = terms_list.iter().find(|t| t.id.as_deref().unwrap_or(&t.name) == key) {
        if t.retired {
            return None;
        }
        return Some((NKind::Term, t.name.clone(), Target::Term(key.to_string())));
    }
    if !state::is_current(led, key) {
        return None;
    }
    let name = headings.get(key).cloned().unwrap_or_else(|| key.to_string());
    Some((NKind::Decision, name, Target::Decision(key.to_string())))
}

/// The pool is empty: no current decision and no term that is not retired.
fn pool_is_empty(all: &[RecordFile], led: &Ledger, terms_list: &[Term]) -> bool {
    index::pool(all, led, terms_list).is_empty()
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
    let phrase_lookup = terms::phrase_lookup(terms_list);
    let cross = depends::cross_file_heading_index(all, led);
    let headings = decision_headings(all);
    let rows = reader::read_file(root).rows;
    let mut by_hash: HashMap<&str, Vec<&reader::Row>> = HashMap::new();
    for r in &rows {
        by_hash.entry(r.text_hash.as_str()).or_default().push(r);
    }
    let unasked: Vec<usize> = (0..rf.sections.len()).filter(|&i| is_unasked(led, &rf.sections[i])).collect();
    let mut by_body: HashMap<String, usize> = HashMap::new();
    for &i in &unasked {
        if !rf.sections[i].is_skeleton {
            by_body.entry(record::body_hash(rf, &rf.sections[i])).or_insert(i);
        }
    }
    let pool_empty = pool_is_empty(all, led, terms_list);
    let neighbours = |hash: &str, targets: &HashSet<Target>| -> Option<Vec<Neighbour>> {
        let rs = by_hash.get(hash)?;
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
            let Some((kind, name, target)) = describe(key, rf, &by_body, terms_list, &headings, led) else {
                continue;
            };
            out.push(Neighbour {
                kind,
                key: key.to_string(),
                name,
                word: r.word.clone(),
                reason: r.reason.clone(),
                on_line: targets.contains(&target),
            });
        }
        Some(out)
    };
    let mut questions = Vec::new();
    for &i in &unasked {
        let sec = &rf.sections[i];
        let targets = line_targets(i, rf, terms_list, &phrase_lookup, &cross, led);
        let heading_hash = record::heading_hash(sec);
        let h_rows = neighbours(&heading_hash, &targets);
        let heading_done = h_rows.is_some() || pool_empty;
        let has_body = !sec.is_skeleton;
        let body_hash = has_body.then(|| record::body_hash(rf, sec));
        let others = unasked.iter().filter(|&&j| j != i && !rf.sections[j].is_skeleton).count();
        let b_rows = body_hash.as_deref().and_then(|h| neighbours(h, &targets));
        let body_done = has_body && (b_rows.is_some() || (pool_empty && others == 0));
        questions.push(Question {
            idx: i,
            heading: sec.heading_text.clone(),
            heading_hash,
            has_body,
            body_hash,
            heading_done,
            heading_neighbours: h_rows.unwrap_or_default(),
            body_done,
            body_neighbours: b_rows.unwrap_or_default(),
        });
    }
    Planning { rel: rf.rel.clone(), questions }
}

impl Planning {
    /// Questions whose heading has no rows.
    pub fn missing_heading(&self) -> Vec<&Question> {
        self.questions.iter().filter(|q| !q.heading_done).collect()
    }

    /// Questions with a body whose body has no rows.
    pub fn missing_body(&self) -> Vec<&Question> {
        self.questions.iter().filter(|q| q.has_body && !q.body_done).collect()
    }

    /// Questions with no body.
    pub fn without_body(&self) -> Vec<&Question> {
        self.questions.iter().filter(|q| !q.has_body).collect()
    }

    /// Every neighbour of `step` that is on no line, with its question.
    pub fn lines_missing(&self, step: Step) -> Vec<(&Question, &Neighbour)> {
        let mut out = Vec::new();
        for q in &self.questions {
            let ns = match step {
                Step::Heading => &q.heading_neighbours,
                Step::Body => &q.body_neighbours,
            };
            out.extend(ns.iter().filter(|n| !n.on_line).map(|n| (q, n)));
        }
        out
    }

    /// The next move, in the order of the steps.
    pub fn next(&self) -> NextMove {
        if !self.missing_heading().is_empty() {
            return NextMove::Plan;
        }
        if !self.lines_missing(Step::Heading).is_empty() {
            return NextMove::WriteLines;
        }
        let bare = self.without_body();
        if !bare.is_empty() {
            return NextMove::WriteBodies(bare.iter().map(|q| q.heading.clone()).collect());
        }
        if !self.missing_body().is_empty() {
            return NextMove::Plan;
        }
        if !self.lines_missing(Step::Body).is_empty() {
            return NextMove::WriteLines;
        }
        NextMove::Ready
    }

    /// What approve refuses: a question with no rows at its heading hash or
    /// its body hash, and each neighbour on no line. A question with no
    /// body is refused for that elsewhere. Needs neither the model nor the
    /// reader.
    pub fn approve_refusals(&self) -> Vec<String> {
        let rel = &self.rel;
        let mut out = Vec::new();
        for q in &self.questions {
            if !q.heading_done {
                out.push(msg::plan_rows_missing(rel, &q.heading, "heading"));
            }
            if q.has_body && !q.body_done {
                out.push(msg::plan_rows_missing(rel, &q.heading, "heading with its body"));
            }
            for n in q.heading_neighbours.iter().chain(&q.body_neighbours).filter(|n| !n.on_line) {
                out.push(msg::plan_line_missing(rel, &q.heading, n.kind.label(), &n.name));
            }
        }
        out
    }
}

/// The text a heading is read as: the heading alone.
pub fn heading_text(sec: &Section) -> String {
    sec.heading_text.clone()
}

/// The reader's pairs for the heading of `sec`: each neighbour of the pool.
pub fn heading_pairs(sec: &Section, near: &[index::Neighbour]) -> Vec<Pair> {
    let new_text = heading_text(sec);
    near.iter()
        .map(|n| Pair {
            kind: Kind::Heading,
            new_text: new_text.clone(),
            neighbour: n.id.clone(),
            neighbour_text: n.text.clone(),
        })
        .collect()
}

/// The reader's pairs for the question at `idx` with its body: each
/// neighbour of the pool (`near`), then each of the other questions of the
/// file given in `own` as (index, text) nearest first.
pub fn body_pairs(rf: &RecordFile, idx: usize, near: &[index::Neighbour], own: &[usize]) -> Vec<Pair> {
    let new_text = record::body_text(rf, &rf.sections[idx]);
    let mut out: Vec<Pair> = near
        .iter()
        .map(|n| Pair {
            kind: Kind::Body,
            new_text: new_text.clone(),
            neighbour: n.id.clone(),
            neighbour_text: n.text.clone(),
        })
        .collect();
    for &j in own {
        let other = record::body_text(rf, &rf.sections[j]);
        out.push(Pair {
            kind: Kind::Plan,
            new_text: new_text.clone(),
            neighbour: reader::neighbour_key_of_text(&other),
            neighbour_text: other,
        });
    }
    out
}
