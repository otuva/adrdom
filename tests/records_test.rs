// 9.5 A: loaded phrases. `src/tells.rs` owns the lists and the matcher
// (unit-tested there); these are the integration refusals/reports built on
// top of it: `approve` on a heading and on context prose, `ask` on a
// candidate's body, `check` on unfrozen text only, and a frozen answered
// section with a tell passing `check`.

mod common;

use common::*;

const HEADING_TELL_RECORD: &str = r#"# Example

Context prose line.

### Which store holds the ledger?
depends: decision

Problem: the store is unclear.

Standard: s.

- A) Answer one.
  Why: reasons.
  Costs: costs.
- B) Answer two.
  Why: other reasons.
  Costs: other costs.
"#;

// approve stops a phrase-list
// hit in a heading (clause: "approve stops a hit in headings").
#[test]
fn approve_refuses_a_heading_with_a_loaded_opening() {
    // adr: 149c9a73-16eb-46aa-9806-2950423ee1a2
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", HEADING_TELL_RECORD);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should refuse: {}",
        stdout(&out)
    );
    let text = stderr(&out);
    assert!(text.contains("\"which\""), "{}", text);
    assert!(
        text.contains("the heading opens with \"which\"; agent rewrites the heading"),
        "{}",
        text
    );
    assert!(text.contains("agent rewrites the heading"), "{}", text);
}

const CONTEXT_TELL_RECORD: &str = r#"# Example

Claude decided this design already.

### Is the store exist?
depends: decision

Problem: p.

Standard: s.

- A) Answer one.
  Why: reasons.
  Costs: costs.
- B) Answer two.
  Why: other reasons.
  Costs: other costs.
"#;

// approve stops a phrase-list
// hit in the context prose (clause: "approve stops a hit ... in the
// prose").
#[test]
fn approve_refuses_a_context_line_with_a_provenance_tell() {
    // adr: 149c9a73-16eb-46aa-9806-2950423ee1a2
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", CONTEXT_TELL_RECORD);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should refuse: {}",
        stdout(&out)
    );
    let text = stderr(&out);
    assert!(text.contains("\"claude\""), "{}", text);
    assert!(text.contains("names who said it"), "{}", text);
    assert!(text.contains("\"context\""), "{}", text);
}

const BODY_TELL_RECORD: &str = r#"# Example

Context prose.

### Is the store exist?
depends: decision

Problem: the team built the store for the decision.

Standard: s.

- A) Answer one.
  Why: reasons.
  Costs: costs.
- B) Answer two.
  Why: other reasons.
  Costs: other costs.
"#;

// approve stops a phrase-list hit in a question body.
#[test]
fn approve_refuses_a_candidate_with_a_code_state_tell_in_its_body() {
    // adr: 149c9a73-16eb-46aa-9806-2950423ee1a2
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", BODY_TELL_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    // Approve cannot take the loaded word, so it is put back after it and
    // the approval rows are written by hand: approve is the second stop.
    let loaded = std::fs::read_to_string(&record).unwrap().replace(
        "the team built the store",
        "the team already built the store",
    );
    std::fs::write(&record, &loaded).unwrap();
    let rf = adr::record::load(&record).unwrap();
    let rel = "adrdom/decisions/example.md";
    let later = "2099-01-01T00:00:00Z";
    append_ledger_row_at(
        &dir,
        "approved",
        "-",
        &adr::record::plan_hash(&rf),
        rel,
        later,
    );
    append_ledger_row_at(
        &dir,
        "context",
        "-",
        &adr::record::context_hash(&rf),
        rel,
        later,
    );

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should refuse: {}",
        stdout(&out)
    );
    let text = stderr(&out);
    assert!(text.contains("\"already\""), "{}", text);
    assert!(text.contains("states what the code does"), "{}", text);
}

const UNFROZEN_TELL_RECORD: &str = r#"# Example

The team said yes.

### How many stores exist?
depends: decision

Problem: p of the decision.

Standard: s.

- A) Answer one.
  Why: reasons.
  Costs: costs.
- B) Answer two.
  Why: other reasons.
  Costs: other costs.
"#;

// check reports phrase-list
// hits in text that can still change: an unasked (unfrozen) heading and
// its context prose (clause: "check reports hits in text that can still
// change").
#[test]
fn check_reports_a_tell_in_an_unfrozen_heading_and_in_context_prose() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision"]);
    write_record(&dir, "adrdom/decisions/example.md", UNFROZEN_TELL_RECORD);
    // No approve/ask: the section is unfrozen (never asked at all), so
    // `check` must still report it.

    let out = adr(&dir, &["check"], &[], "");
    // A file awaiting approval: the findings are notes, not a failure.
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("\"how many\""), "{}", text);
    assert!(
        text.contains("the heading opens with \"how many\"; agent rewrites the heading"),
        "{}",
        text
    );
    assert!(text.contains("\"said\""), "{}", text);
    assert!(text.contains("names who said it"), "{}", text);

    let tells_lines: Vec<&str> = text
        .lines()
        .filter(|l| {
            l.contains("the heading opens with \"how many\"") || l.contains("names who said it")
        })
        .collect();
    assert_eq!(
        tells_lines.len(),
        2,
        "expected exactly one heading hit and one context hit, got: {:?}",
        tells_lines
    );
}

