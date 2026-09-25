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

// adr: da59b448-1afb-4469-b40e-69eda8f253c2
//
// "ask stops any question whose heading is not in the plan the owner
// approved": plan approval is a hash over the file exactly as the owner
// approved it. B's `depends:` line, hand-reverted back to the heading
// words `approve` rewrote to A's id, still resolves to that same id in
// memory (the same cross-file pre-pass `ask`/`check` run before reading
// approval state) -- but it is no longer the bytes the owner approved.
// Before this fix, `ask`/`check` computed the plan hash off that
// normalized in-memory copy instead of the file exactly as it sits on
// disk, so the revert went unnoticed and both treated B as still
// approved; this proves they now catch it.
#[test]
fn ask_and_check_refuse_a_cross_file_depends_line_reverted_by_hand_after_approve() {
    let dir = init_repo();
    setup(&dir);

    // A: a real, fully approved current decision of its own. "decision"
    // (the fixture term `setup` approves) is repeated in the body so `ask`
    // pairs a term.
    let a_content = "# Topic A\n\n### 1.1 Does A do a thing?\ndepends: decision\n\nProblem: a decision.\n\nStandard: a decision.\n\n- A) x, a decision.\n  Why: w.\n  Costs: c.\n- B) y, a decision.\n  Why: w.\n  Costs: c.\n";
    let record_a = write_record(&dir, "adrdom/decisions/topic-a.md", a_content);
    let out_a = adr_owner(&dir, &["approve", record_a.to_str().unwrap()], "");
    assert!(out_a.status.success(), "approving A failed: {}", stderr(&out_a));
    let approved_a = std::fs::read_to_string(&record_a).unwrap();
    let a_id = approved_a
        .lines()
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();
    let ask_a = adr(&dir, &["ask", record_a.to_str().unwrap()], &[], "");
    assert!(ask_a.status.success(), "asking A failed: {}", stderr(&ask_a));
    assert!(
        adr_owner(&dir, &["answer", &a_id, "A"], "").status.success(),
        "answering A failed"
    );

    // B: its only question stands on A, named by A's heading text.
    let b_text = "1.1 Does A do a thing?";
    let b_content = format!(
        "# Topic B\n\n### 1.1 Does B stand on A?\ndepends: {}\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        b_text
    );
    let record_b = write_record(&dir, "adrdom/decisions/topic-b.md", &b_content);
    let out_b = adr_owner(&dir, &["approve", record_b.to_str().unwrap()], "");
    assert!(out_b.status.success(), "approving B failed: {}", stderr(&out_b));

    let approved_b = std::fs::read_to_string(&record_b).unwrap();
    assert!(
        approved_b.contains(&format!("depends: {}\n", a_id)),
        "expected B's depends line rewritten to A's id: {}",
        approved_b
    );

    // Hand-revert the rewritten line back to the heading words `approve`
    // replaced it with.
    let reverted = approved_b.replacen(
        &format!("depends: {}\n", a_id),
        &format!("depends: {}\n", b_text),
        1,
    );
    assert_ne!(reverted, approved_b, "the fixture should still hold the rewritten line");
    std::fs::write(&record_b, &reverted).unwrap();

    let ask_out = adr(&dir, &["ask", record_b.to_str().unwrap()], &[], "");
    assert!(
        !ask_out.status.success(),
        "ask must refuse a plan reverted back to its pre-approval wording: {}",
        stdout(&ask_out)
    );
    assert!(
        stderr(&ask_out).contains("adrdom/decisions/topic-b.md: plan not approved"),
        "{}",
        stderr(&ask_out)
    );

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(!check_out.status.success(), "{}", stdout(&check_out));
    let checked = stdout(&check_out);
    assert!(
        checked.contains("plan: 1 file(s) awaiting approval: adrdom/decisions/topic-b.md"),
        "{}",
        checked
    );
    assert!(
        checked.contains("body under unapproved heading \"1.1 Does B stand on A?\""),
        "{}",
        checked
    );
}

