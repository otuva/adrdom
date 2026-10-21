// A topic file awaiting approval: every plan finding in it is a note in
// `adr check`, never a failure, and the hook commit of it passes. A finding
// of the same kind in an approved file still fails. A fault of the file's
// shape, a stray line, a line the tool writes and a heading in two topic
// files are notes in a draft too (05141f49 "What does check do with a fault
// on a file awaiting approval?"); a finding about a question with force and
// the ledger findings fail either way.
//
// adr: 912f4f3b-1213-435a-af0d-7617017fcff4 "Does check fail a topic file
// awaiting approval?"
mod common;

use common::*;
use std::path::Path;

const REL: &str = "adrdom/decisions/gate.md";

const OPTS: &str = "- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";

fn q(heading: &str, depends: &str, problem: &str, opts: &str) -> String {
    format!(
        "### {}\ndepends: {}\n\nProblem: {}\n\nStandard: plain words.\n\n{}",
        heading, depends, problem, opts
    )
}

fn good() -> String {
    q(
        "Does the widget hold?",
        "widget",
        "the widget breaks.",
        OPTS,
    )
}

/// A draft with one finding each: (name, file text, what check prints).
fn cases(tid: &str) -> Vec<(&'static str, String, &'static str)> {
    let long = "- A) Yes.\n  Why: one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";
    let does2 = "- A) Yes indeed.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";
    vec![
        ("long why", format!("# Gate\n\n{}", q("Does the widget hold?", "widget", "the widget breaks.", long)), "the Why line has 21 words, the most is 20; agent rewrites the line"),
        ("does option two words", format!("# Gate\n\n{}", q("Does the widget hold?", "widget", "the widget breaks.", does2)), "Does"),
        ("loaded phrase in body", format!("# Gate\n\n{}", q("Does the widget hold?", "widget", "the widget currently breaks.", OPTS)), "\"currently\""),
        ("loaded heading", format!("# Gate\n\n{}", q("Which widget holds?", "widget", "the widget breaks.", OPTS)), "\"which\""),
        ("loaded prose", format!("# Gate\n\nAs decided in chat.\n\n{}", good()), "names who said it"),
        ("no term", format!("# Gate\n\n{}", q("Does the lid hold?", "widget", "the lid breaks.", OPTS)), "uses no term"),
        ("set on a term", format!("# Gate\n\n{}", q("Does the widget hold?", &format!("{}=A", tid), "the widget breaks.", OPTS)), "answer set"),
        ("set missing on a question", format!("# Gate\n\n{}\n{}", good(), q("Does the widget stay?", "Does the widget hold?", "the widget breaks.", OPTS)), "without an answer set"),
        ("missing depends line", format!("# Gate\n\n### Does the widget hold?\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n{}", OPTS), "has no depends line"),
        ("unresolved entry", format!("# Gate\n\n{}", q("Does the widget hold?", "nosuchterm", "the widget breaks.", OPTS)), "which is no term"),
        ("cycle", format!("# Gate\n\n{}\n{}", q("Does the widget hold?", "Does the widget stay?=A", "the widget breaks.", OPTS), q("Does the widget stay?", "Does the widget hold?=A", "the widget breaks.", OPTS)), "lead back to their own question"),
    ]
}

/// The found text each shape fault of a case below prints.
fn shape_want(name: &str) -> &'static str {
    match name {
        "no decision sentence" => "has no decision sentence; agent rewrites the option",
        "two sentences" => {
            "decision text has 2 sentences, more than one; agent rewrites the option"
        }
        "second depends line" => "depends second line, line 5; agent rewrites the depends line",
        "second supersedes line" => {
            "supersedes second line, line 6; agent rewrites the supersedes line"
        }
        "second unrelated line" => {
            "unrelated second line, line 6; agent rewrites the unrelated line"
        }
        "second terms line" => "terms second line, line 8; agent rewrites the terms line",
        "duplicate depends entry" => "depends: duplicate entry, line 4; agent removes the entry",
        "duplicate id line" => "duplicate id line; agent removes the id line",
        "second answer line" => "second answer line, line 19; agent removes the answer line",
        "answer line not last" => "answer line not last, line 17; agent removes the line",
        "answer names a missing option" => {
            "answer names missing option C; agent restores the answer line"
        }
        "missing h1" => "missing h1; agent rewrites adrdom/decisions/gate.md",
        "second h1" => "duplicate h1, line 17; agent rewrites adrdom/decisions/gate.md",
        "text before h1" => "text before h1, line 1; agent rewrites adrdom/decisions/gate.md",
        "tier heading" => "tier heading, line 3; agent removes the heading",
        "heading without a question mark" => {
            "line 17: a heading is not a question ending in ?; agent rewrites the line"
        }
        _ => panic!("no finding text for {}", name),
    }
}

