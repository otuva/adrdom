// Integration tests for the five decisions answered in
// adrdom/decisions/decision.md, each tied to the decision it shows via an
// `adr: <id>` comment inside its own test function.

mod common;

use common::*;

#[test]
fn principle_block_is_gone_a_p1_line_inside_a_body_is_stray_and_gets_no_id() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");

    let content = "# Example\n\n### 1.1 Is a decision one commitment?\n\nProblem: p.\n\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n\n**P1. Every decision is a question.**\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let stray_line_no = content
        .lines()
        .position(|l| l == "**P1. Every decision is a question.**")
        .unwrap()
        + 1;

    // `**P1. ...**` no longer starts a principle block: with no options
    // section of its own, it just sits as ordinary text inside 1.1's body,
    // which `ask` refuses as a stray line.
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(&format!("stray line {}", stray_line_no)),
        "{}",
        err
    );

    // Nothing written: the file is untouched, so certainly no id was ever
    // assigned to the P1 line (or anywhere else).
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, content, "ask must not modify the file when it refuses");
    assert!(
        extract_ids(&after).is_empty(),
        "no id should have been assigned: {}",
        after
    );
}

// adr: 055d303b-c286-4f6c-975a-d9d8090936d4
//
// Decision A's "two or three lettered options" clause.
#[test]
fn a_question_needs_two_or_three_options() {

    // One option: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        let content = "# Example\n\n### 1.1 One option?\n\nProblem: p.\n\nStandard: s.\n\n- A) Only choice.\n  Why: w.\n  Costs: c.\n";
        write_record(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["check"], &[], "");
        assert!(!out.status.success());
        let text = stdout(&out);
        assert!(text.contains("parse:"), "{}", text);
        assert!(text.contains("1 options; at least two"), "{}", text);
    }

    // Four options: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        let content = "# Example\n\n### 1.1 Four options?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n- D) Four.\n  Why: w.\n  Costs: c.\n";
        write_record(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["check"], &[], "");
        assert!(!out.status.success());
        let text = stdout(&out);
        assert!(text.contains("parse:"), "{}", text);
        assert!(text.contains("4 options; at most three"), "{}", text);
    }

    // Two options: accepted.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "options");
        let content = "# Example\n\n### 1.1 Two options?\ndepends: options\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        approve_record(&dir, "adrdom/decisions/example.md");

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(out.status.success(), "two options should be accepted: {}", stderr(&out));
    }

    // Three options: accepted.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "options");
        let content = "# Example\n\n### 1.1 Three options?\ndepends: options\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        approve_record(&dir, "adrdom/decisions/example.md");

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(out.status.success(), "three options should be accepted: {}", stderr(&out));
    }
}

// adr: 055d303b-c286-4f6c-975a-d9d8090936d4
//
// Decision A's "a one-line Problem, a one-line Standard" clause: this test
// covers presence (exactly one of each); `a_problem_or_standard_value_stays_on_one_line`
// below covers the "one-line" half.
#[test]
fn a_question_needs_a_problem_line_and_a_standard_line() {

    // Missing Problem: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
        assert!(stderr(&out).contains("missing Problem line"), "{}", stderr(&out));
    }

    // Missing Standard: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\n\nProblem: p.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
        assert!(stderr(&out).contains("missing Standard line"), "{}", stderr(&out));
    }

    // Both present: accepted.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        approve_record(&dir, "adrdom/decisions/example.md");

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(out.status.success(), "ask should have accepted it: {}", stderr(&out));
    }
}

// adr: 055d303b-c286-4f6c-975a-d9d8090936d4
//
// Decision A's "an indented Why line and an indented Costs line" clause.
#[test]
fn every_option_needs_a_why_line_and_a_costs_line() {

    // Option A has no Costs: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
        assert!(stderr(&out).contains("option A missing Costs"), "{}", stderr(&out));
    }

    // Option A has no Why: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
        assert!(stderr(&out).contains("option A missing Why"), "{}", stderr(&out));
    }

    // Every option carries both: accepted.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        approve_record(&dir, "adrdom/decisions/example.md");

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(out.status.success(), "ask should have accepted it: {}", stderr(&out));
    }
}

// adr: 055d303b-c286-4f6c-975a-d9d8090936d4
//
// Decision A's "a one-line Problem, a one-line Standard" clause, the half
// `a_question_needs_a_problem_line_and_a_standard_line` above does not
// cover: the parser has no continuation syntax for either field, so a
// second physical line is refused rather than folded into the value.
// A plain continuation line (not itself shaped like a recognized field)
// is refused as a stray line; a second `Problem:`/`Standard:` line is
// refused too, but as the same "missing Problem/Standard line" a zero
// count gets, since the parser only accepts a count of exactly one and
// does not distinguish "too many" from "none".
#[test]
fn a_problem_or_standard_value_stays_on_one_line() {

    // A continuation line under Problem: refused as a stray line.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\n\nProblem: p.\ncontinued on a second line.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        let stray_line_no = content
            .lines()
            .position(|l| l == "continued on a second line.")
            .unwrap()
            + 1;

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
        assert!(
            stderr(&out).contains(&format!("stray line {}", stray_line_no)),
            "{}",
            stderr(&out)
        );
    }

    // A second Problem line: also refused, as a missing-Problem error.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nProblem: p again.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
        assert!(stderr(&out).contains("missing Problem line"), "{}", stderr(&out));
    }
}

