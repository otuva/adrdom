// `adr plan <file>` and the lines: the planning steps, the records they
// leave (the neighbours of a heading in adrdom/state/neighbours.tsv, the
// reader's rows of a body in adrdom/state/reader.tsv), and what approve
// refuses without them.
// The embedding model and the reader are the fakes; nothing here goes
// through `adr_owner`, whose approve would plan first, so every refusal is
// the tool's own.

mod common;

use adr::reader::{self, Kind};
use adr::record;
use adr::repo;
use common::*;
use std::path::{Path, PathBuf};
use std::process::Output;

const REL: &str = "adrdom/decisions/topic.md";
const BASE: &str = "adrdom/decisions/base.md";
const BASE_Q: &str = "Does the base hold?";
const H1: &str = "Does the first thing glow?";
const H2: &str = "Does the second thing glow?";
const PAINT_ID: &str = "99999991-9999-9999-9999-999999999999";
const WIDGET_ID: &str = "99999990-9999-9999-9999-999999999999";

fn body(n: &str) -> String {
    format!(
        "\nProblem: problem {n} of the widget.\n\nStandard: standard {n}.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        n = n
    )
}

fn section(heading: &str, body: &str) -> String {
    format!("\n### {}\ndepends: widget\n{}", heading, body)
}

/// An installed repository with two terms and one current decision.
fn repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget", "paint"]);
    let base = write_record(
        &dir,
        BASE,
        &format!("# Base\n{}", section(BASE_Q, &body("base widget"))),
    );
    approve_record(&dir, BASE);
    assert!(adr(&dir, &["ask", base.to_str().unwrap()], &[], "")
        .status
        .success());
    let id = extract_ids(&std::fs::read_to_string(&base).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    dir
}

fn plan_with(dir: &Path, envs: &[(&str, &str)]) -> Output {
    adr(dir, &["plan", dir.join(REL).to_str().unwrap()], envs, "")
}

fn plan(dir: &Path) -> Output {
    plan_with(dir, &[])
}

fn approve_with(dir: &Path, envs: &[(&str, &str)]) -> Output {
    let mut all = vec![("ADR_OWNER_BYPASS", "1")];
    all.extend_from_slice(envs);
    adr(dir, &["approve", dir.join(REL).to_str().unwrap()], &all, "")
}

fn approve(dir: &Path) -> Output {
    approve_with(dir, &[])
}

fn write(dir: &Path, text: &str) {
    std::fs::write(dir.join(REL), text).unwrap();
}

fn read(dir: &Path) -> String {
    std::fs::read_to_string(dir.join(REL)).unwrap()
}

fn rows(dir: &Path) -> Vec<reader::Row> {
    reader::read_file(dir).rows
}

/// The rows of the neighbours file: the heading hash and the neighbour's id.
fn neighbour_rows(dir: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(dir.join(repo::NEIGHBOURS_PATH))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(h, id)| (h.to_string(), id.to_string()))
        .collect()
}

fn neighbours_text(dir: &Path) -> String {
    std::fs::read_to_string(dir.join(repo::NEIGHBOURS_PATH)).unwrap_or_default()
}

fn reader_text(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("adrdom/state/reader.tsv")).unwrap_or_default()
}

/// Writes each neighbour `adr plan` printed as on no line, as an
/// `unrelated:` entry under its heading.
fn write_lines(dir: &Path, plan_stdout: &str) -> usize {
    let wanted = lines_wanted(plan_stdout);
    write(dir, &add_unrelated(&read(dir), &wanted));
    wanted.len()
}

fn ok(out: &Output) -> String {
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        stdout(out),
        stderr(out)
    );
    stdout(out)
}

fn refused(out: &Output) -> String {
    assert!(
        !out.status.success(),
        "should have refused: {}",
        stdout(out)
    );
    stderr(out)
}

/// True when `out` holds `line` as one whole line.
fn has_line(out: &str, line: &str) -> bool {
    out.lines().any(|l| l == line)
}

/// Whether `out` names the move `mv`: as a move line, or as the move a
/// finding line ends with.
fn names_move(out: &str, mv: &str) -> bool {
    out.lines()
        .any(|l| l == mv || l.ends_with(&format!("; {}", mv)))
}

/// True when `out` holds a line that starts with `prefix`.
fn has_line_starting(out: &str, prefix: &str) -> bool {
    out.lines().any(|l| l.starts_with(prefix))
}

