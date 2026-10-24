//! `adr plan <file>`: the agent's one command for planning a topic file
//! ("What does the agent do to plan a topic file?",
//! 46fc51ff-66b9-4841-a953-ae0d4c2fc424 A). It takes no flag: the state of
//! the file picks what it does, heading by heading ("Does a planning step
//! run without the record of the step before?",
//! d3be71e4-ce7b-43b0-86a8-e86868521f99 A).
//!
//! 1. Every heading not yet asked with no neighbours on record at its
//!    heading hash gets the ten nearest neighbours of the pool recorded in
//!    the neighbours file, whatever state the rest of the file is in. The
//!    reader reads none of them ("Does the reader read a neighbour of a
//!    heading?", 60341be0-fd05-4281-92d7-668584a8539a A: no).
//! 2. A neighbour on no line is a finding, and no body is read.
//! 3. While a heading has no body, no body is read.
//! 4. When every heading has a body and no finding the agent clears without
//!    the reader stands on the file (a word over the limit, a banned word, a
//!    depends line over its bounds, a tell), each question with no rows at
//!    its body hash gets the ten nearest neighbours of the pool, read with
//!    the body prompt ("What word does the reader give a neighbour of a
//!    question with its body?", d10dab89-1d42-4764-9d72-22d6fd6a9260 A), and
//!    the ten nearest other questions of the file, read with the plan prompt
//!    ("Does the tool bring a plan's own questions as neighbours of each
//!    other?", d984e425-3e88-4756-88fb-18241f9f62a7 A). While such a finding
//!    stands, the run prints it and reads nothing: the agent clears it first.
//!    A pair the reader gives no word for, twice, is a finding whose move is
//!    plan again; that question's rows wait, the others are read.
//! 5. What it prints is the file's status, what it read, the findings on the
//!    file and the file's moves, the one computation `show` reads too.
//!
//! The model is loaded when a heading has no neighbours on record, and the
//! reader only when a body is to be read. Planning stops without them
//! ("Does planning stop without the embedding model?",
//! 21cdad59-4f0c-42a5-b4f3-6af946637df3; "Does planning stop with no reader
//! on the machine?", bd94a89f-7142-4dab-b757-6482a718a0a0 A); `adr read`
//! loads them too.
use crate::index;
use crate::ledger;
use crate::msg;
use crate::out::{self, Line};
use crate::plan::{self, Planning, Step};
use crate::reader;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::rules::{Class, Command, Finding, Subject};
use crate::state;
use std::path::{Path, PathBuf};

/// Per "What does a command print when it writes nothing?" (1af76f91 A): the
/// file's status, the neighbours it brought (with the reader's word for a
/// body) and whether a line carries each, the findings on the file, and its
/// moves, and nothing else.
pub fn run(file: PathBuf) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            out::eprint(&[out::refusal(
                Command::Plan,
                &msg::not_a_repo("plan", &[&file.display().to_string()], msg::Actor::Agent),
            )]);
            return 1;
        }
    };
    if super::refused_in_satellite(
        &root,
        Command::Plan,
        &[&file.display().to_string()],
        msg::Actor::Agent,
    ) {
        return 1;
    }
    // Planning is the planning of a topic file ("What does the agent do to
    // plan a topic file?"): the terms file, a blame file or any other path
    // is refused with the move, never read as a topic file. The path is
    // read as the shell that ran plan reads it.
    let rel = match state::given_rel(&root, &file) {
        Some(rel) if crate::rules::is_topic_file_path(&rel) => rel,
        rel => {
            let rel = rel.unwrap_or_else(|| file.display().to_string());
            out::eprint(&[out::refusal(
                Command::Plan,
                &msg::plan_not_a_topic_file(&rel),
            )]);
            return 1;
        }
    };
    let mut rf = match record::load(&root.join(&rel)) {
        Ok(rf) => rf,
        Err(e) => {
            out::eprint(&[out::refusal(
                Command::Plan,
                &msg::read_failed(&rel, &e, msg::topic_file_unread("plan", &rel, &e)),
            )]);
            return 1;
        }
    };
    // Planning a topic file that loads makes the session a planner
    // (6c406624), whatever the run finds after.
    if let Err(e) = crate::session::mark_planner(&root) {
        out::eprint(&[out::refusal(
            Command::Plan,
            &msg::write_failed(
                repo::ROLES_PATH,
                &e,
                msg::Next::owner(msg::Act::Rewrites(msg::Part::File(
                    repo::ROLES_PATH.to_string(),
                ))),
            ),
        )]);
        return 1;
    }
    rf.rel = rel;
    match plan_file(&root, &rf) {
        Ok(lines) => {
            out::print(&lines);
            0
        }
        Err(lines) => {
            out::eprint(&lines);
            1
        }
    }
}