// adr: fa90692a-150e-4a89-91e2-1d575cb59d0f
//
// "Does a depends entry carry the answers it holds under?", the cross-file
// half: a depends entry naming a current decision elsewhere by id, with an
// answer set naming a letter that decision has no option for, is refused
// by `approve` too, not only reported by `check` -- before this fix,
// nothing checked a `DepTarget::Decision` target's letters at all, in any
// command.
#[test]
fn approve_refuses_a_depends_set_naming_an_option_a_cross_file_decision_does_not_have() {
    let dir = init_repo();
    setup(&dir);

    let a_id = "11111111-2222-3333-4444-555555555555";
    append_ledger_row(&dir, "asked", a_id, "h1", "1.1 Does A do a thing?");
    append_ledger_row(&dir, "answered", a_id, "h1", "A");
    let a_content = format!(
        "# Topic A\n\n### 1.1 Does A do a thing?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        a_id
    );
    write_record(&dir, "adrdom/decisions/topic-a.md", &a_content);

    let b_content = format!(
        "# Topic B\n\n### 1.1 Does B stand on A?\ndepends: {}=C\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        a_id
    );
    let record_b = write_record(&dir, "adrdom/decisions/topic-b.md", &b_content);

    let out = adr_owner(&dir, &["approve", record_b.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(
            "\"1.1 Does B stand on A?\" depends on \"1.1 Does A do a thing?\" names option C which \"1.1 Does A do a thing?\" does not have"
        ),
        "{}",
        err
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tapproved\t") && l.contains("topic-b.md")),
        "no approved row should have been written for B on refusal: {}",
        ledger
    );
}

// adr: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
//
// Same decision, the cross-file half: "a current decision" a depends line
// names by heading text is not only one living in the same topic file --
// it can be a current decision in a *different* topic file, named the same
// way, by its heading text alone (never its id, never written by hand).
// File A holds an already-answered, current decision of its own; file B's
// only question depends on it by A's heading text. `approve` on file B
// resolves that name to A's id and rewrites the depends line to it, the
// same as it would a same-file heading or a term; `show` then reads that
// id back off B and prints A's heading text again, unprompted.
#[test]
fn approve_resolves_a_cross_file_heading_dependency_and_show_turns_it_back_into_words() {
    let dir = init_repo();
    setup(&dir);

    // A: an already-answered, current decision living in a topic file of
    // its own -- the same shape
    // `approve_accepts_all_three_depends_target_kinds_and_refuses_a_missing_line`
    // uses for "a current decision", just with a real heading of its own
    // for B to name.
    let a_id = "77777777-7777-7777-7777-777777777777";
    append_ledger_row(&dir, "asked", a_id, "h1", "1.1 Does A do a thing?");
    append_ledger_row(&dir, "answered", a_id, "h1", "A");
    let a_content = format!(
        "# Topic A\n\n### 1.1 Does A do a thing?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        a_id
    );
    write_record(&dir, "adrdom/decisions/topic-a.md", &a_content);

    // B: a fresh topic file whose only question stands on A, named by A's
    // heading text.
    let b_content = "# Topic B\n\n### 1.1 Does B stand on A?\ndepends: 1.1 Does A do a thing?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record_b = write_record(&dir, "adrdom/decisions/topic-b.md", b_content);

    let out = adr_owner(&dir, &["approve", record_b.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));

    let approved_b = std::fs::read_to_string(&record_b).unwrap();
    assert!(
        approved_b.contains(&format!("depends: {}\n", a_id)),
        "expected B's depends line rewritten to A's id: {}",
        approved_b
    );
    assert!(
        !approved_b.contains("depends: 1.1 Does A do a thing?"),
        "the written words must not survive approve: {}",
        approved_b
    );

    let show = adr(&dir, &["show"], &[], "");
    assert!(show.status.success(), "{}", stderr(&show));
    assert!(
        stdout(&show).contains("stands on: 1.1 Does A do a thing?"),
        "{}",
        stdout(&show)
    );
}

// adr: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
//
// The cross-file half's edge: a heading text that names a current decision
// in more than one other topic file is ambiguous, so `approve` refuses
// rather than silently picking one.
#[test]
fn approve_refuses_a_cross_file_heading_dependency_naming_two_current_decisions() {
    let dir = init_repo();
    setup(&dir);

    let a_id = "88888888-8888-8888-8888-888888888888";
    append_ledger_row(&dir, "asked", a_id, "h1", "1.1 Shared heading?");
    append_ledger_row(&dir, "answered", a_id, "h1", "A");
    let a_content = format!(
        "# Topic A\n\n### 1.1 Shared heading?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        a_id
    );
    write_record(&dir, "adrdom/decisions/topic-a.md", &a_content);

    let c_id = "99999999-8888-8888-8888-888888888888";
    append_ledger_row(&dir, "asked", c_id, "h1", "1.1 Shared heading?");
    append_ledger_row(&dir, "answered", c_id, "h1", "A");
    let c_content = format!(
        "# Topic C\n\n### 1.1 Shared heading?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        c_id
    );
    write_record(&dir, "adrdom/decisions/topic-c.md", &c_content);

    let b_content = "# Topic B\n\n### 1.1 Does B stand on the shared heading?\ndepends: 1.1 Shared heading?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record_b = write_record(&dir, "adrdom/decisions/topic-b.md", b_content);

    let out = adr_owner(&dir, &["approve", record_b.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Does B stand on the shared heading?\" depends on \"1.1 Shared heading?\" which is a current decision in more than one file"),
        "{}",
        err
    );
    assert!(err.contains("adrdom/decisions/topic-a.md"), "{}", err);
    assert!(err.contains("adrdom/decisions/topic-c.md"), "{}", err);
    // The ambiguous entry must earn only its own refusal: before this fix,
    // `normalize_cross_file_deps` left the entry as an unresolved same-file
    // `Text` heading dep whenever it found no *unambiguous* substitution to
    // make, so `derive_tiers` (recomputed, or in this case never
    // recomputed at all, since there was nothing to substitute) still
    // called it `Dangling` and this same line earned a second, wrong
    // refusal alongside the ambiguous one.
    assert!(
        !err.contains("which is no heading in this file"),
        "the ambiguous entry must not also be reported as dangling: {}",
        err
    );
}

// adr: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
//
// The same ambiguity, but every match lives in one other topic file (two
// current decisions there happen to carry the exact same heading text --
// `check`'s cross-file duplicate-heading check only flags a duplicate
// heading text *between* files, so nothing else in this repo refuses on a
// same-file one): the refusal must say a heading names several decisions
// in that one file, never "more than one file" when only one file is
// involved.
#[test]
fn approve_refuses_a_cross_file_heading_dependency_naming_two_decisions_in_one_file() {
    let dir = init_repo();
    setup(&dir);

    let a_id = "aaaaaaaa-8888-8888-8888-888888888888";
    append_ledger_row(&dir, "asked", a_id, "h1", "1.1 Shared heading?");
    append_ledger_row(&dir, "answered", a_id, "h1", "A");
    let c_id = "cccccccc-8888-8888-8888-888888888888";
    append_ledger_row(&dir, "asked", c_id, "h2", "1.1 Shared heading?");
    append_ledger_row(&dir, "answered", c_id, "h2", "A");
    let a_content = format!(
        "# Topic A\n\n### 1.1 Shared heading?\ndepends: decision\n\nid: {a}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n\n### 1.1 Shared heading?\ndepends: decision\n\nid: {c}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        a = a_id,
        c = c_id,
    );
    write_record(&dir, "adrdom/decisions/topic-a.md", &a_content);

    let b_content = "# Topic B\n\n### 1.1 Does B stand on the shared heading?\ndepends: 1.1 Shared heading?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record_b = write_record(&dir, "adrdom/decisions/topic-b.md", b_content);

    let out = adr_owner(&dir, &["approve", record_b.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(
            "\"1.1 Does B stand on the shared heading?\" depends on \"1.1 Shared heading?\" which names more than one current decision in adrdom/decisions/topic-a.md"
        ),
        "{}",
        err
    );
    assert!(
        !err.contains("more than one file"),
        "only one file is involved, the wording must not say otherwise: {}",
        err
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
    append_ledger_row(&dir, "asked", y_id, "h1", "1.1 Does Y do a thing?");
    append_ledger_row(&dir, "answered", y_id, "h1", "A");
    let y_content = format!(
        "# Topic Y\n\n### 1.1 Does Y do a thing?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        y_id
    );
    write_record(&dir, "adrdom/decisions/topic-y.md", &y_content);

    // X: an already-answered section whose depends line is hand-edited,
    // after the fact, to a cross-file heading reference -- exactly the
    // edit `record_refusals`'s frozen check exists to catch.
    let x_id = "66666666-6666-6666-6666-666666666666";
    append_ledger_row(&dir, "asked", x_id, "h1", "1.1 Does X stand on something?");
    append_ledger_row(&dir, "answered", x_id, "h1", "A");
    let x_content = format!(
        "# Topic X\n\n### 1.1 Does X stand on something?\ndepends: 1.1 Does Y do a thing?\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        x_id
    );
    let record_x = write_record(&dir, "adrdom/decisions/topic-x.md", &x_content);

    let out = adr_owner(&dir, &["approve", record_x.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    assert!(
        stderr(&out).contains("\"1.1 Does X stand on something?\": answered; agent removes the edit"),
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
        stdout(&check).contains("\"1.1 Does X stand on something?\": answered; agent removes the edit"),
        "check must report the same frozen-edit refusal: {}",
        stdout(&check)
    );
}

// adr: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
//
// `ask` builds its own repo-wide cross-file heading index the same way
// `approve`/`check` do (per "Does a reference in a topic file carry the
// identifier of what it names?"), now loading every topic file itself
// (rather than that load living inside `cross_file_heading_index`, shared
// with callers who load their own copy for other reasons): a plan whose
// only question names a heading text that is a current decision in more
// than one other topic file is exactly as ambiguous to `ask` as it is to
// `approve` -- and `ask`'s own ambiguity check runs before the
// plan-approved gate, so this refuses on the ambiguity itself rather than
// on the plan never having been approved.
#[test]
fn ask_refuses_a_cross_file_heading_dependency_naming_two_current_decisions() {
    let dir = init_repo();
    setup(&dir);

    let a_id = "88888888-8888-8888-8888-888888888888";
    append_ledger_row(&dir, "asked", a_id, "h1", "1.1 Shared heading?");
    append_ledger_row(&dir, "answered", a_id, "h1", "A");
    let a_content = format!(
        "# Topic A\n\n### 1.1 Shared heading?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        a_id
    );
    write_record(&dir, "adrdom/decisions/topic-a.md", &a_content);

    let c_id = "99999999-8888-8888-8888-888888888888";
    append_ledger_row(&dir, "asked", c_id, "h1", "1.1 Shared heading?");
    append_ledger_row(&dir, "answered", c_id, "h1", "A");
    let c_content = format!(
        "# Topic C\n\n### 1.1 Shared heading?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        c_id
    );
    write_record(&dir, "adrdom/decisions/topic-c.md", &c_content);

    let b_content = "# Topic B\n\n### 1.1 Does B stand on the shared heading?\ndepends: 1.1 Shared heading?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record_b = write_record(&dir, "adrdom/decisions/topic-b.md", b_content);

    let out = adr(&dir, &["ask", record_b.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused on the ambiguity: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(
            "\"1.1 Does B stand on the shared heading?\" depends on \"1.1 Shared heading?\" which is a current decision in more than one file"
        ),
        "{}",
        err
    );
    assert!(
        !err.contains("plan not approved"),
        "the ambiguity must refuse before the plan-approved gate: {}",
        err
    );
    assert!(
        !err.contains("which is no heading in this file"),
        "the ambiguous entry must not also be reported as dangling: {}",
        err
    );
}
