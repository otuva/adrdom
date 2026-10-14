// An answer strikes the section of every superseded question from every
// topic file; check reports one that is still there.
//
// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252 "What strikes the section of a
// superseded question from its topic file?"
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

fn sec(heading: &str, sup: Option<&str>) -> String {
    let mut s = format!("\n### {}\ndepends: widget\n", heading);
    if let Some(x) = sup {
        s.push_str(&format!("supersedes: {}\n", x));
    }
    s.push_str(BODY);
    s
}

fn rel(name: &str) -> String {
    format!("adrdom/decisions/{}.md", name)
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
    panic!("no id under {}", heading);
}

fn approve_ok(dir: &Path, name: &str) {
    let out = adr_owner(dir, &["approve", dir.join(rel(name)).to_str().unwrap()], "");
    assert!(out.status.success(), "approve {} failed: {}", name, stderr(&out));
}

fn ask_ok(dir: &Path, name: &str) {
    let out = adr(dir, &["ask", dir.join(rel(name)).to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask {} failed: {}{}", name, stdout(&out), stderr(&out));
}

fn answer_ok(dir: &Path, name: &str, heading: &str, letter: &str) -> std::process::Output {
    let id = id_of(&dir.join(rel(name)), heading);
    let out = adr_answer(dir, &id, letter);
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    out
}

/// A file `name` holding `heading` (superseding `sup` when given), approved
/// and asked.
fn put(dir: &Path, name: &str, heading: &str, sup: Option<&str>) -> PathBuf {
    let p = write_record(dir, &rel(name), &format!("# {}\n{}", name.to_uppercase(), sec(heading, sup)));
    approve_ok(dir, name);
    ask_ok(dir, name);
    p
}

fn setup() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    dir
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(rel(name))).unwrap()
}

fn show_file(dir: &Path, name: &str) -> String {
    let out = adr(dir, &["show", dir.join(rel(name)).to_str().unwrap()], &[], "");
    format!("{}{}", stdout(&out), stderr(&out))
}

