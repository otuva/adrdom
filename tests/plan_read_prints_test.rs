// What `adr plan` and `adr read` print: the move is the one computation
// `show` reads ("Does a command compute a move of its own?", aac5c313 A),
// and a command that writes nothing prints what it reads, its status, the
// findings and the move, and nothing else ("What does a command print when it
// writes nothing?", 1af76f91 A). The embedding model and the reader are the
// fakes.

mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/topic.md";
const BASE: &str = "adrdom/decisions/base.md";
const H1: &str = "Does the first thing glow?";
const H2: &str = "Does the second thing glow?";

fn body(n: &str) -> String {
    format!(
        "\nProblem: problem {n} of the widget.\n\nStandard: standard {n}.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        n = n
    )
}

fn section(heading: &str, body: &str) -> String {
    format!("\n### {}\ndepends: widget\n{}", heading, body)
}

/// An installed repository with two terms and one current decision, tied to
/// a test so it names no move of its own.
fn repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget", "paint"]);
    let base = write_record(&dir, BASE, &format!("# Base\n{}", section("Does the base hold?", &body("base widget"))));
    approve_record(&dir, BASE);
    assert!(adr(&dir, &["ask", base.to_str().unwrap()], &[], "").status.success());
    let id = extract_ids(&std::fs::read_to_string(&base).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    write_test_comment(&dir, &id);
    dir
}

fn write(dir: &Path, text: &str) {
    std::fs::write(dir.join(REL), text).unwrap();
}

fn read(dir: &Path) -> String {
    std::fs::read_to_string(dir.join(REL)).unwrap()
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let out = adr(dir, args, &[], "");
    assert!(out.status.success(), "{:?}: stdout={} stderr={}", args, stdout(&out), stderr(&out));
    stdout(&out)
}

fn plan(dir: &Path) -> String {
    ok(dir, &["plan", dir.join(REL).to_str().unwrap()])
}

fn show(dir: &Path) -> String {
    ok(dir, &["show"])
}

/// A move line, as every command prints one: who makes it first.
fn is_move(line: &str) -> bool {
    let l = line.trim_start();
    l.starts_with("agent ") || l.starts_with("owner runs:") || l.starts_with("not yet")
}

/// The moves `plan` printed.
fn plan_moves(out: &str) -> Vec<String> {
    out.lines().filter(|l| is_move(l)).map(|l| l.trim().to_string()).collect()
}

/// The findings and the moves a command printed: every line at the left
/// edge that is no status line, no topic line and no question `plan` read.
fn report(out: &str) -> Vec<String> {
    out.lines()
        .filter(|l| !l.starts_with([' ', '[', '"', '#', '+', '-']))
        .map(str::to_string)
        .collect()
}

/// The findings and moves `show` printed about the topic file: those naming
/// its path or one of its headings.
fn show_report_of_file(out: &str) -> Vec<String> {
    report(out).into_iter().filter(|l| l.contains(REL) || l.contains(H1) || l.contains(H2)).collect()
}

/// Plan and show name the same findings and the same moves for the file,
/// in the same order.
fn same_moves(dir: &Path, stage: &str) -> String {
    let p = plan(dir);
    let s = show(dir);
    let pr = report(&p);
    assert!(!pr.is_empty(), "{}: plan names no move: {}", stage, p);
    assert_eq!(pr, show_report_of_file(&s), "{}: plan\n{}\nshow\n{}", stage, p, s);
    p
}

fn write_lines(dir: &Path, plan_stdout: &str) {
    let wanted = lines_wanted(plan_stdout);
    write(dir, &add_unrelated(&read(dir), &wanted));
}

