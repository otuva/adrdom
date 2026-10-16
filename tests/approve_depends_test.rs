mod common;

use common::*;

/// One root-tier question block, self-contained: heading, `depends:
/// decision` (a term every test in this file approves via
/// `approve_minimal_terms(&dir, "decision")`), Problem/Standard, two
/// options.
fn root_question(n: usize) -> String {
    format!(
        "### {n}.1 Root {n}?\ndepends: decision\n\nProblem: p{n} of the decision.\n\nStandard: s{n}.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n",
        n = n
    )
}

fn setup(dir: &std::path::Path) {
    assert!(adr(dir, &["install"], &[], "").status.success());
    approve_minimal_terms(dir, "decision");
}

// --- no depends line ----------------------------------------------------

// adr: 9bf76db8-4821-4257-ac49-4c22c2a0e538
#[test]
fn approve_refuses_a_heading_with_no_depends_line() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n### 1.1 No depends?\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 No depends?\" has no depends line; agent writes the depends line of"),
        "{}",
        err
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger
            .lines()
            .any(|l| l.contains("\tapproved\t") || l.contains("\tcontext\t")),
        "no row should have been written on refusal: {}",
        ledger
    );
}

// adr: 9afe335c-2a7a-4625-b115-f74d3f091cac
// adr: 9bf76db8-4821-4257-ac49-4c22c2a0e538
//
// A depends entry names a term, or a question of the same topic file;
// approve stops a line that is missing. A heading with no depends line at
// all refuses; a heading whose depends line names a real term, and a
// heading whose line names a question elsewhere in the same file, resolve,
// and approve accepts both.
#[test]
fn approve_accepts_a_term_and_a_question_of_the_file_and_refuses_a_missing_line() {
    let dir = init_repo();
    setup(&dir);

    let missing = "# Missing\n\n### 1.1 No depends?\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let missing_record = write_record(&dir, "adrdom/decisions/missing.md", missing);
    let out = adr_owner(&dir, &["approve", missing_record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("\"1.1 No depends?\" has no depends line"),
        "{}",
        stderr(&out)
    );

    let content = "# Example\n\n\
        ### First?\ndepends: decision\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
        ### Second?\ndepends: First?=A\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    let out2 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        out2.status.success(),
        "approve should accept a term and a question of the file: {}",
        stderr(&out2)
    );
}

/// An answered question in `adrdom/decisions/earlier.md`, a current decision
/// of another topic file; returns its identifier.
fn earlier_decision(dir: &std::path::Path) -> String {
    let decided_id = "66666666-6666-6666-6666-666666666666";
    append_ledger_row_at(
        dir,
        "asked",
        decided_id,
        "h1",
        "Some earlier heading?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(
        dir,
        "answered",
        decided_id,
        "h1",
        "A",
        "2024-01-01T00:00:01Z",
    );
    let earlier = format!(
        "# Earlier\n\n### Some earlier heading?\ndepends: decision\n\nid: {}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        decided_id
    );
    write_record(dir, "adrdom/decisions/earlier.md", &earlier);
    decided_id.to_string()
}

fn one_question(heading: &str, depends: &str) -> String {
    format!(
        "# Example\n\n### {}\ndepends: {}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        heading, depends
    )
}

const OTHER_FILE_REFUSAL: &str =
    "a decision of another topic file; agent rewrites the depends line of";