fn no_superseded_failure(dir: &Path) {
    let f = check_failures(dir);
    assert!(!f.contains("superseded"), "check fails on a superseded section: {}", f);
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn an_answer_strikes_the_target_and_an_older_superseded_section_from_other_files() {
    let dir = setup();
    put(&dir, "a", Q1, None);
    answer_ok(&dir, "a", Q1, "A");
    // Q2 answered, then superseded by Q3 in c; its section stays until the
    // next answer in a file that is not c (put back by hand below).
    put(&dir, "b", Q2, None);
    answer_ok(&dir, "b", Q2, "A");
    let b_text = read(&dir, "b");
    put(&dir, "c", Q3, Some(Q2));
    let out = answer_ok(&dir, "c", Q3, "A");
    assert!(!read(&dir, "b").contains(Q2), "target not struck: {}", read(&dir, "b"));
    assert!(stdout(&out).contains(&format!("written: {}", rel("b"))), "no written line: {}", stdout(&out));
    // an older superseded section left in a third file goes at the next answer
    std::fs::write(dir.join(rel("b")), b_text).unwrap();
    assert!(check_failures(&dir).contains("superseded"));
    put(&dir, "d", Q4, Some(Q1));
    answer_ok(&dir, "d", Q4, "A");
    assert!(!read(&dir, "a").contains(Q1));
    assert!(!read(&dir, "b").contains(Q2), "older superseded section left: {}", read(&dir, "b"));
    assert!(read(&dir, "c").contains(Q3));
    for n in ["a", "b", "c", "d"] {
        let s = show_file(&dir, n);
        assert!(s.contains("approved"), "{} not approved: {}", n, s);
        // a struck section never shows as a removed line
        assert!(!s.contains(Q1) && !s.contains(Q2), "{} shows a struck section: {}", n, s);
    }
    no_superseded_failure(&dir);
    // ask works on a struck file
    let out = adr(&dir, &["ask", dir.join(rel("a")).to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask on struck file: {}{}", stdout(&out), stderr(&out));
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn a_refusal_strikes_too() {
    let dir = setup();
    put(&dir, "a", Q1, None);
    answer_ok(&dir, "a", Q1, "A");
    put(&dir, "b", Q4, Some(Q1));
    let a_text = read(&dir, "a");
    // a section put back by hand after the answer would be struck at the
    // next run, whatever it is: answer Q4, put Q1 back, refuse another.
    answer_ok(&dir, "b", Q4, "A");
    std::fs::write(dir.join(rel("a")), a_text).unwrap();
    assert!(check_failures(&dir).contains("superseded"));
    put(&dir, "c", Q5, None);
    answer_ok(&dir, "c", Q5, "refuse");
    assert!(!read(&dir, "a").contains(Q1), "refuse did not strike: {}", read(&dir, "a"));
    no_superseded_failure(&dir);
    assert!(show_file(&dir, "a").contains("approved"));
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn a_superseded_section_put_back_fails_check_with_the_agent_move() {
    let dir = setup();
    put(&dir, "a", Q1, None);
    answer_ok(&dir, "a", Q1, "A");
    let a_text = read(&dir, "a");
    put(&dir, "b", Q4, Some(Q1));
    answer_ok(&dir, "b", Q4, "A");
    no_superseded_failure(&dir);
    std::fs::write(dir.join(rel("a")), a_text).unwrap();
    let f = check_failures(&dir);
    assert!(f.contains(&format!("\"{}\" is superseded", Q1)), "{}", f);
    assert!(f.contains("agent removes the question from the file"), "{}", f);
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn a_chain_leaves_neither_older_section() {
    let dir = setup();
    put(&dir, "a", Q1, None);
    answer_ok(&dir, "a", Q1, "A");
    put(&dir, "b", Q4, Some(Q1));
    answer_ok(&dir, "b", Q4, "A");
    put(&dir, "c", Q5, Some(Q4));
    answer_ok(&dir, "c", Q5, "A");
    assert!(!read(&dir, "a").contains(Q1));
    assert!(!read(&dir, "b").contains(Q4));
    assert!(read(&dir, "c").contains(Q5));
    no_superseded_failure(&dir);
    for n in ["a", "b", "c"] {
        let s = show_file(&dir, n);
        assert!(s.contains("approved"), "{}: {}", n, s);
    }
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn two_questions_naming_one_old_decision_both_answer_and_print_the_heading() {
    let dir = setup();
    put(&dir, "a", Q1, None);
    answer_ok(&dir, "a", Q1, "A");
    let id1 = id_of(&dir.join(rel("a")), Q1);
    let b = write_record(&dir, &rel("b"), &format!("# B\n{}", sec(Q4, Some(Q1))));
    let c = write_record(&dir, &rel("c"), &format!("# C\n{}", sec(Q5, Some(Q1))));
    approve_ok(&dir, "b");
    approve_ok(&dir, "c");
    ask_ok(&dir, "b");
    ask_ok(&dir, "c");
    answer_ok(&dir, "b", Q4, "A");
    assert!(!read(&dir, "a").contains(Q1));
    let out = answer_ok(&dir, "c", Q5, "A");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(text.contains(Q1), "heading not printed: {}", text);
    assert!(!text.contains(&id1), "uuid printed: {}", text);
    // answering the struck id says it is closed
    let out = adr_owner(&dir, &["answer", &format!("{}@00000000", id1), "A"], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("already superseded"), "{}", stderr(&out));
    let _ = (b, c);
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn an_unrelated_line_naming_a_struck_decision_still_resolves() {
    let dir = setup();
    put(&dir, "a", Q1, None);
    answer_ok(&dir, "a", Q1, "A");
    let id1 = id_of(&dir.join(rel("a")), Q1);
    put(&dir, "b", Q4, Some(Q1));
    answer_ok(&dir, "b", Q4, "A");
    assert!(!read(&dir, "a").contains(Q1));
    let p = write_record(
        &dir,
        &rel("c"),
        &format!("# C\n\n### {}\ndepends: widget\nunrelated: {}\n{}", Q5, id1, BODY),
    );
    let out = adr(&dir, &["plan", p.to_str().unwrap()], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(!text.contains("names nothing"), "{}", text);
    approve_ok(&dir, "c");
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn show_superseded_reads_a_struck_section_from_the_ledger_and_git() {
    let dir = setup();
    put(&dir, "a", Q1, None);
    answer_ok(&dir, "a", Q1, "A");
    put(&dir, "b", Q4, Some(Q1));
    run_git(&dir, &["add", "-A"]);
    assert!(run_git(&dir, &["commit", "-q", "-m", "both asked"]).status.success());
    answer_ok(&dir, "b", Q4, "A");
    assert!(!read(&dir, "a").contains(Q1));
    let show = |flags: &[&str]| {
        let p = dir.join(rel("a"));
        let mut args = vec!["show", p.to_str().unwrap()];
        args.extend_from_slice(flags);
        let out = adr(&dir, &args, &[], "");
        assert!(out.status.success(), "{}", stderr(&out));
        stdout(&out)
    };
    assert!(!show(&[]).contains(Q1));
    let s = show(&["--superseded"]);
    assert!(s.contains(&format!("[superseded] {}\n  replaced by: {}\n", Q1, Q4)), "{}", s);
    let s = show(&["--superseded", "--dependents"]);
    assert!(s.contains(&format!("[superseded] {}\n  depends: widget\n  replaced by: {}\n", Q1, Q4)), "{}", s);
}