const FROZEN_TELL_ID: &str = "22222222-2222-2222-2222-222222222222";

/// Just the section (from its heading, no h1/context above it): a heading
/// with a loaded opening, already answered. Hand-written, never through
/// `adr approve`/`adr answer` -- approve would refuse this heading outright,
/// so the only way to get a *frozen* section carrying a tell is to write
/// the ledger row directly, the same way `check_refused_test.rs` builds a
/// refused fixture.
fn frozen_tell_section() -> String {
    format!(
        "### Which store holds the ledger?\ndepends: widget\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A",
        FROZEN_TELL_ID
    )
}

// the other half of "text that
// can still change": once a section is frozen (answered), check must never
// report a tell in it -- the clause bounds check to unfrozen text, not all
// text.
#[test]
fn check_passes_a_frozen_answered_section_with_a_tell() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = format!("# Example\n\n{}\n", frozen_tell_section());
    write_record(&dir, "adrdom/decisions/example.md", &content);
    append_ledger_row(&dir, "answered", FROZEN_TELL_ID, "somehash", "A");

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains("the heading opens with \"which\""),
        "a frozen answered section must never fail on a tell: {}",
        text
    );
}

/// Like `frozen_tell_section`, but its `depends:` line is already the
/// term's uuid rather than the word "widget": a real approve of a frozen
/// section refuses a still-text depends entry regardless of the tell
/// question this test is about, so this fixture is
/// pre-resolved the way a real approve would have left it, matching
/// `approve_minimal_terms`'s fixed term id.
fn frozen_tell_section_resolved() -> String {
    format!(
        "### Which store holds the ledger?\ndepends: 99999999-9999-9999-9999-999999999999\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A",
        FROZEN_TELL_ID
    )
}

#[test]
fn approve_skips_a_frozen_headings_tell_and_still_mints_a_new_skeletons_id() {
    // A frozen heading can never be edited, so approve must never refuse on
    // one: refusing would block this file forever once the list grows to
    // include a word an already-answered heading happens to carry. approve
    // must still process the rest of the file normally, minting an id for
    // the new skeleton heading below it.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let frozen = frozen_tell_section_resolved();
    let content = format!(
        "# Example\n\nContext prose.\n\n{}\n\n### A new clean heading?\ndepends: widget\n\nProblem: p of the widget.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        frozen
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    // The answered row carries the section's real hash, as `answer` writes it.
    append_ledger_row(
        &dir,
        "answered",
        FROZEN_TELL_ID,
        &mirror_section_hash(&frozen),
        "A",
    );

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve should skip the frozen heading's tell: {}",
        stderr(&out)
    );

    let saved = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&saved);
    assert_eq!(
        ids.len(),
        2,
        "expected the frozen id plus a newly minted one for the skeleton: {}",
        saved
    );
}

// E1.2: `do_approve_record` used to return at the first refusing category
// (shape, tells, depends...), so a heading with both a loaded-opening tell
// and a depends entry naming no term would only ever show the tell. `check`
// already reports both (`tells::check_tells` and `plan::check_plan` both
// run, unconditionally, every time), so this is really pinning `approve`
// down to what `check` already does.
const TELL_AND_BAD_DEPENDS_RECORD: &str = r#"# Example

Context prose line.

### Which store holds the ledger?
depends: nosuchterm

Problem: the store is unclear.

Standard: s.

- A) Answer one.
  Why: reasons.
  Costs: costs.
- B) Answer two.
  Why: other reasons.
  Costs: other costs.
"#;

#[test]
fn approve_and_check_report_a_tell_and_a_bad_depends_entry_together() {
    // adr: 912f4f3b-1213-435a-af0d-7617017fcff4
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    let record = write_record(
        &dir,
        "adrdom/decisions/example.md",
        TELL_AND_BAD_DEPENDS_RECORD,
    );

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should refuse: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("the heading opens with \"which\"; agent rewrites the heading"),
        "{}",
        err
    );
    assert!(
        err.contains("depends on \"nosuchterm\" which is no term"),
        "{}",
        err
    );

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(check_out.status.success(), "{}", stdout(&check_out));
    let text = stdout(&check_out);
    assert!(
        text.contains("the heading opens with \"which\"; agent rewrites the heading"),
        "{}",
        text
    );
    assert!(
        text.contains("depends on \"nosuchterm\" which is no term"),
        "{}",
        text
    );
}