// adr: 9afe335c-2a7a-4625-b115-f74d3f091cac
//
// A depends entry names a term, or a question of the same topic file: a
// current decision of another file, written by its identifier or by its
// heading text, is refused at approve and the entry is named; the refusal is
// check's finding on the draft too.
#[test]
fn approve_refuses_an_entry_naming_a_current_decision_of_another_file() {
    let dir = init_repo();
    setup(&dir);
    let decided_id = earlier_decision(&dir);

    for entry in [decided_id.clone(), "Some earlier heading?".to_string()] {
        let record = write_record(
            &dir,
            "adrdom/decisions/example.md",
            &one_question("Third?", &entry),
        );
        let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
        assert!(
            !out.status.success(),
            "approve should have refused {}: {}",
            entry,
            stdout(&out)
        );
        let err = stderr(&out);
        assert!(err.contains("\"Third?\" depends on "), "{}", err);
        assert!(err.contains(OTHER_FILE_REFUSAL), "{}", err);
        assert!(err.contains(&"agent rewrites the depends line of \"Third?\" to name terms and questions of this file".to_string()), "{}", err);
        assert!(
            err.contains("Some earlier heading?") || err.contains(&decided_id),
            "{}",
            err
        );
        assert_eq!(
            err.matches("depends on ").count(),
            1,
            "one finding for the entry: {}",
            err
        );

        let check = stdout(&adr(&dir, &["check"], &[], ""));
        assert!(
            check.contains(OTHER_FILE_REFUSAL),
            "check must report the same finding: {}",
            check
        );

        let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
        assert!(
            !ledger
                .lines()
                .any(|l| l.contains("\tapproved\t") && l.contains("example.md")),
            "no approved row on refusal: {}",
            ledger
        );
    }
}

// adr: 9afe335c-2a7a-4625-b115-f74d3f091cac
//
// ask refuses the same entry in a plan the owner approved and the agent then
// edited; the entry is named.
#[test]
fn ask_refuses_an_entry_naming_a_current_decision_of_another_file() {
    let dir = init_repo();
    setup(&dir);
    let decided_id = earlier_decision(&dir);
    let record = write_record(
        &dir,
        "adrdom/decisions/example.md",
        &one_question("Third?", "decision"),
    );
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    let approved = std::fs::read_to_string(&record).unwrap();

    for entry in [decided_id.clone(), "Some earlier heading?".to_string()] {
        let line = approved
            .lines()
            .find(|l| l.starts_with("depends: "))
            .unwrap()
            .to_string();
        std::fs::write(
            &record,
            approved.replacen(&line, &format!("depends: {}", entry), 1),
        )
        .unwrap();
        let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
        assert!(
            !out.status.success(),
            "ask should have refused {}: {}",
            entry,
            stdout(&out)
        );
        let err = stderr(&out);
        assert!(err.contains(OTHER_FILE_REFUSAL), "{}", err);
    }
}

// adr: 9afe335c-2a7a-4625-b115-f74d3f091cac
//
// A closed question keeps the record of what it stood on: an answered
// question whose depends line holds the identifier of a decision of another
// file is no finding of check, and when that decision is superseded the
// question falls with it.
#[test]
fn check_does_not_report_an_answered_question_standing_on_a_decision_of_another_file() {
    let dir = init_repo();
    setup(&dir);
    let decided_id = earlier_decision(&dir);
    let x_id = "55555555-5555-5555-5555-555555555555";
    approve_via_ledger_fixture(&dir, x_id, &decided_id);
    let check = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(!check.contains(OTHER_FILE_REFUSAL), "{}", check);
    assert!(!check.contains("depends on"), "{}", check);

    let show = stdout(&adr(
        &dir,
        &["show", "--dependents", "adrdom/decisions/earlier.md"],
        &[],
        "",
    ));
    assert!(
        show.contains("Does X stand on the earlier one?"),
        "{}",
        show
    );
}

