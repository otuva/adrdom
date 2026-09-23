// Batch 8 (8.3/8.4/8.8): the `## ` tier-heading shape error, the "no
// depends line" rule, entry resolution (term/decision/dangling/cycle), the
// tier cap, and the per-file `tiers:` note. Fixtures are hand-written and
// approved via hand-written ledger rows (`approve_via_ledger`,
// `approve_minimal_terms`), never through `adr ask`/`adr approve`, which are
// being built in a separate slice at the same time.

mod common;

use common::*;

#[test]
fn check_fails_a_tier_heading_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/tier.md",
        "# Topic\n\n## Old Group\n\n### 1.1 Q?\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("parse:"), "{}", text);
    assert!(
        text.contains("tier heading, line 3; tiers are derived"),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_malformed_depends_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/bad_depends.md",
        "# Topic\n\n### 1.1 Q?\ndepends: 1.1 Q?=a\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("parse:"), "{}", text);
    assert!(text.contains("depends: bad set syntax, line 4"), "{}", text);
}

const QUESTION_BODY: &str = "\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";

#[test]
fn check_fails_a_section_with_no_depends_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = format!("# Topic\n\n### 1.1 Does it have a line?{}", QUESTION_BODY);
    write_record(&dir, "adrdom/decisions/nodep.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/nodep.md", &content);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("plan: adrdom/decisions/nodep.md: \"1.1 Does it have a line?\" has no depends line"),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_skeleton_with_no_depends_line_too() {
    // 8.3/8.6 B: the "at least one depends line" rule applies to a
    // skeleton heading exactly as it does to a real question -- a skeleton
    // is otherwise never a failure (4.7 A), but this one rule is the
    // explicit exception.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/skeleton.md",
        "# Topic\n\n### 1.1 Not written yet?\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("plan: adrdom/decisions/skeleton.md: \"1.1 Not written yet?\" has no depends line"),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_depends_entry_that_resolves_no_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    let content = format!(
        "# Topic\n\n### 1.1 Q?\ndepends: widget{}",
        QUESTION_BODY
    );
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "plan: adrdom/decisions/example.md: \"1.1 Q?\" depends on \"widget\" which is no term; agent adds the term or names another"
        ),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_depends_entry_on_a_retired_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms_content = "# Terms\n\n## widget\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A small thing.\naliases: \nretired: yes\n";
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();
    approve_terms_via_ledger(&dir, terms_content);
    let content = format!("# Topic\n\n### 1.1 Q?\ndepends: widget{}", QUESTION_BODY);
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "plan: adrdom/decisions/example.md: \"1.1 Q?\" depends on \"widget\" which is no term; agent adds the term or names another"
        ),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_depends_entry_on_a_decision_that_is_not_current() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    let other_id = "22222222-2222-2222-2222-222222222222";
    let content = format!("# Topic\n\n### 1.1 Q?\ndepends: {}{}", other_id, QUESTION_BODY);
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    // `other_id` never asked/answered: not a current decision.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(&format!(
            "plan: adrdom/decisions/example.md: \"1.1 Q?\" depends on {} which is not a current decision; agent names a current decision or removes the line",
            other_id
        )),
        "{}",
        text
    );
}

#[test]
fn check_passes_a_depends_entry_on_a_current_decision() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    let other_id = "33333333-3333-3333-3333-333333333333";
    append_ledger_row(&dir, "asked", other_id, "h1", "old heading");
    append_ledger_row(&dir, "answered", other_id, "h1", "A");
    let content = format!("# Topic\n\n### 1.1 Q?\ndepends: {}{}", other_id, QUESTION_BODY);
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains("which is not a current decision"),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_dangling_depends_entry() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Topic\n\n### 1.1 Q?\ndepends: 2.2 Missing?\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "plan: adrdom/decisions/example.md: \"1.1 Q?\" depends on \"2.2 Missing?\" which is no heading in this file; agent adds the heading or fixes the text"
        ),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_cycle() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Topic\n\n### A?\ndepends: B?\n\n### B?\ndepends: A?\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("plan: adrdom/decisions/example.md: cycle:"),
        "{}",
        text
    );
    assert!(text.contains("\"A?\""), "{}", text);
    assert!(text.contains("\"B?\""), "{}", text);
    assert!(text.contains("; agent breaks it"), "{}", text);
}

#[test]
fn check_fails_a_self_dep() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Topic\n\n### A?\ndepends: A?\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "plan: adrdom/decisions/example.md: \"A?\" depends on itself; agent removes the line or names something else"
        ),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_tier_over_the_cap() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    // Five roots (no heading deps): one tier, five questions, over TIER_CAP
    // (4).
    let content = "# Topic\n\n\
### 1.1 Q1?\ndepends: widget\n\n\
### 1.2 Q2?\ndepends: widget\n\n\
### 1.3 Q3?\ndepends: widget\n\n\
### 1.4 Q4?\ndepends: widget\n\n\
### 1.5 Q5?\ndepends: widget\n";
    write_record(&dir, "adrdom/decisions/cap.md", content);
    approve_via_ledger(&dir, "adrdom/decisions/cap.md", content);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("plan: adrdom/decisions/cap.md: tier 1 holds 5 questions, the cap is 4"),
        "{}",
        text
    );
    // Derivation still succeeds (the cap is a separate failure, not a
    // derive error), so the tiers note is still printed.
    assert!(
        text.contains("tiers: adrdom/decisions/cap.md: 1 (5)"),
        "{}",
        text
    );
}

#[test]
fn check_passes_a_tier_at_the_cap() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = "# Topic\n\n\
### 1.1 Q1?\ndepends: widget\n\n\
### 1.2 Q2?\ndepends: widget\n\n\
### 1.3 Q3?\ndepends: widget\n\n\
### 1.4 Q4?\ndepends: widget\n";
    write_record(&dir, "adrdom/decisions/atcap.md", content);
    approve_via_ledger(&dir, "adrdom/decisions/atcap.md", content);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("holds"), "{}", text);
    assert!(
        text.contains("tiers: adrdom/decisions/atcap.md: 1 (4)"),
        "{}",
        text
    );
}

#[test]
fn check_prints_tiers_note_for_a_chain() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = "# Topic\n\n\
### A?\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### B?\ndepends: A?=A\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    write_record(&dir, "adrdom/decisions/chain.md", content);
    approve_via_ledger(&dir, "adrdom/decisions/chain.md", content);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        text.contains("tiers: adrdom/decisions/chain.md: 1 (1) 2 (1)"),
        "{}",
        text
    );
}
