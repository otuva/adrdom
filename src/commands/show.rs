//! `adr show`: what the records hold and every move. With no file, every
//! current decision and every move (per "What does show print with no file
//! named?", c84a4bd9 A). With a file approve takes -- a topic file, the
//! terms file or a blame file -- every question, entry or row with its
//! status, the lines it carries and the body of an asked question (per
//! "What does show print with a file named?", 6addc2f4 A), with what the
//! file holds beyond its latest approval marked (per "Does show print what a
//! file holds beyond its latest approval?", bceceda8 A), then the moves of
//! that file. What it computes from the records comes under a flag (per
//! "Does a command print under a flag what it computes from the records?",
//! 36dfbeaa A): `--neighbours` and `--stood-on`. Every line goes through
//! `out` (per "What does a command that writes nothing print?", 848e1b76 A).
use crate::approval::{self, Change, Side};
use crate::blame;
use crate::ledger::{self, Ledger};
use crate::moves::{self, Move};
use crate::msg::{self, Actor};
use crate::out::{self, Line, Status};
use crate::plan;
use crate::record::{self, RecordFile, Section};
use crate::repo;
use crate::rules::{self, Finding};
use crate::state;
use crate::status::{self, FileStatus, QuestionStatus};
use crate::terms::{self, Term, TermsFile};
use crate::view;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// The computed views `show` prints under a flag.
#[derive(Debug, Clone, Copy, Default)]
pub struct Views {
    /// Each neighbour of a question not yet answered, with the reader's word
    /// and the agent's line.
    pub neighbours: bool,
    /// What stands on each question, and what replaced a superseded one.
    pub stood_on: bool,
}

/// Everything `show` reads, loaded once.
struct Records {
    root: PathBuf,
    led: Ledger,
    terms_file: Option<TermsFile>,
    terms_list: Vec<Term>,
    topic_files: Vec<RecordFile>,
    unreadable_topic_files: Vec<(String, String)>,
    blame_files: Vec<blame::BlameFile>,
    unreadable_blame_files: Vec<(String, String)>,
    tests: HashMap<String, Vec<String>>,
    /// Every question's heading and every term's name by its identifier:
    /// what turns an identifier on a line back into words.
    names: HashMap<String, String>,
}

impl Records {
    fn load(root: PathBuf) -> Records {
        let led = ledger::read(&root.join(repo::LEDGER_PATH));
        let terms_file = state::load_terms(&root);
        let terms_list: Vec<Term> = terms_file.as_ref().map(|tf| tf.terms.clone()).unwrap_or_default();
        let (topic_files, unreadable) = record::load_all_via(&root, &repo::Source::WorkingTree);
        let unreadable_topic_files =
            unreadable.into_iter().map(|(f, e)| (state::rel_path(&root, &f), e.to_string())).collect();
        let (blame_files, unreadable_blame_files) = blame::load_all_with_errors(&root);
        let tests = crate::commands::check::test_index(&root, &repo::Source::WorkingTree);
        let mut names: HashMap<String, String> =
            terms_list.iter().filter_map(|t| Some((t.id.clone()?, t.name.clone()))).collect();
        // A question no topic file holds any more (struck, or its file
        // removed) by the heading its latest `asked` row carries; one a
        // topic file holds by its heading there.
        for row in led.rows.iter().filter(|r| r.event == "asked" && !r.detail.is_empty()) {
            names.insert(row.id.clone(), row.detail.clone());
        }
        names.extend(view::heading_texts(&topic_files));
        Records {
            root,
            led,
            terms_file,
            terms_list,
            topic_files,
            unreadable_topic_files,
            blame_files,
            unreadable_blame_files,
            tests,
            names,
        }
    }

    fn present_ids(&self) -> HashSet<String> {
        self.topic_files.iter().flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.clone())).collect()
    }

    fn removed_topic_files(&self) -> Vec<String> {
        rules::removal_awaiting(&self.led, &|rel| self.root.join(rel).exists())
    }
}

pub fn run(record_arg: Option<PathBuf>, all: bool, views: Views) -> i32 {
    let Some(root) = repo::find_root() else {
        out::eprint(&[Line::Finding(Finding::new("show", msg::not_a_repo("show", "", Actor::Agent)))]);
        return 1;
    };
    let r = Records::load(root);
    let lines = match &record_arg {
        Some(p) => match file_form(&r, &state::rel_path(&r.root, p), views) {
            Ok(lines) => lines,
            Err(refusal) => {
                out::eprint(&[Line::Finding(Finding::new("show", refusal))]);
                return 1;
            }
        },
        None if all => whole(&r, views),
        None => summary(&r),
    };
    out::print(&lines);
    0
}