fn ids(dir: &Path) -> Vec<String> {
    extract_ids(&read(dir))
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
#[test]
fn plan_names_the_moves_show_names_at_every_step() {
    let dir = repo();
    write(&dir, &format!("# Topic\n\n### {}\ndepends: widget\n\n### {}\ndepends: widget\n", H1, H2));

    // Headings read: the lines are the agent's.
    let out = same_moves(&dir, "headings");
    write_lines(&dir, &out);
    // Lines written: the bodies.
    same_moves(&dir, "bodies");
    write(&dir, &read(&dir).replacen(&format!("### {}\ndepends: widget\n", H1), &format!("### {}\ndepends: widget\n{}", H1, body("one")), 1));
    write(&dir, &read(&dir).replacen(&format!("### {}\ndepends: widget\n", H2), &format!("### {}\ndepends: widget\n{}", H2, body("two")), 1));
    // Bodies read: the lines of the new neighbours.
    let out = same_moves(&dir, "bodies read");
    write_lines(&dir, &out);
    // Ready: the owner's approval.
    let out = same_moves(&dir, "ready");
    assert!(plan_moves(&out).contains(&format!("owner runs: adr approve {}", REL)), "{}", out);

    let a = adr_owner(&dir, &["approve", dir.join(REL).to_str().unwrap()], "");
    assert!(a.status.success(), "{}{}", stdout(&a), stderr(&a));
    // Approved: the agent asks, and plan no longer names the owner's approval.
    let out = same_moves(&dir, "approved");
    assert!(!out.contains("owner runs: adr approve"), "{}", out);

    assert!(adr(&dir, &["ask", dir.join(REL).to_str().unwrap()], &[], "").status.success());
    // Asked: the owner's answers.
    let out = same_moves(&dir, "asked");
    assert!(plan_moves(&out).iter().any(|m| m.starts_with("owner runs: adr answer")), "{}", out);
    assert!(!out.contains("owner runs: adr approve"), "{}", out);

    for id in ids(&dir) {
        let r = adr_answer(&dir, &id, "A");
        assert!(r.status.success(), "{}{}", stdout(&r), stderr(&r));
    }
    // Answered: the build.
    let out = same_moves(&dir, "answered");
    assert!(plan_moves(&out).iter().all(|m| m.starts_with("agent builds")), "{}", out);
    assert!(!out.contains("owner runs: adr approve"), "{}", out);
}

/// Every line `plan` prints is one of: the file's status, a question it
/// read, a neighbour of it, a finding on the file, or a move.
fn plan_line_kind(line: &str) -> Option<&'static str> {
    if line.starts_with('[') && line.ends_with(&format!("({})", REL)) {
        Some("status")
    } else if line.starts_with('"') && (line.ends_with("\": heading") || line.ends_with("\": heading with its body")) {
        Some("read")
    } else if ["  decision \"", "  term \"", "  question \""].iter().any(|p| line.starts_with(p)) {
        Some("neighbour")
    } else if line.starts_with(&format!("{}: ", REL)) {
        Some("finding")
    } else if is_move(line) {
        Some("move")
    } else {
        None
    }
}

// adr: 1af76f91-5644-42ef-97ea-8f70f934c26c
#[test]
fn plan_prints_the_status_what_it_read_the_findings_and_the_moves_and_nothing_else() {
    let dir = repo();
    write(&dir, &format!("# Topic\n{}{}", section(H1, &body("one")), section(H2, "")));
    let mut outs = vec![plan(&dir)];
    write_lines(&dir, &outs[0]);
    outs.push(plan(&dir));
    write(&dir, &format!("# Topic\n{}{}", section(H1, &body("one")), section(H2, &body("two"))));
    write_lines(&dir, &plan(&dir));
    outs.push(plan(&dir));
    write_lines(&dir, outs.last().unwrap());
    outs.push(plan(&dir));
    for out in &outs {
        assert!(out.lines().next().is_some_and(|l| plan_line_kind(l) == Some("status")), "the status first: {}", out);
        for l in out.lines() {
            assert!(plan_line_kind(l).is_some(), "a line of no kind: {:?} in\n{}", l, out);
        }
        assert!(!out.contains("next"), "no next heading: {}", out);
        assert!(!out.lines().any(|l| l.contains(" neighbours") || l.contains(" read, ")), "no counts: {}", out);
    }
    // Ready: the status and the move, nothing else.
    assert_eq!(outs.last().unwrap(), &format!("[new] Topic ({})\nowner runs: adr approve {}\n", REL, REL));
}

// adr: 1af76f91-5644-42ef-97ea-8f70f934c26c
#[test]
fn read_prints_the_marked_pairs_and_the_move_and_nothing_else() {
    let dir = repo();
    // A second decision; with the base, one pair, which the fake reader marks
    // `same` from the marker in the first decision's heading.
    write(&dir, &format!("# Topic\n{}", section(H1, &body("one"))));
    let a = adr_owner(&dir, &["approve", dir.join(REL).to_str().unwrap()], "");
    assert!(a.status.success(), "{}{}", stdout(&a), stderr(&a));
    assert!(adr(&dir, &["ask", dir.join(REL).to_str().unwrap()], &[], "").status.success());
    let id = ids(&dir)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());

    // Nothing marked: nothing printed.
    assert_eq!(ok(&dir, &["read"]), "");

    // Both decisions marked `same` (their text read again under the
    // marker; the fake reader's word is the first decision's marker): one
    // pair line, then the move.
    let base = dir.join(BASE);
    let text = std::fs::read_to_string(&base).unwrap().replace("### Does the base hold?", "### Does the base [word:same] hold?");
    std::fs::write(&base, text).unwrap();
    write(&dir, &read(&dir).replace(&format!("### {}", H1), "### Does the first [word:same] thing glow?"));
    let out = ok(&dir, &["read"]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{}", out);
    assert!(lines[0].starts_with("same: \""), "{}", out);
    assert_eq!(lines[1], "agent writes a question that supersedes one decision of each same or inconsistent pair");
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
//
// plan plans topic files alone: the terms file, a blame file or any other
// path is refused with the move, never misread as a topic file.
#[test]
fn plan_refuses_a_path_that_is_no_topic_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    for path in ["adrdom/terms.md", "README.md"] {
        let out = adr(&dir, &["plan", dir.join(path).to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "{}", path);
        assert!(stdout(&out).is_empty(), "{}: {}", path, stdout(&out));
        assert_eq!(
            stderr(&out).trim_end(),
            format!("plan: {} is not a topic file; agent names a topic file under adrdom/decisions", path)
        );
    }
}
