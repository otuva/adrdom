// Batch 8.1/8.2 (`answer <id> refuse`): the refusal-line/ledger-row
// consistency checks, `done` excluding a refused section, `tested`
// treating a refused id as never-untested but its ties as stale, and a
// refused id still having to live in a topic file until `approve`
// withdraws it. Fixtures are hand-written (record content plus
// `append_ledger_row`), never through `adr answer refuse` (built in a
// separate slice at the same time).

mod common;

use common::*;

const REFUSABLE_ID: &str = "11111111-1111-1111-1111-111111111111";

/// Just the section (from its heading, no h1/context above it), the shape
/// `mirror_section_hash` expects.
fn refusable_section(answer: Option<&str>) -> String {
    let mut s = format!(
        "### 1.1 Q?\ndepends: widget\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.",
        REFUSABLE_ID
    );
    if let Some(a) = answer {
        s.push_str(&format!("\n\n**Answer:** {}", a));
    }
    s
}

fn refusable_content(answer: Option<&str>) -> String {
    format!("# Topic\n\n{}\n", refusable_section(answer))
}

#[test]
fn check_fails_a_refusal_line_without_a_ledger_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let content = refusable_content(Some("refused"));
    write_record(&dir, "adrdom/decisions/example.md", &content);
    // No `refused` row at all.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("answers: adrdom/decisions/example.md: 1.1 Q?: refusal line without ledger row"),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_refused_row_without_the_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let content = refusable_content(None);
    write_record(&dir, "adrdom/decisions/example.md", &content);
    append_ledger_row(&dir, "refused", REFUSABLE_ID, "somehash", "1.1 Q?");

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("answers: adrdom/decisions/example.md: 1.1 Q?: refused row without answer line"),
        "{}",
        text
    );
}

#[test]
fn check_passes_a_refusal_line_with_its_matching_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = refusable_content(Some("refused"));
    write_record(&dir, "adrdom/decisions/example.md", &content);
    // The row's hash must be the section hash *before* the `**Answer:**
    // refused` line was inserted: exactly the text `refusable_content(None)`
    // would produce.
    let hash = mirror_section_hash(&refusable_section(None));
    append_ledger_row(&dir, "refused", REFUSABLE_ID, &hash, "1.1 Q?");

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains("refusal line without ledger row"),
        "{}",
        text
    );
    assert!(
        !text.contains("refused row without answer line"),
        "{}",
        text
    );
}

#[test]
fn check_done_excludes_a_refused_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = refusable_content(Some("refused"));
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    let hash = mirror_section_hash(&refusable_section(None));
    append_ledger_row(&dir, "refused", REFUSABLE_ID, &hash, "1.1 Q?");

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains("done: adrdom/decisions/example.md"),
        "a refused section must not count as done: {}",
        text
    );
}

#[test]
fn check_a_refused_id_is_never_untested_but_a_tie_to_it_is_stale() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = refusable_content(Some("refused"));
    write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    let hash = mirror_section_hash(&refusable_section(None));
    append_ledger_row(&dir, "refused", REFUSABLE_ID, &hash, "1.1 Q?");
    write_test_comment(&dir, REFUSABLE_ID);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains(&format!("{}: untested", REFUSABLE_ID)),
        "a refused id must never be reported untested: {}",
        text
    );
    assert!(
        text.contains(&format!("{}: stale (not a current decision)", REFUSABLE_ID)),
        "a tie to a refused id must be stale: {}",
        text
    );
}

#[test]
fn check_fails_a_refused_id_missing_from_every_file_until_withdrawn() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    append_ledger_row(&dir, "asked", REFUSABLE_ID, "h1", "1.1 Q?");
    append_ledger_row(&dir, "refused", REFUSABLE_ID, "h1", "1.1 Q?");
    // `REFUSABLE_ID` appears in no topic file at all, and was never
    // withdrawn.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(&format!("ledger id {} not found in any topic file", REFUSABLE_ID)),
        "{}",
        text
    );
}

