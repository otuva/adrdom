// The `## ` tier-heading shape error, the "no depends line" rule, entry
// resolution (term/decision/dangling/cycle), and the tier cap; the tiers
// are no line of check's (bd83aa22). Fixtures are hand-written and approved via
// hand-written ledger rows (`approve_via_ledger`, `approve_minimal_terms`),
// never through a real `adr ask`/`adr approve`.

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
    assert!(
        text.contains("tier heading, line 3; agent removes the heading, then writes what each question stands on in its depends line"),
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
        "adrdom/decisions/bad-depends.md",
        "# Topic\n\n### 1.1 Q?\ndepends: 1.1 Q?=a\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("depends: bad set syntax, line 4"), "{}", text);
    assert!(
        text.contains("; agent rewrites the depends line"),
        "{}",
        text
    );
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
        text.contains("adrdom/decisions/nodep.md: \"1.1 Does it have a line?\" has no depends line; agent writes the depends line of \"1.1 Does it have a line?\""),
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
        text.contains("adrdom/decisions/skeleton.md: \"1.1 Not written yet?\" has no depends line; agent writes the depends line of \"1.1 Not written yet?\""),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_depends_entry_that_resolves_no_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    let content = format!("# Topic\n\n### 1.1 Q?\ndepends: widget{}", QUESTION_BODY);
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "adrdom/decisions/example.md: \"1.1 Q?\" depends on \"widget\" which is no term; agent writes the term in adrdom/terms.md"
        ),
        "{}",
        text
    );
}

// E1.3: a `depends:` entry naming a real term in a terms.md that has never
// been approved (so it carries no id on disk yet) must not be reported as
// "which is no term" -- that flooded one line per entry in the lab run,
// drowning out what the file's approval state is.
//
// Per "Does the tool stop while the terms file waits for approval?"
// (aa475aa3 A): an unapproved terms.md now fails `check` outright on its
// own, but the suppression this test is about still holds -- the failure
// is the one "terms not approved" line, never a flood of "which is no
// term" lines for every real, not-yet-minted term a depends line names.
#[test]
fn check_does_not_report_no_term_for_a_real_term_in_an_unapproved_terms_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // Hand-written, never approved: no `id:` line, no `terms` ledger row.
    std::fs::write(
        dir.join("adrdom/terms.md"),
        "# Terms\n\n## widget\ndefinition: A small thing.\naliases: \n",
    )
    .unwrap();
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Topic\n\n### 1.1 Q?\ndepends: widget\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "adrdom/terms.md: \"widget\" is new; owner runs: adr approve adrdom/terms.md widget"
        ),
        "{}",
        text
    );
    assert!(
        text.contains("not yet: commit, after the owner approves adrdom/terms.md"),
        "{}",
        text
    );
    assert!(!text.contains("no term"), "{}", text);
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
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
            "adrdom/decisions/example.md: \"1.1 Q?\" names the retired term \"widget\"; agent rewrites the entry"
        ),
        "{}",
        text
    );
}

// The above uses a `Text` entry ("widget"), which never resolves to a
// retired term at all (`phrase_lookup` skips retired terms). This exercises
// the other shape: an already-approved `Id` entry that names a retired
// term directly, on a section that is still open (asked, unanswered) --
// still a failure, same as unasked.
#[test]
fn check_fails_a_depends_entry_on_a_retired_term_by_id_while_open() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let term_id = "11111111-1111-1111-1111-111111111111";
    let terms_content = format!(
        "# Terms\n\n## widget\nid: {}\ndefinition: A small thing.\naliases: \nretired: yes\n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), &terms_content).unwrap();
    approve_terms_via_ledger(&dir, &terms_content);

    let question_id = "22222222-2222-2222-2222-222222222222";
    let content = format!(
        "# Topic\n\n### 1.1 Q?\ndepends: {term}\n\nid: {id}{body}",
        term = term_id,
        id = question_id,
        body = QUESTION_BODY
    );
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    append_ledger_row(&dir, "asked", question_id, "deadbeef", "1.1 Q?");
    // Asked but never answered: still open.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(&"adrdom/decisions/example.md: \"1.1 Q?\" depends on the term \"widget\", which is retired; agent rewrites the entry to name a term that is not retired".to_string()),
        "{}",
        text
    );
}

// Once the question is no longer open -- superseded here -- a `depends:`
// entry frozen back when the term was still current is a record of what it
// stood on then, not a dangling reference to re-plan: `check` stays clean
// on it, the mirror of `check_fails_a_depends_entry_on_a_retired_term_by_id_while_open`
// above.
#[test]
fn check_passes_a_depends_entry_on_a_retired_term_once_the_section_is_superseded() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let term_id = "11111111-1111-1111-1111-111111111111";
    let terms_content = format!(
        "# Terms\n\n## widget\nid: {}\ndefinition: A small thing.\naliases: \nretired: yes\n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), &terms_content).unwrap();
    approve_terms_via_ledger(&dir, &terms_content);

    let question_id = "22222222-2222-2222-2222-222222222222";
    let content = format!(
        "# Topic\n\n### 1.1 Q?\ndepends: {term}\n\nid: {id}{body}",
        term = term_id,
        id = question_id,
        body = QUESTION_BODY
    );
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    append_ledger_row(&dir, "asked", question_id, "deadbeef", "1.1 Q?");
    append_ledger_row(
        &dir,
        "superseded",
        question_id,
        "-",
        "33333333-3333-3333-3333-333333333333",
    );

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains("which names no term and no question of this file"),
        "{}",
        text
    );
    assert!(!text.contains("no term"), "{}", text);
}

