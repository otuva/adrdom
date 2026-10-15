// The move an answer refusal prints works: the agent rewrites the supersedes line to add the missing headings,
// the owner approves, the agent asks again and the owner
// answers. (Follows the supersedes line names every current decision standing on its target
// (in every topic file, and through the decisions standing on those), and no
// heading of a plan stands on what the same plan supersedes. The guard runs
// at approve, ask and answer.)
#![allow(dead_code)]
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

/// An answered question of file `name` that stands on `target` (an id) by
/// identifier: the record of a question that names a decision of another
/// file, which approve no longer writes. Built from an answered ledger state,
/// as the other answered fixtures are.
fn old_dependent(dir: &Path, name: &str, heading: &str, target: &str, id: &str) {
    let section = format!(
        "### {}\ndepends: 99999999-9999-9999-9999-999999999999, {}\n\nid: {}\n{}",
        heading, target, id, BODY
    );
    let hash = mirror_section_hash(&section);
    append_ledger_row_at(dir, "asked", id, &hash, heading, "2024-01-01T00:00:00Z");
    append_ledger_row_at(dir, "answered", id, &hash, "A", "2024-01-01T00:00:01Z");
    write_record(dir, &rel(name), &format!("# Old\n\n{}\n**Answer:** A\n", section.trim_end()));
}

/// The case where approve passed (Q4 names Q1 only, Q1 had no dependents)
/// and Q1 then gains a dependent: Q7 in c.md, an answered question standing
/// on Q1 by identifier.
fn q1_gains_a_dependent_after_approve(dir: &Path, ask_q4_first: bool) -> PathBuf {
    let a = with_q1(dir);
    append(&a, &sec(Q4, "widget", Some(Q1), true));
    approve_ok(dir, "a");
    if ask_q4_first {
        let out = adr(dir, &["ask", a.to_str().unwrap()], &[], "");
        assert!(out.status.success(), "ask a failed: {}", stderr(&out));
    }
    old_dependent(dir, "c", Q7, &id_of(&a, Q1), "77777777-7777-7777-7777-777777777777");
    a
}

// adr: 796a2544-43e5-43db-b5d3-8a4e737ec9c6
#[test]
fn the_move_printed_by_a_refused_answer_leads_to_a_successful_answer() {
    let dir = setup();
    let a = q1_gains_a_dependent_after_approve(&dir, true);
    let id1 = id_of(&a, Q1);
    let id4 = id_of(&a, Q4);
    let out = adr_owner(&dir, &["answer", &id_at_version(&dir, &id4), "A"], "");
    assert_refused_naming(&out, &[Q7, "c.md", "agent rewrites the supersedes line of", "to add adrdom/decisions/c.md", "then runs: adr ask adrdom/decisions/a.md"]);

    let c = std::fs::read_to_string(&a).unwrap();
    let edited = c.replace(
        &format!("supersedes: {}\n", id1),
        &format!("supersedes: {}, {}\n", id1, Q7),
    );
    assert_ne!(c, edited);
    std::fs::write(&a, edited).unwrap();
    let out = approve(&dir, "a");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    let out = adr(&dir, &["ask", a.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}{}", stdout(&out), stderr(&out));
    let out = adr_owner(&dir, &["answer", &id_at_version(&dir, &id4), "A"], "");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
}