fn repo() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget"]);
    dir
}

fn term_id(dir: &Path) -> String {
    let terms = std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap();
    terms
        .lines()
        .find_map(|l| l.strip_prefix("id: "))
        .expect("approved term has an id")
        .trim()
        .to_string()
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
        assert!(
            ok,
            "{}: a draft finding must not fail check: {}",
            name, text
        );
        assert!(
            text.contains(want),
            "{}: the finding is printed as a note: {}",
            name,
            text
        );
        assert!(!text.contains("awaiting approval"), "{}: {}", name, text);
        assert!(failures(&dir).is_empty(), "{}: {}", name, failures(&dir));
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
        assert!(
            !ok,
            "{}: an approved file with a finding fails: {}",
            name, text
        );
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
    assert!(text.contains("appears twice"), "{}", text);

    let dir = repo();
    write_record(
        &dir,
        "adrdom/decisions/other-name.md",
        &format!("# Gate\n\n{}", good()),
    );
    let (ok, text) = check(&dir);
    assert!(ok, "{}", text);
    assert!(text.contains("other-name.md"), "{}", text);
    assert!(
        text.contains("file name does not match its topic"),
        "{}",
        text
    );
}

#[test]
fn a_duplicate_heading_in_one_file_and_a_wrong_file_name_fail_when_approved() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    approve_via_ledger(&dir, REL, &two_same());
    let (ok, text) = check(&dir);
    assert!(!ok, "{}", text);
    assert!(text.contains("appears twice"), "{}", text);

    let dir = repo();
    approve_via_ledger(
        &dir,
        "adrdom/decisions/other-name.md",
        &format!("# Gate\n\n{}", good()),
    );
    let (ok, text) = check(&dir);
    assert!(!ok, "{}", text);
    assert!(text.contains("other-name.md"), "{}", text);
}

#[test]
fn a_note_between_questions_is_a_note_in_a_draft_and_a_failure_when_approved() {
    // adr: 05141f49-d9d2-407c-b60c-ebb195a05864
    let content = format!(
        "# Gate\n\n{}\nA stray note between questions.\n\n{}",
        good(),
        q(
            "Does the widget stay?",
            "widget",
            "the widget breaks.",
            OPTS
        )
    );
    let dir = repo();
    write_record(&dir, REL, &content);
    let (ok, text) = check(&dir);
    assert!(ok, "{}", text);
    assert!(text.contains("stray line 17"), "{}", text);
    assert!(failures(&dir).is_empty());
    let dir = repo();
    approve_via_ledger(&dir, REL, &content);
    assert!(
        failures(&dir).contains("stray line 17"),
        "{}",
        failures(&dir)
    );
}

#[test]
fn a_heading_in_two_files_is_a_note_while_one_is_a_draft_and_a_failure_when_both_are_approved() {
    // adr: 05141f49-d9d2-407c-b60c-ebb195a05864
    let a = format!("# A\n\n{}", good());
    let b = format!("# B\n\n{}", good());
    let dir = repo();
    write_record(&dir, "adrdom/decisions/a.md", &a);
    write_record(&dir, "adrdom/decisions/b.md", &b);
    let (ok, text) = check(&dir);
    assert!(ok, "{}", text);
    assert!(text.contains("duplicate heading"), "{}", text);
    assert!(failures(&dir).is_empty());
    // One approved, one a draft: the draft can still be renamed.
    let dir = repo();
    approve_via_ledger(&dir, "adrdom/decisions/a.md", &a);
    write_record(&dir, "adrdom/decisions/b.md", &b);
    let (ok, text) = check(&dir);
    assert!(ok && text.contains("duplicate heading"), "{}", text);
    // Both approved: it fails.
    let dir = repo();
    approve_via_ledger(&dir, "adrdom/decisions/a.md", &a);
    approve_via_ledger(&dir, "adrdom/decisions/b.md", &b);
    assert!(
        failures(&dir).contains("duplicate heading"),
        "{}",
        failures(&dir)
    );
}

fn failures(dir: &Path) -> String {
    check_failures(dir)
}