fn sha(text: &str) -> String {
    record::sha256_hex(text)
}

/// The file written again with these sections, each `(heading, body)`, in
/// order: a heading keeps the `unrelated:` line it has in the file now.
fn rewrite(dir: &Path, sections: &[(&str, String)]) {
    let now = read(dir);
    let mut out = String::from("# Topic\n");
    for (heading, body) in sections {
        let at = now.find(&format!("### {}\n", heading));
        let line = at
            .and_then(|i| {
                let rest = &now[i..];
                let end = rest[1..].find("\n### ").map_or(rest.len(), |j| j + 1);
                rest[..end]
                    .lines()
                    .find(|l| l.starts_with("unrelated: "))
                    .map(String::from)
            })
            .map(|l| format!("{}\n", l))
            .unwrap_or_default();
        out.push_str(&format!(
            "\n### {}\ndepends: widget\n{}{}",
            heading, line, body
        ));
    }
    write(dir, &out);
}

/// A fresh topic file with two headings and no body.
fn two_headings(dir: &Path) {
    write(
        dir,
        &format!("# Topic\n{}{}", section(H1, ""), section(H2, "")),
    );
}

// adr: 46fc51ff-66b9-4841-a953-ae0d4c2fc424
#[test]
fn the_steps_in_order_from_headings_to_approval() {
    let dir = repo();
    let before = rows(&dir).len();
    let held_n = neighbour_rows(&dir).len();
    two_headings(&dir);

    // Step 2: the neighbours of each heading, ten at most, the current
    // decision and both terms; the heading's own depends line holds one term.
    // They go to the neighbours file; the reader reads none of them.
    let out = ok(&plan(&dir));
    assert_eq!(neighbour_rows(&dir).len() - held_n, 6, "{}", out);
    assert_eq!(rows(&dir).len(), before, "{}", out);
    assert!(has_line(&out, &format!("[new] Topic ({})", REL)), "{}", out);
    assert!(has_line(&out, &format!("\"{}\": heading", H1)), "{}", out);
    assert!(
        out.contains(&format!("  decision \"{}\": heading; no line", BASE_Q)),
        "{}",
        out
    );
    assert!(
        out.contains("  term \"widget\": heading; on the depends line"),
        "{}",
        out
    );
    assert!(
        lines_wanted(&out).contains(&(H1.to_string(), BASE_Q.to_string())),
        "{}",
        out
    );
    assert!(
        lines_wanted(&out).contains(&(H1.to_string(), "paint".to_string())),
        "{}",
        out
    );
    // Each neighbour on no line is a finding that carries the move.
    for h in [H1, H2] {
        assert!(
            out.lines()
                .any(|l| l.ends_with(&format!("; agent writes the missing line under \"{}\"", h))),
            "{}",
            out
        );
    }
    assert!(
        !out.contains("heading with its body"),
        "step 5 waits for the lines and the bodies: {}",
        out
    );
    assert!(
        !out.contains("neighbours,") && !out.contains(" read, "),
        "no counts: {}",
        out
    );

    // Approve refuses what is still missing, and names the move.
    let err = refused(&approve(&dir));
    assert!(
        err.contains(&format!(
            "is on no line; agent writes the missing line under \"{}\"",
            H1
        )),
        "{}",
        err
    );

    // Step 3: the lines. Step 4 has not come: no heading has a body.
    assert_eq!(write_lines(&dir, &out), 4);
    let out = ok(&plan(&dir));
    for h in [H1, H2] {
        assert!(
            names_move(&out, &format!("agent writes the body of \"{}\"", h)),
            "{}",
            out
        );
    }

    // A body under one heading only: the other is named, and no body is read
    // until it has its body.
    rewrite(&dir, &[(H1, body("one")), (H2, String::new())]);
    let out = ok(&plan(&dir));
    assert!(
        names_move(&out, &format!("agent writes the body of \"{}\"", H2)),
        "{}",
        out
    );
    assert!(out.contains(&format!("\"{}\" is [no body]", H2)), "{}", out);
    assert!(!out.contains("heading with its body"), "{}", out);

    // Step 4: the other body. Step 5: heading and body against the pool and
    // against the other question.
    rewrite(&dir, &[(H1, body("one")), (H2, body("two"))]);
    let before = rows(&dir).len();
    let out = ok(&plan(&dir));
    assert_eq!(rows(&dir).len() - before, 8, "{}", out);
    assert!(
        has_line(&out, &format!("\"{}\": heading with its body", H1)),
        "{}",
        out
    );
    assert!(
        out.contains(&format!("  question \"{}\": body unrelated; no line", H2)),
        "{}",
        out
    );
    assert!(
        lines_wanted(&out).contains(&(H1.to_string(), H2.to_string())),
        "{}",
        out
    );
    assert_eq!(write_lines(&dir, &out), 2);

    // Step 6: nothing is missing, and the owner approves. Approve needs
    // neither the model nor the reader.
    let out = ok(&plan(&dir));
    assert_eq!(
        out,
        format!("[new] Topic ({})\nowner runs: adr approve {}\n", REL, REL),
        "the status and the move, nothing else"
    );
    let again = rows(&dir).len();
    ok(&plan(&dir));
    assert_eq!(rows(&dir).len(), again, "a second run reads nothing");
    let out = approve_with(&dir, &[("ADR_NO_MODEL", "1"), ("ADR_NO_READER", "1")]);
    ok(&out);
}

