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

    let content = "# Example\n\n## Group\n\n### 1.1 Is a decision one commitment?\n\nProblem: p.\n\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n\n**P1. Every decision is a question.**\n";
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

#[test]
fn a_question_needs_two_or_three_options() {

    // One option: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        let content = "# Example\n\n## Group\n\n### 1.1 One option?\n\nProblem: p.\n\nStandard: s.\n\n- A) Only choice.\n  Why: w.\n  Costs: c.\n";
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
        let content = "# Example\n\n## Group\n\n### 1.1 Four options?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n- D) Four.\n  Why: w.\n  Costs: c.\n";
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
        let content = "# Example\n\n## Group\n\n### 1.1 Two options?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        approve_via_ledger(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(out.status.success(), "two options should be accepted: {}", stderr(&out));
    }

    // Three options: accepted.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "options");
        let content = "# Example\n\n## Group\n\n### 1.1 Three options?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        approve_via_ledger(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(out.status.success(), "three options should be accepted: {}", stderr(&out));
    }
}

#[test]
fn a_question_needs_a_problem_line_and_a_standard_line() {

    // Missing Problem: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n## Group\n\n### 1.1 What is a decision?\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
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
        let content = "# Example\n\n## Group\n\n### 1.1 What is a decision?\n\nProblem: p.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
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
        let content = "# Example\n\n## Group\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        approve_via_ledger(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(out.status.success(), "ask should have accepted it: {}", stderr(&out));
    }
}

#[test]
fn every_option_needs_a_why_line_and_a_costs_line() {

    // Option A has no Costs: refused.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n## Group\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
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
        let content = "# Example\n\n## Group\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
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
        let content = "# Example\n\n## Group\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        approve_via_ledger(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(out.status.success(), "ask should have accepted it: {}", stderr(&out));
    }
}

#[test]
fn a_lean_line_in_the_topic_file_is_stray() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");

    // The topic file carries no agent lean; a `Lean: ` line under a
    // heading is not a recognized field, so it is a stray line.
    let content = "# Example\n\n## Group\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nStandard: s.\nLean: A.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
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