#[test]
fn a_finding_in_an_asked_question_stays_a_failure_in_a_draft_file() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let alpha = "11111111-1111-1111-1111-111111111111";
    let beta = "22222222-2222-2222-2222-222222222222";
    let dir = repo();
    append_ledger_row_at(
        &dir,
        "asked",
        alpha,
        "h1",
        "Does the widget hold?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(&dir, "answered", alpha, "h1", "A", "2024-01-01T00:00:01Z");
    append_ledger_row_at(
        &dir,
        "superseded",
        alpha,
        "-",
        "replaced",
        "2024-01-01T00:00:02Z",
    );
    let beta_section = format!(
        "### Does the widget stay?\ndepends: {}\n\nid: {}\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n{}",
        alpha,
        beta,
        OPTS.trim_end_matches('\n')
    );
    append_ledger_row_at(
        &dir,
        "asked",
        beta,
        &mirror_section_hash(&beta_section),
        "Does the widget stay?",
        "2024-01-01T00:00:03Z",
    );
    let gamma = q("Does the widget fall?", alpha, "the widget breaks.", OPTS);
    // The file holds only the asked question: its finding fails check.
    write_record(&dir, REL, &format!("# Gate\n\n{}\n", beta_section));
    let before = failures(&dir);
    assert!(
        before.contains("which names no term and no question of this file"),
        "{}",
        before
    );
    // One unasked question added: the file now awaits approval, but the
    // finding about the asked question still fails; the same finding about
    // the new one is a note.
    write_record(&dir, REL, &format!("# Gate\n\n{}\n{}", beta_section, gamma));
    let failed = failures(&dir);
    assert!(
        failed.contains("\"Does the widget stay?\" depends on"),
        "{}",
        failed
    );
    assert!(
        !failed.contains("\"Does the widget fall?\" depends on"),
        "{}",
        failed
    );
    let all = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(
        all.contains("\"Does the widget fall?\" depends on"),
        "{}",
        all
    );
}

#[test]
fn a_prose_finding_is_a_note_when_only_the_prose_awaits_approval() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    approve_via_ledger(&dir, REL, &format!("# Gate\n\nPlain prose.\n\n{}", good()));
    let (ok, text) = check(&dir);
    assert!(ok, "{}", text);
    assert!(!text.contains("awaiting approval"), "{}", text);
    write_record(
        &dir,
        REL,
        &format!("# Gate\n\nAs decided in chat.\n\n{}", good()),
    );
    let (ok, text) = check(&dir);
    assert!(ok, "{}", text);
    assert!(text.contains("names who said it"), "{}", text);
    assert!(failures(&dir).is_empty());
}

#[test]
fn a_body_shape_finding_is_a_note_in_a_draft_and_a_failure_when_approved() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let one_option = "- A) Yes.\n  Why: w.\n  Costs: c.\n";
    let no_costs = "- A) Yes.\n  Why: w.\n- B) No.\n  Why: w.\n  Costs: c.\n";
    let four = format!(
        "{}{}{}",
        OPTS, "- C) Maybe.\n  Why: w.\n  Costs: c.\n", "- D) Never.\n  Why: w.\n  Costs: c.\n"
    );
    let no_problem = format!(
        "### Does the widget hold?\ndepends: widget\n\nStandard: plain words.\n\n{}",
        OPTS
    );
    let mut drafts: Vec<String> = [one_option, no_costs, four.as_str()]
        .iter()
        .map(|o| {
            format!(
                "# Gate\n\n{}",
                q("Does the widget hold?", "widget", "the widget breaks.", o)
            )
        })
        .collect();
    drafts.push(format!("# Gate\n\n{}", no_problem));
    for content in drafts {
        let dir = repo();
        write_record(&dir, REL, &content);
        let (ok, text) = check(&dir);
        assert!(ok, "{}", text);
        assert!(text.contains("agent rewrites the question"), "{}", text);
        let dir = repo();
        approve_via_ledger(&dir, REL, &content);
        assert!(
            failures(&dir).contains("agent rewrites the question"),
            "{}",
            content
        );
    }
}