// adr: 60341be0-fd05-4281-92d7-668584a8539a
#[test]
fn the_reader_reads_no_neighbour_of_a_heading() {
    let dir = repo();
    let before = reader_text(&dir);
    let held_n = neighbour_rows(&dir).len();
    two_headings(&dir);
    // The heading step runs on a machine with no reader: it asks none.
    let out = ok(&plan_with(&dir, &[("ADR_NO_READER", "1")]));
    assert_eq!(
        reader_text(&dir),
        before,
        "no row was added to the reader file: {}",
        out
    );
    // The neighbours are on record and printed with no word.
    assert_eq!(neighbour_rows(&dir).len() - held_n, 6, "{}", out);
    for h in [H1, H2] {
        assert!(has_line(&out, &format!("\"{}\": heading", h)), "{}", out);
    }
    assert!(
        out.contains(&format!("  decision \"{}\": heading; no line", BASE_Q)),
        "{}",
        out
    );
    assert!(
        out.contains("  term \"widget\": heading; on the depends line"),
        "{}",
        out
    );
    assert!(
        !out.contains("heading unrelated") && !out.contains("heading same"),
        "no word at the heading step: {}",
        out
    );
    // The model is still needed for it.
    let dir = repo();
    two_headings(&dir);
    let (want, next) = if cfg!(feature = "model") {
        (
            "the embedding model is not installed",
            "agent runs: adr install --planning",
        )
    } else {
        (
            "the embedding model is missing",
            "agent builds adr with the model feature",
        )
    };
    let err = refused(&plan_with(&dir, &[("ADR_NO_MODEL", "1")]));
    assert!(err.contains(want), "{}", err);
    assert!(err.contains(next), "{}", err);
    assert_eq!(neighbour_rows(&dir).len(), held_n, "nothing was written");
}

// adr: 60341be0-fd05-4281-92d7-668584a8539a
#[test]
fn the_neighbours_file_is_not_tracked_and_a_second_run_writes_neither_file() {
    let dir = repo();
    two_headings(&dir);
    let out = ok(&plan(&dir));
    assert!(dir.join(repo::NEIGHBOURS_PATH).is_file());
    let status = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let status = String::from_utf8_lossy(&status.stdout).to_string();
    assert!(!status.contains("neighbours.tsv"), "{}", status);
    // Nothing changed: a second run writes to neither file.
    write_lines(&dir, &out);
    ok(&plan(&dir));
    let (n, r) = (neighbours_text(&dir), reader_text(&dir));
    ok(&plan(&dir));
    ok(&plan(&dir));
    assert_eq!(neighbours_text(&dir), n);
    assert_eq!(reader_text(&dir), r);
}

/// The reader file written again with `edit` applied to the rows it picks.
fn edit_rows(dir: &Path, pick: impl Fn(&reader::Row) -> bool, edit: impl Fn(&mut reader::Row)) {
    let mut text = String::new();
    for mut r in rows(dir) {
        if pick(&r) {
            edit(&mut r);
        }
        text.push_str(&reader::row_line(&r));
        text.push('\n');
    }
    std::fs::write(dir.join("adrdom/state/reader.tsv"), text).unwrap();
}

