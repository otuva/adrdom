// What `adr check` prints: every line a finding with its move, nothing on a
// pass, by hand or in a hook (3fc35d4f "Does check print a line that names no
// move?"); and every rule `approve` or `ask` refuses on is one judgement
// check reads, so what they refuse, check reports with the same move
// (fe14991d "Does a command refuse what check passes?").
mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/gate.md";

const OPTS: &str = "- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";

fn q(heading: &str, problem: &str) -> String {
    format!("### {}\ndepends: widget\n\nProblem: {}\n\nStandard: plain words.\n\n{}", heading, problem, OPTS)
}

fn repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget"]);
    dir
}

/// The owner's approve with nothing planned on the way: `adr_owner` plans
/// the file first, which would write the very lines a case leaves out.
fn approve(dir: &Path, path: &Path) -> std::process::Output {
    adr(dir, &["approve", path.to_str().unwrap()], &[("ADR_OWNER_BYPASS", "1")], "")
}

fn check(dir: &Path) -> (bool, String, String) {
    let out = adr(dir, &["check"], &[], "");
    (out.status.success(), stdout(&out), stderr(&out))
}

/// True when `line` names the move that follows it: who makes it, or that
/// its turn has not come.
fn names_a_move(line: &str) -> bool {
    line.contains("; agent ") || line.contains("; owner runs: ") || line.contains("; not yet: ")
}