#[test]
fn a_stray_line_is_a_note_in_a_draft_and_a_failure_when_approved() {
    // adr: 05141f49-d9d2-407c-b60c-ebb195a05864
    let content = format!(
        "# Gate\n\n{}",
        good().replace("Standard:", "Unknown line.\n\nStandard:")
    );
    let dir = repo();
    write_record(&dir, REL, &content);
    let (ok, text) = check(&dir);
    assert!(ok && text.contains("stray line"), "{}", text);
    assert!(failures(&dir).is_empty());
    let dir = repo();
    approve_via_ledger(&dir, REL, &content);
    assert!(failures(&dir).contains("stray line"), "{}", failures(&dir));
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
    append_ledger_row_at(
        &dir,
        "asked",
        beta,
        &mirror_section_hash(&beta_section),
        "Does the widget stay?",
        "2024-01-01T00:00:03Z",
    );
    write_record(
        &dir,
        REL,
        &format!(
            "# Gate\n\n{}\n\n{}",
            beta_section,
            good().replace("hold", "fall")
        ),
    );
    let failed = failures(&dir);
    assert!(
        failed.contains("Does the widget stay?")
            && failed.contains("which is not a current decision"),
        "{}",
        failed
    );
}

#[test]
fn plan_text_shape_findings_are_notes_in_a_draft_and_failures_when_approved() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let with = |depends: &str, opts: &str| {
        format!(
            "# Gate\n\n{}",
            q("Does the widget hold?", depends, "the widget breaks.", opts)
        )
    };
    let cases = vec![
        (
            "no decision sentence",
            with(
                "widget",
                "- A) Yes\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
            ),
        ),
        (
            "two sentences",
            with(
                "widget",
                "- A) Yes. And more.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
            ),
        ),
        (
            "second depends line",
            format!(
                "# Gate\n\n{}",
                good().replace("depends: widget\n", "depends: widget\ndepends: widget\n")
            ),
        ),
        ("duplicate depends entry", with("widget, widget", OPTS)),
        (
            "second supersedes line",
            format!(
                "# Gate\n\n{}",
                good().replace(
                    "depends: widget\n",
                    "depends: widget\nsupersedes: aaa\nsupersedes: bbb\n"
                )
            ),
        ),
        (
            "second unrelated line",
            format!(
                "# Gate\n\n{}",
                good().replace(
                    "depends: widget\n",
                    "depends: widget\nunrelated: aaa\nunrelated: bbb\n"
                )
            ),
        ),
    ];
    for (name, content) in cases {
        let dir = repo();
        write_record(&dir, REL, &content);
        let (ok, text) = check(&dir);
        assert!(ok, "{}: {}", name, text);
        eprintln!(
            "DBG {} => {}",
            name,
            text.lines()
                .filter(|l| !l.contains("reader rows"))
                .collect::<Vec<_>>()
                .join(" || ")
        );
        assert!(text.contains(shape_want(name)), "{}: {}", name, text);
        assert!(failures(&dir).is_empty(), "{}: {}", name, failures(&dir));
        let dir = repo();
        approve_via_ledger(&dir, REL, &content);
        assert!(
            failures(&dir).contains(shape_want(name)),
            "{}: {}",
            name,
            failures(&dir)
        );
    }
}

#[test]
fn the_tools_own_lines_and_a_file_shape_fault_are_notes_in_a_draft_and_failures_when_approved() {
    // adr: 05141f49-d9d2-407c-b60c-ebb195a05864
    let id = "11111111-1111-1111-1111-111111111111";
    let cases = vec![
        (
            "second terms line",
            format!(
                "# Gate\n\n{}",
                good().replace(
                    "depends: widget\n",
                    "depends: widget\n\nterms: widget\n\nterms: widget\n"
                )
            ),
        ),
        (
            "duplicate id line",
            format!(
                "# Gate\n\n{}",
                good().replace(
                    "depends: widget\n",
                    &format!("depends: widget\n\nid: {}\n\nid: {}\n", id, id)
                )
            ),
        ),
        (
            "second answer line",
            format!("# Gate\n\n{}\n**Answer:** A\n\n**Answer:** B\n", good()),
        ),
        (
            "answer line not last",
            format!("# Gate\n\n{}\n**Answer:** A\n\nTrailing line.\n", good()),
        ),
        (
            "answer names a missing option",
            format!("# Gate\n\n{}\n**Answer:** C\n", good()),
        ),
        ("missing h1", good()),
        ("second h1", format!("# Gate\n\n{}\n# Again\n", good())),
        (
            "text before h1",
            format!("Preface.\n\n# Gate\n\n{}", good()),
        ),
        (
            "tier heading",
            format!("# Gate\n\n## Tier one\n\n{}", good()),
        ),
        (
            "heading without a question mark",
            format!("# Gate\n\n{}\n### Not a question\n", good()),
        ),
    ];
    for (name, content) in cases {
        let dir = repo();
        write_record(&dir, REL, &content);
        let (ok, text) = check(&dir);
        assert!(ok, "{}: a draft fault is a note: {}", name, text);
        assert!(text.contains(shape_want(name)), "{}: {}", name, text);
        assert!(failures(&dir).is_empty(), "{}: {}", name, failures(&dir));
        let dir = repo();
        approve_via_ledger(&dir, REL, &content);
        let failed = failures(&dir);
        assert!(
            failed.contains(shape_want(name)),
            "{}: an approved file fails: {}",
            name,
            failed
        );
    }
}