/// Two questions with bodies, planned at both steps, every neighbour on a line.
fn planned(dir: &Path) {
    write(
        dir,
        &format!(
            "# Topic\n{}{}",
            section(H1, &body("one")),
            section(H2, &body("two"))
        ),
    );
    for _ in 0..3 {
        let out = ok(&plan(dir));
        write_lines(dir, &out);
    }
    assert!(has_line(
        &ok(&plan(dir)),
        &format!("owner runs: adr approve {}", REL)
    ));
}

// adr: 55a6a2d8-03af-4db9-a4c7-e2790e38357e
#[test]
fn rows_from_another_prompt_or_model_are_read_again() {
    let dir = repo();
    planned(&dir);
    let old = sha("the prompt before");

    // The neighbours of the headings are no reader rows: they stay as they are.
    let held = neighbours_text(&dir);

    // Every prompt changed: the body step of both questions is read again,
    // 4 body pairs each; the heading neighbours are not read.
    edit_rows(&dir, |_| true, |r| r.prompt_hash = old.clone());
    let before = rows(&dir).len();
    let out = ok(&plan(&dir));
    assert_eq!(rows(&dir).len() - before, 8, "{}", out);
    assert!(!has_line(&out, &format!("\"{}\": heading", H1)), "{}", out);
    assert!(
        has_line(&out, &format!("\"{}\": heading with its body", H2)),
        "{}",
        out
    );
    assert!(
        has_line(&out, &format!("owner runs: adr approve {}", REL)),
        "{}",
        out
    );

    // Only the plan prompt changed: only the rows of the other question are
    // read again; the rows at the current body prompt stay.
    let plan_hash = Kind::Plan.prompt_hash();
    edit_rows(
        &dir,
        |r| r.prompt_hash == plan_hash,
        |r| r.prompt_hash = old.clone(),
    );
    let before = rows(&dir).len();
    let out = ok(&plan(&dir));
    let made = &rows(&dir)[before..];
    assert_eq!(made.len(), 2, "{}", out);
    assert!(made
        .iter()
        .all(|r| r.prompt_hash == plan_hash && r.neighbour.starts_with("text:")));
    assert!(
        !has_line(&out, &format!("\"{}\": heading", H1))
            && !has_line(&out, &format!("\"{}\": heading", H2)),
        "{}",
        out
    );

    // Another model: every body and plan pair is read again.
    edit_rows(&dir, |_| true, |r| r.model = "an-older-model".to_string());
    let before = rows(&dir).len();
    let out = ok(&plan(&dir));
    let made = &rows(&dir)[before..];
    assert_eq!(made.len(), 8, "{}", out);
    assert!(made.iter().all(|r| r.model == reader::MODEL));
    // And nothing more on the next run.
    let again = rows(&dir).len();
    ok(&plan(&dir));
    assert_eq!(rows(&dir).len(), again);
    assert_eq!(
        neighbours_text(&dir),
        held,
        "the heading neighbours were never read again"
    );
}

// adr: d3be71e4-ce7b-43b0-86a8-e86868521f99
#[test]
fn a_step_needs_the_record_of_the_step_before() {
    let dir = repo();
    // Bodies everywhere and no plan run: approve refuses the missing rows.
    write(
        &dir,
        &format!(
            "# Topic\n{}{}",
            section(H1, &body("one")),
            section(H2, &body("two"))
        ),
    );
    let err = refused(&approve(&dir));
    assert!(
        err.contains(&format!(
            "\"{}\" has no neighbours on record for its heading; agent runs: adr plan",
            H1
        )),
        "{}",
        err
    );
    // The steps run in order: the body rows wait on the heading's neighbours.
    assert!(
        err.contains(&format!(
            "\"{}\" has no neighbours on record for its heading; agent runs: adr plan",
            H2
        )),
        "{}",
        err
    );
    assert!(
        !err.contains("has no reader rows for its heading with its body"),
        "{}",
        err
    );
    assert!(
        err.contains(&format!("agent runs: adr plan {}", REL)),
        "{}",
        err
    );

    // A step-2 neighbour on no line stops step 5: the neighbours are
    // recorded, the body rows are not made.
    let before = rows(&dir).len();
    let held_n = neighbour_rows(&dir).len();
    let out = ok(&plan(&dir));
    assert_eq!(neighbour_rows(&dir).len() - held_n, 6, "{}", out);
    assert_eq!(rows(&dir).len(), before, "{}", out);
    assert!(!lines_wanted(&out).is_empty(), "{}", out);
    assert!(!out.contains("heading with its body"), "{}", out);

    // Approve names the missing line.
    let err = refused(&approve(&dir));
    assert!(err.contains("is on no line"), "{}", err);

    // Once the lines are there, step 5 runs.
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    assert!(
        has_line_starting(&out, &format!("\"{}\": heading with its body", H1)),
        "{}",
        out
    );
    assert_eq!(rows(&dir).len() - before, 8, "{}", out);
}