// adr: 055d303b-c286-4f6c-975a-d9d8090936d4
//
// Decision A's "each one sentence that responds to the heading" clause.
// A missing decision sentence ("option X has no decision sentence") is a
// *parse* error, and `ask` only reads `shape_errors` -- parse errors are a
// separate list it never looks at (only `check` does, in
// src/commands/check/parse.rs). So an option with no sentence terminator
// at all still gets asked (an id minted, a ledger row written): only the
// later, whole-repo `check` catches it. And once an option does carry a
// sentence, the parser never looks past its first terminator, so a second
// sentence tacked on is accepted by both `ask` and `check` -- "one
// sentence" is enforced (by `check` alone) as "at least one", not
// "exactly one" -- and whether the sentence answers the heading is a
// semantic call the parser makes no attempt at.
#[test]
fn a_missing_decision_sentence_is_caught_by_check_not_ask() {

    // No sentence terminator anywhere in the option: `ask` still succeeds;
    // `check` is what refuses.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) no terminator here\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        approve_record(&dir, "adrdom/decisions/example.md");

        let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(
            ask_out.status.success(),
            "ask does not read parse errors, so a missing decision sentence does not stop it: {}",
            stderr(&ask_out)
        );

        let check_out = adr(&dir, &["check"], &[], "");
        assert!(!check_out.status.success(), "check should have refused: {}", stdout(&check_out));
        assert!(
            stdout(&check_out).contains("option A has no decision sentence"),
            "{}",
            stdout(&check_out)
        );
    }

    // A second sentence after the first: both `ask` and `check` accept it.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) One. And a second sentence the parser never inspects.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        approve_record(&dir, "adrdom/decisions/example.md");

        let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(ask_out.status.success(), "ask should have accepted it: {}", stderr(&ask_out));

        let check_out = adr(&dir, &["check"], &[], "");
        assert!(check_out.status.success(), "check should have accepted it too: {}", stdout(&check_out));
    }
}

// adr: 055d303b-c286-4f6c-975a-d9d8090936d4
//
// Decision A's "an indented Why line and an indented Costs line" clause,
// the half `every_option_needs_a_why_line_and_a_costs_line` above does
// not cover: each detail line must appear exactly once per option, so a
// repeated Why (or Costs) line is refused too, not just a missing one.
#[test]
fn an_option_with_a_repeated_why_or_costs_line_is_refused() {

    // Option A has two Why lines: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Why: w2.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
        assert!(stderr(&out).contains("option A has two Why lines"), "{}", stderr(&out));
    }

    // Option A has two Costs lines: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n  Costs: c2.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
        assert!(stderr(&out).contains("option A has two Costs lines"), "{}", stderr(&out));
    }
}

// adr: 055d303b-c286-4f6c-975a-d9d8090936d4
//
// Decision A's "the answer line comes last" clause. Nothing checks
// ordering: `scan_body` reads a body top to bottom and records whatever it
// finds regardless of position, and `ask`'s own candidate test looks only
// at the ledger (does this id have a matching `asked` row yet?), never at
// where the `**Answer:**` line sits in the file. A body whose
// `**Answer:**` line comes first -- the exact opposite of "comes last" --
// still asks cleanly. (Such a line does trip `check`'s separate rule that
// an `**Answer:**` line must be backed by a matching ledger row -- but a
// hand-written line at the *correct*, last position gets that identical
// complaint, so that rule is not evidence of an ordering check either.)
#[test]
fn an_answer_line_before_the_rest_of_the_body_is_not_rejected_for_its_position() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\n### 1.1 What is a decision?\ndepends: decision\n\n**Answer:** A\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        out.status.success(),
        "ask does not check the answer line's position: {}",
        stderr(&out)
    );
}

// adr: 0a742360-83c2-479d-b858-4bea646c9b36
//
// "the topic file ... hold[s] none": a `Lean: ` line under a heading is
// not a recognized field, so both `ask` and `check` refuse it as a stray
// line, exactly the fixed-shape refusal every other unrecognized line
// gets. The other half of this decision -- the page's question view
// renders no lean -- is tied in serve_test.rs's
// `question_page_never_renders_a_lean_field`.
#[test]
fn a_lean_line_in_the_topic_file_is_stray() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");

    // The topic file carries no agent lean; a `Lean: ` line under a
    // heading is not a recognized field, so it is a stray line.
    let content = "# Example\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nStandard: s.\nLean: A.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let stray_line_no = content.lines().position(|l| l == "Lean: A.").unwrap() + 1;

    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!ask_out.status.success(), "ask should have refused: {}", stdout(&ask_out));
    assert!(
        stderr(&ask_out).contains(&format!("stray line {}", stray_line_no)),
        "{}",
        stderr(&ask_out)
    );

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(!check_out.status.success());
    assert!(
        stdout(&check_out).contains(&format!("stray line {}", stray_line_no)),
        "{}",
        stdout(&check_out)
    );
}
