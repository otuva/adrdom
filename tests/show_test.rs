mod common;

use common::*;

const RECORD: &str = r#"# Example

## Group

### 1.1 Is this current?

Problem: whether it is current.

Standard: clear yes or no.

- A) Yes it is current.
  Why: reasons.
- B) No it is not.
  Why: other reasons.

### 1.2 Is this only asked?

Problem: whether it is only asked.

Standard: clear yes or no.

- A) Yes only asked.
  Why: reasons.
- B) No.
  Why: other reasons.
"#;

#[test]
fn show_prints_only_current_sentences() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "reasons");
    let record = write_record(&dir, "adrdom/decisions/example.md", RECORD);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", RECORD);
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
    assert!(out.status.success());
    let shown = stdout(&out);

    assert!(shown.contains("1.1 Is this current?"));
    assert!(shown.contains("Yes it is current."));
    assert!(!shown.contains("1.2 Is this only asked?"));
    assert!(!shown.contains("Yes only asked."));
}