// adr: d3be71e4-ce7b-43b0-86a8-e86868521f99
#[test]
fn a_heading_added_after_the_bodies_gets_its_neighbours_first() {
    let dir = repo();
    write(
        &dir,
        &format!(
            "# Topic\n{}{}",
            section(H1, &body("one")),
            section(H2, &body("two"))
        ),
    );
    // Plan to the end.
    for _ in 0..4 {
        let out = ok(&plan(&dir));
        if write_lines(&dir, &out) == 0 {
            break;
        }
    }
    let out = ok(&plan(&dir));
    assert!(
        has_line(&out, &format!("owner runs: adr approve {}", REL)),
        "{}",
        out
    );
    let settled = rows(&dir).len();
    let settled_neighbours = neighbour_rows(&dir).len();

    // A third heading, no body yet.
    let h3 = "Does the third thing glow?";
    write(&dir, &format!("{}{}", read(&dir), section(h3, "")));
    // Step 2 comes first, for that heading alone, with the bodies there: its
    // neighbours are recorded before any body is read.
    let out = ok(&plan(&dir));
    assert_eq!(
        neighbour_rows(&dir).len() - settled_neighbours,
        3,
        "{}",
        out
    );
    assert_eq!(rows(&dir).len(), settled, "no body was read: {}", out);
    assert!(has_line(&out, &format!("\"{}\": heading", h3)), "{}", out);
    assert!(!has_line(&out, &format!("\"{}\": heading", H1)), "{}", out);
    assert!(lines_wanted(&out).iter().any(|(h, _)| h == h3), "{}", out);
    assert!(
        out.lines()
            .any(|l| l.ends_with(&format!("; agent writes the missing line under \"{}\"", h3))),
        "{}",
        out
    );
    // Approve refuses it too.
    assert!(refused(&approve(&dir)).contains(h3));

    // Then the lines, then the move for the missing body.
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    assert!(
        names_move(&out, &format!("agent writes the body of \"{}\"", h3)),
        "{}",
        out
    );

    // Then the body, and step 5 for that question alone; the two others are
    // not read again from their side.
    rewrite(
        &dir,
        &[(H1, body("one")), (H2, body("two")), (h3, body("three"))],
    );
    let settled = rows(&dir).len();
    let out = ok(&plan(&dir));
    assert!(
        has_line(&out, &format!("\"{}\": heading with its body", h3)),
        "{}",
        out
    );
    assert!(
        !has_line(&out, &format!("\"{}\": heading with its body", H1)),
        "{}",
        out
    );
    assert_eq!(rows(&dir).len() - settled, 3 + 2, "{}", out);
    assert_eq!(
        neighbour_rows(&dir).len() - settled_neighbours,
        3,
        "{}",
        out
    );
    for _ in 0..3 {
        let out = ok(&plan(&dir));
        write_lines(&dir, &out);
    }
    let out = ok(&plan(&dir));
    assert!(
        has_line(&out, &format!("owner runs: adr approve {}", REL)),
        "{}",
        out
    );
}

