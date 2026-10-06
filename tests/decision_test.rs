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

//
// Decision A's "a one-line Problem, a one-line Standard" clause, the half
// `a_question_needs_a_problem_line_and_a_standard_line` above does not
// cover: the parser has no continuation syntax for either field, so a
// second physical line is refused rather than folded into the value.
// A plain continuation line (not itself shaped like a recognized field)
// is refused as a stray line; a second `Problem:`/`Standard:` line is
// refused too, and (point 4) as its own "duplicate Problem/Standard line"
// message, distinct from the "missing" message a zero count gets.
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

    // A second Problem line: refused as a duplicate, not a missing-Problem
    // error.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let content = "# Example\n\n### 1.1 What is a decision?\n\nProblem: p.\n\nProblem: p again.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);

        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
        assert!(stderr(&out).contains("duplicate Problem line"), "{}", stderr(&out));
        assert!(!stderr(&out).contains("missing Problem line"), "{}", stderr(&out));
    }
}

//
// Decision A's "each one sentence that responds to the heading" clause.
// An option with no decision sentence at all, and an option whose decision
// text holds a second sentence, are both parse errors, and `ask` now reads
// them (point 1/point 2) the same way `check` does, through the shared
// "parse" category in `actions::record_refusals` -- so neither an id nor a
// ledger row is ever written for either mistake, and `check` finds the
// identical text afterward.
#[test]
fn a_missing_or_multi_sentence_decision_is_caught_by_ask_and_check() {
    // `approve` now reads the same shared "parse" category, so it refuses
    // on either mistake too -- a file carrying one can never get an id
    // minted in the first place. Each broken case is built by approving a
    // clean, one-sentence version (so the plan is approved and the heading
    // has an id) and then editing only the option's decision text: that
    // edit moves neither the plan hash (heading/id/depends) nor any
    // ledger-backed section hash (nothing has been asked or answered yet),
    // so the file is still considered approved and `ask` reaches its own
    // "parse" check on the edited text.
    let clean = |letter_a: &str| {
        format!(
            "# Example\n\n### 1.1 What is a decision?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) {}\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n",
            letter_a
        )
    };

    // No sentence terminator anywhere in the option: `ask` refuses; `check`
    // reports the same text.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let record = write_record(&dir, "adrdom/decisions/example.md", &clean("One."));
        approve_record(&dir, "adrdom/decisions/example.md");
        let approved = std::fs::read_to_string(&record).unwrap();
        std::fs::write(&record, approved.replace("- A) One.\n", "- A) no terminator here\n")).unwrap();

        let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!ask_out.status.success(), "ask should have refused: {}", stdout(&ask_out));
        assert!(
            stderr(&ask_out).contains("option A has no decision sentence"),
            "{}",
            stderr(&ask_out)
        );

        let check_out = adr(&dir, &["check"], &[], "");
        // The edited file awaits approval again: a note (912f4f3b-1213-435a-af0d-7617017fcff4).
        assert!(check_out.status.success(), "check prints it as a note: {}", stdout(&check_out));
        assert!(
            stdout(&check_out).contains("option A has no decision sentence"),
            "{}",
            stdout(&check_out)
        );
    }

    // A second sentence after the first: same treatment.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let record = write_record(&dir, "adrdom/decisions/example.md", &clean("One."));
        approve_record(&dir, "adrdom/decisions/example.md");
        let approved = std::fs::read_to_string(&record).unwrap();
        std::fs::write(
            &record,
            approved.replace("- A) One.\n", "- A) One. And a second sentence.\n"),
        )
        .unwrap();

        let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(!ask_out.status.success(), "ask should have refused: {}", stdout(&ask_out));
        assert!(
            stderr(&ask_out).contains("option A decision text has 2 sentences, more than one"),
            "{}",
            stderr(&ask_out)
        );

        let check_out = adr(&dir, &["check"], &[], "");
        assert!(check_out.status.success(), "check prints it as a note: {}", stdout(&check_out));
        assert!(
            stdout(&check_out).contains("option A decision text has 2 sentences, more than one"),
            "{}",
            stdout(&check_out)
        );
    }

    // Exactly one sentence: both accept it.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        approve_minimal_terms(&dir, "decision");
        let record = write_record(&dir, "adrdom/decisions/example.md", &clean("One."));
        approve_record(&dir, "adrdom/decisions/example.md");

        let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(ask_out.status.success(), "ask should have accepted it: {}", stderr(&ask_out));

        let check_out = adr(&dir, &["check"], &[], "");
        assert!(check_out.status.success(), "check should have accepted it too: {}", stdout(&check_out));
    }
}

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