fn approve_via_ledger_fixture(dir: &std::path::Path, x_id: &str, decided_id: &str) {
    append_ledger_row_at(
        dir,
        "asked",
        x_id,
        "h2",
        "Does X stand on the earlier one?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(dir, "answered", x_id, "h2", "A", "2024-01-01T00:00:01Z");
    let x = format!(
        "# Topic X\n\n### Does X stand on the earlier one?\ndepends: {}\n\nid: {}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        decided_id, x_id
    );
    write_record(dir, "adrdom/decisions/topic-x.md", &x);
}

// adr: 9afe335c-2a7a-4625-b115-f74d3f091cac
//
// A draft in another file whose depends entry is the heading text of a
// decision is not listed as standing on it by `show --dependents`.
#[test]
fn show_dependents_does_not_list_a_draft_naming_a_decision_of_another_file_by_heading() {
    let dir = init_repo();
    setup(&dir);
    earlier_decision(&dir);
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        &one_question("Third?", "Some earlier heading?"),
    );
    let shown = stdout(&adr(&dir, &["show", "--all", "--dependents"], &[], ""));
    assert!(shown.contains("Some earlier heading?"), "{}", shown);
    assert!(!shown.contains("dependents: Third?"), "{}", shown);
    assert!(
        !shown.contains("Third? (adrdom/decisions/example.md)"),
        "{}",
        shown
    );
}

// adr: 010901e0-c02d-49eb-98bc-d584756d7f51
//
// A cap of two: a depends line naming two questions passes, three are
// refused at approve with the question named; terms do not count.
#[test]
fn approve_refuses_a_depends_line_naming_three_questions_and_counts_no_term() {
    let dir = init_repo();
    setup(&dir);
    let q = |h: &str, d: &str| {
        format!(
            "### {}\ndepends: {}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n",
            h, d
        )
    };
    let roots = format!(
        "{}{}{}",
        q("Root a?", "decision"),
        q("Root b?", "decision"),
        q("Root c?", "decision")
    );

    let two = format!(
        "# Example\n\n{}{}",
        roots,
        q("Over two?", "Root a?=A, Root b?=A, decision")
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &two);
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "two questions and a term pass: {}{}",
        stdout(&out),
        stderr(&out)
    );

    let three = format!(
        "# Example\n\n{}{}",
        roots,
        q("Over three?", "Root a?=A, Root b?=A, Root c?=A, decision")
    );
    let record = write_record(&dir, "adrdom/decisions/three.md", &three);
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "three questions are refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    let refusal = "\"Over three?\" depends on 3 questions, the most is 2: \"Root a?\", \"Root b?\", \"Root c?\"; agent rewrites the depends line of \"Over three?\"";
    assert!(err.contains(refusal), "{}", err);
    assert!(!err.contains("Over two?"), "{}", err);

    let check = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(
        check.contains(refusal),
        "check reports the same finding: {}",
        check
    );
}

// adr: 010901e0-c02d-49eb-98bc-d584756d7f51
//
// ask refuses a plan whose depends line was edited over the cap after approval.
#[test]
fn ask_refuses_a_depends_line_naming_three_questions() {
    let dir = init_repo();
    setup(&dir);
    let q = |h: &str, d: &str| {
        format!(
            "### {}\ndepends: {}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n",
            h, d
        )
    };
    let body = format!(
        "# Example\n\n{}{}{}{}",
        q("Root a?", "decision"),
        q("Root b?", "decision"),
        q("Root c?", "decision"),
        q("Over?", "Root a?=A, Root b?=A")
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &body);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    let approved = std::fs::read_to_string(&record).unwrap();
    let line = approved
        .lines()
        .find(|l| l.starts_with("depends: ") && l.contains(","))
        .unwrap()
        .to_string();
    let edited = approved.replacen(&line, &format!("{}, Root c?=A", line), 1);
    std::fs::write(&record, edited).unwrap();
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stderr(&out).contains("\"Over?\" depends on 3 questions, the most is 2"),
        "{}",
        stderr(&out)
    );
}

// --- term entry resolution ------------------------------------------------

// adr: 49a1c3a8-07ea-448a-8926-2362c0617758
#[test]
fn approve_refuses_a_term_entry_that_resolves_to_nothing() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n### 1.1 Q?\ndepends: nosuchterm\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Q?\" depends on \"nosuchterm\" which is no term; agent writes the term in adrdom/terms.md"),
        "{}",
        err
    );
}

