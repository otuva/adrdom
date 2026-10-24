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
    assert!(
        out.status.success(),
        "approve {} failed: {}",
        name,
        stderr(&out)
    );
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
    let p = write_record(
        dir,
        &rel(name),
        &format!("# {}\n{}", name.to_uppercase(), sec(heading, sup)),
    );
    approve_ok(dir, name);
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
    let out = adr(
        dir,
        &["show", dir.join(rel(name)).to_str().unwrap()],
        &[],
        "",
    );
    format!("{}{}", stdout(&out), stderr(&out))
}

fn no_superseded_failure(dir: &Path) {
    let f = check_failures(dir);
    assert!(
        !f.contains("superseded"),
        "check fails on a superseded section: {}",
        f
    );
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
    assert!(
        !read(&dir, "b").contains(Q2),
        "target not struck: {}",
        read(&dir, "b")
    );
    assert!(
        stdout(&out).contains(&format!("written: {}", rel("b"))),
        "no written line: {}",
        stdout(&out)
    );
    // an older superseded section left in a third file goes at the next answer
    std::fs::write(dir.join(rel("b")), b_text).unwrap();
    assert!(check_failures(&dir).contains("superseded"));
    put(&dir, "d", Q4, Some(Q1));
    answer_ok(&dir, "d", Q4, "A");
    assert!(!read(&dir, "a").contains(Q1));
    assert!(
        !read(&dir, "b").contains(Q2),
        "older superseded section left: {}",
        read(&dir, "b")
    );
    assert!(read(&dir, "c").contains(Q3));
    for n in ["a", "b", "c", "d"] {
        let s = show_file(&dir, n);
        assert!(s.contains("approved"), "{} not approved: {}", n, s);
        // a struck section never shows as a removed line
        assert!(
            !s.contains(Q1) && !s.contains(Q2),
            "{} shows a struck section: {}",
            n,
            s
        );
    }
    no_superseded_failure(&dir);
    // approve works on a struck file
    let out = adr_owner(&dir, &["approve", dir.join(rel("a")).to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve on struck file: {}{}",
        stdout(&out),
        stderr(&out)
    );
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
    assert!(
        !read(&dir, "a").contains(Q1),
        "refuse did not strike: {}",
        read(&dir, "a")
    );
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
    assert!(f.contains("agent removes the question"), "{}", f);
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
    write_record(&dir, &rel("b"), &format!("# B\n{}", sec(Q4, Some(Q1))));
    write_record(&dir, &rel("c"), &format!("# C\n{}", sec(Q5, Some(Q1))));
    approve_ok(&dir, "b");
    approve_ok(&dir, "c");
    answer_ok(&dir, "b", Q4, "A");
    assert!(!read(&dir, "a").contains(Q1));
    let out = answer_ok(&dir, "c", Q5, "A");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(text.contains(Q1), "heading not printed: {}", text);
    assert!(!text.contains(&id1), "uuid printed: {}", text);
    // answering the struck id says it is closed
    let out = adr_owner(&dir, &["answer", &format!("{}@00000000", id1), "A"], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("already superseded"),
        "{}",
        stderr(&out)
    );
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn show_superseded_reads_a_struck_section_from_the_ledger_and_git() {
    let dir = setup();
    put(&dir, "a", Q1, None);
    answer_ok(&dir, "a", Q1, "A");
    put(&dir, "b", Q4, Some(Q1));
    run_git(&dir, &["add", "-A"]);
    assert!(run_git(&dir, &["commit", "-q", "-m", "both asked"])
        .status
        .success());
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
    assert!(
        s.contains(&format!("[superseded] {}\n  replaced by: {}\n", Q1, Q4)),
        "{}",
        s
    );
    let s = show(&["--superseded", "--dependents"]);
    assert!(
        s.contains(&format!(
            "[superseded] {}\n  depends: widget\n  replaced by: {}\n",
            Q1, Q4
        )),
        "{}",
        s
    );
}

fn many_heading(i: usize) -> String {
    format!("Is the widget part {} sound?", i)
}

fn replacement_heading(i: usize) -> String {
    format!("Is the widget part {} sturdy?", i)
}

/// The file `name` holding `count` questions in chains of four, each after
/// the first depending on the one before, approved (which asks it).
fn put_many(dir: &Path, name: &str, count: usize) {
    let mut text = format!("# {}\n", name.to_uppercase());
    for i in 0..count {
        let dep = if i % 4 == 0 {
            "widget".to_string()
        } else {
            format!("widget, {}=*", many_heading(i - 1))
        };
        text.push_str(&format!(
            "\n### {}\ndepends: {}\n{}",
            many_heading(i),
            dep,
            BODY
        ));
    }
    write_record(dir, &rel(name), &text);
    approve_ok(dir, name);
}

/// The file `name` holding one question that supersedes the four questions
/// of chain `k` of the file "m", approved (which asks it).
fn put_replacement(dir: &Path, name: &str, k: usize) {
    let sup: Vec<String> = (4 * k..4 * k + 4).map(many_heading).collect();
    write_record(
        dir,
        &rel(name),
        &format!(
            "# {}\n{}",
            name.to_uppercase(),
            sec(&replacement_heading(k), Some(&sup.join(", ")))
        ),
    );
    approve_ok(dir, name);
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn a_file_with_more_than_twelve_struck_sections_in_its_history_stays_approved() {
    let dir = setup();
    put_many(&dir, "m", 16);
    for i in 0..16 {
        answer_ok(&dir, "m", &many_heading(i), "A");
    }
    for k in 0..4 {
        put_replacement(&dir, ["ra", "rb", "rc", "rd"][k], k);
    }
    answer_ok(&dir, "ra", &replacement_heading(0), "A");
    answer_ok(&dir, "rb", &replacement_heading(1), "A");
    assert!(
        !read(&dir, "m").contains(&many_heading(7)),
        "{}",
        read(&dir, "m")
    );
    // the file is approved again with eight sections struck, then four more
    // and a last four go: sixteen struck in its history
    let m = dir.join(rel("m"));
    std::fs::write(
        &m,
        read(&dir, "m").replace("# M\n", "# M\n\nParts of the widget.\n"),
    )
    .unwrap();
    approve_ok(&dir, "m");
    answer_ok(&dir, "rc", &replacement_heading(2), "A");
    assert!(
        show_file(&dir, "m").contains("approved"),
        "{}",
        show_file(&dir, "m")
    );
    answer_ok(&dir, "rd", &replacement_heading(3), "A");
    assert!(
        !read(&dir, "m").contains(&many_heading(15)),
        "{}",
        read(&dir, "m")
    );
    let s = show_file(&dir, "m");
    assert!(s.starts_with("[approved]"), "{}", s);
    no_superseded_failure(&dir);
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn a_file_approved_while_holding_superseded_sections_reads_approved_after_they_are_removed() {
    let dir = setup();
    put(&dir, "a", Q1, None);
    answer_ok(&dir, "a", Q1, "A");
    put(&dir, "b", Q2, None);
    answer_ok(&dir, "b", Q2, "A");
    let a_with = read(&dir, "a");
    put(&dir, "c", Q4, Some(Q1));
    answer_ok(&dir, "c", Q4, "A");
    // the file as it was before any strike: Q1 superseded, still in the
    // file, and the file approved so
    std::fs::write(dir.join(rel("a")), &a_with).unwrap();
    approve_ok(&dir, "a");
    assert!(
        show_file(&dir, "a").starts_with("[approved]"),
        "{}",
        show_file(&dir, "a")
    );
    // removed by hand
    std::fs::write(dir.join(rel("a")), "# A\n").unwrap();
    let s = show_file(&dir, "a");
    assert!(s.starts_with("[approved]"), "{}", s);
}

// A question standing on a question whose section is struck keeps its depends
// entry, answers, and carries a new question standing on it, as it did while
// the section stood in the file.
// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn a_question_standing_on_a_struck_sibling_behaves_as_before_the_strike() {
    let dir = setup();
    let tall = "Is the widget tall?";
    let wide = "Is the widget wide?";
    let a = write_record(
        &dir,
        &rel("a"),
        &format!(
            "# A\n\n### {}\ndepends: widget\n{}\n### {}\ndepends: widget, {}=A\n{}",
            Q1, BODY, tall, Q1, BODY
        ),
    );
    approve_ok(&dir, "a");
    answer_ok(&dir, "a", Q1, "A");
    put(&dir, "b", Q4, Some(Q1));
    answer_ok(&dir, "b", Q4, "A");
    assert!(
        !read(&dir, "a").contains(&format!("### {}", Q1)),
        "{}",
        read(&dir, "a")
    );
    // check, show and show <file> name no depends failure
    for args in [
        vec!["check"],
        vec!["show"],
        vec!["show", a.to_str().unwrap()],
    ] {
        let o = adr(&dir, &args, &[], "");
        let all = format!("{}{}", stdout(&o), stderr(&o));
        assert!(
            !all.contains("names no term and no question"),
            "{:?}: {}",
            args,
            all
        );
    }
    assert!(
        show_file(&dir, "a").contains(&"[asked".to_string()),
        "{}",
        show_file(&dir, "a")
    );
    // a new question standing on the sibling is approved and asked, and
    // waits for it
    let c = std::fs::read_to_string(&a).unwrap();
    write_record(
        &dir,
        &rel("a"),
        &format!("{}\n### {}\ndepends: widget, {}=A\n{}", c, wide, tall, BODY),
    );
    approve_ok(&dir, "a");
    let early = adr_answer(&dir, &id_of(&a, wide), "A");
    assert!(!early.status.success());
    assert!(
        stderr(&early).contains("has no decision yet"),
        "{}",
        stderr(&early)
    );
    // the sibling is answered, then the question above it
    answer_ok(&dir, "a", tall, "A");
    answer_ok(&dir, "a", wide, "A");
    let o = adr(&dir, &["check"], &[], "");
    assert!(
        !format!("{}{}", stdout(&o), stderr(&o)).contains("names no term"),
        "{}{}",
        stdout(&o),
        stderr(&o)
    );
}

/// File a with Q1 asked, then answered and struck by Q4 in file b; returns
/// Q1's id. `before` is written into a between Q1's ask and its answer.
fn struck_q1(before: Option<&str>) -> (PathBuf, String, PathBuf) {
    let dir = setup();
    let a = put(&dir, "a", Q1, None);
    let q1 = id_of(&a, Q1);
    if let Some(h) = before {
        let c = read(&dir, "a");
        write_record(
            &dir,
            &rel("a"),
            &format!("{}\n### {}\ndepends: widget, {}=A\n{}", c, h, q1, BODY),
        );
        approve_ok(&dir, "a");
    }
    answer_ok(&dir, "a", Q1, "A");
    put(&dir, "b", Q4, Some(Q1));
    answer_ok(&dir, "b", Q4, "A");
    assert!(!read(&dir, "a").contains(&format!("### {}", Q1)));
    (dir, q1, a)
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn approved_question_on_a_struck_one_is_asked_and_answered() {
    let (dir, _q1, a) = struck_q1(Some(Q3));
    // approve reads the file by its absolute path, as a shell may give it,
    // and the question Q3 added before the strike was asked by the approve
    // that took it in
    let o = adr_owner(&dir, &["approve", a.to_str().unwrap()], "");
    assert!(o.status.success(), "{}{}", stdout(&o), stderr(&o));
    let o = adr_answer(&dir, &id_of(&a, Q3), "A");
    assert!(o.status.success(), "{}", stderr(&o));
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn new_question_on_a_struck_one_under_a_set_is_refused() {
    let (dir, q1, a) = struck_q1(None);
    let c = read(&dir, "a");
    write_record(
        &dir,
        &rel("a"),
        &format!("{}\n### {}\ndepends: widget, {}=A\n{}", c, Q3, q1, BODY),
    );
    let o = adr_owner(&dir, &["approve", a.to_str().unwrap()], "");
    assert!(!o.status.success(), "{}{}", stdout(&o), stderr(&o));
    assert!(
        stderr(&o).contains("which is answered, under an answer set"),
        "{}{}",
        stdout(&o),
        stderr(&o)
    );
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn new_question_on_a_struck_one_without_a_set_is_approved_asked_answered() {
    let (dir, q1, a) = struck_q1(None);
    let c = read(&dir, "a");
    write_record(
        &dir,
        &rel("a"),
        &format!("{}\n### {}\ndepends: widget, {}\n{}", c, Q3, q1, BODY),
    );
    approve_ok(&dir, "a");
    answer_ok(&dir, "a", Q3, "A");
    let _ = a;
}

// adr: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
#[test]
fn a_struck_question_counts_toward_the_two_question_cap() {
    let dir = setup();
    let a = write_record(
        &dir,
        &rel("a"),
        &format!("# A\n{}{}{}", sec(Q1, None), sec(Q2, None), sec(Q3, None)),
    );
    approve_ok(&dir, "a");
    let q1 = id_of(&a, Q1);
    let q2 = id_of(&a, Q2);
    let q3 = id_of(&a, Q3);
    answer_ok(&dir, "a", Q1, "A");
    put(&dir, "b", Q4, Some(Q1));
    answer_ok(&dir, "b", Q4, "A");
    assert!(!read(&dir, "a").contains(&format!("### {}", Q1)));
    let c = read(&dir, "a");
    write_record(
        &dir,
        &rel("a"),
        &format!(
            "{}\n### {}\ndepends: widget, {}, {}=A, {}=A\n{}",
            c, Q5, q1, q2, q3, BODY
        ),
    );
    let o = adr_owner(&dir, &["approve", a.to_str().unwrap()], "");
    assert!(!o.status.success());
    assert!(
        stderr(&o).contains("the most is"),
        "{}{}",
        stdout(&o),
        stderr(&o)
    );
}
