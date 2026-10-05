// The supersedes line names every current decision standing on its target
// (in every topic file, and through the decisions standing on those), and no
// heading of a plan stands on what the same plan supersedes. The guard runs
// at approve, ask and answer.
mod common;

use common::*;
use std::path::{Path, PathBuf};

const BODY: &str = "\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";

const Q1: &str = "Is the widget red?";
const Q2: &str = "Is the widget glossy?";
const Q3: &str = "Is the widget polished?";
const Q4: &str = "Is the widget blue?";
const Q5: &str = "Is the widget hollow?";
const Q6: &str = "Is the widget heavy?";
const Q7: &str = "Is the widget light?";

fn sec(heading: &str, deps: &str, sup: Option<&str>, body: bool) -> String {
    let mut s = format!("\n### {}\ndepends: {}\n", heading, deps);
    if let Some(x) = sup {
        s.push_str(&format!("supersedes: {}\n", x));
    }
    if body {
        s.push_str(BODY);
    }
    s
}

fn rel(name: &str) -> String {
    format!("adrdom/decisions/{}.md", name)
}

fn append(path: &Path, text: &str) {
    let mut c = std::fs::read_to_string(path).unwrap();
    c.push_str(text);
    std::fs::write(path, c).unwrap();
}

fn id_of(path: &Path, heading: &str) -> String {
    let c = std::fs::read_to_string(path).unwrap();
    let mut lines = c.lines().skip_while(|l| *l != format!("### {}", heading));
    assert!(lines.next().is_some(), "no heading {} in {}", heading, c);
    for l in lines {
        if let Some(id) = l.strip_prefix("id: ") {
            return id.trim().to_string();
        }
        assert!(!l.starts_with("### "), "no id under {}: {}", heading, c);
    }
    panic!("no id under {}: {}", heading, c);
}

fn approve(dir: &Path, name: &str) -> std::process::Output {
    adr_owner(dir, &["approve", dir.join(rel(name)).to_str().unwrap()], "")
}

fn approve_ok(dir: &Path, name: &str) {
    let out = approve(dir, name);
    assert!(out.status.success(), "approve {} failed: {}", name, stderr(&out));
}