const TERMS_ONE_RETIRED: &str = "# Terms\n\n\
## gadget\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A fixture term for tests.\n\
aliases: \n\
retired: yes\n";

// adr: 49a1c3a8-07ea-448a-8926-2362c0617758
#[test]
fn approve_refuses_a_term_entry_that_resolves_to_a_retired_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/terms.md", TERMS_ONE_RETIRED);
    approve_terms_via_ledger(&dir, TERMS_ONE_RETIRED);

    let content = "# Example\n\n### 1.1 Q?\ndepends: gadget\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Q?\" names the retired term \"gadget\"; agent rewrites the entry"),
        "{}",
        err
    );
}

// --- decision entry resolution --------------------------------------------

// adr: 49a1c3a8-07ea-448a-8926-2362c0617758
#[test]
fn approve_refuses_a_decision_entry_that_is_not_current() {
    let dir = init_repo();
    setup(&dir);
    let uuid = "22222222-2222-2222-2222-222222222222";
    let content = format!(
        "# Example\n\n### 1.1 Q?\ndepends: {}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        uuid
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains(&format!(
            "\"1.1 Q?\" depends on {} which names no term and no question of this file; agent rewrites the entry to name a term or a question of this file",
            uuid
        )),
        "{}",
        err
    );
}

// --- heading dep: dangling and cycle --------------------------------------

// adr: 49a1c3a8-07ea-448a-8926-2362c0617758
#[test]
fn approve_refuses_a_dangling_heading_dependency() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n### 1.1 Q?\ndepends: Missing?\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Q?\" depends on \"Missing?\" which is no heading in this file; agent rewrites the entry"),
        "{}",
        err
    );
}

//
// "approve stops a cycle": a tier can only be derived when the depends
// graph has no cycle, so approve refuses outright rather than print a
// tier order it cannot compute.
// adr: f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2
#[test]
fn approve_refuses_a_cycle_between_two_headings() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n### A?\ndepends: B?\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n### B?\ndepends: A?\n\nProblem: p2 of the decision.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("depends lines lead back to their own question"),
        "{}",
        err
    );
    assert!(err.contains("\"A?\""), "{}", err);
    assert!(err.contains("\"B?\""), "{}", err);
    assert!(
        err.contains("agent rewrites the depends lines of these questions"),
        "{}",
        err
    );
}

#[test]
fn approve_refuses_a_heading_that_depends_on_itself() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n### A?\ndepends: A?\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("\"A?\" depends on itself; agent removes the entry"),
        "{}",
        err
    );
}

// --- cap (8.8 A) -----------------------------------------------------------

// adr: 0e38f001-0226-4927-9a78-2e9fc1ec18d4
#[test]
fn approve_refuses_five_root_questions_in_one_tier() {
    let dir = init_repo();
    setup(&dir);
    let mut content = String::from("# Example\n\n");
    for n in 1..=5 {
        content.push_str(&root_question(n));
    }
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("5 questions in one tier, the most is 4"),
        "{}",
        err
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger
            .lines()
            .any(|l| l.contains("\tapproved\t") || l.contains("\tcontext\t")),
        "no row should have been written on refusal: {}",
        ledger
    );
}

