mod common;

use common::*;

/// One root-tier question block, self-contained: heading, `depends:
/// decision` (a term every test in this file approves via
/// `approve_minimal_terms(&dir, "decision")`), Problem/Standard, two
/// options.
fn root_question(n: usize) -> String {
    format!(
        "### {n}.1 Root {n}?\ndepends: decision\n\nProblem: p{n}.\n\nStandard: s{n}.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n",
        n = n
    )
}

fn setup(dir: &std::path::Path) {
    assert!(adr(dir, &["install"], &[], "").status.success());
    approve_minimal_terms(dir, "decision");
}

// --- no depends line ----------------------------------------------------

#[test]
fn approve_refuses_a_heading_with_no_depends_line() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n### 1.1 No depends?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 No depends?\" has no depends line; agent writes what the question stands on"),
        "{}",
        err
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tapproved\t") || l.contains("\tcontext\t")),
        "no row should have been written on refusal: {}",
        ledger
    );
}

// --- term entry resolution ------------------------------------------------

#[test]
fn approve_refuses_a_term_entry_that_resolves_to_nothing() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n### 1.1 Q?\ndepends: nosuchterm\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Q?\" depends on \"nosuchterm\" which is no term; agent adds the term or names another"),
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

#[test]
fn approve_refuses_a_term_entry_that_resolves_to_a_retired_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/terms.md", TERMS_ONE_RETIRED);
    approve_terms_via_ledger(&dir, TERMS_ONE_RETIRED);

    let content = "# Example\n\n### 1.1 Q?\ndepends: gadget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Q?\" depends on \"gadget\" which is no term; agent adds the term or names another"),
        "{}",
        err
    );
}

// --- decision entry resolution --------------------------------------------

#[test]
fn approve_refuses_a_decision_entry_that_is_not_current() {
    let dir = init_repo();
    setup(&dir);
    let uuid = "22222222-2222-2222-2222-222222222222";
    let content = format!(
        "# Example\n\n### 1.1 Q?\ndepends: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        uuid
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(&format!(
            "\"1.1 Q?\" depends on {} which is not a current decision; agent names a current decision or removes the line",
            uuid
        )),
        "{}",
        err
    );
}

// --- heading dep: dangling and cycle --------------------------------------

#[test]
fn approve_refuses_a_dangling_heading_dependency() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n### 1.1 Q?\ndepends: Missing?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Q?\" depends on \"Missing?\" which is no heading in this file; agent adds the heading or fixes the text"),
        "{}",
        err
    );
}

#[test]
fn approve_refuses_a_cycle_between_two_headings() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n### A?\ndepends: B?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n### B?\ndepends: A?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("cycle:"), "{}", err);
    assert!(err.contains("\"A?\""), "{}", err);
    assert!(err.contains("\"B?\""), "{}", err);
    assert!(err.contains("agent breaks it"), "{}", err);
}

#[test]
fn approve_refuses_a_heading_that_depends_on_itself() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n### A?\ndepends: A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("\"A?\" depends on itself; agent removes the line or names something else"),
        "{}",
        err
    );
}

// --- cap (8.8 A) -----------------------------------------------------------

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
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("tier 1 holds 5 questions, the cap is 4; agent moves questions under a higher heading or splits the topic"),
        "{}",
        err
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tapproved\t") || l.contains("\tcontext\t")),
        "no row should have been written on refusal: {}",
        ledger
    );
}

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
    let text = stdout(&out);
    assert!(text.contains("approve: tiers: 1 (4)"), "{}", text);
}

// --- the tiers print --------------------------------------------------------

#[test]
fn approve_prints_tiers_across_a_chain() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n\
        ### A?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
        ### B?\ndepends: A?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("approve: tiers: 1 (1) 2 (1)"), "{}", text);
}

// --- refused id withdrawn when absent from every file (8.2 A) -------------

#[test]
fn approve_withdraws_a_refused_id_missing_from_every_file() {
    let dir = init_repo();
    setup(&dir);
    let fake_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    append_ledger_row(&dir, "refused", fake_id, "-", "Some vanished question?");

    let content = "# Example\n\n### 1.1 Only question?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    assert!(stdout(&out).contains(&format!("approve: withdrawn: Some vanished question?")), "{}", stdout(&out));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger.lines().any(|l| l.contains("\twithdrawn\t") && l.contains(fake_id)),
        "ledger missing withdrawn row for refused id: {}",
        ledger
    );
}

// --- plan hash strips the answer set (8.7 A) --------------------------------

#[test]
fn approve_plan_hash_is_unaffected_by_changing_only_an_answer_set() {
    let dir = init_repo();
    setup(&dir);
    let content = "# Example\n\n\
        ### 1.1 A?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
        ### 1.2 B?\ndepends: 1.1 A?=A\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out1 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out1.status.success(), "approve failed: {}", stderr(&out1));
    assert!(stdout(&out1).contains("plan approved"), "{}", stdout(&out1));

    let ledger1 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let approved_count1 = ledger1.lines().filter(|l| l.contains("\tapproved\t")).count();

    // Only the answer set changes (A -> B); the depends target is the same.
    let changed = content.replace("depends: 1.1 A?=A", "depends: 1.1 A?=B");
    std::fs::write(&record, &changed).unwrap();

    let out2 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out2.status.success(), "approve failed: {}", stderr(&out2));
    assert!(stdout(&out2).contains("plan unchanged"), "{}", stdout(&out2));

    let ledger2 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let approved_count2 = ledger2.lines().filter(|l| l.contains("\tapproved\t")).count();
    assert_eq!(
        approved_count1, approved_count2,
        "changing only the answer set must not move the plan hash"
    );
}