/// The `parse` findings of the file (`rules::parse_findings`, the list
/// `show` and `check` print) and a heading used twice, filed under `plan`:
/// the planning keys its rows by heading text.
fn shape_refusals(rf: &RecordFile) -> Vec<crate::rules::Finding> {
    let mut out = crate::rules::parse_findings(&rf.rel, rf);
    out.extend(crate::rules::duplicate_heading_refusals(
        &rf.rel,
        &rf.sections,
    ));
    out
}

struct Env {
    all: Vec<RecordFile>,
    terms: Vec<crate::terms::Term>,
}

fn env(root: &Path) -> Env {
    Env {
        all: record::load_all(root),
        terms: crate::rules::judged_terms(state::load_terms(root).as_ref()),
    }
}

/// The planning records of `rf` as they stand, against the records `e`
/// and the ledger `led` loaded once for the run: `adr plan` writes
/// neighbours and reader rows, never a topic file, the terms or the ledger.
fn planning(root: &Path, rf: &RecordFile, e: &Env, led: &ledger::Ledger) -> Planning {
    plan::load(root, rf, &e.all, &e.terms, led)
}

/// What the questions in `idxs` brought at `step`: each question, then each
/// of its neighbours with, for a body, the reader's word, whether a line
/// carries it, and the reader's sentence of reason, which the move on its lines
/// needs.
fn did(pl: &Planning, idxs: &[usize], step: Step, out: &mut Vec<Line>) {
    for q in pl.questions.iter().filter(|q| idxs.contains(&q.idx)) {
        let what = match step {
            Step::Heading => "heading",
            Step::Body => "heading with its body",
        };
        out.push(Line::Record {
            depth: 0,
            text: msg::plan_question_read(&q.heading, what),
        });
        out.extend(
            plan::neighbour_lines_at(q, step)
                .into_iter()
                .map(|text| Line::Record { depth: 1, text }),
        );
    }
}