// --- the listing with no file named ------------------------------------------

/// Every current decision as its heading and the option picked, under its
/// topic file, and every move: of each topic file, each removed topic file,
/// the terms file and each blame file.
fn summary(r: &Records) -> Vec<Line> {
    let mut lines = Vec::new();
    for (rel, e) in &r.unreadable_topic_files {
        lines.push(Line::Move(Finding::new("parse", msg::topic_file_unreadable(rel, e)).mv()));
    }
    for rf in &r.topic_files {
        lines.push(Line::Record { depth: 0, text: msg::show_topic(rf.topic.as_deref().unwrap_or(""), &rf.rel) });
        for sec in rf.sections.iter().filter(|s| status::question_status(&r.led, s) == QuestionStatus::Answered) {
            lines.push(Line::Record { depth: 1, text: sec.heading_text.clone() });
            if let Some(s) = sec.chosen_sentence() {
                lines.push(Line::Record { depth: 2, text: s.to_string() });
            }
        }
        lines.extend(topic_file_moves(r, rf).into_iter().map(Line::Move));
    }
    for rel in r.removed_topic_files() {
        lines.push(file_status(&rel, FileStatus::Removed));
        lines.extend(removed_moves(r, &rel).into_iter().map(Line::Move));
    }
    if let Some(tf) = &r.terms_file {
        let (st, moves) = terms_moves(r, tf);
        if !moves.is_empty() {
            lines.push(file_status(repo::TERMS_PATH, st));
            lines.extend(moves.into_iter().map(Line::Move));
        }
    }
    for bf in &r.blame_files {
        let (st, moves) = blame_moves(r, bf);
        if !moves.is_empty() {
            lines.push(file_status(&bf.rel, st));
            lines.extend(moves.into_iter().map(Line::Move));
        }
    }
    for (rel, _) in &r.unreadable_blame_files {
        lines.push(Line::Move(Finding::new("blame", msg::blame_file_unreadable(rel)).mv()));
    }
    lines
}

// --- the file form -------------------------------------------------------------

/// Every file approve takes, each in the file form.
fn whole(r: &Records, views: Views) -> Vec<Line> {
    let mut lines = Vec::new();
    for (rel, e) in &r.unreadable_topic_files {
        lines.push(Line::Move(Finding::new("parse", msg::topic_file_unreadable(rel, e)).mv()));
    }
    for rf in &r.topic_files {
        lines.extend(topic_file(r, rf, views));
    }
    for rel in r.removed_topic_files() {
        lines.extend(removed_topic_file(r, &rel));
    }
    if let Some(tf) = &r.terms_file {
        lines.extend(terms_file(r, tf));
    }
    for bf in &r.blame_files {
        lines.extend(blame_file(r, bf));
    }
    for (rel, _) in &r.unreadable_blame_files {
        lines.push(Line::Move(Finding::new("blame", msg::blame_file_unreadable(rel)).mv()));
    }
    lines
}

/// The file form of `rel`, or the refusal when it is not a file approve
/// takes.
fn file_form(r: &Records, rel: &str, views: Views) -> Result<Vec<Line>, String> {
    if rel == repo::TERMS_PATH {
        return match &r.terms_file {
            Some(tf) => Ok(terms_file(r, tf)),
            None => Err(msg::read_failed(rel, "no such file", msg::Next::agent("names a file that exists"))),
        };
    }
    if let Some(rf) = r.topic_files.iter().find(|rf| rf.rel == rel) {
        return Ok(topic_file(r, rf, views));
    }
    if let Some(bf) = r.blame_files.iter().find(|bf| bf.rel == rel) {
        return Ok(blame_file(r, bf));
    }
    if let Some((_, e)) = r.unreadable_topic_files.iter().find(|(f, _)| f == rel) {
        return Err(msg::read_failed(rel, e, msg::Next::agent("fixes the file so it can be read as text")));
    }
    if r.unreadable_blame_files.iter().any(|(f, _)| f == rel) {
        return Err(msg::blame_file_unreadable(rel));
    }
    if r.removed_topic_files().iter().any(|f| f == rel) {
        return Ok(removed_topic_file(r, rel));
    }
    Err(msg::show_not_a_file_approve_takes(rel))
}