/// asks the file, answers `heading` A.
fn ask_answer(dir: &Path, name: &str, heading: &str) {
    let p = dir.join(rel(name));
    let out = adr(dir, &["ask", p.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask {} failed: {}", name, stderr(&out));
    let id = id_of(&p, heading);
    let out = adr_answer(dir, &id, "A");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
}

fn setup() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    dir
}

/// a.md with Q1 answered.
fn with_q1(dir: &Path) -> PathBuf {
    let p = write_record(dir, &rel("a"), &format!("# A\n{}", sec(Q1, "widget", None, true)));
    approve_ok(dir, "a");
    ask_answer(dir, "a", Q1);
    p
}

fn assert_refused_naming(out: &std::process::Output, names: &[&str]) {
    assert!(!out.status.success(), "should have refused: {}{}", stdout(out), stderr(out));
    let err = stderr(out);
    for n in names {
        assert!(err.contains(n), "stderr should name {:?}: {}", n, err);
    }
}

// adr: 796a2544-43e5-43db-b5d3-8a4e737ec9c6
#[test]
fn approve_refuses_a_supersedes_line_that_leaves_out_the_chain_above_the_target() {
    let dir = setup();
    let a = with_q1(&dir);
    append(&a, &sec(Q2, &format!("widget, {}", Q1), None, true));
    approve_ok(&dir, "a");
    ask_answer(&dir, "a", Q2);
    append(&a, &sec(Q3, &format!("widget, {}", Q2), None, true));
    approve_ok(&dir, "a");
    ask_answer(&dir, "a", Q3);

    append(&a, &sec(Q4, "widget", Some(Q1), true));
    plan_record(&dir, &rel("a"));
    let before = std::fs::read_to_string(&a).unwrap();
    let out = approve(&dir, "a");
    assert_refused_naming(&out, &[Q2, Q3]);
    assert_eq!(std::fs::read_to_string(&a).unwrap(), before, "a refused approve writes nothing");

    let full = before.replace(
        &format!("supersedes: {}\n", Q1),
        &format!("supersedes: {}, {}, {}\n", Q1, Q2, Q3),
    );
    assert_ne!(full, before);
    std::fs::write(&a, full).unwrap();
    let out = approve(&dir, "a");
    assert!(out.status.success(), "naming all three must pass: {}", stderr(&out));
}

// adr: 796a2544-43e5-43db-b5d3-8a4e737ec9c6
#[test]
fn approve_refuses_a_supersedes_line_that_leaves_out_a_dependent_in_another_file() {
    let dir = setup();
    let a = with_q1(&dir);
    let _b = write_record(&dir, &rel("b"), &format!("# B\n{}", sec(Q5, &format!("widget, {}", Q1), None, true)));
    approve_ok(&dir, "b");
    ask_answer(&dir, "b", Q5);

    append(&a, &sec(Q4, "widget", Some(Q1), true));
    let out = approve(&dir, "a");
    assert_refused_naming(&out, &[Q5, "b.md"]);

    let p = dir.join(rel("a"));
    let c = std::fs::read_to_string(&p).unwrap();
    std::fs::write(
        &p,
        c.replace(&format!("supersedes: {}\n", Q1), &format!("supersedes: {}, {}\n", Q1, Q5)),
    )
    .unwrap();
    let out = approve(&dir, "a");
    assert!(out.status.success(), "naming q1 and q5 must pass: {}", stderr(&out));
}

// adr: 796a2544-43e5-43db-b5d3-8a4e737ec9c6
#[test]
fn approve_refuses_a_heading_standing_on_what_the_same_plan_supersedes() {
    let dir = setup();
    let a = with_q1(&dir);
    append(&a, &sec(Q4, "widget", Some(Q1), true));
    append(&a, &sec(Q6, &format!("widget, {}", Q1), None, true));
    let out = approve(&dir, "a");
    assert_refused_naming(&out, &[Q6]);
}

// adr: 796a2544-43e5-43db-b5d3-8a4e737ec9c6
#[test]
fn approve_refuses_a_heading_standing_on_what_it_supersedes_itself() {
    let dir = setup();
    let a = with_q1(&dir);
    append(&a, &sec(Q4, &format!("widget, {}", Q1), Some(Q1), true));
    let out = approve(&dir, "a");
    assert_refused_naming(&out, &[Q4]);
}

/// The case where approve passed (Q4 names Q1 only, Q1 had no dependents)
/// and Q1 then gains a dependent: Q7 in c.md, approved and answered.
fn q1_gains_a_dependent_after_approve(dir: &Path, ask_q4_first: bool) -> PathBuf {
    let a = with_q1(dir);
    append(&a, &sec(Q4, "widget", Some(Q1), true));
    approve_ok(dir, "a");
    if ask_q4_first {
        let out = adr(dir, &["ask", a.to_str().unwrap()], &[], "");
        assert!(out.status.success(), "ask a failed: {}", stderr(&out));
    }
    write_record(dir, &rel("c"), &format!("# C\n{}", sec(Q7, &format!("widget, {}", Q1), None, true)));
    approve_ok(dir, "c");
    ask_answer(dir, "c", Q7);
    a
}

// adr: 796a2544-43e5-43db-b5d3-8a4e737ec9c6
#[test]
fn ask_refuses_a_replacement_whose_target_gained_a_dependent_after_approve() {
    let dir = setup();
    let a = q1_gains_a_dependent_after_approve(&dir, false);
    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let out = adr(&dir, &["ask", a.to_str().unwrap()], &[], "");
    assert_refused_naming(&out, &[Q7]);
    assert_eq!(
        std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap(),
        ledger_before
    );
}

// adr: 796a2544-43e5-43db-b5d3-8a4e737ec9c6
#[test]
fn answer_refuses_a_replacement_whose_target_gained_a_dependent_after_ask() {
    let dir = setup();
    let a = q1_gains_a_dependent_after_approve(&dir, true);
    let id4 = id_of(&a, Q4);
    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let file_before = std::fs::read_to_string(&a).unwrap();
    let out = adr_owner(&dir, &["answer", &id_at_version(&dir, &id4), "A"], "");
    assert_refused_naming(&out, &[Q7]);
    assert_eq!(
        std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap(),
        ledger_before,
        "a refused answer writes no ledger row"
    );
    assert_eq!(std::fs::read_to_string(&a).unwrap(), file_before);
}

// adr: 796a2544-43e5-43db-b5d3-8a4e737ec9c6
#[test]
fn check_prints_the_tree_refusal_once_for_an_approved_unasked_replacement_whose_target_gained_a_dependent() {
    let dir = setup();
    let _a = q1_gains_a_dependent_after_approve(&dir, false);
    let out = adr(&dir, &["check"], &[], "");
    let all = format!("{}{}", stdout(&out), stderr(&out));
    assert!(!out.status.success(), "check should fail: {}", all);
    for n in [Q7, "c.md", "agent adds them to the supersedes line"] {
        assert!(all.contains(n), "check output should name {:?}: {}", n, all);
    }
    assert_eq!(
        all.matches("supersedes a decision that current decisions stand on").count(),
        1,
        "the tree refusal is printed once: {}",
        all
    );
}
