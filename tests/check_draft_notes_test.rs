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

fn quiet(dir: &Path) -> String {
    stdout(&adr(dir, &["check", "--quiet"], &[], ""))
}

#[test]
fn a_finding_in_an_asked_question_stays_a_failure_in_a_draft_file() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let alpha = "11111111-1111-1111-1111-111111111111";
    let beta = "22222222-2222-2222-2222-222222222222";
    let dir = repo();
    append_ledger_row_at(&dir, "asked", alpha, "h1", "Does the widget hold?", "2024-01-01T00:00:00Z");
    append_ledger_row_at(&dir, "answered", alpha, "h1", "A", "2024-01-01T00:00:01Z");
    append_ledger_row_at(&dir, "superseded", alpha, "-", "replaced", "2024-01-01T00:00:02Z");
    let beta_section = format!(
        "### Does the widget stay?\ndepends: {}\n\nid: {}\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n{}",
        alpha,
        beta,
        OPTS.trim_end_matches('\n')
    );
    append_ledger_row_at(&dir, "asked", beta, &mirror_section_hash(&beta_section), "Does the widget stay?", "2024-01-01T00:00:03Z");
    let gamma = q("Does the widget fall?", &alpha.to_string(), "the widget breaks.", OPTS);
    // The file holds only the asked question: its finding fails check.
    write_record(&dir, REL, &format!("# Gate\n\n{}\n", beta_section));
    let before = quiet(&dir);
    assert!(before.contains("which is not a current decision"), "{}", before);
    // One unasked question added: the file now awaits approval, but the
    // finding about the asked question still fails; the same finding about
    // the new one is a note.
    write_record(&dir, REL, &format!("# Gate\n\n{}\n{}", beta_section, gamma));
    let failures = quiet(&dir);
    assert!(failures.contains("\"Does the widget stay?\" depends on"), "{}", failures);
    assert!(!failures.contains("\"Does the widget fall?\" depends on"), "{}", failures);
    let all = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(all.contains("\"Does the widget fall?\" depends on"), "{}", all);
}

#[test]
fn a_prose_finding_is_a_note_when_only_the_prose_awaits_approval() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    approve_via_ledger(&dir, REL, &format!("# Gate\n\nPlain prose.\n\n{}", good()));
    let (ok, text) = check(&dir);
    assert!(ok, "{}", text);
    assert!(!text.contains("awaiting approval"), "{}", text);
    write_record(&dir, REL, &format!("# Gate\n\nAs decided in chat.\n\n{}", good()));
    let (ok, text) = check(&dir);
    assert!(ok, "{}", text);
    assert!(text.contains("tells:") && text.contains("awaiting approval"), "{}", text);
    assert!(quiet(&dir).is_empty());
}

#[test]
fn a_body_shape_finding_is_a_note_in_a_draft_and_a_failure_when_approved() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let one_option = "- A) Yes.\n  Why: w.\n  Costs: c.\n";
    let no_costs = "- A) Yes.\n  Why: w.\n- B) No.\n  Why: w.\n  Costs: c.\n";
    let four = format!("{}{}{}", OPTS, "- C) Maybe.\n  Why: w.\n  Costs: c.\n", "- D) Never.\n  Why: w.\n  Costs: c.\n");
    let no_problem = format!("### Does the widget hold?\ndepends: widget\n\nStandard: plain words.\n\n{}", OPTS);
    let mut drafts: Vec<String> = [one_option, no_costs, four.as_str()]
        .iter()
        .map(|o| format!("# Gate\n\n{}", q("Does the widget hold?", "widget", "the widget breaks.", o)))
        .collect();
    drafts.push(format!("# Gate\n\n{}", no_problem));
    for content in drafts {
        let dir = repo();
        write_record(&dir, REL, &content);
        let (ok, text) = check(&dir);
        assert!(ok, "{}", text);
        assert!(text.contains("parse:"), "{}", text);
        let dir = repo();
        approve_via_ledger(&dir, REL, &content);
        assert!(quiet(&dir).contains("parse:"), "{}", content);
    }
    // Text that belongs to no part of the shape keeps failing in a draft.
    let dir = repo();
    write_record(&dir, REL, &format!("# Gate\n\n{}", good().replace("Standard:", "Unknown line.\n\nStandard:")));
    assert!(quiet(&dir).contains("parse:"), "a stray line fails");
}