#[test]
fn approve_accepts_two_new_headings_when_two_in_the_tier_are_dead() {
    let dir = init_repo();
    setup(&dir);
    let term_id = "99999999-9999-9999-9999-999999999999";
    let old_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let old_b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    let old_c = "cccccccc-cccc-cccc-cccc-cccccccccccc";
    let old_d = "dddddddd-dddd-dddd-dddd-dddddddddddd";

    // Tier 1 starts with four questions, already approved and frozen:
    // "Dead A" refused, "Dead B" superseded, "Live C"/"Live D" still asked.
    // The frozen ones carry their depends entry already resolved to the
    // term's id, as a real prior approve would have left it. Two
    // brand new skeleton headings are added to the same tier.
    let dead_a = format!(
        "### 1.1 Dead A?\ndepends: {term}\n\nid: {a}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** refused\n",
        term = term_id,
        a = old_a
    );
    let dead_b = format!(
        "### 1.2 Dead B?\ndepends: {term}\n\nid: {b}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        term = term_id,
        b = old_b
    );
    let content = format!(
        "# Example\n\n{dead_a}\n{dead_b}\n\
### 1.3 Live C?\ndepends: decision\n\nid: {c}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.4 Live D?\ndepends: decision\n\nid: {d}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.5 New E?\ndepends: decision\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.6 New F?\ndepends: decision\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        dead_a = dead_a,
        dead_b = dead_b,
        c = old_c,
        d = old_d
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    // The refused and answered rows carry the sections' real hashes, as
    // `answer` writes them.
    append_ledger_row(
        &dir,
        "refused",
        old_a,
        &mirror_section_hash(&dead_a),
        "1.1 Dead A?",
    );
    append_ledger_row(&dir, "answered", old_b, &mirror_section_hash(&dead_b), "A");
    append_ledger_row(&dir, "superseded", old_b, "-", "some-other-id");

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve should accept a tier with only 4 live questions: {}",
        stderr(&out)
    );
    // Live count only (8.8 A fix): the two dead questions (refused A,
    // superseded B) do not count toward the tier, so no cap refusal.
    assert!(!stderr(&out).contains("holds"), "{}", stderr(&out));
}

// adr: 0e38f001-0226-4927-9a78-2e9fc1ec18d4
#[test]
fn approve_accepts_four_root_questions_in_one_tier() {
    let dir = init_repo();
    setup(&dir);
    let mut content = String::from("# Example\n\n");
    for n in 1..=4 {
        content.push_str(&root_question(n));
    }
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    assert_eq!(tiers_of(&record), vec![1, 1, 1, 1]);
}

// --- the tiers the tool derives -----------------------------------------------

//
// "the tool derives the tiers from the depends lines and nothing in the
// topic file states them; a question standing on no other question in its
// file is in the first tier, any other one tier above the highest question
// it stands on". Neither heading below carries a written tier of any kind
// -- only a `depends:` line -- yet the tool derives "A?" in tier 1 (it
// stands on nothing in this file, only the `decision` term) and "B?" in
// tier 2 (one above "A?", the only thing it stands on).
// adr: f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2
#[test]
fn the_tool_derives_tiers_across_a_chain() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n\
        ### A?\ndepends: decision\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
        ### B?\ndepends: A?=A\n\nProblem: p2 of the decision.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
        ### C?\ndepends: A?=A, B?=A\n\nProblem: p3 of the decision.\n\nStandard: s3.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    // "C?" stands on "A?" (tier 1) and "B?" (tier 2): one above the highest.
    assert_eq!(tiers_of(&record), vec![1, 2, 3]);
}

// The tier the tool derives for each question of the topic file at `path`,
// in file order.
fn tiers_of(path: &std::path::Path) -> Vec<usize> {
    let rf = adr::record::load(path).unwrap();
    (0..rf.sections.len())
        .map(|i| rf.tier_of(i).unwrap())
        .collect()
}

// --- refused id withdrawn when absent from every file (8.2 A) -------------

#[test]
fn approve_withdraws_a_refused_id_missing_from_every_file() {
    let dir = init_repo();
    setup(&dir);
    let fake_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    append_ledger_row(&dir, "refused", fake_id, "-", "Some vanished question?");

    let content = "# Example\n\n### 1.1 Only question?\ndepends: decision\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    assert!(
        stdout(&out)
            .lines()
            .any(|l| l == "withdrawn: \"Some vanished question?\""),
        "{}",
        stdout(&out)
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger
            .lines()
            .any(|l| l.contains("\twithdrawn\t") && l.contains(fake_id)),
        "ledger missing withdrawn row for refused id: {}",
        ledger
    );
}

