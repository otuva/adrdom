//! `adr plan <file>`: the agent's one command for planning a topic file
//! ("What does the agent do to plan a topic file?",
//! 46fc51ff-66b9-4841-a953-ae0d4c2fc424 A). It takes no flag: the state of
//! the file picks what it does, heading by heading ("Does a planning step
//! run without the record of the step before?",
//! d3be71e4-ce7b-43b0-86a8-e86868521f99 A).
//!
//! 1. Every heading not yet asked with no reader rows at its heading hash
//!    gets the ten nearest neighbours of the pool, read with the heading
//!    prompt, whatever state the rest of the file is in.
//! 2. A neighbour on no line is a finding, and no body is read.
//! 3. While a heading has no body, no body is read.
//! 4. When every heading has a body, each question with no rows at its body
//!    hash gets the ten nearest neighbours of the pool, read with the body
//!    prompt, and the ten nearest other questions of the file, read with
//!    the plan prompt ("Does the tool bring the plan's own questions as
//!    neighbours of each other?", 4838baa4-a5df-49e7-929c-ed63b5adaf4e A).
//! 5. What it prints is the file's status, what it read, the findings on the
//!    file and the file's moves, the one computation `show` reads too.
//!
//! The model and the reader are loaded only when there is something to read.
//! Planning stops without them ("Does planning stop without the embedding
//! model?", 21cdad59-4f0c-42a5-b4f3-6af946637df3; "Does planning stop
//! without the reader?", 3dba486c-a399-4ce3-a794-e38c4f733403 A); `adr read`
//! loads them too.
use crate::index;
use crate::ledger;
use crate::msg::{self, Next};
use crate::out::{self, Line};
use crate::plan::{self, Planning, Step};
use crate::reader;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state;
use crate::status;
use std::path::{Path, PathBuf};

/// Per "What does a command that writes nothing print?" (848e1b76 A): the
/// file's status, the neighbours it read with the reader's word and whether
/// a line carries each, the findings on the file, and its moves, and
/// nothing else.
pub fn run(file: PathBuf) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            out::eprint(&[out::refusal(
                "plan",
                &msg::not_a_repo("plan", &file.display().to_string(), msg::Actor::Agent),
            )]);
            return 1;
        }
    };
    let rel = state::rel_path(&root, &file);
    let mut rf = match record::load(&file) {
        Ok(rf) => rf,
        Err(e) => {
            out::eprint(&[out::refusal(
                "plan",
                &msg::read_failed(&file.display().to_string(), e, Next::agent("checks the path and retries")),
            )]);
            return 1;
        }
    };
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

/// The shape and parse errors of the file, a `###` line that is no question
/// (9913568f A), and a heading used twice: the
/// planning keys its rows by heading text.
fn shape_refusals(rf: &RecordFile) -> Vec<String> {
    let rel = &rf.rel;
    let mut out: Vec<String> = rf.shape_errors.iter().map(|e| msg::shape_error(rel, e)).collect();
    for sec in &rf.sections {
        out.extend(sec.shape_errors.iter().map(|e| msg::section_shape_error(rel, &sec.heading_text, e)));
        out.extend(sec.parse_errors.iter().map(|e| msg::check_parse_error(rel, &sec.heading_text, e)));
    }
    for i in record::non_question_heading_lines(&rf.lines) {
        out.push(msg::heading_not_a_question(rel, i + 1));
    }
    out.extend(crate::rules::duplicate_heading_refusals(rel, &rf.sections));
    out
}

struct Env {
    all: Vec<RecordFile>,
    terms: Vec<crate::terms::Term>,
}

fn env(root: &Path) -> Env {
    Env {
        all: record::load_all(root),
        terms: state::load_terms(root).map(|t| t.terms).unwrap_or_default(),
    }
}

fn planning(root: &Path, rf: &RecordFile) -> Planning {
    let led = ledger::read(&root.join(repo::LEDGER_PATH));
    let e = env(root);
    plan::load(root, rf, &e.all, &e.terms, &led)
}

/// What the questions in `idxs` read at `step`: each question, then each of
/// its neighbours with the reader's word, whether a line carries it, and the
/// reader's sentence of reason, which the move on its lines needs.
fn did(pl: &Planning, idxs: &[usize], step: Step, out: &mut Vec<Line>) {
    for q in pl.questions.iter().filter(|q| idxs.contains(&q.idx)) {
        let (what, ns) = match step {
            Step::Heading => ("heading", &q.heading_neighbours),
            Step::Body => ("heading with its body", &q.body_neighbours),
        };
        out.push(Line::Record { depth: 0, text: msg::plan_question_read(&q.heading, what) });
        for n in ns {
            let reason = reader::flatten(&n.reason);
            out.push(Line::Record {
                depth: 1,
                text: msg::plan_neighbour(n.kind.label(), &n.name, &n.word, n.on_line, &reason),
            });
        }
    }
}

