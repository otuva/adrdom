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
    assert_parity_with(dir, record, needle, true);
}

/// The same parity for a planning finding on a file awaiting approval: check
/// prints it as a note and exits 0 (per "Does check fail a topic file
/// awaiting approval?", 912f4f3b-1213-435a-af0d-7617017fcff4), approve still
/// refuses.
fn assert_parity_note(dir: &std::path::Path, record: &std::path::Path, needle: &str) {
    assert_parity_with(dir, record, needle, false);
}

fn assert_parity_with(dir: &std::path::Path, record: &std::path::Path, needle: &str, fails: bool) {
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
    assert_eq!(!check_out.status.success(), fails, "check exit: {}", stdout(&check_out));
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
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = init_repo();
    setup(&dir);
    approve_empty_terms(&dir);
    let content = "# Example\n\n### 1.1 Q?\ndepends: nosuchterm\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity_note(
        &dir,
        &record,
        "depends on \"nosuchterm\" which is no term",
    );
}

#[test]
fn parity_tell_in_heading() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = init_repo();
    setup(&dir);
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\nContext prose.\n\n### How many stores exist?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity_note(&dir, &record, "opens assuming an answer");
}

#[test]
fn parity_tier_over_cap() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
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

    assert_parity_note(&dir, &record, "5 questions would be put to the owner together, the most is 4");
}

// adr: 5892981c-f213-404f-b93f-551e4d081ebf -- a depends list split across
// two lines is refused by both `adr approve` and `adr check` (a real CLI
// run of each), not silently read as a continuation of the same list.
#[test]
fn parity_two_depends_lines() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = init_repo();
    setup(&dir);
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\n### 1.1 Q?\ndepends: decision\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity_note(&dir, &record, "second depends line");
}

// adr: 5892981c-f213-404f-b93f-551e4d081ebf -- a supersedes list split
// across two lines is refused by both `adr approve` and `adr check`, not
// silently cut to its first line.
#[test]
fn parity_two_supersedes_lines() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = init_repo();
    setup(&dir);
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\n### 1.1 Q?\ndepends: decision\nsupersedes: aaa\nsupersedes: bbb\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity_note(&dir, &record, "second supersedes line");
}

#[test]
fn parity_missing_depends_line() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = init_repo();
    setup(&dir);
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\n### 1.1 No depends?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity_note(&dir, &record, "\"1.1 No depends?\" has no depends line");
}

const TERMS_BASIC: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice answered by the owner.\n\
aliases: \n";

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// "Does the tool stop while the terms file waits for approval?" (A,
// supersedes "Does check fail a terms file that differs from its latest
// approval?"): parity now holds for the terms-not-approved case too --
// `approve` on a *new* record refuses outright while adrdom/terms.md isn't
// approved (`require_terms`), and `check` now fails the same file outright
// for the same reason, even with nothing yet paired to any of its terms.
#[test]
fn parity_unapproved_terms() {
    let dir = init_repo();
    setup(&dir);
    // `adrdom/terms.md` exists (with a hand-written id, so a naive depends
    // resolution would silently succeed) but is never approved -- no
    // `terms` ledger row.
    write_record(&dir, "adrdom/terms.md", TERMS_BASIC);
    let content = "# Example\n\n### 1.1 Q?\ndepends: decision\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    assert_parity(&dir, &record, "owner runs: adr approve adrdom/terms.md");
}

// --- terms: a definition longer than two sentences refuses -------------
// adr: fcc1c1ec-d8e8-428e-9731-9817cdeb8e76

const TERMS_THREE_SENTENCE_DEFINITION: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice answered by the owner. It stays current until reversed. It is never made twice.\n\
aliases: \n";

// `check` only reaches the definition-length check once the file is
// approved (per "Does the tool stop while the terms file waits for
// approval?", aa475aa3 A) -- faking that approval via a hand-written
// ledger row matching this exact (still invalid) content is what lets
// `check`'s own validation run at all, the same defense-in-depth case as a
// cycle check finds.
#[test]
fn parity_definition_over_two_sentences() {
    let dir = init_repo();
    setup(&dir);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_THREE_SENTENCE_DEFINITION);
    approve_terms_via_ledger(&dir, TERMS_THREE_SENTENCE_DEFINITION);

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
