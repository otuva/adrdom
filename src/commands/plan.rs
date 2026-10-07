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
use crate::rules::Rule;
use crate::index;
use crate::ledger;
use crate::msg;
use crate::out::{self, Line};
use crate::plan::{self, Planning, Step};
use crate::reader;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state;
use std::path::{Path, PathBuf};

/// Per "What does a command that writes nothing print?" (848e1b76 A): the
/// file's status, the neighbours it read with the reader's word and whether
/// a line carries each, the findings on the file, and its moves, and
/// nothing else.
pub fn run(file: PathBuf) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            out::eprint(&[out::refusal(Rule::Plan,
                &msg::not_a_repo("plan", &[&file.display().to_string()], msg::Actor::Agent),
            )]);
            return 1;
        }
    };
    // Planning is the planning of a topic file ("What does the agent do to
    // plan a topic file?"): the terms file, a blame file or any other path
    // is refused with the move, never read as a topic file. The path is
    // read as the shell that ran plan reads it.
    let rel = match state::given_rel(&root, &file) {
        Some(rel) if crate::rules::is_topic_file_path(&rel) => rel,
        rel => {
            let rel = rel.unwrap_or_else(|| file.display().to_string());
            out::eprint(&[out::refusal(Rule::Plan, &msg::plan_not_a_topic_file(&rel))]);
            return 1;
        }
    };
    let mut rf = match record::load(&root.join(&rel)) {
        Ok(rf) => rf,
        Err(e) => {
            out::eprint(&[out::refusal(Rule::Plan,
                &msg::read_failed(&rel, &e, msg::topic_file_unread("plan", &rel, &e)),
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
/// Each is filed under the rule it is a finding of, as `show` and `check`
/// file it: `parse` for the file's shape, `plan` for a heading twice.
fn shape_refusals(rf: &RecordFile) -> Vec<crate::rules::Finding> {
    use crate::rules::Finding;
    let rel = &rf.rel;
    let mut parse: Vec<String> = rf.shape_errors.iter().map(|e| msg::shape_error(rel, e)).collect();
    for sec in &rf.sections {
        parse.extend(sec.shape_errors.iter().map(|e| msg::section_shape_error(rel, &sec.heading_text, e)));
        parse.extend(sec.parse_errors.iter().map(|e| msg::check_parse_error(rel, &sec.heading_text, e)));
    }
    for i in record::non_question_heading_lines(&rf.lines) {
        parse.push(msg::heading_not_a_question(rel, i + 1));
    }
    let mut out: Vec<Finding> = parse.into_iter().map(|m| Finding::new(Rule::Parse, m)).collect();
    out.extend(crate::rules::duplicate_heading_refusals(rel, &rf.sections).into_iter().map(|m| Finding::new(Rule::Plan, m)));
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
/// and the ledger `led` loaded once for the run: `adr plan` writes reader
/// rows, never a topic file, the terms or the ledger.
fn planning(root: &Path, rf: &RecordFile, e: &Env, led: &ledger::Ledger) -> Planning {
    plan::load(root, rf, &e.all, &e.terms, led)
}

/// What the questions in `idxs` read at `step`: each question, then each of
/// its neighbours with the reader's word, whether a line carries it, and the
/// reader's sentence of reason, which the move on its lines needs.
fn did(pl: &Planning, idxs: &[usize], step: Step, out: &mut Vec<Line>) {
    for q in pl.questions.iter().filter(|q| idxs.contains(&q.idx)) {
        let what = match step {
            Step::Heading => "heading",
            Step::Body => "heading with its body",
        };
        out.push(Line::Record { depth: 0, text: msg::plan_question_read(&q.heading, what) });
        out.extend(plan::neighbour_lines_at(q, step).into_iter().map(|text| Line::Record { depth: 1, text }));
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

    // The neighbours of each heading with no rows.
    let mut pl = planning(root, rf, &e, &led);
    let todo: Vec<usize> = pl.missing_heading().iter().map(|q| q.idx).collect();
    if !todo.is_empty() {
        let queries: Vec<String> = todo.iter().map(|&i| plan::heading_text(&rf.sections[i])).collect();
        let near = index::nearest(root, &led, &queries, index::NEIGHBOURS).map_err(stop)?;
        for (&i, n) in todo.iter().zip(&near) {
            reader::read(root, &plan::heading_pairs(&rf.sections[i], n)).map_err(stop)?;
        }
        pl = planning(root, rf, &e, &led);
        did(&pl, &todo, Step::Heading, &mut read);
    }

    // Once every heading's neighbours are on a line and every heading has
    // its body, the neighbours of each question with its body.
    let todo: Vec<usize> = pl.missing_body().iter().map(|q| q.idx).collect();
    if pl.lines_missing(Step::Heading).is_empty() && pl.without_body().is_empty() && !todo.is_empty() {
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
        pl = planning(root, rf, &e, &led);
        did(&pl, &todo, Step::Body, &mut read);
    }

    // What the run prints is read after the rows it wrote: the status, the
    // findings and the moves of the file as it stands now.
    let tests = crate::commands::check::test_index(root, &repo::Source::WorkingTree);
    let report = crate::moves::of_topic_file_in(root, rf, &e.all, &e.terms, &led, &tests);
    let mut lines = vec![crate::commands::show::topic_file_status_line(&led, rf)];
    lines.extend(read);
    lines.extend(out::report(&report));
    Ok(lines)
}

/// A stop of the model or the reader: the error, which names its move.
/// What stopped `plan`, every line of it led by the command's name once
/// (`out::stop`).
fn stop(e: String) -> Vec<Line> {
    out::stop(Rule::Plan, &e)
}