/// A topic file as the status line names it: its topic and its path.
fn topic_subject(rf: &RecordFile) -> String {
    match rf.topic.as_deref() {
        Some(t) if !t.is_empty() => format!("{} ({})", t, rf.rel),
        _ => rf.rel.clone(),
    }
}

fn file_status(subject: &str, st: FileStatus) -> Line {
    Line::Status { subject: subject.to_string(), status: Status::File(st) }
}

/// The lines of a change, marked, each identifier turned back into words.
/// The heading line is left out, the status line above names it, unless
/// the heading itself changed.
fn change_lines(change: &Change, names: &HashMap<String, String>) -> Vec<Line> {
    let is_heading = |t: &str| t.starts_with("### ") || t.starts_with("## ");
    let heading_changed = [Side::Removed, Side::Added]
        .iter()
        .all(|side| change.lines.iter().any(|l| l.side == *side && is_heading(&l.text)));
    change
        .lines
        .iter()
        .filter(|l| heading_changed || !is_heading(&l.text))
        .map(|l| Line::Record { depth: 1, text: named(&l.marked(), names) })
        .collect()
}

/// A line the records hold with every identifier on it turned back into
/// the heading or the name it stands for (per "Does a reference in a topic
/// file carry the identifier of what it names?": show turns it back into
/// the name). An identifier no question or term holds stays as it is.
fn named(line: &str, names: &HashMap<String, String>) -> String {
    const LEN: usize = 36;
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < line.len() {
        if let Some(cand) = line.get(i..i + LEN) {
            let bounded = !line[..i].ends_with(|c: char| c.is_ascii_hexdigit())
                && !line[i + LEN..].starts_with(|c: char| c.is_ascii_hexdigit());
            if bounded && record::is_uuid(cand) {
                if let Some(name) = names.get(cand) {
                    out.push_str(name);
                    i += LEN;
                    continue;
                }
            }
        }
        let c = line[i..].chars().next().expect("i is on a char boundary");
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// The reference lines a question carries (`depends:`, `supersedes:`,
/// `unrelated:`), and its body, each as the file holds it.
fn reference_and_body(lines: &[String]) -> (Vec<String>, Vec<String>) {
    let is_reference =
        |l: &&String| ["depends:", "supersedes:", "unrelated:"].iter().any(|p| l.starts_with(p));
    let rest = lines.iter().skip(1);
    (rest.clone().filter(is_reference).cloned().collect(), rest.filter(|l| !is_reference(l)).cloned().collect())
}

/// One topic file: its status, its prose when beyond its latest approval,
/// every question, every question of the approved plan struck from it, then
/// its moves.
fn topic_file(r: &Records, rf: &RecordFile, views: Views) -> Vec<Line> {
    let led = &r.led;
    let mut lines = vec![file_status(&topic_subject(rf), status::topic_file_status(led, rf))];
    let changes = approval::topic_file(&r.root, rf, led);
    if let Some(prose) = &changes.prose {
        lines.push(file_status("prose", prose.status));
        lines.extend(change_lines(prose, &r.names));
    }
    let shown = (views.neighbours).then(|| plan::load_both(&r.root, rf, &r.topic_files, &r.terms_list, led).1);
    for (i, sec) in rf.sections.iter().enumerate() {
        let st = status::question_status(led, sec);
        let version = (st == QuestionStatus::Asked && view::asked_text_stands(led, rf, sec))
            .then(|| record::version_of(&rf.lines, sec));
        lines.push(Line::Status {
            subject: sec.heading_text.clone(),
            status: Status::Question { status: st, letter: sec.answer_letter(), version },
        });
        match changes.questions.get(i).and_then(Option::as_ref) {
            Some(change) => lines.extend(change_lines(change, &r.names)),
            None => {
                let (refs, body) = reference_and_body(&approval::question_lines(rf, sec));
                let body = if st == QuestionStatus::Asked { body } else { Vec::new() };
                lines.extend(refs.iter().chain(&body).map(|l| Line::Record { depth: 1, text: named(l, &r.names) }));
            }
        }
        if views.neighbours && matches!(st, QuestionStatus::NoBody | QuestionStatus::NotAsked | QuestionStatus::Asked) {
            if let Some(q) = shown.as_ref().and_then(|p| p.questions.iter().find(|q| q.heading == sec.heading_text)) {
                lines.extend(
                    plan::neighbour_lines(&rf.rel, q)
                        .into_iter()
                        .map(|l| Line::Record { depth: 1, text: l.trim_start().to_string() }),
                );
            }
        }
        if views.stood_on {
            lines.extend(stood_on_lines(r, rf, sec, st));
        }
    }
    for (heading, change) in &changes.struck {
        lines.push(file_status(heading, change.status));
        lines.extend(change_lines(change, &r.names));
    }
    lines.extend(topic_file_moves(r, rf).into_iter().map(Line::Move));
    lines
}

/// What stands on `sec`, and for a superseded question what replaced it.
fn stood_on_lines(r: &Records, rf: &RecordFile, sec: &Section, st: QuestionStatus) -> Vec<Line> {
    let mut lines = Vec::new();
    let dependents = stood_on_by(rf, sec, &r.topic_files, &r.led);
    if !dependents.is_empty() {
        lines.push(Line::Record { depth: 1, text: format!("stood on by: {}", dependents.join(", ")) });
    }
    // The ledger's `superseded` row holds the id of the question that
    // replaced it.
    if st == QuestionStatus::Superseded {
        let by = sec.id.as_deref().and_then(|id| r.led.latest_state(id)).map(|row| row.detail.trim().to_string());
        if let Some(by) = by.filter(|b| !b.is_empty()) {
            let name = r.names.get(&by).cloned().unwrap_or(by);
            lines.push(Line::Record { depth: 1, text: format!("replaced by: {}", name) });
        }
    }
    lines
}

/// The questions whose depends line names `sec`, in the order of the topic
/// files, each by its heading and, for a question of another file, the
/// path of that file. A refused, superseded or withdrawn question is left
/// out: its depends line is a record of what it stood on, nothing rests on
/// `sec` through it any more. An entry written as a heading text names
/// `sec` in its own file, and in another file only while `sec` is a
/// current decision (per "What does a depends line name?").
fn stood_on_by(rf: &RecordFile, sec: &Section, all: &[RecordFile], led: &Ledger) -> Vec<String> {
    let heading = sec.heading_text.trim();
    let current = sec.id.as_deref().is_some_and(|id| state::is_current(led, id));
    let mut out = Vec::new();
    for other in all {
        let same_file = other.rel == rf.rel;
        for dependent in &other.sections {
            if (same_file && dependent.heading_text.trim() == heading) || state::is_dead(led, dependent) {
                continue;
            }
            let names_it = dependent.depends.iter().any(|d| match d {
                record::Dep::Id { id, .. } => sec.id.as_deref() == Some(id.as_str()),
                record::Dep::Text { text, .. } => text.trim() == heading && (same_file || current),
            });
            if names_it {
                out.push(if same_file {
                    dependent.heading_text.clone()
                } else {
                    msg::show_question_of(&dependent.heading_text, &other.rel)
                });
            }
        }
    }
    out
}

/// A topic file gone from disk whose removal the owner has not approved:
/// its status, each question of its approved plan, struck, and its move.
fn removed_topic_file(r: &Records, rel: &str) -> Vec<Line> {
    let mut lines = vec![file_status(rel, FileStatus::Removed)];
    if let Some(old) = rules::latest_approved_version(&r.root, rel, &r.led) {
        for sec in &old.sections {
            lines.push(file_status(&sec.heading_text, FileStatus::Removed));
        }
    }
    lines.extend(removed_moves(r, rel).into_iter().map(Line::Move));
    lines
}

/// The terms file: its status, every entry with its status and its lines,
/// each entry approved and gone from the file, then its moves.
fn terms_file(r: &Records, tf: &TermsFile) -> Vec<Line> {
    let (st, moves) = terms_moves(r, tf);
    let mut lines = vec![file_status(repo::TERMS_PATH, st)];
    let (entries, gone) = approval::terms_file(&r.root, tf, &r.led);
    for (t, change) in tf.terms.iter().zip(&entries) {
        lines.push(Line::Status { subject: t.name.clone(), status: Status::Term(status::term_status(&r.led, tf, t)) });
        match change {
            Some(change) => lines.extend(change_lines(change, &r.names)),
            None => lines.extend(
                approval::entry_lines(tf, t)
                    .into_iter()
                    .skip(1)
                    .map(|l| Line::Record { depth: 1, text: named(&l, &r.names) }),
            ),
        }
    }
    for (name, change) in &gone {
        lines.push(Line::Status { subject: name.clone(), status: Status::Term(status::TermStatus::Removed) });
        lines.extend(change_lines(change, &r.names));
    }
    lines.extend(moves.into_iter().map(Line::Move));
    lines
}

/// A blame file: its status, its origin and every row, each row naming a
/// question with that question's status, marked when beyond its latest
/// approval, then its moves.
fn blame_file(r: &Records, bf: &blame::BlameFile) -> Vec<Line> {
    let (st, moves) = blame_moves(r, bf);
    let mut lines = vec![file_status(&bf.rel, st)];
    match approval::blame_file(&r.root, &bf.rel, &bf.lines, &r.led) {
        Some(change) => lines.extend(change_lines(&change, &r.names)),
        None => {
            if let Some(origin) = &bf.origin {
                lines.push(Line::Record { depth: 1, text: format!("origin: {}", origin) });
            }
            let (any_headings, known_ids) = blame::heading_ids(&r.topic_files);
            for row in &bf.rows {
                let range = format!("{}-{}", row.start, row.end);
                let dest = named(row.dest.trim(), &r.names);
                let status = row_status(r, &row.dest, &any_headings, &known_ids);
                let text = match status {
                    Some(s) => format!("{}\t{} [{}]", range, dest, s),
                    None => format!("{}\t{}", range, dest),
                };
                lines.push(Line::Record { depth: 1, text });
            }
        }
    }
    lines.extend(moves.into_iter().map(Line::Move));
    lines
}

/// The status of the question a blame row names (a `decision:` row, or a
/// `dropped: copy of` a question), `None` for any other row.
fn row_status(
    r: &Records,
    dest: &str,
    any_headings: &HashMap<String, Vec<(String, String)>>,
    known_ids: &HashSet<String>,
) -> Option<String> {
    let value = blame::decision_target(dest).or_else(|| blame::copy_target(dest.trim()))?;
    let id = blame::resolve_heading_value(value, any_headings, known_ids)?;
    let sec = r.topic_files.iter().flat_map(|rf| rf.sections.iter()).find(|s| s.id.as_deref() == Some(id.as_str()))?;
    Some(status::question_status(&r.led, sec).name_with(sec.answer_letter()))
}

// --- moves ------------------------------------------------------------------

fn topic_file_moves(r: &Records, rf: &RecordFile) -> Vec<Move> {
    moves::of_topic_file_in(&r.root, rf, &r.topic_files, &r.terms_list, &r.led, &r.tests)
}

fn removed_moves(r: &Records, rel: &str) -> Vec<Move> {
    let holds = rules::removal_holds(&r.root, rel, &r.led, &r.present_ids()).holds;
    vec![moves::removal(rel, &holds)]
}

/// The status of the terms file and its moves (`moves::of_terms_file`),
/// judged as `approve` judges it.
fn terms_moves(r: &Records, tf: &TermsFile) -> (FileStatus, Vec<Move>) {
    let st = status::terms_file_status(&r.led, tf);
    if st == FileStatus::Approved {
        return (st, Vec::new());
    }
    let approved_plans: HashSet<String> = r
        .topic_files
        .iter()
        .filter(|rf| state::plan_approved(&r.led, rf, &rf.rel))
        .map(|rf| rf.rel.clone())
        .collect();
    let minted = terms::minted_copy(tf);
    let (findings, _) = rules::judge_terms_file(
        &r.root,
        &r.topic_files,
        &minted,
        &r.led,
        &repo::Source::WorkingTree,
        true,
        &approved_plans,
    );
    (st, moves::of_terms_file(&findings, st))
}

/// The status of a blame file and its moves (`moves::of_blame_file`).
fn blame_moves(r: &Records, bf: &blame::BlameFile) -> (FileStatus, Vec<Move>) {
    let content = record::join_lines(&bf.lines);
    let st = status::blame_file_status(&r.led, &bf.rel, &content);
    let (any_headings, known_ids) = blame::heading_ids(&r.topic_files);
    let fs = blame::file_state(
        &r.root,
        &repo::Source::WorkingTree,
        bf,
        &content,
        &r.topic_files,
        &r.led,
        &any_headings,
        &known_ids,
    );
    let findings = rules::judge_blame_file(&r.root, &fs, bf);
    (st, moves::of_blame_file(&bf.rel, &findings, fs.awaiting))
}

