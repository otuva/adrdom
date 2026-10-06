// A topic file awaiting approval: every plan finding in it is a note in
// `adr check`, never a failure, and the hook commit of it passes. A finding
// of the same kind in an approved file still fails, and so do a note between
// questions and a heading in two topic files, whose own decisions say they
// fail check.
//
// adr: 912f4f3b-1213-435a-af0d-7617017fcff4 "Does check fail a topic file
// awaiting approval?"
mod common;

use common::*;
use std::path::Path;

const REL: &str = "adrdom/decisions/gate.md";

const OPTS: &str = "- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";

fn q(heading: &str, depends: &str, problem: &str, opts: &str) -> String {
    format!("### {}\ndepends: {}\n\nProblem: {}\n\nStandard: plain words.\n\n{}", heading, depends, problem, opts)
}

fn good() -> String {
    q("Does the widget hold?", "widget", "the widget breaks.", OPTS)
}

/// A draft with one finding each: (name, file text, what check prints).
fn cases(tid: &str) -> Vec<(&'static str, String, &'static str)> {
    let long = "- A) Yes.\n  Why: one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";
    let does2 = "- A) Yes indeed.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";
    vec![
        ("long why", format!("# Gate\n\n{}", q("Does the widget hold?", "widget", "the widget breaks.", long)), "limit is 20"),
        ("does option two words", format!("# Gate\n\n{}", q("Does the widget hold?", "widget", "the widget breaks.", does2)), "Does"),
        ("loaded phrase in body", format!("# Gate\n\n{}", q("Does the widget hold?", "widget", "the widget currently breaks.", OPTS)), "\"currently\""),
        ("loaded heading", format!("# Gate\n\n{}", q("Which widget holds?", "widget", "the widget breaks.", OPTS)), "\"which\""),
        ("loaded prose", format!("# Gate\n\nAs decided in chat.\n\n{}", good()), "tells:"),
        ("no term", format!("# Gate\n\n{}", q("Does the lid hold?", "widget", "the lid breaks.", OPTS)), "uses no term"),
        ("set on a term", format!("# Gate\n\n{}", q("Does the widget hold?", &format!("{}=A", tid), "the widget breaks.", OPTS)), "answer set"),
        ("set missing on a question", format!("# Gate\n\n{}\n{}", good(), q("Does the widget stay?", "Does the widget hold?", "the widget breaks.", OPTS)), "without an answer set"),
        ("missing depends line", format!("# Gate\n\n### Does the widget hold?\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n{}", OPTS), "plan:"),
        ("unresolved entry", format!("# Gate\n\n{}", q("Does the widget hold?", "nosuchterm", "the widget breaks.", OPTS)), "which is no term"),
        ("cycle", format!("# Gate\n\n{}\n{}", q("Does the widget hold?", "Does the widget stay?=A", "the widget breaks.", OPTS), q("Does the widget stay?", "Does the widget hold?=A", "the widget breaks.", OPTS)), "plan:"),
    ]
}

fn repo() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget"]);
    dir
}

fn term_id(dir: &Path) -> String {
    let terms = std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap();
    terms.lines().find_map(|l| l.strip_prefix("id: ")).expect("approved term has an id").trim().to_string()
}

fn check(dir: &Path) -> (bool, String) {
    let out = adr(dir, &["check"], &[], "");
    (out.status.success(), stdout(&out) + &stderr(&out))
}

#[test]
fn a_draft_finding_is_a_note_not_a_failure() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    for i in 0..cases("").len() {
        let dir = repo();
        let (name, content, want) = cases(&term_id(&dir)).remove(i);
        write_record(&dir, REL, &content);
        let (ok, text) = check(&dir);
        assert!(ok, "{}: a draft finding must not fail check: {}", name, text);
        assert!(text.contains(want), "{}: the finding is printed as a note: {}", name, text);
        assert!(text.contains("awaiting approval"), "{}: {}", name, text);
        let quiet = adr(&dir, &["check", "--quiet"], &[], "");
        assert!(quiet.status.success() && stdout(&quiet).is_empty(), "{}: {}", name, stdout(&quiet));
    }
}

#[test]
fn the_commit_of_a_draft_with_findings_passes_the_hook() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "chore: install"]);
    let (_, content, _) = cases("").remove(2);
    write_record(&dir, REL, &content);
    run_git(&dir, &["add", "-A"]);
    let out = run_git(&dir, &["commit", "-q", "-m", "docs: draft the gate plan"]);
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn the_same_findings_in_an_approved_file_still_fail() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    for i in 0..cases("").len() {
        let dir = repo();
        let (name, content, want) = cases(&term_id(&dir)).remove(i);
        approve_via_ledger(&dir, REL, &content);
        let (ok, text) = check(&dir);
        assert!(!ok, "{}: an approved file with a finding fails: {}", name, text);
        assert!(text.contains(want), "{}: {}", name, text);
    }
}

fn two_same() -> String {
    format!("# Gate\n\n{}\n{}", good(), good())
}

#[test]
fn a_duplicate_heading_in_one_file_and_a_wrong_file_name_are_notes_in_a_draft() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    write_record(&dir, REL, &two_same());
    let (ok, text) = check(&dir);
    assert!(ok, "{}", text);
    assert!(text.contains("duplicate heading"), "{}", text);

    let dir = repo();
    write_record(&dir, "adrdom/decisions/other-name.md", &format!("# Gate\n\n{}", good()));
    let (ok, text) = check(&dir);
    assert!(ok, "{}", text);
    assert!(text.contains("other-name.md"), "{}", text);
    assert!(text.contains("plan:"), "{}", text);
}

#[test]
fn a_duplicate_heading_in_one_file_and_a_wrong_file_name_fail_when_approved() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    approve_via_ledger(&dir, REL, &two_same());
    let (ok, text) = check(&dir);
    assert!(!ok, "{}", text);
    assert!(text.contains("duplicate heading"), "{}", text);

    let dir = repo();
    approve_via_ledger(&dir, "adrdom/decisions/other-name.md", &format!("# Gate\n\n{}", good()));
    let (ok, text) = check(&dir);
    assert!(!ok, "{}", text);
    assert!(text.contains("other-name.md"), "{}", text);
}

#[test]
fn a_note_between_questions_in_a_draft_still_fails() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    write_record(&dir, REL, &format!("# Gate\n\n{}\nA stray note between questions.\n\n{}", good(), q("Does the widget stay?", "widget", "the widget breaks.", OPTS)));
    let (ok, text) = check(&dir);
    assert!(!ok, "{}", text);
}

#[test]
fn a_heading_in_two_draft_files_still_fails() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    write_record(&dir, "adrdom/decisions/a.md", &format!("# A\n\n{}", good()));
    write_record(&dir, "adrdom/decisions/b.md", &format!("# B\n\n{}", good()));
    let (ok, text) = check(&dir);
    assert!(!ok, "{}", text);
    assert!(text.contains("duplicate heading"), "{}", text);
}