#[test]
fn check_passes_a_refused_id_missing_once_withdrawn() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    append_ledger_row(&dir, "asked", REFUSABLE_ID, "h1", "1.1 Q?");
    append_ledger_row(&dir, "refused", REFUSABLE_ID, "h1", "1.1 Q?");
    append_ledger_row(&dir, "withdrawn", REFUSABLE_ID, "-", "1.1 Q?");

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains(&format!("ledger id {} not found", REFUSABLE_ID)),
        "{}",
        text
    );
}

#[test]
fn check_names_the_file_when_an_asked_heading_is_dropped_by_an_unapproved_edit() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let rel = "adrdom/decisions/example.md";
    let content = refusable_content(None);
    write_record(&dir, rel, &content);
    approve_via_ledger(&dir, rel, &content);
    let hash = mirror_section_hash(&refusable_section(None));
    append_ledger_row(&dir, "asked", REFUSABLE_ID, &hash, "1.1 Q?");

    // The heading is edited out of the file, but the file is never
    // re-approved: a plan change awaiting approval, not a formal
    // withdrawal.
    write_record(&dir, rel, "# Topic\n\nContext prose.\n");

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(&format!(
            "{} (\"1.1 Q?\", asked) was removed from the plan; owner runs: adr approve {}",
            REFUSABLE_ID, rel
        )),
        "{}",
        text
    );
}

#[test]
fn check_stays_silent_on_a_superseded_id_once_the_file_is_reapproved() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let rel = "adrdom/decisions/example.md";
    let content = refusable_content(None);
    write_record(&dir, rel, &content);
    approve_via_ledger(&dir, rel, &content);
    let hash = mirror_section_hash(&refusable_section(None));
    append_ledger_row(&dir, "asked", REFUSABLE_ID, &hash, "1.1 Q?");
    append_ledger_row(&dir, "superseded", REFUSABLE_ID, "-", "new-id");

    // The heading is gone and the file has since been re-approved without
    // it: the ordinary, finished state after a supersede, not a plan
    // change still awaiting approval.
    let after = "# Topic\n\nContext prose.\n";
    write_record(&dir, rel, after);
    approve_via_ledger(&dir, rel, after);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains(REFUSABLE_ID),
        "a superseded id must earn no ids failure at all: {}",
        text
    );
}

#[test]
fn check_stays_silent_on_a_superseded_id_despite_an_unrelated_stale_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");

    let rel_a = "adrdom/decisions/a.md";
    let content_a = refusable_content(None);
    write_record(&dir, rel_a, &content_a);
    approve_via_ledger(&dir, rel_a, &content_a);
    let hash = mirror_section_hash(&refusable_section(None));
    append_ledger_row(&dir, "asked", REFUSABLE_ID, &hash, "1.1 Q?");
    append_ledger_row(&dir, "superseded", REFUSABLE_ID, "-", "new-id");

    // A's heading is gone and A is re-approved without it: the ordinary,
    // finished state after a supersede.
    let after_a = "# Topic\n\nContext prose.\n";
    write_record(&dir, rel_a, after_a);
    approve_via_ledger(&dir, rel_a, after_a);

    // B has nothing to do with A's id, but is itself a stale, unapproved
    // plan edit: approved once, then edited again without re-approving.
    // Its mere presence must not turn A's superseded id into a
    // "removed from the plan" failure.
    let rel_b = "adrdom/decisions/b.md";
    let content_b = "# Other\n\nContext prose.\n";
    write_record(&dir, rel_b, content_b);
    approve_via_ledger(&dir, rel_b, content_b);
    write_record(&dir, rel_b, "# Other\n\nEdited prose.\n");

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains(REFUSABLE_ID),
        "an unrelated stale file must not turn a superseded id into a removed-from-plan failure: {}",
        text
    );
}
