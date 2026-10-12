// A decision named on the supersedes line of a question not yet answered is
// left out of its tier's count, so the cap (4) holds by what the tier holds
// once the replacement is answered.
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
    assert!(out.status.success(), "approve {} failed: {}", name, stderr(&out));
}

fn ask(dir: &Path, name: &str) -> std::process::Output {
    adr(dir, &["ask", dir.join(rel(name)).to_str().unwrap()], &[], "")
}

fn cap_refusal(out: &std::process::Output) -> bool {
    stderr(out).contains("the most is 4") || stdout(out).contains("the most is 4")
}

/// a.md with four tier-1 questions, approved, asked and answered A.
fn full_tier(dir: &Path) -> PathBuf {
    assert!(adr(dir, &["install"], &[], "").status.success());
    approve_minimal_terms(dir, "widget");
    let mut c = String::from("# A\n");
    for n in 1..=4 {
        c.push_str(&sec(&h(n), None, true));
    }
    let p = write_record(dir, &rel("a"), &c);
    approve_ok(dir, "a");
    let out = ask(dir, "a");
    assert!(out.status.success(), "ask a failed: {}", stderr(&out));
    for n in 1..=4 {
        let out = adr_answer(dir, &id_of(&p, &h(n)), "A");
        assert!(out.status.success(), "answer failed: {}", stderr(&out));
    }
    p
}

// adr: 86d5e944-ecfa-44bc-9819-83ee4b96890d
#[test]
fn a_fifth_heading_superseding_one_of_four_passes_approve_check_and_ask() {
    let dir = init_repo();
    let a = full_tier(&dir);
    append(&a, &sec(&h(5), Some(&h(1)), true));
    let out = approve(&dir, "a");
    assert!(out.status.success(), "approve should pass the cap: {}", stderr(&out));
    let chk = adr(&dir, &["check"], &[], "");
    assert!(!cap_refusal(&chk), "check printed a cap refusal: {}{}", stdout(&chk), stderr(&chk));
    let out = ask(&dir, "a");
    assert!(out.status.success(), "ask should pass the cap: {}", stderr(&out));
}

#[test]
fn a_fifth_heading_superseding_nothing_still_fails_the_cap() {
    let dir = init_repo();
    let a = full_tier(&dir);
    append(&a, &sec(&h(5), None, true));
    let out = approve(&dir, "a");
    assert!(!out.status.success(), "approve should refuse: {}", stdout(&out));
    assert!(cap_refusal(&out), "{}", stderr(&out));
}

#[test]
fn two_replacements_of_one_decision_in_a_full_tier_fail_the_cap() {
    let dir = init_repo();
    let a = full_tier(&dir);
    append(&a, &sec(&h(5), Some(&h(1)), true));
    append(&a, &sec(&h(6), Some(&h(1)), true));
    let out = approve(&dir, "a");
    assert!(!out.status.success(), "approve should refuse: {}", stdout(&out));
    assert!(cap_refusal(&out), "{}", stderr(&out));
}

// adr: 86d5e944-ecfa-44bc-9819-83ee4b96890d
#[test]
fn a_refused_replacement_returns_the_old_decision_to_the_count() {
    let dir = init_repo();
    let a = full_tier(&dir);
    append(&a, &sec(&h(5), Some(&h(1)), true));
    approve_ok(&dir, "a");
    let out = ask(&dir, "a");
    assert!(out.status.success(), "ask should pass the cap: {}", stderr(&out));
    let out = adr_answer(&dir, &id_of(&a, &h(5)), "refuse");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));

    append(&a, &sec(&h(6), None, true));
    let out = approve(&dir, "a");
    assert!(!out.status.success(), "approve should refuse: {}", stdout(&out));
    assert!(cap_refusal(&out), "{}", stderr(&out));
}

#[test]
fn an_open_replacement_in_another_file_frees_a_slot_in_the_full_tier() {
    let dir = init_repo();
    let a = full_tier(&dir);
    write_record(&dir, &rel("b"), &format!("# B\n{}", sec(&h(9), Some(&h(1)), true)));
    approve_ok(&dir, "b");
    append(&a, &sec(&h(5), None, true));
    let out = approve(&dir, "a");
    assert!(out.status.success(), "a.md counts 3 + 1: {}", stderr(&out));
}

// adr: 0e38f001-0226-4927-9a78-2e9fc1ec18d4
// adr: 86d5e944-ecfa-44bc-9819-83ee4b96890d
//
// a.md holds four answered questions and a fifth approved while an open
// question of b.md replaced one of the four. When b.md's question is
// refused, the old decision is back in the count: a.md's tier holds five
// and `ask` stops it on the cap, though a.md's plan is still approved.
#[test]
fn ask_stops_a_tier_that_a_refused_replacement_in_another_file_took_over_the_cap() {
    let dir = init_repo();
    let a = full_tier(&dir);
    let b = write_record(&dir, &rel("b"), &format!("# B\n{}", sec(&h(9), Some(&h(1)), true)));
    approve_ok(&dir, "b");
    append(&a, &sec(&h(5), None, true));
    approve_ok(&dir, "a");
    let out = ask(&dir, "b");
    assert!(out.status.success(), "ask b failed: {}", stderr(&out));
    let out = adr_answer(&dir, &id_of(&b, &h(9)), "refuse");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));

    let out = ask(&dir, "a");
    assert!(!out.status.success(), "ask a should refuse: {}", stdout(&out));
    assert!(cap_refusal(&out), "{}", stderr(&out));
}