//
// Decision A's "the answer line comes last" clause (point 3). A body whose
// `**Answer:**` line comes first -- the exact opposite of "comes last" --
// is refused by both `ask` and `check`, naming the position. `approve` now
// reads the same shared "parse" category too, so the file is approved
// clean first and then edited to insert the misplaced line: that edit
// touches neither the plan hash nor any ledger-backed section hash, so the
// file stays approved and `ask` reaches its own "parse" check on it.
#[test]
fn an_answer_line_before_the_rest_of_the_body_is_rejected_for_its_position() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\n### 1.1 What is a decision?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_record(&dir, "adrdom/decisions/example.md");

    let approved = std::fs::read_to_string(&record).unwrap();
    let broken = approved.replacen("\n\nProblem: p.", "\n\n**Answer:** A\n\nProblem: p.", 1);
    assert_ne!(broken, approved, "the insertion point must exist in the approved file");
    std::fs::write(&record, &broken).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    assert!(stderr(&out).contains("answer line not last"), "{}", stderr(&out));

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(!check_out.status.success(), "check should have refused: {}", stdout(&check_out));
    assert!(stdout(&check_out).contains("answer line not last"), "{}", stdout(&check_out));
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

// adr: 0a742360-83c2-479d-b858-4bea646c9b36
//
// "the lean lives in the conversation where the question is presented;
// the topic file, the ledger and the page hold none": the ledger side.
// The row `ask` writes for a question and the row `answer` writes for it
// both keep the ledger's fixed six-field shape (time, event, id, hash,
// detail, tool version, per ledger.md's "What does a ledger row hold?")
// -- the `asked` row's detail is the plain heading text and the
// `answered` row's detail is just the letter, neither one a field a lean
// sentence could be tucked into -- and no row anywhere in the ledger
// contains the word "lean".
#[test]
fn ledger_rows_for_ask_and_answer_carry_no_lean() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\n### 1.1 What is a decision?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let after_ask = std::fs::read_to_string(&record).unwrap();
    let question_id = extract_ids(&after_ask)[0].clone();
    assert!(adr_answer(&dir, &question_id, "A")
        .status
        .success());

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_row = ledger
        .lines()
        .find(|l| l.contains("\tasked\t") && l.contains(question_id.as_str()))
        .unwrap_or_else(|| panic!("no asked row: {}", ledger));
    let answered_row = ledger
        .lines()
        .find(|l| l.contains("\tanswered\t") && l.contains(question_id.as_str()))
        .unwrap_or_else(|| panic!("no answered row: {}", ledger));

    let asked_fields: Vec<&str> = asked_row.split('\t').collect();
    let answered_fields: Vec<&str> = answered_row.split('\t').collect();
    assert_eq!(asked_fields.len(), 6, "{:?}", asked_fields);
    assert_eq!(answered_fields.len(), 6, "{:?}", answered_fields);

    // detail = index 4: the heading text for `asked`, just the letter for
    // `answered` -- no room for a lean sentence in either.
    assert_eq!(asked_fields[4], "1.1 What is a decision?");
    assert_eq!(answered_fields[4], "A");

    assert!(
        !ledger.to_lowercase().contains("lean"),
        "ledger carries no lean text anywhere: {}",
        ledger
    );
}
