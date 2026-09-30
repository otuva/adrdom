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

/// Writes `body` under the id line of `heading`.
fn write_body(path: &Path, heading: &str) {
    let id = id_of(path, heading);
    let c = std::fs::read_to_string(path).unwrap();
    let with = c.replacen(&format!("id: {}\n", id), &format!("id: {}\n{}", id, BODY), 1);
    assert_ne!(with, c);
    std::fs::write(path, with).unwrap();
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

// adr: f416062c-211f-4d20-a6bf-23a561b5f59b
#[test]
fn a_fifth_heading_superseding_one_of_four_passes_approve_check_and_ask() {
    let dir = init_repo();
    let a = full_tier(&dir);
    append(&a, &sec(&h(5), Some(&h(1)), false));
    let out = approve(&dir, "a");
    assert!(out.status.success(), "approve should pass the cap: {}", stderr(&out));
    let chk = adr(&dir, &["check"], &[], "");
    assert!(!cap_refusal(&chk), "check printed a cap refusal: {}{}", stdout(&chk), stderr(&chk));
    write_body(&a, &h(5));
    let out = ask(&dir, "a");
    assert!(out.status.success(), "ask should pass the cap: {}", stderr(&out));
}

// adr: f416062c-211f-4d20-a6bf-23a561b5f59b
#[test]
fn a_fifth_heading_superseding_nothing_still_fails_the_cap() {
    let dir = init_repo();
    let a = full_tier(&dir);
    append(&a, &sec(&h(5), None, false));
    let out = approve(&dir, "a");
    assert!(!out.status.success(), "approve should refuse: {}", stdout(&out));
    assert!(cap_refusal(&out), "{}", stderr(&out));
}

// adr: f416062c-211f-4d20-a6bf-23a561b5f59b
#[test]
fn two_replacements_of_one_decision_in_a_full_tier_fail_the_cap() {
    let dir = init_repo();
    let a = full_tier(&dir);
    append(&a, &sec(&h(5), Some(&h(1)), false));
    append(&a, &sec(&h(6), Some(&h(1)), false));
    let out = approve(&dir, "a");
    assert!(!out.status.success(), "approve should refuse: {}", stdout(&out));
    assert!(cap_refusal(&out), "{}", stderr(&out));
}

// adr: f416062c-211f-4d20-a6bf-23a561b5f59b
#[test]
fn a_refused_replacement_returns_the_old_decision_to_the_count() {
    let dir = init_repo();
    let a = full_tier(&dir);
    append(&a, &sec(&h(5), Some(&h(1)), false));
    approve_ok(&dir, "a");
    write_body(&a, &h(5));
    let out = ask(&dir, "a");
    assert!(out.status.success(), "ask should pass the cap: {}", stderr(&out));
    let out = adr_answer(&dir, &id_of(&a, &h(5)), "refuse");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));

    append(&a, &sec(&h(6), None, false));
    let out = approve(&dir, "a");
    assert!(!out.status.success(), "approve should refuse: {}", stdout(&out));
    assert!(cap_refusal(&out), "{}", stderr(&out));
}

// adr: f416062c-211f-4d20-a6bf-23a561b5f59b
#[test]
fn an_open_replacement_in_another_file_frees_a_slot_in_the_full_tier() {
    let dir = init_repo();
    let a = full_tier(&dir);
    write_record(&dir, &rel("b"), &format!("# B\n{}", sec(&h(9), Some(&h(1)), true)));
    approve_ok(&dir, "b");
    append(&a, &sec(&h(5), None, false));
    let out = approve(&dir, "a");
    assert!(out.status.success(), "a.md counts 3 + 1: {}", stderr(&out));
}

// adr: f416062c-211f-4d20-a6bf-23a561b5f59b
#[test]
fn check_counts_a_tier_as_approve_does_when_the_replacing_file_has_a_cross_file_heading_entry() {
    let dir = init_repo();
    let a = full_tier(&dir);
    // b.md names a decision of a.md by its heading (approve rewrites the
    // entry to an id on disk; check normalizes heading entries in memory
    // before it counts) and holds an open question replacing option 1.
    let b = format!(
        "# B\n\n### {}\ndepends: {}\nsupersedes: {}\n{}",
        h(9),
        h(2),
        h(1),
        BODY
    );
    write_record(&dir, &rel("b"), &b);
    approve_ok(&dir, "b");
    append(&a, &sec(&h(5), None, false));
    let out = approve(&dir, "a");
    assert!(out.status.success(), "approve counts 3 + 1: {}", stderr(&out));
    let chk = adr(&dir, &["check"], &[], "");
    let both = format!("{}{}", stdout(&chk), stderr(&chk));
    assert!(!cap_refusal(&chk), "check printed a cap refusal: {}", both);
    assert!(both.contains("tiers: adrdom/decisions/a.md: 1 (4)"), "{}", both);
}