/// What the run prints, or the refusal that stopped it. The run reads what
/// the step the file is at needs, then prints the file's status, what it
/// read, the findings on the file that are not already a move, and the
/// file's moves: the one computation `show` reads too (per "Does a command
/// compute a move of its own?", aac5c313 A).
fn plan_file(root: &Path, rf: &RecordFile) -> Result<Vec<Line>, Vec<Line>> {
    let shape = shape_refusals(rf);
    if !shape.is_empty() {
        return Err(shape.iter().map(|m| out::refusal("plan", m)).collect());
    }
    let mut read = Vec::new();

    // The neighbours of each heading with no rows.
    let mut pl = planning(root, rf);
    let todo: Vec<usize> = pl.missing_heading().iter().map(|q| q.idx).collect();
    if !todo.is_empty() {
        let led = ledger::read(&root.join(repo::LEDGER_PATH));
        let queries: Vec<String> = todo.iter().map(|&i| plan::heading_text(&rf.sections[i])).collect();
        let near = index::nearest(root, &led, &queries, index::NEIGHBOURS).map_err(stop)?;
        for (&i, n) in todo.iter().zip(&near) {
            reader::read(root, &plan::heading_pairs(&rf.sections[i], n)).map_err(stop)?;
        }
        pl = planning(root, rf);
        did(&pl, &todo, Step::Heading, &mut read);
    }

    // Once every heading's neighbours are on a line and every heading has
    // its body, the neighbours of each question with its body.
    let todo: Vec<usize> = pl.missing_body().iter().map(|q| q.idx).collect();
    if pl.lines_missing(Step::Heading).is_empty() && pl.without_body().is_empty() && !todo.is_empty() {
        let led = ledger::read(&root.join(repo::LEDGER_PATH));
        // The other questions not yet asked are ranked by their own text. One
        // call embeds the texts of the questions to read and of every
        // planned question.
        let planned: Vec<usize> = pl.questions.iter().filter(|q| q.has_body).map(|q| q.idx).collect();
        let mut queries: Vec<String> = todo.iter().map(|&i| record::body_text(rf, &rf.sections[i])).collect();
        queries.extend(planned.iter().map(|&i| record::body_text(rf, &rf.sections[i])));
        let (near, qv) = index::nearest_with_vectors(root, &led, &queries, index::NEIGHBOURS).map_err(stop)?;
        let vecs = &qv[todo.len()..];
        for (&i, n) in todo.iter().zip(&near) {
            let own: Vec<usize> = match planned.iter().position(|&p| p == i) {
                Some(me) if planned.len() > 1 => {
                    index::rank(&vecs[me], vecs, |k| k != me, index::NEIGHBOURS).into_iter().map(|(k, _)| planned[k]).collect()
                }
                _ => Vec::new(),
            };
            reader::read(root, &plan::body_pairs(rf, i, n, &own)).map_err(stop)?;
        }
        pl = planning(root, rf);
        did(&pl, &todo, Step::Body, &mut read);
    }

    // What the run prints is read after the rows it wrote: the status, the
    // findings and the moves of the file as it stands now.
    let led = ledger::read(&root.join(repo::LEDGER_PATH));
    let e = env(root);
    let tests = crate::commands::check::test_index(root, &repo::Source::WorkingTree);
    let moves = crate::moves::of_topic_file_in(root, rf, &e.all, &e.terms, &led, &tests);
    // Every finding `approve` refuses on is printed once: a finding with a
    // move of its own as that move, a finding of the planning steps here,
    // its move being the planning move. Missing rows are what this command
    // writes: rows still missing after the run wait on the step before,
    // whose move is printed, so they are not printed as findings too.
    let (_, steps) = crate::rules::judge_topic_file_split_at(root, rf, &e.all, &e.terms, &led);
    let mut pl = planning(root, rf);
    pl.rel = rf.rel.clone();
    let rows_missing = pl.rows_missing();
    let steps = steps.into_iter().filter(|f| !rows_missing.contains(&f.message));

    let mut lines = vec![Line::Status {
        subject: rf.rel.clone(),
        status: out::Status::File(status::topic_file_status(&led, rf)),
    }];
    lines.extend(read);
    lines.extend(steps.map(Line::Finding));
    lines.extend(moves.into_iter().map(Line::Move));
    Ok(lines)
}

/// A stop of the model or the reader: the error, which names its move.
fn stop(e: String) -> Vec<Line> {
    vec![out::refusal("plan", &e)]
}