// adr: d984e425-3e88-4756-88fb-18241f9f62a7
#[test]
fn the_plans_own_questions_are_neighbours_of_each_other_at_the_body_step() {
    let dir = repo();
    let h1 = "Does the first [word:same@text:] thing glow?".to_string();
    // The marker sits in the first question's text: the plan prompt's word
    // for the other question is `same`.
    write(
        &dir,
        &format!(
            "# Topic\n{}{}",
            section(&h1, &body("one")),
            section(H2, &body("two"))
        ),
    );
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    assert!(
        out.contains(&format!("  question \"{}\": body same; no line", H2)),
        "{}",
        out
    );

    let rf = record::load(&dir.join(REL)).unwrap();
    let text1 = record::body_text(&rf, &rf.sections[0]);
    let text2 = record::body_text(&rf, &rf.sections[1]);
    let key2 = format!("text:{}", sha(&text2));
    let key1 = format!("text:{}", sha(&text1));
    assert_eq!(record::body_hash(&rf, &rf.sections[1]), sha(&text2));
    let all = rows(&dir);
    let of_one: Vec<&reader::Row> = all.iter().filter(|r| r.text_hash == sha(&text1)).collect();
    // Four neighbours: the decision, two terms, and the other question.
    assert_eq!(of_one.len(), 4, "{:?}", of_one);
    let plan_row = of_one
        .iter()
        .find(|r| r.neighbour == key2)
        .expect("a row for the other question");
    assert_eq!(plan_row.word, "same");
    assert_eq!(plan_row.prompt_hash, Kind::Plan.prompt_hash());
    // The other question reads the first one from its side.
    assert!(all.iter().any(|r| r.text_hash == sha(&text2)
        && r.neighbour == key1
        && r.prompt_hash == Kind::Plan.prompt_hash()));
    // The pool neighbours of the body use the body prompt.
    assert!(of_one
        .iter()
        .filter(|r| !r.neighbour.starts_with("text:"))
        .all(|r| r.prompt_hash == Kind::Body.prompt_hash()));
    // A line for the other question settles it; its body changing leaves the
    // old key naming no question, and no line is wanted for it.
    write_lines(&dir, &out);
    assert!(has_line(
        &ok(&plan(&dir)),
        &format!("owner runs: adr approve {}", REL)
    ));
    write(
        &dir,
        &read(&dir).replace("problem two", "problem two changed"),
    );
    let out = ok(&plan(&dir));
    assert!(
        has_line(&out, &format!("\"{}\": heading with its body", H2)),
        "{}",
        out
    );
    assert!(
        !has_line(&out, &format!("\"{}\": heading with its body", h1)),
        "{}",
        out
    );
}

// adr: c6451ecc-17a9-447e-a9c9-fe9b35ec789c
#[test]
fn approve_refuses_a_neighbour_on_no_line_and_turns_the_names_into_identifiers() {
    let dir = repo();
    write(
        &dir,
        &format!(
            "# Topic\n{}{}",
            section(H1, &body("one")),
            section(H2, &body("two"))
        ),
    );
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    // One neighbour of the body step stays without a line.
    let wanted = lines_wanted(&out);
    assert_eq!(wanted.len(), 2);
    write(&dir, &add_unrelated(&read(&dir), &wanted[1..]));
    let err = refused(&approve(&dir));
    assert!(err.contains(&format!("\"{}\" is a neighbour of \"{}\" and is on no line; agent writes the missing line under \"{}\"", wanted[0].1, wanted[0].0, wanted[0].0)), "{}", err);
    assert!(!err.contains("no reader rows"), "{}", err);
    // The refusal changed nothing.
    let held = read(&dir);
    write(&dir, &add_unrelated(&held, &wanted[..1]));
    ok(&approve(&dir));

    // The names became identifiers: a term by its id, a question of the file
    // by its id, the decision by its id.
    let text = read(&dir);
    let ids = extract_ids(&text);
    let base_id = extract_ids(&std::fs::read_to_string(dir.join(BASE)).unwrap())[0].clone();
    let unrelated: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("unrelated: "))
        .collect();
    assert_eq!(unrelated.len(), 2);
    assert!(
        unrelated
            .iter()
            .all(|l| l.contains(PAINT_ID) && l.contains(&base_id)),
        "{:?}",
        unrelated
    );
    assert!(
        unrelated[0].contains(&ids[1]) && unrelated[1].contains(&ids[0]),
        "{:?} {:?}",
        unrelated,
        ids
    );
    assert!(
        !unrelated
            .iter()
            .any(|l| l.contains("paint") || l.contains("glow")),
        "{:?}",
        unrelated
    );
}

