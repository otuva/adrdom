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
//! 2. A neighbour on no line is listed, and the command stops.
//! 3. Headings with a body next to headings with none are refused.
//! 4. When every heading has a body, each question with no rows at its body
//!    hash gets the ten nearest neighbours of the pool, read with the body
//!    prompt, and the ten nearest other questions of the file, read with
//!    the plan prompt ("Does the tool bring the plan's own questions as
//!    neighbours of each other?", 4838baa4-a5df-49e7-929c-ed63b5adaf4e A).
//! 5. A neighbour on no line is listed; with none, the file is ready and
//!    the owner approves.
//!
//! The model and the reader are loaded only when there is something to read.
//! Planning stops without them ("Does planning stop without the embedding
//! model?", 21cdad59-4f0c-42a5-b4f3-6af946637df3; "Does planning stop
//! without the reader?", 3dba486c-a399-4ce3-a794-e38c4f733403 A); `adr read`
//! loads them too.
use crate::index;
use crate::ledger;
use crate::msg::{self, Actor, Next};
use crate::plan::{self, NextMove, Planning, Step};
use crate::reader;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state;
use std::path::{Path, PathBuf};

pub fn run(file: PathBuf) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("plan", &file.display().to_string(), Actor::Agent));
            return 1;
        }
    };
    let rel = state::rel_path(&root, &file);
    let mut rf = match record::load(&file) {
        Ok(rf) => rf,
        Err(e) => {
            eprintln!(
                "plan: {}",
                msg::read_failed(&file.display().to_string(), e, Next::agent("checks the path and retries"))
            );
            return 1;
        }
    };
    rf.rel = rel.clone();
    match plan_file(&root, &rf) {
        Ok((out, code)) => {
            print!("{}", out);
            code
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

/// The shape and parse errors of the file, and a heading used twice: the
/// planning keys its rows by heading text.
fn shape_refusals(rf: &RecordFile) -> Vec<String> {
    let rel = &rf.rel;
    let mut out: Vec<String> = rf.shape_errors.iter().map(|e| msg::shape_error(rel, e)).collect();
    for sec in &rf.sections {
        out.extend(sec.shape_errors.iter().map(|e| msg::section_shape_error(rel, &sec.heading_text, e)));
        out.extend(sec.parse_errors.iter().map(|e| msg::check_parse_error(rel, &sec.heading_text, e)));
    }
    out.extend(crate::actions::duplicate_heading_refusals(rel, &rf.sections));
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

/// The lines of what the questions in `idxs` read at `step`.
fn did(pl: &Planning, idxs: &[usize], step: Step, out: &mut String) {
    for q in pl.questions.iter().filter(|q| idxs.contains(&q.idx)) {
        let (what, ns) = match step {
            Step::Heading => ("heading", &q.heading_neighbours),
            Step::Body => ("heading with its body", &q.body_neighbours),
        };
        out.push_str(&msg::plan_read(&q.heading, what, ns.len()));
        out.push('\n');
        for n in ns {
            out.push_str(&msg::plan_neighbour(n.kind.label(), &n.name, &n.word, n.on_line));
            out.push('\n');
        }
    }
}

fn wanted(pl: &Planning, step: Step, out: &mut String) {
    for (q, n) in pl.lines_missing(step) {
        out.push_str(&msg::plan_line_wanted(&q.heading, n.kind.label(), &n.name));
        out.push('\n');
    }
}

/// What the run prints, and its exit code, or the refusal that stopped it.
fn plan_file(root: &Path, rf: &RecordFile) -> Result<(String, i32), String> {
    let shape = shape_refusals(rf);
    if !shape.is_empty() {
        return Err(shape.iter().map(|m| format!("plan: {}", m)).collect::<Vec<_>>().join("\n"));
    }
    let rel = rf.rel.clone();
    let mut out = msg::plan_head(&rel);
    out.push('\n');

    // Step 2: the neighbours of each heading with no rows.
    let mut pl = planning(root, rf);
    let todo: Vec<usize> = pl.missing_heading().iter().map(|q| q.idx).collect();
    if !todo.is_empty() {
        let led = ledger::read(&root.join(repo::LEDGER_PATH));
        let queries: Vec<String> = todo.iter().map(|&i| plan::heading_text(&rf.sections[i])).collect();
        let near = index::nearest(root, &led, &queries, index::NEIGHBOURS)?;
        for (&i, n) in todo.iter().zip(&near) {
            reader::read(root, &plan::heading_pairs(&rf.sections[i], n))?;
        }
        pl = planning(root, rf);
        did(&pl, &todo, Step::Heading, &mut out);
    }
    // The length hits in every body written so far, whatever step the file
    // is at: shown, never a stop.
    let led_now = ledger::read(&root.join(repo::LEDGER_PATH));
    let mut hits = crate::actions::length_refusals(&rel, rf, &led_now);
    // A loaded phrase in a body and a question that pairs no term: shown
    // the same way.
    hits.extend(crate::actions::body_tell_refusals(&rel, rf, &led_now));
    hits.extend(crate::actions::no_term_refusals(&rel, rf, &env(root).terms, &led_now));
    // A set letter its same-file target has no option for: shown the same way.
    hits.extend(crate::actions::letter_refusals(&rel, rf, &led_now));
    // The three shapes of a depends entry approve stops: shown the same way.
    {
        let e = env(root);
        let lookup = crate::terms::phrase_lookup(&e.terms);
        let headings = crate::depends::cross_file_heading_index(&e.all, &led_now);
        hits.extend(crate::actions::depends_shape_refusals(&rel, rf, &e.terms, &lookup, &headings, &e.all, &led_now));
    }
    for h in &hits {
        out.push_str(h);
        out.push('\n');
    }
    if !pl.lines_missing(Step::Heading).is_empty() {
        wanted(&pl, Step::Heading, &mut out);
        out.push_str(&msg::plan_next_lines(&rel));
        out.push('\n');
        return Ok((out, 0));
    }
    let bare = pl.without_body();
    if !bare.is_empty() {
        // Bodies under some headings only is refused; headings none of which
        // has a body yet wait for the agent to write them.
        if bare.len() < pl.questions.len() {
            print!("{}", out);
            let heads: Vec<String> = bare.iter().map(|q| q.heading.clone()).collect();
            return Err(msg::plan_bodies_missing(&rel, &heads));
        }
        out.push_str(&msg::plan_next_bodies(&rel));
        out.push('\n');
        return Ok((out, 0));
    }

    // Step 5: the neighbours of each question with its body.
    let todo: Vec<usize> = pl.missing_body().iter().map(|q| q.idx).collect();
    if !todo.is_empty() {
        let led = ledger::read(&root.join(repo::LEDGER_PATH));
        // The other questions not yet asked are ranked by their own text. One
        // call embeds the texts of the questions to read and of every
        // planned question.
        let planned: Vec<usize> = pl.questions.iter().filter(|q| q.has_body).map(|q| q.idx).collect();
        let mut queries: Vec<String> = todo.iter().map(|&i| record::body_text(rf, &rf.sections[i])).collect();
        queries.extend(planned.iter().map(|&i| record::body_text(rf, &rf.sections[i])));
        let (near, qv) = index::nearest_with_vectors(root, &led, &queries, index::NEIGHBOURS)?;
        let vecs = &qv[todo.len()..];
        for (&i, n) in todo.iter().zip(&near) {
            let own: Vec<usize> = match planned.iter().position(|&p| p == i) {
                Some(me) if planned.len() > 1 => {
                    index::rank(&vecs[me], vecs, |k| k != me, index::NEIGHBOURS).into_iter().map(|(k, _)| planned[k]).collect()
                }
                _ => Vec::new(),
            };
            reader::read(root, &plan::body_pairs(rf, i, n, &own))?;
        }
        pl = planning(root, rf);
        did(&pl, &todo, Step::Body, &mut out);
    }
    if !pl.lines_missing(Step::Body).is_empty() {
        wanted(&pl, Step::Body, &mut out);
        out.push_str(&msg::plan_next_lines(&rel));
        out.push('\n');
        return Ok((out, 0));
    }
    match pl.next() {
        NextMove::Ready if !hits.is_empty() => {
            out.push_str(&msg::plan_next_length(&rel));
            out.push('\n');
        }
        NextMove::Ready => {
            out.push_str(&msg::plan_ready(&rel));
            out.push('\n');
            out.push_str(&msg::plan_next_approve(&rel));
            out.push('\n');
        }
        _ => {
            out.push_str(&msg::plan_next_plan(&rel));
            out.push('\n');
        }
    }
    Ok((out, 0))
}