#[test]
fn check_names_a_retired_term_under_an_answered_question_as_a_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let term_id = "11111111-1111-1111-1111-111111111111";
    let approved = format!(
        "# Terms\n\n## tag\nid: {}\ndefinition: A small thing.\naliases: \n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), &approved).unwrap();
    approve_terms_via_ledger(&dir, &approved);

    let question_id = "22222222-2222-2222-2222-222222222222";
    let content = format!(
        "# Topic\n\n### Is the tool keep tags?\ndepends: {term}\n\nid: {id}{body}",
        term = term_id,
        id = question_id,
        body = QUESTION_BODY
    );
    write_record(&dir, "adrdom/decisions/notes.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/notes.md", &content);
    append_ledger_row_at(
        &dir,
        "asked",
        question_id,
        "h1",
        "Is the tool keep tags?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(
        &dir,
        "answered",
        question_id,
        "h1",
        "A",
        "2024-01-01T00:00:01Z",
    );

    // Retire the term by editing the file only; check reads it as it is.
    std::fs::write(
        dir.join("adrdom/terms.md"),
        format!("{}retired: yes\n", approved),
    )
    .unwrap();

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        text.contains("adrdom/decisions/notes.md: \"Is the tool keep tags?\" depends on the term \"tag\", which is retired; agent rewrites the entry to name a term that is not retired"),
        "{}",
        text
    );
    assert!(
        !text.contains("which names no term and no question of this file"),
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
    let content = format!(
        "# Topic\n\n### 1.1 Q?\ndepends: {}{}",
        other_id, QUESTION_BODY
    );
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    // `other_id` never asked/answered: not a current decision.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(&format!(
            "adrdom/decisions/example.md: \"1.1 Q?\" depends on {} which names no term and no question of this file; agent rewrites the entry to name a term or a question of this file",
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
    append_ledger_row_at(
        &dir,
        "asked",
        other_id,
        "h1",
        "old heading",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(
        &dir,
        "answered",
        other_id,
        "h1",
        "A",
        "2024-01-01T00:00:01Z",
    );
    let content = format!(
        "# Topic\n\n### 1.1 Q?\ndepends: {}{}",
        other_id, QUESTION_BODY
    );
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains("which names no term and no question of this file"),
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
            "adrdom/decisions/example.md: \"1.1 Q?\" depends on \"2.2 Missing?\" which is no heading in this file; agent rewrites the entry"
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
        text.contains("adrdom/decisions/example.md: depends lines lead back to their own question: \"A?\", \"B?\""),
        "{}",
        text
    );
    assert!(
        text.contains("agent rewrites the depends lines of these questions"),
        "{}",
        text
    );
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
            "adrdom/decisions/example.md: \"A?\" depends on itself; agent removes the entry"
        ),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_depends_set_naming_an_option_the_target_does_not_have() {
    // 8.7 A: `ask` also refuses this
    // (ask_tiers_test.rs's `ask_refuses_a_set_naming_an_option_the_target_does_not_have`),
    // but `check`'s `plan` check must catch it unconditionally, the moment
    // the target already has options, without waiting for that tier.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = "# Topic\n\n\
### A?\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### B?\ndepends: A?=C\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    write_record(&dir, "adrdom/decisions/badset.md", content);
    approve_via_ledger(&dir, "adrdom/decisions/badset.md", content);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "adrdom/decisions/badset.md: \"B?\" depends on \"A?\" under an answer set naming option C, which it does not have; agent rewrites the answer set"
        ),
        "{}",
        text
    );
}

#[test]
fn check_passes_a_depends_set_whose_letters_are_all_options_the_target_has() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = "# Topic\n\n\
### A?\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### B?\ndepends: A?=A\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    write_record(&dir, "adrdom/decisions/goodset.md", content);
    approve_via_ledger(&dir, "adrdom/decisions/goodset.md", content);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("does not have"), "{}", text);
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
        text.contains("adrdom/decisions/cap.md: 5 questions in one tier, the most is 4: "),
        "{}",
        text
    );
    assert!(
        text.contains("agent writes the question these stand on"),
        "{}",
        text
    );
    assert!(!text.contains("tiers:"), "{}", text);
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
    assert!(!text.contains("tiers:"), "{}", text);
}

#[test]
fn check_tier_cap_ignores_a_refused_question() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let term_id = "99999999-9999-9999-9999-999999999999";
    let refused_id = "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee";
    // A frozen (refused) section carries its depends entry already
    // resolved to the term's id, as a real prior approve would have left
    // it -- otherwise the frozen/Text-depends refusal fires instead of the
    // cap check this test means to exercise.
    let refused_section = format!(
        "### 1.1 Dead?\ndepends: {term}\n\nid: {id}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.",
        term = term_id,
        id = refused_id
    );
    let content = format!(
        "# Topic\n\n{refused}\n\n**Answer:** refused\n\n\
### 1.2 Q1?\ndepends: widget\n\n\
### 1.3 Q2?\ndepends: widget\n\n\
### 1.4 Q3?\ndepends: widget\n\n\
### 1.5 Q4?\ndepends: widget\n",
        refused = refused_section
    );
    write_record(&dir, "adrdom/decisions/deadcap.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/deadcap.md", &content);
    let hash = mirror_section_hash(&refused_section);
    append_ledger_row(&dir, "refused", refused_id, &hash, "1.1 Dead?");

    // Five raw questions in tier 1, one of them refused: the cap (4) reads
    // only the four live ones, so this passes.
    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("holds"), "{}", text);
}

#[test]
fn check_prints_no_tiers_line_for_a_chain() {
    // adr: bd83aa22-94fc-49ac-b215-a82990ff8c65
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
    assert!(!text.contains("tiers:"), "{}", text);
}
