// `adr plan <file>` and the lines: the planning steps, the records they
// leave in adrdom/state/reader.tsv, and what approve refuses without them.
// The embedding model and the reader are the fakes; nothing here goes
// through `adr_owner`, whose approve would plan first, so every refusal is
// the tool's own.

mod common;

use adr::reader::{self, Kind};
use adr::record;
use common::*;
use std::path::{Path, PathBuf};
use std::process::Output;

const REL: &str = "adrdom/decisions/topic.md";
const BASE: &str = "adrdom/decisions/base.md";
const BASE_Q: &str = "Does the base hold?";
const H1: &str = "Does the first thing glow?";
const H2: &str = "Does the second thing glow?";
const PAINT_ID: &str = "99999991-9999-9999-9999-999999999999";

fn body(n: &str) -> String {
    format!(
        "\nProblem: problem {n}.\n\nStandard: standard {n}.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
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
    let base = write_record(&dir, BASE, &format!("# Base\n{}", section(BASE_Q, &body("base widget"))));
    approve_record(&dir, BASE);
    assert!(adr(&dir, &["ask", base.to_str().unwrap()], &[], "").status.success());
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

/// Writes each neighbour `adr plan` printed as on no line, as an
/// `unrelated:` entry under its heading.
fn write_lines(dir: &Path, plan_stdout: &str) -> usize {
    let wanted = lines_wanted(plan_stdout);
    write(dir, &add_unrelated(&read(dir), &wanted));
    wanted.len()
}

fn ok(out: &Output) -> String {
    assert!(out.status.success(), "stdout={} stderr={}", stdout(out), stderr(out));
    stdout(out)
}

fn refused(out: &Output) -> String {
    assert!(!out.status.success(), "should have refused: {}", stdout(out));
    stderr(out)
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
                rest[..end].lines().find(|l| l.starts_with("unrelated: ")).map(String::from)
            })
            .map(|l| format!("{}\n", l))
            .unwrap_or_default();
        out.push_str(&format!("\n### {}\ndepends: widget\n{}{}", heading, line, body));
    }
    write(dir, &out);
}

/// A fresh topic file with two headings and no body.
fn two_headings(dir: &Path) {
    write(dir, &format!("# Topic\n{}{}", section(H1, ""), section(H2, "")));
}