#[test]
fn a_supersedes_finding_in_an_asked_question_stays_a_failure_in_a_draft_file() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let beta = "22222222-2222-2222-2222-222222222222";
    let nothing = "33333333-3333-3333-3333-333333333333";
    let dir = repo();
    let beta_section = format!(
        "### Does the widget stay?\ndepends: widget\nsupersedes: {}\n\nid: {}\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n{}",
        nothing,
        beta,
        OPTS.trim_end_matches('\n')
    );
    append_ledger_row_at(&dir, "asked", beta, &mirror_section_hash(&beta_section), "Does the widget stay?", "2024-01-01T00:00:03Z");
    write_record(&dir, REL, &format!("# Gate\n\n{}\n\n{}", beta_section, good().replace("hold", "fall")));
    let failures = quiet(&dir);
    assert!(failures.contains("Does the widget stay?") && failures.contains("which is not a current decision"), "{}", failures);
    let all = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(all.contains("awaiting approval"), "{}", all);
}

#[test]
fn plan_text_shape_findings_are_notes_in_a_draft_and_failures_when_approved() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let with = |depends: &str, opts: &str| format!("# Gate\n\n{}", q("Does the widget hold?", depends, "the widget breaks.", opts));
    let cases = vec![
        ("no decision sentence", with("widget", "- A) Yes\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n")),
        ("two sentences", with("widget", "- A) Yes. And more.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n")),
        ("second depends line", format!("# Gate\n\n{}", good().replace("depends: widget\n", "depends: widget\ndepends: widget\n"))),
        ("duplicate depends entry", with("widget, widget", OPTS)),
        ("second supersedes line", format!("# Gate\n\n{}", good().replace("depends: widget\n", "depends: widget\nsupersedes: aaa\nsupersedes: bbb\n"))),
        ("second unrelated line", format!("# Gate\n\n{}", good().replace("depends: widget\n", "depends: widget\nunrelated: aaa\nunrelated: bbb\n"))),
    ];
    for (name, content) in cases {
        let dir = repo();
        write_record(&dir, REL, &content);
        let (ok, text) = check(&dir);
        assert!(ok, "{}: {}", name, text);
        assert!(text.contains("awaiting approval") && text.contains("parse:"), "{}: {}", name, text);
        assert!(quiet(&dir).is_empty(), "{}: {}", name, quiet(&dir));
        let dir = repo();
        approve_via_ledger(&dir, REL, &content);
        assert!(quiet(&dir).contains("parse:"), "{}: {}", name, quiet(&dir));
    }
}

#[test]
fn the_tools_own_lines_and_the_ledger_findings_stay_failures_in_a_draft() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let id = "11111111-1111-1111-1111-111111111111";
    let cases = vec![
        ("second terms line", good().replace("depends: widget\n", "depends: widget\n\nterms: widget\n\nterms: widget\n")),
        ("duplicate id line", good().replace("depends: widget\n", &format!("depends: widget\n\nid: {}\n\nid: {}\n", id, id))),
        ("second answer line", format!("{}\n**Answer:** A\n\n**Answer:** B\n", good())),
        ("answer names a missing option", format!("{}\n**Answer:** C\n", good())),
        ("unknown line", good().replace("Standard:", "Unknown line.\n\nStandard:")),
    ];
    for (name, body) in cases {
        let dir = repo();
        write_record(&dir, REL, &format!("# Gate\n\n{}", body));
        let failures = quiet(&dir);
        assert!(failures.contains("parse:"), "{}: {}", name, failures);
    }
}