// adr: c6451ecc-17a9-447e-a9c9-fe9b35ec789c
#[test]
fn an_unrelated_entry_that_names_nothing_is_refused_and_a_dropped_id_passes() {
    let dir = repo();
    write(&dir, &format!("# Topic\n{}", section(H1, &body("one"))));
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    ok(&plan(&dir));
    let settled = read(&dir);

    write(
        &dir,
        &settled.replacen("unrelated: ", "unrelated: nothing at all, ", 1),
    );
    let err = refused(&approve(&dir));
    assert!(
        err.contains(
            "the unrelated entry \"nothing at all\" names no current decision, term or question"
        ),
        "{}",
        err
    );

    // A dropped question's id is a known one.
    let dropped = "aaaaaaaa-1111-2222-3333-444444444444";
    append_ledger_row(&dir, "dropped", dropped, &sha("gone"), "x y");
    write(
        &dir,
        &settled.replacen("unrelated: ", &format!("unrelated: {}, ", dropped), 1),
    );
    ok(&approve(&dir));
    assert!(read(&dir).contains(dropped));

    // A supersedes line names current decisions only: a dropped id is none.
    let dir = repo();
    write(
        &dir,
        &format!(
            "# Topic\n{}",
            section(H1, &body("one")).replacen(
                "depends: widget\n",
                &format!("depends: widget\nsupersedes: {}\n", dropped),
                1
            )
        ),
    );
    append_ledger_row(&dir, "dropped", dropped, &sha("gone"), "x y");
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    let err = refused(&approve(&dir));
    assert!(err.contains("which is not a current decision"), "{}", err);
}

// adr: e78b8297-fd89-4e09-9c3e-9eeb0fd0f654
#[test]
fn the_readers_word_never_stops_a_command() {
    let dir = repo();
    // `inconsistent` for the decision, `same` for the term paint and
    // `closes B` for the term widget at the body step (a letter the question
    // has): the agent's lines are the ruling.
    let id = extract_ids(&std::fs::read_to_string(dir.join(BASE)).unwrap())[0].clone();
    let b1 = body("one").replace(
        "problem one",
        &format!(
            "problem one [word:inconsistent@{}] [word:same@{}] [word:closes B@{}]",
            id, PAINT_ID, WIDGET_ID
        ),
    );
    write(&dir, &format!("# Topic\n{}", section(H1, &b1)));
    let out = ok(&plan(&dir));
    assert!(
        out.contains(&format!("decision \"{}\": heading; no line", BASE_Q)),
        "{}",
        out
    );
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    assert!(out.contains(": body inconsistent;"), "{}", out);
    assert!(out.contains(": body same;"), "{}", out);
    assert!(out.contains(": body closes B;"), "{}", out);
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    assert!(
        has_line(&out, &format!("owner runs: adr approve {}", REL)),
        "{}",
        out
    );
    let text = read(&dir);
    assert!(text.contains("unrelated: "), "{}", text);
    // The words never stop approve: the lines are the ruling.
    ok(&approve(&dir));
}

// adr: 21cdad59-4f0c-42a5-b4f3-6af946637df3
#[test]
fn planning_stops_without_the_model() {
    let dir = repo();
    two_headings(&dir);
    let before = rows(&dir).len();
    let (want, next) = if cfg!(feature = "model") {
        (
            "the embedding model is not installed",
            "agent runs: adr install --planning",
        )
    } else {
        (
            "the embedding model is missing",
            "agent builds adr with the model feature",
        )
    };
    let err = refused(&plan_with(&dir, &[("ADR_NO_MODEL", "1")]));
    assert!(err.contains(want), "{}", err);
    assert!(err.contains(next), "{}", err);
    assert_eq!(rows(&dir).len(), before, "nothing was read");
}