// --- the question hash covers the answer set ----------------------------------

#[test]
fn approve_question_hash_moves_when_only_an_answer_set_changes() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n\
        ### 1.1 A?\ndepends: decision\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
        ### 1.2 B?\ndepends: 1.1 A?=A\n\nProblem: p2 of the decision.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out1 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out1.status.success(), "approve failed: {}", stderr(&out1));
    assert!(
        stdout(&out1)
            .lines()
            .any(|l| l == "approved: plan of adrdom/decisions/example.md"),
        "{}",
        stdout(&out1)
    );

    let ledger1 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let approved_count1 = ledger1
        .lines()
        .filter(|l| l.contains("\tapproved\t"))
        .count();

    // Only the answer set changes (A -> B); the depends target is the same.
    // Edited on the file as `approve` actually left it (with 1.2's depends
    // line already rewritten to 1.1's id) -- not the original text-form
    // content, which would make `approve` mint fresh ids all over again and
    // move the plan hash for an unrelated reason (an id is part of
    // the plan).
    let approved_content = std::fs::read_to_string(&record).unwrap();
    assert!(
        approved_content.contains("=A\n"),
        "expected the depends line to still carry the =A suffix after id rewrite: {}",
        approved_content
    );
    let changed = approved_content.replacen("=A\n", "=B\n", 1);
    std::fs::write(&record, &changed).unwrap();

    // The answer set is part of the depends line, which the question hash
    // covers: the file awaits approval again, and approve writes a new file
    // row and a new row for 1.2 only.
    let asked = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        stderr(&asked).contains("plan is changed"),
        "{}",
        stderr(&asked)
    );
    let out2 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out2.status.success(), "approve failed: {}", stderr(&out2));
    assert!(
        stdout(&out2)
            .lines()
            .any(|l| l == "approved: plan of adrdom/decisions/example.md"),
        "{}",
        stdout(&out2)
    );

    let ledger2 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let approved_count2 = ledger2
        .lines()
        .filter(|l| l.contains("\tapproved\t"))
        .count();
    assert_eq!(
        approved_count1 + 2,
        approved_count2,
        "changing only the answer set must move the hash of that question and the file"
    );
}

// --- a reference carries an identifier, not the words it was written with -

// adr: 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40
//
// "every reference carries the identifier of what it names; the agent
// writes the name, approve turns it into the identifier, and show turns it
// back into the name". B's `depends:` line is written by hand as the
// heading text "1.1 Is the widget red?"; after `approve` that line holds
// A's freshly minted uuid, not the words -- and `show`, which only ever
// reads the file back, prints the words again. Renaming A's heading text
// afterward (its `id:` line, and B's depends line, both untouched) still
// approves clean and `show` now prints B as standing on the *new* wording,
// proving the depends line was never the text at all.
#[test]
fn approve_turns_a_depends_name_into_an_id_and_show_turns_it_back_into_words() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n\
        ### 1.1 Is the widget red?\ndepends: decision\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
        ### 1.2 Does the paint match?\ndepends: 1.1 Is the widget red?=A\n\nProblem: p2 of the decision.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    let approved = std::fs::read_to_string(&record).unwrap();
    let a_id = approved
        .lines()
        .skip_while(|l| !l.starts_with("### 1.1 Is the widget red?"))
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();
    // B's depends line now holds A's uuid, not the words it was written
    // with.
    assert!(
        approved.contains(&format!(
            "### 1.2 Does the paint match?\ndepends: {}=A\n",
            a_id
        )),
        "expected B's depends line rewritten to A's id: {}",
        approved
    );
    assert!(
        !approved.contains("depends: 1.1 Is the widget red?"),
        "the written words must not survive approve: {}",
        approved
    );

    // `show` reads that same id back and prints the words, unprompted.
    let show1 = adr(&dir, &["show", "--all"], &[], "");
    assert!(show1.status.success(), "{}", stderr(&show1));
    assert!(
        stdout(&show1).contains("depends: 1.1 Is the widget red?"),
        "{}",
        stdout(&show1)
    );

    // Rename A's heading text only -- its `id:` line and B's depends line
    // (already an id) are untouched.
    let renamed = approved.replace("### 1.1 Is the widget red?", "### 1.1 Is the gadget red?");
    assert_ne!(
        renamed, approved,
        "the fixture should still have that heading"
    );
    std::fs::write(&record, &renamed).unwrap();

    let out2 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        out2.status.success(),
        "a heading rename must break no reference: {}",
        stderr(&out2)
    );
    assert!(
        stdout(&out2)
            .lines()
            .any(|l| l == "approved: plan of adrdom/decisions/example.md"),
        "{}",
        stdout(&out2)
    );

    // B's dependency still resolves -- and now prints the *new* wording,
    // proving the depends line was never the text.
    let show2 = adr(&dir, &["show", "--all"], &[], "");
    assert!(show2.status.success(), "{}", stderr(&show2));
    let shown2 = stdout(&show2);
    assert!(
        shown2.contains("[not asked] 1.1 Is the gadget red?"),
        "{}",
        shown2
    );
    assert!(
        shown2.contains("depends: 1.1 Is the gadget red?=A"),
        "{}",
        shown2
    );
    assert!(!shown2.contains("Is the widget red?"), "{}", shown2);
}

