// What `adr check` prints: every line a finding with its move, nothing on a
// pass, by hand or in a hook (3fc35d4f "Does check print a line that names no
// move?"); and every rule `approve` or `ask` refuses on is one judgement
// check reads, so what they refuse, check reports with the same move
// (fe14991d "Does a command refuse what check passes?").
mod common;

use common::*;
use common::{approve_unplanned as approve, gate_question as q, refusal_lines as refusals, widget_repo as repo, GATE_OPTIONS as OPTS};
use std::path::Path;

const REL: &str = "adrdom/decisions/gate.md";

fn check(dir: &Path) -> (bool, String, String) {
    let out = adr(dir, &["check"], &[], "");
    (out.status.success(), stdout(&out), stderr(&out))
}

/// True when `line` names the move that follows it: who makes it, or that
/// its turn has not come.
fn names_a_move(line: &str) -> bool {
    line.contains("; agent ") || line.contains("; owner runs: ") || line.contains("; not yet: ")
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
    // adr: 80b89c24-7d65-43f4-bb03-55e9d85c327d
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
            assert!(stdout(&plan).contains("is on no line"), "{}", stdout(&plan));
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
    // adr: 80b89c24-7d65-43f4-bb03-55e9d85c327d
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
    // adr: 80b89c24-7d65-43f4-bb03-55e9d85c327d
    // A row in the old form (it names a question) is a row whose value is
    // neither word: a shape fault of the draft.
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/old.md", "one\ntwo\n");
    let blame = write_blame_file(&dir, "adrdom/blame/old.md", "docs/old.md", &blob, "1-2\tdecision: Is there no such thing?\n");
    let out = approve(&dir, &blame);
    let (ok, _, _) = check(&dir);
    assert!(ok, "a draft blame file's finding is a note");
    assert_check_reports(&dir, &refusals(&out, "approve: "), "blame row in the old form");
}

#[test]
fn a_blame_draft_with_a_shape_fault_is_a_note() {
    // adr: 80b89c24-7d65-43f4-bb03-55e9d85c327d
    let dir = blame_setup();
    // The origin is there: the shape fault is the file's only finding.
    write_origin(&dir, "docs/old.md", "one\ntwo\n");
    write_record(&dir, "adrdom/blame/old.md", "origin: docs/old.md\n\n1-2\tdetail\n");
    let out = approve(&dir, &dir.join("adrdom/blame/old.md"));
    let (ok, text, _) = check(&dir);
    assert!(ok, "a shape fault on a draft blame file is a note: {}", text);
    assert_check_reports(&dir, &refusals(&out, "approve: "), "blame file with no blob line");
}

#[test]
fn what_approve_refuses_of_an_edited_asked_question_check_reports_with_the_same_move() {
    // adr: 80b89c24-7d65-43f4-bb03-55e9d85c327d
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
    // adr: 80b89c24-7d65-43f4-bb03-55e9d85c327d
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

#[test]
fn check_prints_every_finding_on_standard_output_whatever_its_exit() {
    // adr: 3fc35d4f-90f3-4425-9dac-951a89631fdf
    // One kind, one routing: a finding goes to standard output whether
    // check passes with notes or fails; standard error is for a stop of
    // check itself.
    let dir = repo();
    // A note on a draft: check passes and prints the finding.
    write_record(&dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget currently breaks.")));
    plan_record(&dir, REL);
    let (ok, out, err) = check(&dir);
    assert!(ok, "{}{}", out, err);
    assert!(!out.is_empty(), "the note is printed");
    assert_eq!(err, "", "a note goes to standard output");
    // A failure: a ledger row edited fails check, its finding on stdout too.
    let ledger = dir.join("adrdom/state/ledger.tsv");
    run_git(&dir, &["add", "-A"]);
    assert!(run_git(&dir, &["commit", "-q", "-m", "x"]).status.success());
    let text = std::fs::read_to_string(&ledger).unwrap();
    std::fs::write(&ledger, text.replacen("\t", "\t ", 1)).unwrap();
    let (ok, out, err) = check(&dir);
    assert!(!ok, "an edited ledger row fails check");
    assert!(!out.is_empty(), "the failure is printed");
    assert_eq!(err, "", "a failure goes to standard output");
    // A stop of check itself (no repository) goes to standard error.
    let outside = OutsideDir::new("not-a-repo");
    let stop = adr(outside.path(), &["check"], &[], "");
    assert!(!stop.status.success(), "check outside a repository stops");
    assert_eq!(stdout(&stop), "", "nothing on standard output");
    assert!(!stderr(&stop).is_empty(), "the stop is on standard error");
}

#[test]
fn a_commit_prints_each_finding_once_and_the_message_hook_reads_the_message_alone() {
    // adr: 3fc35d4f-90f3-4425-9dac-951a89631fdf
    // The pre-commit hook judges the staged files; the commit-msg hook reads
    // the message alone, for banned words (per "What does the ban scan
    // read?"), so an untested decision is named once a commit.
    let dir = repo();
    let rec = write_record(&dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", rec.to_str().unwrap()], &[], "").status.success());
    let id = extract_ids(&std::fs::read_to_string(&rec).unwrap()).remove(0);
    assert!(adr_answer(&dir, &id, "A").status.success());
    run_git(&dir, &["add", "-A"]);
    let out = run_git(&dir, &["commit", "-q", "-m", "docs: answer the gate"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let said = stdout(&out) + &stderr(&out);
    assert_eq!(said.matches(&format!("adr: {}\" in a test", id)).count(), 1, "{}", said);

    // A banned word in the message still stops the commit.
    let ban = adr(&dir, &["ban"], &[], "zorblax\n");
    assert!(ban.status.success(), "{}", stderr(&ban));
    write_test_comment(&dir, &id);
    run_git(&dir, &["add", "-A"]);
    let out = run_git(&dir, &["commit", "-q", "-m", "chore: zorblax"]);
    assert!(!out.status.success(), "a banned word in the message stops the commit");
    let said = stdout(&out) + &stderr(&out);
    assert!(said.contains("banned word"), "{}", said);
    assert!(!said.to_lowercase().contains("zorblax"), "the word itself is never printed: {}", said);
}