/// The refusals of an owner or agent command, each as check words it: the
/// command's own prefix taken off.
fn refusals(out: &std::process::Output, prefix: &str) -> Vec<String> {
    stderr(out)
        .lines()
        .map(|l| l.strip_prefix(prefix).unwrap_or(l).to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn assert_check_reports(dir: &Path, refused: &[String], case: &str) {
    assert!(!refused.is_empty(), "{}: the command must refuse", case);
    let (_, out, _) = check(dir);
    for r in refused {
        assert!(out.contains(r.as_str()), "{}: check reports what was refused, move and all:\n{}\n--- check:\n{}", case, r, out);
    }
}

#[test]
fn a_passing_check_prints_nothing() {
    // adr: 3fc35d4f-90f3-4425-9dac-951a89631fdf
    let dir = repo();
    let (ok, out, err) = check(&dir);
    assert!(ok, "{}{}", out, err);
    assert_eq!(out, "", "a pass prints nothing");
    assert_eq!(err, "");
}

#[test]
fn a_passing_check_prints_nothing_with_a_file_awaiting_approval() {
    // adr: 3fc35d4f-90f3-4425-9dac-951a89631fdf
    // A draft with no finding is a status, not a finding: its move is in
    // `adr show`, and check prints no count of it.
    let dir = repo();
    write_record(&dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks.")));
    plan_record(&dir, REL);
    let (ok, out, err) = check(&dir);
    assert!(ok, "{}{}", out, err);
    assert_eq!(out, "", "nothing but findings: {}", out);
}

#[test]
fn every_line_check_prints_names_a_move() {
    // adr: 3fc35d4f-90f3-4425-9dac-951a89631fdf
    // An untested decision, a draft finding and a failing approved file at
    // once: each line is a finding with the move that clears it.
    let dir = repo();
    let rec = write_record(&dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", rec.to_str().unwrap()], &[], "").status.success());
    let id = extract_ids(&std::fs::read_to_string(&rec).unwrap()).remove(0);
    assert!(adr_answer(&dir, &id, "A").status.success());
    write_record(
        &dir,
        "adrdom/decisions/lid.md",
        &format!("# Lid\n\n{}", q("Does the widget fit?", "the widget currently breaks.")),
    );
    approve_via_ledger(&dir, "adrdom/decisions/other.md", &format!("# Other\n\n### Does the widget stay?\n\nProblem: p.\n\nStandard: s.\n\n{}", OPTS));
    let (ok, out, _) = check(&dir);
    assert!(!ok, "the approved file with no depends line fails: {}", out);
    assert!(out.contains(&format!("adr: {}", id)), "the untested decision is printed: {}", out);
    assert!(out.contains("\"currently\""), "the draft's finding is printed: {}", out);
    for line in out.lines() {
        assert!(names_a_move(line), "a line with no move: {}\n--- all:\n{}", line, out);
    }
}

#[test]
fn an_untested_decision_carries_the_move_that_builds_it() {
    // adr: 3fc35d4f-90f3-4425-9dac-951a89631fdf
    let dir = repo();
    let rec = write_record(&dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", rec.to_str().unwrap()], &[], "").status.success());
    let id = extract_ids(&std::fs::read_to_string(&rec).unwrap()).remove(0);
    assert!(adr_answer(&dir, &id, "A").status.success());
    let (ok, out, _) = check(&dir);
    assert!(ok, "an untested decision fails nothing: {}", out);
    assert!(
        out.contains(&format!("agent builds \"Does the widget hold?\" and puts \"adr: {}\" in a test that shows it", id)),
        "{}",
        out
    );
    write_test_comment(&dir, &id);
    let (ok, out, _) = check(&dir);
    assert!(ok && out.is_empty(), "once tested, nothing: {}", out);
}

#[test]
fn check_takes_no_flag_that_hides_lines() {
    // adr: 3fc35d4f-90f3-4425-9dac-951a89631fdf
    let dir = repo();
    let out = adr(&dir, &["check", "--quiet"], &[], "");
    assert!(!out.status.success(), "--quiet is no flag of check");
}

#[test]
fn the_hooks_install_writes_run_check_with_no_flag_that_hides_lines() {
    // adr: 3fc35d4f-90f3-4425-9dac-951a89631fdf
    let dir = repo();
    for hook in [".githooks/pre-commit", ".githooks/commit-msg", ".githooks/pre-push"] {
        let text = std::fs::read_to_string(dir.join(hook)).unwrap();
        assert!(text.contains("adr check"), "{}: {}", hook, text);
        assert!(!text.contains("--quiet"), "{}: {}", hook, text);
    }
}

#[test]
fn a_passing_commit_prints_nothing_from_the_hooks() {
    // adr: 3fc35d4f-90f3-4425-9dac-951a89631fdf
    let dir = repo();
    run_git(&dir, &["add", "-A"]);
    let out = run_git(&dir, &["commit", "-q", "-m", "chore: install"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out) + &stderr(&out), "", "a passing commit prints nothing");
}

#[test]
fn what_approve_refuses_in_a_draft_check_reports_with_the_same_move() {
    // adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
    let cases: Vec<(&str, String)> = vec![
        ("loaded word in a body", format!("# Gate\n\n{}", q("Does the widget hold?", "the widget currently breaks."))),
        ("heading with no body", format!("# Gate\n\n{}\n### Does the widget stay?\ndepends: widget\n", q("Does the widget hold?", "the widget breaks."))),
        ("neighbour on no line", format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks."))),
    ];
    for (case, content) in cases {
        let dir = if case == "neighbour on no line" {
            // A second term the question names on no line: a neighbour.
            let dir = init_repo();
            assert!(adr(&dir, &["install"], &[], "").status.success());
            approve_terms_with(&dir, &["widget", "paint"]);
            dir
        } else {
            repo()
        };
        let rec = write_record(&dir, REL, &content);
        if case == "neighbour on no line" {
            // The reader's rows are written; no neighbour is put on a line.
            let plan = adr(&dir, &["plan", rec.to_str().unwrap()], &[], "");
            assert!(stdout(&plan).contains("line missing: "), "{}", stdout(&plan));
        } else {
            plan_record(&dir, REL);
        }
        let out = approve(&dir, &rec);
        assert!(!out.status.success(), "{}: approve refuses", case);
        let (ok, _, _) = check(&dir);
        assert!(ok, "{}: a draft's finding is a note", case);
        assert_check_reports(&dir, &refusals(&out, "approve: "), case);
    }
}

#[test]
fn what_approve_refuses_of_a_removed_file_check_reports_with_the_same_move() {
    // adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
    let dir = repo();
    let rec = write_record(&dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", rec.to_str().unwrap()], &[], "").status.success());
    std::fs::remove_file(&rec).unwrap();
    let out = approve(&dir, &rec);
    assert_check_reports(&dir, &refusals(&out, "approve: "), "removed file with an asked question");
}

#[test]
fn what_approve_refuses_of_a_blame_draft_check_reports_with_the_same_move() {
    // adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
    let (dir, _, _) = blame_setup();
    let blob = write_origin(&dir, "docs/old.md", "one\ntwo\n");
    let blame = write_blame_file(&dir, "adrdom/blame/old.md", "docs/old.md", &blob, "1-2\tdecision: Is there no such thing?\n");
    let out = approve(&dir, &blame);
    let (ok, _, _) = check(&dir);
    assert!(ok, "a draft blame file's finding is a note");
    assert_check_reports(&dir, &refusals(&out, "approve: "), "blame row naming no question");
}

#[test]
fn a_blame_draft_with_a_shape_fault_is_a_note() {
    // adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
    let (dir, _, _) = blame_setup();
    write_record(&dir, "adrdom/blame/old.md", "origin: docs/old.md\n\n1-2\tdropped: dead\n");
    let out = approve(&dir, &dir.join("adrdom/blame/old.md"));
    let (ok, text, _) = check(&dir);
    assert!(ok, "a shape fault on a draft blame file is a note: {}", text);
    assert_check_reports(&dir, &refusals(&out, "approve: "), "blame file with no blob line");
}

#[test]
fn what_approve_refuses_of_an_edited_asked_question_check_reports_with_the_same_move() {
    // adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
    let dir = repo();
    let rec = write_record(&dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", rec.to_str().unwrap()], &[], "").status.success());
    let text = std::fs::read_to_string(&rec).unwrap().replace("the widget breaks.", "the widget currently breaks.");
    std::fs::write(&rec, text).unwrap();
    // The edit puts the plan back to the owner: ask waits for the approval,
    // and approve refuses the loaded word, which check reports.
    let asked = adr(&dir, &["ask", rec.to_str().unwrap()], &[], "");
    assert!(!asked.status.success(), "ask waits for the approval");
    let out = approve(&dir, &rec);
    assert!(stderr(&out).contains("\"currently\""), "{}", stderr(&out));
    assert_check_reports(&dir, &refusals(&out, "approve: "), "asked question edited with a loaded word");
}

#[test]
fn an_approved_file_is_not_judged_by_the_planning_rules_by_approve_or_check() {
    // adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
    // A heading approved with no body before the planning rules: the one
    // scope both commands read leaves it be while the file is approved.
    let dir = repo();
    let content = format!("# Gate\n\n{}\n### Does the widget stay?\ndepends: widget\n", q("Does the widget hold?", "the widget breaks."));
    approve_via_ledger(&dir, REL, &content);
    let (ok, out, _) = check(&dir);
    assert!(ok, "{}", out);
    assert!(!out.contains("no body"), "{}", out);
    let rec = dir.join(REL);
    let out = approve(&dir, &rec);
    assert!(
        !stderr(&out).contains("body") && !stderr(&out).contains("neighbour"),
        "approve on the unchanged file names no planning rule: {}",
        stderr(&out)
    );
}