// adr: bd94a89f-7142-4dab-b757-6482a718a0a0
#[test]
fn planning_stops_without_the_reader() {
    let dir = repo();
    write(
        &dir,
        &format!(
            "# Topic\n{}{}",
            section(H1, &body("one")),
            section(H2, &body("two"))
        ),
    );
    // The heading step needs no reader; the body step is the first that does.
    let out = ok(&plan_with(&dir, &[("ADR_NO_READER", "1")]));
    write_lines(&dir, &out);
    let (before, held) = (rows(&dir).len(), reader_text(&dir));
    let err = refused(&plan_with(&dir, &[("ADR_NO_READER", "1")]));
    // The repository is installed for no agent: there is no program to name.
    assert!(
        err.contains("no agent is installed for this repository"),
        "{}",
        err
    );
    assert!(err.contains("agent installs an agent's files"), "{}", err);
    assert_eq!(rows(&dir).len(), before, "nothing was read");
    assert_eq!(reader_text(&dir), held, "nothing was written");
    // The other commands run without it.
    let shown = adr(&dir, &["show"], &[("ADR_NO_READER", "1")], "");
    assert!(shown.status.success(), "{}", stderr(&shown));
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
#[test]
fn plan_shows_a_set_letter_the_target_lacks_and_does_not_say_ready() {
    let dir = repo();
    let second = format!("\n### {}\ndepends: widget, {}=D\n{}", H2, H1, body("two"));
    write(
        &dir,
        &format!("# Topic\n{}{}", section(H1, &body("one")), second),
    );
    let mut last = String::new();
    for _ in 0..4 {
        last = ok(&plan(&dir));
        assert!(
            last.contains("under an answer set naming option D, which it does not have"),
            "{}",
            last
        );
        assert!(!last.contains("owner runs: adr approve"), "{}", last);
        let wanted = lines_wanted(&last);
        if wanted.is_empty() {
            break;
        }
        write_lines(&dir, &last);
    }
    assert!(!last.contains("owner runs: adr approve"), "{}", last);
}

// adr: d10dab89-1d42-4764-9d72-22d6fd6a9260
#[test]
fn the_body_step_records_closes_and_a_letter_and_refuses_a_word_outside_the_set() {
    let dir = repo();
    let id = extract_ids(&std::fs::read_to_string(dir.join(BASE)).unwrap())[0].clone();
    let b = body("one").replace(
        "problem one",
        &format!("problem one [word:closes B@{}]", id),
    );
    write(&dir, &format!("# Topic\n{}", section(H1, &b)));
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    assert!(
        out.contains(&format!("  decision \"{}\": body closes B; ", BASE_Q)),
        "{}",
        out
    );
    let row = rows(&dir)
        .into_iter()
        .find(|r| r.neighbour == id && r.prompt_hash == Kind::Body.prompt_hash())
        .expect("the body row");
    assert_eq!(row.word, "closes B");

    // A letter the question does not have, and `depends`: the run fails and
    // no row is made.
    for (marker, word) in [
        ("[word:closes C]", "closes C"),
        ("[word:depends]", "depends"),
    ] {
        let dir = repo();
        let b = body("one").replace("problem one", &format!("problem one {}", marker));
        write(&dir, &format!("# Topic\n{}", section(H1, &b)));
        let out = ok(&plan(&dir));
        write_lines(&dir, &out);
        let before = reader_text(&dir);
        let err = refused(&plan(&dir));
        assert!(err.contains(word), "{}", err);
        assert_eq!(reader_text(&dir), before, "{} left no row", word);
    }
}

// adr: 60341be0-fd05-4281-92d7-668584a8539a
#[test]
fn the_neighbours_file_deleted_brings_back_the_heading_step_only_where_a_body_is_missing() {
    let dir = repo();
    write(
        &dir,
        &format!(
            "# Topic\n{}{}",
            section(H1, &body("one")),
            section(H2, &body("two"))
        ),
    );
    for _ in 0..4 {
        let out = ok(&plan(&dir));
        write_lines(&dir, &out);
    }
    let out = ok(&plan(&dir));
    assert!(
        has_line(&out, &format!("owner runs: adr approve {}", REL)),
        "{}",
        out
    );
    let held = reader_text(&dir);

    // Every body step is done: the heading step does not come back.
    std::fs::remove_file(dir.join(repo::NEIGHBOURS_PATH)).unwrap();
    let out = ok(&plan_with(&dir, &[("ADR_NO_MODEL", "1")]));
    assert!(
        has_line(&out, &format!("owner runs: adr approve {}", REL)),
        "{}",
        out
    );
    assert!(!out.contains("has no neighbours on record"), "{}", out);
    assert!(
        !dir.join(repo::NEIGHBOURS_PATH).exists(),
        "nothing was written"
    );
    assert_eq!(reader_text(&dir), held);

    // A heading with no body: its heading step comes back.
    let h3 = "Does the third thing glow?";
    write(&dir, &format!("{}{}", read(&dir), section(h3, "")));
    let out = ok(&plan(&dir));
    assert!(neighbour_rows(&dir).len() >= 3, "{}", out);
    std::fs::remove_file(dir.join(repo::NEIGHBOURS_PATH)).unwrap();
    let shown = adr(&dir, &["show", dir.join(REL).to_str().unwrap()], &[], "");
    let shown = stdout(&shown);
    assert!(
        shown.lines().any(|l| l.contains(&format!(
            "\"{}\" has no neighbours on record for its heading",
            h3
        )) && l.ends_with(&format!("agent runs: adr plan {}", REL))),
        "{}",
        shown
    );
}