// adr: 46fc51ff-66b9-4841-a953-ae0d4c2fc424
#[test]
fn the_steps_in_order_from_headings_to_approval() {
    let dir = repo();
    let before = rows(&dir).len();
    two_headings(&dir);

    // Step 2: the neighbours of each heading, ten at most, the current
    // decision and both terms; the heading's own depends line holds one term.
    let out = ok(&plan(&dir));
    assert_eq!(rows(&dir).len() - before, 6, "{}", out);
    assert!(out.contains(&format!("\"{}\": heading read, 3 neighbours", H1)), "{}", out);
    assert!(out.contains(&format!("  decision \"{}\": unrelated; no line", BASE_Q)), "{}", out);
    assert!(out.contains("  term \"widget\": unrelated; on a line"), "{}", out);
    assert!(out.contains(&format!("line missing: decision \"{}\" under \"{}\"", BASE_Q, H1)), "{}", out);
    assert!(out.contains("line missing: term \"paint\" under"), "{}", out);
    assert!(out.contains("next: agent writes the lines above, then runs: adr plan "), "{}", out);
    assert!(!out.contains("heading with its body"), "step 5 waits for the lines and the bodies: {}", out);

    // Approve refuses what is still missing, and names the move.
    let err = refused(&approve(&dir));
    assert!(err.contains(&format!("agent writes a depends, supersedes or unrelated line for \"paint\" under \"{}\"", H1)), "{}", err);

    // Step 3: the lines. Step 4 has not come: no heading has a body.
    assert_eq!(write_lines(&dir, &out), 4);
    let out = ok(&plan(&dir));
    assert!(out.contains("next: agent writes the bodies, then runs: adr plan "), "{}", out);

    // Bodies under one heading only are refused, and the other is named.
    rewrite(&dir, &[(H1, body("one")), (H2, String::new())]);
    let err = refused(&plan(&dir));
    assert!(err.contains(&format!("have none: \"{}\"", H2)), "{}", err);
    assert!(!err.contains(H1), "{}", err);

    // Step 4: the other body. Step 5: heading and body against the pool and
    // against the other question.
    rewrite(&dir, &[(H1, body("one")), (H2, body("two"))]);
    let before = rows(&dir).len();
    let out = ok(&plan(&dir));
    assert_eq!(rows(&dir).len() - before, 8, "{}", out);
    assert!(out.contains(&format!("\"{}\": heading with its body read, 4 neighbours", H1)), "{}", out);
    assert!(out.contains(&format!("  question \"{}\": unrelated; no line", H2)), "{}", out);
    assert!(out.contains(&format!("line missing: question \"{}\" under \"{}\"", H2, H1)), "{}", out);
    assert_eq!(write_lines(&dir, &out), 2);

    // Step 6: nothing is missing, and the owner approves. Approve needs
    // neither the model nor the reader.
    let out = ok(&plan(&dir));
    assert!(out.contains("every question has its rows and every neighbour is on a line"), "{}", out);
    assert!(out.contains(&format!("next: owner runs: adr approve {}", REL)), "{}", out);
    let again = rows(&dir).len();
    ok(&plan(&dir));
    assert_eq!(rows(&dir).len(), again, "a second run reads nothing");
    let out = approve_with(&dir, &[("ADR_NO_MODEL", "1"), ("ADR_NO_READER", "1")]);
    ok(&out);
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

/// Two questions with bodies, read at both steps, every neighbour on a line.
fn planned(dir: &Path) {
    write(dir, &format!("# Topic\n{}{}", section(H1, &body("one")), section(H2, &body("two"))));
    for _ in 0..3 {
        let out = ok(&plan(dir));
        write_lines(dir, &out);
    }
    assert!(ok(&plan(dir)).contains("next: owner runs: adr approve"));
}

// adr: 08a1b488-32fc-4dc7-b4b3-93a6e768725f
#[test]
fn rows_from_another_prompt_or_model_are_read_again() {
    let dir = repo();
    planned(&dir);
    let old = sha("the prompt before");

    // Every prompt changed: both steps of both questions are read again,
    // 3 heading pairs and 4 body pairs each.
    edit_rows(&dir, |_| true, |r| r.prompt_hash = old.clone());
    let before = rows(&dir).len();
    let out = ok(&plan(&dir));
    assert_eq!(rows(&dir).len() - before, 6 + 8, "{}", out);
    assert!(out.contains(&format!("\"{}\": heading read", H1)), "{}", out);
    assert!(out.contains(&format!("\"{}\": heading with its body read", H2)), "{}", out);
    assert!(out.contains("next: owner runs: adr approve"), "{}", out);

    // Only the plan prompt changed: only the rows of the other question are
    // read again; the rows at the current body prompt stay.
    let plan_hash = Kind::Plan.prompt_hash();
    edit_rows(&dir, |r| r.prompt_hash == plan_hash, |r| r.prompt_hash = old.clone());
    let before = rows(&dir).len();
    let out = ok(&plan(&dir));
    let made = &rows(&dir)[before..];
    assert_eq!(made.len(), 2, "{}", out);
    assert!(made.iter().all(|r| r.prompt_hash == plan_hash && r.neighbour.starts_with("text:")));
    assert!(!out.contains("heading read"), "{}", out);

    // Another model: everything is read again.
    edit_rows(&dir, |_| true, |r| r.model = "an-older-model".to_string());
    let before = rows(&dir).len();
    let out = ok(&plan(&dir));
    let made = &rows(&dir)[before..];
    assert_eq!(made.len(), 6 + 8, "{}", out);
    assert!(made.iter().all(|r| r.model == reader::MODEL));
    // And nothing more on the next run.
    let again = rows(&dir).len();
    ok(&plan(&dir));
    assert_eq!(rows(&dir).len(), again);
}

// adr: d3be71e4-ce7b-43b0-86a8-e86868521f99
#[test]
fn a_step_needs_the_record_of_the_step_before() {
    let dir = repo();
    // Bodies everywhere and no plan run: approve refuses the missing rows.
    write(&dir, &format!("# Topic\n{}{}", section(H1, &body("one")), section(H2, &body("two"))));
    let err = refused(&approve(&dir));
    assert!(err.contains(&format!("\"{}\" has no reader rows for its heading;", H1)), "{}", err);
    assert!(err.contains(&format!("\"{}\" has no reader rows for its heading with its body;", H2)), "{}", err);
    assert!(err.contains(&format!("agent runs: adr plan {}", REL)), "{}", err);

    // A step-2 neighbour on no line stops step 5: the heading rows are made,
    // the body rows are not.
    let before = rows(&dir).len();
    let out = ok(&plan(&dir));
    let made = &rows(&dir)[before..];
    assert_eq!(made.len(), 6, "{}", out);
    assert!(made.iter().all(|r| r.prompt_hash == Kind::Heading.prompt_hash()));
    assert!(out.contains("line missing: "), "{}", out);
    assert!(!out.contains("heading with its body"), "{}", out);

    // Approve names the missing line.
    let err = refused(&approve(&dir));
    assert!(err.contains("is on no line"), "{}", err);

    // Once the lines are there, step 5 runs.
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    assert!(out.contains("heading with its body read"), "{}", out);
}

// adr: d3be71e4-ce7b-43b0-86a8-e86868521f99
#[test]
fn a_heading_added_after_the_bodies_gets_its_neighbours_first() {
    let dir = repo();
    write(&dir, &format!("# Topic\n{}{}", section(H1, &body("one")), section(H2, &body("two"))));
    // Plan to the end.
    for _ in 0..4 {
        let out = ok(&plan(&dir));
        if write_lines(&dir, &out) == 0 {
            break;
        }
    }
    let out = ok(&plan(&dir));
    assert!(out.contains("next: owner runs: adr approve"), "{}", out);
    let settled = rows(&dir).len();

    // A third heading, no body yet.
    let h3 = "Does the third thing glow?";
    write(&dir, &format!("{}{}", read(&dir), section(h3, "")));
    // Step 2 comes first, for that heading alone, with the bodies there.
    let out = ok(&plan(&dir));
    assert_eq!(rows(&dir).len() - settled, 3, "{}", out);
    assert!(out.contains(&format!("\"{}\": heading read, 3 neighbours", h3)), "{}", out);
    assert!(!out.contains(&format!("\"{}\": heading read", H1)), "{}", out);
    assert!(out.contains(&format!("under \"{}\"", h3)), "{}", out);
    assert!(out.contains("next: agent writes the lines above"), "{}", out);
    // Approve refuses it too.
    assert!(refused(&approve(&dir)).contains(h3));

    // Then the lines, then the refusal for the missing body.
    write_lines(&dir, &out);
    let err = refused(&plan(&dir));
    assert!(err.contains(&format!("have none: \"{}\"", h3)), "{}", err);

    // Then the body, and step 5 for that question alone; the two others are
    // not read again from their side.
    rewrite(&dir, &[(H1, body("one")), (H2, body("two")), (h3, body("three"))]);
    let settled = rows(&dir).len();
    let out = ok(&plan(&dir));
    assert!(out.contains(&format!("\"{}\": heading with its body read", h3)), "{}", out);
    assert!(!out.contains(&format!("\"{}\": heading with its body read", H1)), "{}", out);
    assert_eq!(rows(&dir).len() - settled, 3 + 2, "{}", out);
    for _ in 0..3 {
        let out = ok(&plan(&dir));
        write_lines(&dir, &out);
    }
    let out = ok(&plan(&dir));
    assert!(out.contains("next: owner runs: adr approve"), "{}", out);
}

// adr: 4838baa4-a5df-49e7-929c-ed63b5adaf4e
#[test]
fn the_plans_own_questions_are_neighbours_of_each_other_at_the_body_step() {
    let dir = repo();
    let h1 = "Does the first [word:depends@text:] thing glow?".to_string();
    // The marker sits in the first question's text: the plan prompt's word
    // for the other question is `depends`.
    write(&dir, &format!("# Topic\n{}{}", section(&h1, &body("one")), section(H2, &body("two"))));
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    assert!(out.contains(&format!("  question \"{}\": depends; no line", H2)), "{}", out);

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
    let plan_row = of_one.iter().find(|r| r.neighbour == key2).expect("a row for the other question");
    assert_eq!(plan_row.word, "depends");
    assert_eq!(plan_row.prompt_hash, Kind::Plan.prompt_hash());
    // The other question reads the first one from its side.
    assert!(all.iter().any(|r| r.text_hash == sha(&text2) && r.neighbour == key1 && r.prompt_hash == Kind::Plan.prompt_hash()));
    // The pool neighbours of the body use the body prompt.
    assert!(of_one.iter().filter(|r| !r.neighbour.starts_with("text:")).all(|r| r.prompt_hash == Kind::Body.prompt_hash()));
    // A line for the other question settles it; its body changing leaves the
    // old key naming no question, and no line is wanted for it.
    write_lines(&dir, &out);
    assert!(ok(&plan(&dir)).contains("next: owner runs: adr approve"));
    write(&dir, &read(&dir).replace("problem two", "problem two changed"));
    let out = ok(&plan(&dir));
    assert!(out.contains(&format!("\"{}\": heading with its body read", H2)), "{}", out);
    assert!(!out.contains(&format!("\"{}\": heading with its body read", h1)), "{}", out);
}

// adr: c6451ecc-17a9-447e-a9c9-fe9b35ec789c
#[test]
fn approve_refuses_a_neighbour_on_no_line_and_turns_the_names_into_identifiers() {
    let dir = repo();
    write(&dir, &format!("# Topic\n{}{}", section(H1, &body("one")), section(H2, &body("two"))));
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    // One neighbour of the body step stays without a line.
    let wanted = lines_wanted(&out);
    assert_eq!(wanted.len(), 2);
    write(&dir, &add_unrelated(&read(&dir), &wanted[1..]));
    let err = refused(&approve(&dir));
    assert!(err.contains(&format!("agent writes a depends, supersedes or unrelated line for \"{}\" under \"{}\"", wanted[0].1, wanted[0].0)), "{}", err);
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
    let unrelated: Vec<&str> = text.lines().filter(|l| l.starts_with("unrelated: ")).collect();
    assert_eq!(unrelated.len(), 2);
    assert!(unrelated.iter().all(|l| l.contains(PAINT_ID) && l.contains(&base_id)), "{:?}", unrelated);
    assert!(unrelated[0].contains(&ids[1]) && unrelated[1].contains(&ids[0]), "{:?} {:?}", unrelated, ids);
    assert!(!unrelated.iter().any(|l| l.contains("paint") || l.contains("glow")), "{:?}", unrelated);
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

    write(&dir, &settled.replacen("unrelated: ", "unrelated: nothing at all, ", 1));
    let err = refused(&approve(&dir));
    assert!(err.contains("the unrelated entry \"nothing at all\" names no current decision, term or question"), "{}", err);

    // A dropped question's id is a known one.
    let dropped = "aaaaaaaa-1111-2222-3333-444444444444";
    append_ledger_row(&dir, "dropped", dropped, &sha("gone"), "x y");
    write(&dir, &settled.replacen("unrelated: ", &format!("unrelated: {}, ", dropped), 1));
    ok(&approve(&dir));
    assert!(read(&dir).contains(dropped));

    // A supersedes line names current decisions only: a dropped id is none.
    let dir = repo();
    write(&dir, &format!("# Topic\n{}", section(H1, &body("one")).replacen("depends: widget\n", &format!("depends: widget\nsupersedes: {}\n", dropped), 1)));
    append_ledger_row(&dir, "dropped", dropped, &sha("gone"), "x y");
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    write_lines(&dir, &out);
    let err = refused(&approve(&dir));
    assert!(err.contains("which is not a current decision"), "{}", err);
}

// adr: 902bd279-e6c1-4a21-9637-a3a43f360bae
#[test]
fn the_readers_word_never_stops_a_command() {
    let dir = repo();
    // `same` for every neighbour of the heading, `inconsistent` for every
    // neighbour of the body: the agent's lines are the ruling.
    let h1 = "Does the first [word:same] thing glow?".to_string();
    let b1 = body("one").replace("problem one", "problem one [word:inconsistent@-]");
    write(&dir, &format!("# Topic\n{}", section(&h1, &b1)));
    let out = ok(&plan(&dir));
    assert!(out.contains("decision \"Does the base hold?\": same; no line"), "{}", out);
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    assert!(out.contains(": inconsistent;"), "{}", out);
    write_lines(&dir, &out);
    let out = ok(&plan(&dir));
    assert!(out.contains("next: owner runs: adr approve"), "{}", out);
    let text = read(&dir);
    assert!(text.contains("unrelated: "), "{}", text);
    // A `same` word with an unrelated line passes approve.
    ok(&approve(&dir));
}

// adr: 21cdad59-4f0c-42a5-b4f3-6af946637df3
#[test]
fn planning_stops_without_the_model() {
    let dir = repo();
    two_headings(&dir);
    let before = rows(&dir).len();
    let err = refused(&plan_with(&dir, &[("ADR_NO_MODEL", "1")]));
    assert!(err.contains("the embedding model is missing"), "{}", err);
    assert!(err.contains("agent fetches it"), "{}", err);
    assert_eq!(rows(&dir).len(), before, "nothing was read");
}

// adr: 3dba486c-a399-4ce3-a794-e38c4f733403
#[test]
fn planning_stops_without_the_reader() {
    let dir = repo();
    two_headings(&dir);
    let before = rows(&dir).len();
    let err = refused(&plan_with(&dir, &[("ADR_NO_READER", "1")]));
    assert!(err.contains("the claude program is missing"), "{}", err);
    assert_eq!(rows(&dir).len(), before, "nothing was read");
    // The other commands run without it.
    let shown = adr(&dir, &["show"], &[("ADR_NO_READER", "1")], "");
    assert!(shown.status.success(), "{}", stderr(&shown));
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
#[test]
fn plan_shows_a_set_letter_the_target_lacks_and_does_not_say_ready() {
    let dir = repo();
    let second = format!("\n### {}\ndepends: widget, {}=D\n{}", H2, H1, body("two"));
    write(&dir, &format!("# Topic\n{}{}", section(H1, &body("one")), second));
    let mut last = String::new();
    for _ in 0..4 {
        last = ok(&plan(&dir));
        assert!(last.contains(&format!("names option D which \"{}\" does not have", H1)), "{}", last);
        assert!(!last.contains("owner runs: adr approve"), "{}", last);
        let wanted = lines_wanted(&last);
        if wanted.is_empty() {
            break;
        }
        write_lines(&dir, &last);
    }
    assert!(!last.contains("owner runs: adr approve"), "{}", last);
}