// adr: da59b448-1afb-4469-b40e-69eda8f253c2
//
// "ask stops any question whose heading is not in the plan the owner
// approved": plan approval is a hash over the file exactly as the owner
// approved it. B's `depends:` line, hand-reverted back to the heading
// words `approve` rewrote to A's id, still resolves to that same question
// -- but it is no longer the bytes the owner approved, so `ask` refuses and
// `check` takes the file as awaiting approval.
#[test]
fn ask_and_check_refuse_a_depends_line_reverted_by_hand_after_approve() {
    let dir = init_repo();
    setup(&dir);

    let content = "# Topic\n\n### 1.1 Is A do a thing?\ndepends: decision\n\nProblem: a decision.\n\nStandard: a decision.\n\n- A) x, a decision.\n  Why: w.\n  Costs: c.\n- B) y, a decision.\n  Why: w.\n  Costs: c.\n\n### 1.1 Is B stand on A?\ndepends: 1.1 Is A do a thing?=A\n\nProblem: p2 of the decision.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/topic.md", content);
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approving failed: {}", stderr(&out));

    let approved = std::fs::read_to_string(&record).unwrap();
    let a_id = approved
        .lines()
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();
    assert!(
        approved.contains(&format!("depends: {}=A\n", a_id)),
        "{}",
        approved
    );

    let reverted = approved.replacen(
        &format!("depends: {}=A\n", a_id),
        "depends: 1.1 Is A do a thing?=A\n",
        1,
    );
    assert_ne!(
        reverted, approved,
        "the fixture should still hold the rewritten line"
    );
    std::fs::write(&record, &reverted).unwrap();

    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        !ask_out.status.success(),
        "ask must refuse a plan reverted back to its pre-approval wording: {}",
        stdout(&ask_out)
    );
    assert!(
        stderr(&ask_out).contains("adrdom/decisions/topic.md: plan is changed"),
        "{}",
        stderr(&ask_out)
    );

    let check_out = adr(&dir, &["check"], &[], "");
    // An unapproved file is never a failure of its own, and its status is no
    // line of check's (bd83aa22).
    let checked = stdout(&check_out);
    assert!(check_out.status.success(), "{}", checked);
    assert!(!checked.contains("awaiting approval"), "{}", checked);
    assert!(
        !checked.contains("body under unapproved heading"),
        "{}",
        checked
    );
}