/// What the run prints, or the refusal that stopped it. The run reads what
/// the step the file is at needs, then prints the file's status, what it
/// read, the findings on the file and the file's moves: the one computation
/// `show` reads too (per "Does a command
/// compute a move of its own?", aac5c313 A).
fn plan_file(root: &Path, rf: &RecordFile) -> Result<Vec<Line>, Vec<Line>> {
    let shape = shape_refusals(rf);
    if !shape.is_empty() {
        return Err(shape.into_iter().map(Line::Finding).collect());
    }
    let mut read = Vec::new();
    let e = env(root);
    let led = ledger::read(&root.join(repo::LEDGER_PATH));
    // Read once for the run: `adr plan` writes no topic file or terms file.
    let live = reader::Live::of(root);

    // The neighbours of each heading with no rows.
    let mut pl = planning(root, rf, &e, &led);
    let todo: Vec<usize> = pl.missing_heading().iter().map(|q| q.idx).collect();
    if !todo.is_empty() {
        let queries: Vec<String> = todo
            .iter()
            .map(|&i| plan::heading_text(&rf.sections[i]))
            .collect();
        let near = index::nearest(root, &led, &queries, index::NEIGHBOURS).map_err(stop)?;
        let entries: Vec<(String, Vec<String>)> = todo
            .iter()
            .zip(&near)
            .map(|(&i, n)| {
                (
                    record::heading_hash(&rf.sections[i]),
                    n.iter().map(|x| x.id.clone()).collect(),
                )
            })
            .collect();
        crate::neighbours::record(root, &entries, &live).map_err(stop)?;
        pl = planning(root, rf, &e, &led);
        did(&pl, &todo, Step::Heading, &mut read);
    }

    // Once every heading's neighbours are on a line and every heading has
    // its body, the neighbours of each question with its body. The reader
    // reads only when no finding the agent clears without it stands on the
    // file (a word over the limit, a banned word, a depends line over its
    // bounds, a tell): the run prints those at once, and the rows wait on
    // them. The findings of the planning itself (a step without its record,
    // a neighbour on no line) are what the step clears, never what stops it.
    let tests = crate::commands::check::test_index(root, &repo::Source::WorkingTree);
    let index = crate::rules::RepoIndex::new(root, &e.all, &e.terms, &led);
    let todo: Vec<usize> = pl.missing_body().iter().map(|q| q.idx).collect();
    let mut no_word: Vec<Finding> = Vec::new();
    let read_bodies = pl.lines_missing(Step::Heading).is_empty()
        && pl.without_body().is_empty()
        && !todo.is_empty()
        // The one judgement `check` and `approve` read (`judge_topic_file_in`),
        // so the three agree on whether the rows wait.
        && !crate::rules::judge_topic_file_in(root, rf, &e.all, &e.terms, &led, &index)
            .iter()
            .any(Finding::blocks_the_reader);
    if read_bodies {
        // The other questions not yet asked are ranked by their own text. One
        // call embeds the texts of the questions to read and of every
        // planned question.
        let planned: Vec<usize> = pl
            .questions
            .iter()
            .filter(|q| q.has_body)
            .map(|q| q.idx)
            .collect();
        let mut queries: Vec<String> = todo
            .iter()
            .map(|&i| record::body_text(rf, &rf.sections[i]))
            .collect();
        queries.extend(
            planned
                .iter()
                .map(|&i| record::body_text(rf, &rf.sections[i])),
        );
        let (near, qv) =
            index::nearest_with_vectors(root, &led, &queries, index::NEIGHBOURS).map_err(stop)?;
        let vecs = &qv[todo.len()..];
        for (&i, n) in todo.iter().zip(&near) {
            let own: Vec<usize> = match planned.iter().position(|&p| p == i) {
                Some(me) if planned.len() > 1 => {
                    index::rank(&vecs[me], vecs, |k| k != me, index::NEIGHBOURS)
                        .into_iter()
                        .map(|(k, _)| planned[k])
                        .collect()
                }
                _ => Vec::new(),
            };
            // A pair the reader gave no word for, twice, is a finding whose
            // move is this command again; the question's rows wait, the
            // other questions are read on.
            let got =
                reader::read_live(root, &plan::body_pairs(rf, i, n, &own), &live).map_err(stop)?;
            for f in got.failed {
                let said = msg::reader_no_word(
                    &format!("{}: \"{}\"", rf.rel, rf.sections[i].heading_text),
                    &f.neighbour,
                    &f.error,
                    crate::moves::clause(crate::moves::MoveKind::Plan {
                        rel: rf.rel.clone(),
                    }),
                );
                no_word.push(
                    Finding::said(Class::Reader, said)
                        .about(Subject::TopicFile(rf.rel.clone()))
                        .in_section(i),
                );
            }
        }
        pl = planning(root, rf, &e, &led);
        // What it read: the questions whose rows the run made; one with a
        // pair the reader gave no word for has none.
        let made: Vec<usize> = pl
            .questions
            .iter()
            .filter(|q| todo.contains(&q.idx) && q.body_done)
            .map(|q| q.idx)
            .collect();
        did(&pl, &made, Step::Body, &mut read);
    }
    // What the run prints is read after the rows it wrote: the status, the
    // findings and the moves of the file as it stands now. A question with
    // a pair the reader gave no word for has its rows missing for that one
    // reason, so that finding stands in for the rows-missing one.
    let mut report = if read_bodies {
        crate::moves::of_topic_file_in(root, rf, &e.all, &e.terms, &led, &tests)
    } else {
        crate::moves::of_topic_file_with(root, rf, &e.all, &e.terms, &led, &tests, &index)
    };
    let waiting: Vec<String> = no_word
        .iter()
        .flat_map(|f| f.sections.iter())
        .map(|&i| msg::plan_rows_missing(&rf.rel, &rf.sections[i].heading_text).found)
        .collect();
    report.findings.retain(|f| !waiting.contains(&f.found));
    report.findings.splice(0..0, no_word);

    let mut lines = vec![crate::commands::show::topic_file_status_line(&led, rf)];
    lines.extend(read);
    lines.extend(out::report(&report));
    Ok(lines)
}

/// A stop of the model or the reader: the error, which names its move.
/// What stopped `plan`, every line of it led by the command's name once
/// (`out::stop`).
fn stop(e: String) -> Vec<Line> {
    out::stop(Command::Plan, &e)
}