#[test]
fn a_fault_in_a_question_with_force_and_the_ledger_findings_stay_failures_in_a_draft() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    // adr: 05141f49-d9d2-407c-b60c-ebb195a05864
    let dir = repo();
    let term = term_id(&dir);
    // An answered question whose text was edited since: a failure.
    let section = stated("Does the widget stay?", &term, ASKED, &term, OPTS);
    let hash = mirror_section_hash(&section);
    append_ledger_row_at(
        &dir,
        "asked",
        ASKED,
        &hash,
        "Does the widget stay?",
        "2024-01-01T00:00:03Z",
    );
    append_ledger_row_at(&dir, "answered", ASKED, &hash, "A", "2024-01-01T00:00:04Z");
    let edited = section.replace("the widget breaks.", "the widget fails.");
    write_record(
        &dir,
        REL,
        &format!(
            "# Gate\n\n{}\n\n**Answer:** A\n\n{}",
            edited,
            good().replace("hold", "fall")
        ),
    );
    let failed = failures(&dir);
    assert!(
        failed.contains("text changed since being answered"),
        "{}",
        failed
    );
    // An id with an answer row and no topic file heading: a ledger finding.
    let dir = repo();
    append_ledger_row_at(
        &dir,
        "asked",
        ASKED,
        "h",
        "Does the widget stay?",
        "2024-01-01T00:00:03Z",
    );
    append_ledger_row_at(&dir, "answered", ASKED, "h", "A", "2024-01-01T00:00:04Z");
    write_record(&dir, REL, &format!("# Gate\n\n{}", good()));
    let failed = failures(&dir);
    assert!(
        failed.contains("ids:") || failed.contains(ASKED),
        "{}",
        failed
    );
}

const ASKED: &str = "22222222-2222-2222-2222-222222222222";

/// A complete question section carrying an id and the terms line the tool
/// writes, with `depends` as its depends line and `extra` (options) as its
/// options, from its heading to its last line.
fn stated(heading: &str, depends: &str, id: &str, term: &str, opts: &str) -> String {
    format!(
        "### {}\ndepends: {}\n\nid: {}\n\nterms: widget={}\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n{}",
        heading,
        depends,
        id,
        term,
        opts.trim_end_matches('\n')
    )
}

fn asked_row(dir: &Path, id: &str, section: &str, heading: &str) {
    append_ledger_row_at(
        dir,
        "asked",
        id,
        &mirror_section_hash(section),
        heading,
        "2024-01-01T00:00:03Z",
    );
}

#[test]
fn a_derive_error_in_an_asked_question_stays_a_failure_in_a_draft_file() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    let term = term_id(&dir);
    let section = stated(
        "Does the widget stay?",
        "Does a heading that exists nowhere?",
        ASKED,
        &term,
        OPTS,
    );
    asked_row(&dir, ASKED, &section, "Does the widget stay?");
    // The dangling entry sits in the asked question; an unasked one is added.
    write_record(&dir, REL, &format!("# Gate\n\n{}\n\n{}", section, good()));
    let failed = failures(&dir);
    assert!(failed.contains("exists nowhere"), "{}", failed);
    // The same error in the unasked question is a note.
    let dir = repo();
    write_record(
        &dir,
        REL,
        &format!(
            "# Gate\n\n{}",
            q(
                "Does the widget stay?",
                "Does a heading that exists nowhere?",
                "the widget breaks.",
                OPTS
            )
        ),
    );
    let (ok, text) = check(&dir);
    assert!(ok && text.contains("exists nowhere"), "{}", text);
}

#[test]
fn a_cycle_through_a_question_that_cannot_change_stays_a_failure_in_a_draft_file() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    let term = term_id(&dir);
    let a = stated(
        "Does the widget stay?",
        "Does the widget fall?=A",
        ASKED,
        &term,
        OPTS,
    );
    asked_row(&dir, ASKED, &a, "Does the widget stay?");
    let b = q(
        "Does the widget fall?",
        "Does the widget stay?=A",
        "the widget breaks.",
        OPTS,
    );
    write_record(&dir, REL, &format!("# Gate\n\n{}\n\n{}", a, b));
    let failed = failures(&dir);
    assert!(
        failed.contains("lead back to their own question"),
        "{}",
        failed
    );
}

