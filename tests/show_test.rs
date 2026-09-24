mod common;

use common::*;

const RECORD: &str = r#"# Example

### 1.1 Is this current?
depends: reasons

Problem: whether it is current.

Standard: clear yes or no.

- A) Yes it is current.
  Why: reasons.
  Costs: costs.
- B) No it is not.
  Why: other reasons.
  Costs: other costs.

### 1.2 Is this only asked?
depends: reasons

Problem: whether it is only asked.

Standard: clear yes or no.

- A) Yes only asked.
  Why: reasons.
  Costs: costs.
- B) No.
  Why: other reasons.
  Costs: other costs.
"#;

/// Every question is listed now (spec §7: `show` prints every derived
/// tier's questions with their state), not only the current, answered
/// ones — the old (pre-tier) `show` filtered down to just those. Both
/// headings here are root (no `depends:` heading entry, only the fixture
/// term `reasons`), so both land in tier 1.
#[test]
fn show_lists_every_question_with_its_state_and_tier() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "reasons");
    let record = write_record(&dir, "adrdom/decisions/example.md", RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    assert_eq!(ids.len(), 2);
    let answered_id = &ids[0];

    let answer_out = adr_owner(&dir, &["answer", answered_id, "A"], "");
    assert!(
        answer_out.status.success(),
        "answer failed: {}",
        stderr(&answer_out)
    );

    let out = adr(&dir, &["show"], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);

    assert!(shown.starts_with("# Example\n"), "{}", shown);
    assert!(shown.contains("tier 1\n"), "{}", shown);
    assert!(shown.contains("[answered A] 1.1 Is this current?"), "{}", shown);
    assert!(shown.contains("Yes it is current."), "{}", shown);
    // The second question is still open (asked, not answered): it is now
    // listed too, with its state, rather than left out.
    assert!(shown.contains("[open] 1.2 Is this only asked?"), "{}", shown);
    assert!(!shown.contains("Yes only asked."), "no chosen sentence for an unanswered question: {}", shown);
}

const TIERED_RECORD: &str = "# Tiered Topic\n\n\
### 1.1 Root?\n\n\
id: id-root\n\n\
Problem: p.\n\n\
Standard: s.\n\n\
- A) First choice.\n  Why: w.\n  Costs: c.\n\
- B) Second choice.\n  Why: w.\n  Costs: c.\n\n\
**Answer:** A\n\n\
### 1.2 Unasked root?\n\n\
Problem: p2.\n\n\
Standard: s2.\n\n\
- A) x.\n  Why: w.\n  Costs: c.\n\
- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.3 Skeleton?\n\n\
### 2.1 Depends on root?\n\
depends: 1.1 Root?=A\n\n\
id: id-open\n\n\
Problem: p3.\n\n\
Standard: s3.\n\n\
- A) x.\n  Why: w.\n  Costs: c.\n\
- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 2.2 Also depends?\n\
depends: 1.1 Root?=A\n\n\
id: id-refused\n\n\
Problem: p4.\n\n\
Standard: s4.\n\n\
- A) x.\n  Why: w.\n  Costs: c.\n\
- B) y.\n  Why: w.\n  Costs: c.\n\n\
**Answer:** refused\n";

/// Exercises tiers, every state `show` prints, the "stands on:" line, and
/// the chosen sentence, all via hand-crafted ledger rows and `**Answer:**`
/// lines (per the wave-2 brief) rather than `adr ask`/`adr answer`, which
/// are being rebuilt by other agents on this same branch.
// adr: aec860fd-22ad-44a7-99f7-222ea19537e7
//
// "show prints the tiers": the fixture below states no tier anywhere in
// its text (only `depends:` lines and hand-written ledger rows), yet
// `show` prints "tier 1" and "tier 2" derived from those depends lines.
#[test]
fn show_prints_tiers_states_stands_on_and_chosen_sentence() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/tiered.md", TIERED_RECORD);
    append_ledger_row(&dir, "asked", "id-root", "h-root", "1.1 Root?");
    append_ledger_row(&dir, "answered", "id-root", "h-root", "A");
    append_ledger_row(&dir, "asked", "id-open", "h-open", "2.1 Depends on root?");
    append_ledger_row(&dir, "asked", "id-refused", "h-refused", "2.2 Also depends?");
    append_ledger_row(&dir, "refused", "id-refused", "h-refused", "2.2 Also depends?");

    let out = adr(&dir, &["show", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);

    let expected = "\
# Tiered Topic
tier 1
  [answered A] 1.1 Root?
    First choice.
  [unasked] 1.2 Unasked root?
  [skeleton] 1.3 Skeleton?
tier 2
  [open] 2.1 Depends on root?
    stands on: 1.1 Root?=A
  [refused] 2.2 Also depends?
    stands on: 1.1 Root?=A
";
    assert_eq!(shown, expected, "full output: {}", shown);
}

/// A single-file argument shows only that file, as before.
#[test]
fn show_with_a_file_argument_shows_only_that_file() {
    let dir = init_repo();
    let one = write_record(&dir, "adrdom/decisions/one.md", TIERED_RECORD);
    let two_content = TIERED_RECORD.replace("Tiered Topic", "Second Topic");
    write_record(&dir, "adrdom/decisions/two.md", &two_content);

    let out = adr(&dir, &["show", one.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);
    assert!(shown.contains("# Tiered Topic"), "{}", shown);
    assert!(!shown.contains("# Second Topic"), "{}", shown);
}