//
// A depends entry naming an answered question of its file by id must not
// carry an answer set: `approve` refuses it.
#[test]
fn approve_refuses_a_depends_set_on_an_answered_question_of_the_file() {
    let dir = init_repo();
    setup(&dir);

    let a_id = "11111111-2222-3333-4444-555555555555";
    append_ledger_row_at(
        &dir,
        "asked",
        a_id,
        "h1",
        "1.1 Is A do a thing?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(&dir, "answered", a_id, "h1", "A", "2024-01-01T00:00:01Z");
    let content = format!(
        "# Topic A\n\n### 1.1 Is A do a thing?\ndepends: decision\n\nid: {}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n\n### 1.1 Is B stand on A?\ndepends: {}=A\n\nProblem: p2 of the decision.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        a_id, a_id
    );
    let record = write_record(&dir, "adrdom/decisions/topic-a.md", &content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains(
            "\"1.1 Is B stand on A?\" depends on \"1.1 Is A do a thing?\", which is answered, under an answer set"
        ),
        "{}",
        err
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger
            .lines()
            .any(|l| l.contains("\tapproved\t") && l.contains("topic-a.md")),
        "no approved row should have been written on refusal: {}",
        ledger
    );
}

// adr: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d
//
// A frozen (answered) section's `depends:` line, hand-edited to a
// heading text that resolves to exactly one current decision in another
// topic file: the frozen check must still catch it. Before this fix,
// `normalize_cross_file_deps` ran its substitution pass before
// `record_refusals`'s frozen check ever saw the entry, rewriting the
// `Text` entry to an `Id` one first -- the frozen check only recognizes a
// `Text` entry as an edit to refuse, so the rewritten `Id` slipped past it
// and `approve` wrote the file.
#[test]
fn approve_refuses_a_frozen_sections_depends_line_hand_edited_to_a_cross_file_heading() {
    let dir = init_repo();
    setup(&dir);

    // Y: an already-answered, current decision living in a topic file of
    // its own, for X to (wrongly) start naming after X is already answered.
    let y_id = "77777777-7777-7777-7777-777777777777";
    append_ledger_row_at(
        &dir,
        "asked",
        y_id,
        "h1",
        "1.1 Does Y do a thing?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(&dir, "answered", y_id, "h1", "A", "2024-01-01T00:00:01Z");
    let y_content = format!(
        "# Topic Y\n\n### 1.1 Does Y do a thing?\ndepends: decision\n\nid: {}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        y_id
    );
    write_record(&dir, "adrdom/decisions/topic-y.md", &y_content);

    // X: an already-answered section whose depends line is hand-edited,
    // after the fact, to a cross-file heading reference -- exactly the
    // edit `record_refusals`'s frozen check exists to catch.
    let x_id = "66666666-6666-6666-6666-666666666666";
    append_ledger_row_at(
        &dir,
        "asked",
        x_id,
        "h1",
        "1.1 Does X stand on something?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(&dir, "answered", x_id, "h1", "A", "2024-01-01T00:00:01Z");
    let x_content = format!(
        "# Topic X\n\n### 1.1 Does X stand on something?\ndepends: 1.1 Does Y do a thing?\n\nid: {}\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        x_id
    );
    let record_x = write_record(&dir, "adrdom/decisions/topic-x.md", &x_content);

    let out = adr_owner(&dir, &["approve", record_x.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("\"1.1 Does X stand on something?\" is [answered] and its depends line was edited; agent restores the line from git"),
        "{}",
        stderr(&out)
    );

    let content_after = std::fs::read_to_string(&record_x).unwrap();
    assert!(
        content_after.contains("depends: 1.1 Does Y do a thing?\n"),
        "a refused approve must leave the hand-edited line exactly as written, never rewritten to Y's id: {}",
        content_after
    );

    let check = adr(&dir, &["check"], &[], "");
    assert!(
        stdout(&check).contains("\"1.1 Does X stand on something?\" is [answered] and its depends line was edited; agent restores the line from git"),
        "check must report the same frozen-edit refusal: {}",
        stdout(&check)
    );
}
