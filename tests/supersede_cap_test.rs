// A decision named on the supersedes line of a question not yet answered is
// left out of the count of the questions standing on one question (here, on
// terms alone), so the cap (4) holds by what stands there once the
// replacement is answered.
mod common;

use common::*;
use std::path::{Path, PathBuf};

const BODY: &str = "\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";

fn h(n: usize) -> String {
    format!("Is widget option {} chosen?", n)
}

fn sec(heading: &str, sup: Option<&str>, body: bool) -> String {
    let mut s = format!("\n### {}\ndepends: widget\n", heading);
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
    }
    panic!("no id under {}: {}", heading, c);
}

fn approve(dir: &Path, name: &str) -> std::process::Output {
    adr_owner(dir, &["approve", dir.join(rel(name)).to_str().unwrap()], "")
}

fn approve_ok(dir: &Path, name: &str) {
    let out = approve(dir, name);
    assert!(
        out.status.success(),
        "approve {} failed: {}",
        name,
        stderr(&out)
    );
}

fn cap_refusal(out: &std::process::Output) -> bool {
    stderr(out).contains("the most is 4") || stdout(out).contains("the most is 4")
}

/// a.md with four questions on terms alone, approved, asked and answered A.
fn full_group(dir: &Path) -> PathBuf {
    assert!(adr(dir, &["install"], &[], "").status.success());
    approve_minimal_terms(dir, "widget");
    let mut c = String::from("# A\n");
    for n in 1..=4 {
        c.push_str(&sec(&h(n), None, true));
    }
    let p = write_record(dir, &rel("a"), &c);
    approve_ok(dir, "a");
    for n in 1..=4 {
        let out = adr_answer(dir, &id_of(&p, &h(n)), "A");
        assert!(out.status.success(), "answer failed: {}", stderr(&out));
    }
    p
}

// adr: 9c1b476c-9296-444b-a51c-e2957ad40ff5
#[test]
fn a_fifth_heading_superseding_one_of_four_passes_approve_and_check() {
    let dir = init_repo();
    let a = full_group(&dir);
    append(&a, &sec(&h(5), Some(&h(1)), true));
    let out = approve(&dir, "a");
    assert!(
        out.status.success(),
        "approve should pass the cap: {}",
        stderr(&out)
    );
    let chk = adr(&dir, &["check"], &[], "");
    assert!(
        !cap_refusal(&chk),
        "check printed a cap refusal: {}{}",
        stdout(&chk),
        stderr(&chk)
    );
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn a_fifth_heading_superseding_nothing_still_fails_the_cap() {
    let dir = init_repo();
    let a = full_group(&dir);
    append(&a, &sec(&h(5), None, true));
    let out = approve(&dir, "a");
    assert!(
        !out.status.success(),
        "approve should refuse: {}",
        stdout(&out)
    );
    assert!(cap_refusal(&out), "{}", stderr(&out));
}

#[test]
fn two_replacements_of_one_decision_in_a_full_group_fail_the_cap() {
    let dir = init_repo();
    let a = full_group(&dir);
    append(&a, &sec(&h(5), Some(&h(1)), true));
    append(&a, &sec(&h(6), Some(&h(1)), true));
    let out = approve(&dir, "a");
    assert!(
        !out.status.success(),
        "approve should refuse: {}",
        stdout(&out)
    );
    assert!(cap_refusal(&out), "{}", stderr(&out));
}

// adr: 9c1b476c-9296-444b-a51c-e2957ad40ff5
#[test]
fn a_refused_replacement_returns_the_old_decision_to_the_count() {
    let dir = init_repo();
    let a = full_group(&dir);
    append(&a, &sec(&h(5), Some(&h(1)), true));
    approve_ok(&dir, "a");
    let out = adr_answer(&dir, &id_of(&a, &h(5)), "refuse");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));

    append(&a, &sec(&h(6), None, true));
    let out = approve(&dir, "a");
    assert!(
        !out.status.success(),
        "approve should refuse: {}",
        stdout(&out)
    );
    assert!(cap_refusal(&out), "{}", stderr(&out));
}

#[test]
fn an_open_replacement_in_another_file_frees_a_slot_in_the_full_group() {
    let dir = init_repo();
    let a = full_group(&dir);
    write_record(
        &dir,
        &rel("b"),
        &format!("# B\n{}", sec(&h(9), Some(&h(1)), true)),
    );
    approve_ok(&dir, "b");
    append(&a, &sec(&h(5), None, true));
    let out = approve(&dir, "a");
    assert!(out.status.success(), "a.md counts 3 + 1: {}", stderr(&out));
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
// adr: 9c1b476c-9296-444b-a51c-e2957ad40ff5
//
// a.md holds four answered questions and a fifth approved while an open
// question of b.md replaced one of the four. When b.md's question is
// refused, the old decision is back in the count: a.md's group holds five
// and a new approve stops it on the cap, though a.md's plan is still approved.
#[test]
fn approve_stops_a_group_that_a_refused_replacement_in_another_file_took_over_the_cap() {
    let dir = init_repo();
    let a = full_group(&dir);
    let b = write_record(
        &dir,
        &rel("b"),
        &format!("# B\n{}", sec(&h(9), Some(&h(1)), true)),
    );
    approve_ok(&dir, "b");
    append(&a, &sec(&h(5), None, true));
    approve_ok(&dir, "a");
    let out = adr_answer(&dir, &id_of(&b, &h(9)), "refuse");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));

    let out = approve(&dir, "a");
    assert!(
        !out.status.success(),
        "approve a should refuse: {}",
        stdout(&out)
    );
    assert!(cap_refusal(&out), "{}", stderr(&out));
}