#[test]
fn a_finding_in_a_draft_question_is_a_note_though_an_answered_one_shares_its_heading() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    let term = term_id(&dir);
    let answered = stated("Does the widget hold?", &term, ASKED, &term, OPTS);
    let hash = mirror_section_hash(&answered);
    append_ledger_row_at(
        &dir,
        "asked",
        ASKED,
        &hash,
        "Does the widget hold?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(&dir, "answered", ASKED, &hash, "A", "2024-01-01T00:00:01Z");
    let draft = q(
        "Does the widget hold?",
        "nosuchterm",
        "the widget breaks.",
        OPTS,
    );
    write_record(
        &dir,
        REL,
        &format!("# Gate\n\n{}\n\n**Answer:** A\n\n{}", answered, draft),
    );
    let failed = failures(&dir);
    assert!(!failed.contains("nosuchterm"), "{}", failed);
    // The duplicate heading is the draft question's fault, the one that
    // copies the heading of a decision: a note.
    assert!(!failed.contains("appears twice"), "{}", failed);
    let all = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(
        all.contains("nosuchterm") && all.contains("appears twice"),
        "{}",
        all
    );
}

#[test]
fn a_duplicate_heading_between_two_questions_with_force_fails_even_in_a_draft_file() {
    // adr: 05141f49-d9d2-407c-b60c-ebb195a05864
    let other = "33333333-3333-3333-3333-333333333333";
    let dir = repo();
    let term = term_id(&dir);
    let mut texts = Vec::new();
    for id in [ASKED, other] {
        let section = stated("Does the widget hold?", &term, id, &term, OPTS);
        let hash = mirror_section_hash(&section);
        append_ledger_row_at(
            &dir,
            "asked",
            id,
            &hash,
            "Does the widget hold?",
            "2024-01-01T00:00:00Z",
        );
        append_ledger_row_at(&dir, "answered", id, &hash, "A", "2024-01-01T00:00:01Z");
        texts.push(format!("{}\n\n**Answer:** A", section));
    }
    write_record(
        &dir,
        REL,
        &format!(
            "# Gate\n\n{}\n\n{}\n\n{}",
            texts[0],
            texts[1],
            q(
                "Does the widget fall?",
                "widget",
                "the widget breaks.",
                OPTS
            )
        ),
    );
    let failed = failures(&dir);
    assert!(failed.contains("appears twice"), "{}", failed);
}

#[test]
fn a_finding_in_a_question_edited_since_it_was_asked_is_a_note() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = repo();
    let term = term_id(&dir);
    let original = stated("Does the widget stay?", &term, ASKED, &term, OPTS);
    asked_row(&dir, ASKED, &original, "Does the widget stay?");
    let edited = stated("Does the widget stay?", "nosuchterm", ASKED, &term, OPTS);
    write_record(&dir, REL, &format!("# Gate\n\n{}\n", edited));
    // The finding itself is a note; a failure may name it only inside its
    // move (the edit after asking clears by the file's moves).
    let failed = failures(&dir);
    let finding_of = |l: &str| l.split_once("; ").map_or(l, |(f, _)| f).to_string();
    assert!(
        !failed.lines().any(|l| finding_of(l).contains("nosuchterm")),
        "{}",
        failed
    );
    let all = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(all.contains("nosuchterm"), "{}", all);
}

#[test]
fn a_body_shape_finding_in_an_asked_or_answered_question_stays_a_failure_in_a_draft_file() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let no_costs = "- A) Yes.\n  Why: w.\n- B) No.\n  Why: w.\n  Costs: c.\n";
    for answered in [false, true] {
        let dir = repo();
        let term = term_id(&dir);
        let section = stated("Does the widget stay?", &term, ASKED, &term, no_costs);
        asked_row(&dir, ASKED, &section, "Does the widget stay?");
        let mut text = section.clone();
        if answered {
            let hash = mirror_section_hash(&section);
            append_ledger_row_at(&dir, "answered", ASKED, &hash, "A", "2024-01-01T00:00:04Z");
            text.push_str("\n\n**Answer:** A");
        }
        write_record(
            &dir,
            REL,
            &format!("# Gate\n\n{}\n\n{}", text, good().replace("hold", "fall")),
        );
        let failed = failures(&dir);
        assert!(
            failed.contains("missing Costs"),
            "answered={}: {}",
            answered,
            failed
        );
    }
}
