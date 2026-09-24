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

// adr: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf
//
// "every question carries a depends line naming a term, a question in the
// same topic file, or a current decision; approve and ask stop a line that
// is missing or names anything that does not resolve". A heading with no
// depends line at all refuses (the "missing" half, also covered on its own
// by `approve_refuses_a_heading_with_no_depends_line` below); a heading
// whose depends line names a real term, a heading elsewhere in the same
// file, and a current decision (an id answered before this file even
// existed) all resolve, and approve accepts every one of the three kinds.
#[test]
fn approve_accepts_all_three_depends_target_kinds_and_refuses_a_missing_line() {
    let dir = init_repo();
    setup(&dir);

    let missing = "# Missing\n\n### 1.1 No depends?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let missing_record = write_record(&dir, "adrdom/decisions/missing.md", missing);
    let out = adr_owner(&dir, &["approve", missing_record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    assert!(
        stderr(&out).contains("\"1.1 No depends?\" has no depends line"),
        "{}",
        stderr(&out)
    );

    // A current decision: an id answered before this topic file existed,
    // living in a topic file of its own (approve refuses an answered id
    // that appears in no topic file at all -- a different rule from the
    // one this test is about -- so it needs a home).
    let decided_id = "66666666-6666-6666-6666-666666666666";
    append_ledger_row(&dir, "asked", decided_id, "h1", "Some earlier heading?");
    append_ledger_row(&dir, "answered", decided_id, "h1", "A");
    let earlier = format!(
        "# Earlier\n\n### 1.1 Some earlier heading?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        decided_id
    );
    write_record(&dir, "adrdom/decisions/earlier.md", &earlier);

    // One file naming all three kinds: a term ("decision"), a heading in
    // the same file ("First?"), and the current decision above.
    let content = format!(
        "# Example\n\n\
        ### First?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
        ### Second?\ndepends: First?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
        ### Third?\ndepends: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        decided_id
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    let out2 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        out2.status.success(),
        "approve should accept a term, an in-file heading, and a current decision: {}",
        stderr(&out2)
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

// adr: aec860fd-22ad-44a7-99f7-222ea19537e7
//
// "approve stops a cycle": a tier can only be derived when the depends
// graph has no cycle, so approve refuses outright rather than print a
// tier order it cannot compute.
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
    // term's id (9.1 A), as a real prior approve would have left it. Two
    // brand new skeleton headings are added to the same tier.
    let content = format!(
        "# Example\n\n\
### 1.1 Dead A?\ndepends: {term}\n\nid: {a}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** refused\n\n\
### 1.2 Dead B?\ndepends: {term}\n\nid: {b}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n\n\
### 1.3 Live C?\ndepends: decision\n\nid: {c}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.4 Live D?\ndepends: decision\n\nid: {d}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.5 New E?\ndepends: decision\n\n\
### 1.6 New F?\ndepends: decision\n",
        term = term_id,
        a = old_a,
        b = old_b,
        c = old_c,
        d = old_d
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    append_ledger_row(&dir, "refused", old_a, "-", "1.1 Dead A?");
    append_ledger_row(&dir, "superseded", old_b, "-", "some-other-id");

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve should accept a tier with only 4 live questions: {}",
        stderr(&out)
    );
    let text = stdout(&out);
    assert!(!text.contains("holds"), "{}", text);
    // Live count only (8.8 A fix): the two dead questions (refused A,
    // superseded B) no longer count toward the tier, matching `check`'s
    // own "tiers: ..." note.
    assert!(text.contains("approve: tiers: 1 (4)"), "{}", text);
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

// adr: aec860fd-22ad-44a7-99f7-222ea19537e7
//
// "the tool derives the tiers from the depends lines and nothing in the
// topic file states them; a question standing on no other question in its
// file is in the first tier, any other one tier above the highest question
// it stands on". Neither heading below carries a written tier of any kind
// -- only a `depends:` line -- yet `approve` reports "A?" in tier 1 (it
// stands on nothing in this file, only the `decision` term) and "B?" in
// tier 2 (one above "A?", the only thing it stands on).
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
    // Edited on the file as `approve` actually left it (with 1.2's depends
    // line already rewritten to 1.1's id) -- not the original text-form
    // content, which would make `approve` mint fresh ids all over again and
    // move the plan hash for an unrelated reason (9.1 A: an id is part of
    // the plan).
    let approved_content = std::fs::read_to_string(&record).unwrap();
    assert!(
        approved_content.contains("=A\n"),
        "expected the depends line to still carry the =A suffix after id rewrite: {}",
        approved_content
    );
    let changed = approved_content.replacen("=A\n", "=B\n", 1);
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

// --- a reference carries an identifier, not the words it was written with -

// adr: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
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
        ### 1.1 Is the widget red?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
        ### 1.2 Does the paint match?\ndepends: 1.1 Is the widget red?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
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
        approved.contains(&format!("### 1.2 Does the paint match?\ndepends: {}\n", a_id)),
        "expected B's depends line rewritten to A's id: {}",
        approved
    );
    assert!(
        !approved.contains("depends: 1.1 Is the widget red?"),
        "the written words must not survive approve: {}",
        approved
    );

    // `show` reads that same id back and prints the words, unprompted.
    let show1 = adr(&dir, &["show"], &[], "");
    assert!(show1.status.success(), "{}", stderr(&show1));
    assert!(
        stdout(&show1).contains("stands on: 1.1 Is the widget red?"),
        "{}",
        stdout(&show1)
    );

    // Rename A's heading text only -- its `id:` line and B's depends line
    // (already an id) are untouched.
    let renamed = approved.replace(
        "### 1.1 Is the widget red?",
        "### 1.1 Is the gadget red?",
    );
    assert_ne!(renamed, approved, "the fixture should still have that heading");
    std::fs::write(&record, &renamed).unwrap();

    let out2 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        out2.status.success(),
        "a heading rename must break no reference: {}",
        stderr(&out2)
    );
    assert!(stdout(&out2).contains("plan approved"), "{}", stdout(&out2));

    // B's dependency still resolves -- and now prints the *new* wording,
    // proving the depends line was never the text.
    let show2 = adr(&dir, &["show"], &[], "");
    assert!(show2.status.success(), "{}", stderr(&show2));
    let shown2 = stdout(&show2);
    assert!(shown2.contains("[unasked] 1.1 Is the gadget red?"), "{}", shown2);
    assert!(shown2.contains("stands on: 1.1 Is the gadget red?"), "{}", shown2);
    assert!(!shown2.contains("Is the widget red?"), "{}", shown2);
}
