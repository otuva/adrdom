// Review finding: everything `adr approve` refuses on a topic file must
// already show in `adr check`, all at once, computed by one shared
// function (`actions::record_refusals`/`missing_answered_refusals`) rather
// than two hand-duplicated copies of the same rules. This asserts that
// parity holds across every refusal category `record_refusals` covers
// (parse, tells, plan, ids) plus the terms-not-approved gate `approve` now
// checks up front (the same gate `ask` already had): for each broken
// fixture, the key substance of `approve`'s refusal also appears somewhere
// in `check`'s report.

mod common;

use common::*;

fn setup(dir: &std::path::Path) {
    assert!(adr(dir, &["install"], &[], "").status.success());
}

/// Runs `adr approve` (expecting refusal) then `adr check` (expecting
/// failure) on the same repo, asserting `needle` appears in both outputs.
fn assert_parity(dir: &std::path::Path, record: &std::path::Path, needle: &str) {
    let approve_out = adr_owner(dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !approve_out.status.success(),
        "approve should have refused: {}",
        stdout(&approve_out)
    );
    let approve_err = stderr(&approve_out);
    assert!(
        approve_err.contains(needle),
        "approve output missing {:?}: {}",
        needle,
        approve_err
    );

    let check_out = adr(dir, &["check"], &[], "");
    assert!(!check_out.status.success(), "check should have failed");
    let check_text = stdout(&check_out);
    assert!(
        check_text.contains(needle),
        "check output missing the same substance {:?}: {}",
        needle,
        check_text
    );
}

#[test]
fn parity_bad_depends_entry() {
    let dir = init_repo();
    setup(&dir);
    approve_empty_terms(&dir);
    let content = "# Example\n\n### 1.1 Q?\ndepends: nosuchterm\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity(
        &dir,
        &record,
        "depends on \"nosuchterm\" which is no term",
    );
}

#[test]
fn parity_tell_in_heading() {
    let dir = init_repo();
    setup(&dir);
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\nContext prose.\n\n### How many stores exist?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity(&dir, &record, "opens assuming an answer");
}

#[test]
fn parity_tier_over_cap() {
    let dir = init_repo();
    setup(&dir);
    approve_minimal_terms(&dir, "decision");
    // Five roots (no heading deps): one tier, five questions, over TIER_CAP
    // (4).
    let content = "# Example\n\n\
### 1.1 Q1?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.2 Q2?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.3 Q3?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.4 Q4?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.5 Q5?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity(&dir, &record, "tier 1 holds 5 questions, the cap is 4");
}

#[test]
fn parity_two_depends_lines() {
    let dir = init_repo();
    setup(&dir);
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\n### 1.1 Q?\ndepends: decision\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity(&dir, &record, "second depends line");
}

#[test]
fn parity_missing_depends_line() {
    let dir = init_repo();
    setup(&dir);
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\n### 1.1 No depends?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity(&dir, &record, "\"1.1 No depends?\" has no depends line");
}

const TERMS_BASIC: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice answered by the owner.\n\
aliases: \n";

#[test]
fn parity_unapproved_terms() {
    let dir = init_repo();
    setup(&dir);
    // `adrdom/terms.md` exists (with a hand-written id, so a naive depends
    // resolution would silently succeed) but is never approved -- no
    // `terms` ledger row. This is the review finding itself: `approve` on
    // the record must refuse on terms not being approved (not "which is no
    // term"), the same substance `check` already prints.
    write_record(&dir, "adrdom/terms.md", TERMS_BASIC);
    let content = "# Example\n\n### 1.1 Q?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let approve_out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !approve_out.status.success(),
        "approve should have refused: {}",
        stdout(&approve_out)
    );
    let approve_err = stderr(&approve_out);
    assert!(approve_err.contains("terms not approved"), "{}", approve_err);
    assert!(
        approve_err.contains("owner runs: adr approve adrdom/terms.md"),
        "{}",
        approve_err
    );

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(!check_out.status.success());
    let check_text = stdout(&check_out);
    assert!(check_text.contains("not approved"), "{}", check_text);
    assert!(
        check_text.contains("owner runs: adr approve adrdom/terms.md"),
        "{}",
        check_text
    );
}

// --- terms: a definition longer than two sentences refuses -------------
// adr: fcc1c1ec-d8e8-428e-9731-9817cdeb8e76

const TERMS_THREE_SENTENCE_DEFINITION: &str = "# Terms\n\n\
## decision\n\
definition: A choice answered by the owner. It stays current until reversed. It is never made twice.\n\
aliases: \n";

#[test]
fn parity_definition_over_two_sentences() {
    let dir = init_repo();
    setup(&dir);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_THREE_SENTENCE_DEFINITION);

    assert_parity(
        &dir,
        &terms,
        "term \"decision\" definition has 3 sentences, more than two",
    );
}

const TERMS_TWO_SENTENCE_DEFINITION: &str = "# Terms\n\n\
## decision\n\
definition: A choice answered by the owner. It stays current until reversed.\n\
aliases: \n";

#[test]
fn approve_allows_a_two_sentence_definition() {
    let dir = init_repo();
    setup(&dir);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_TWO_SENTENCE_DEFINITION);

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(out.status.success(), "approve should have allowed it: {}", stderr(&out));
}
