mod common;

use common::*;

const ONE_QUESTION: &str = r#"# Example

## Group

### 5.1 Does this pass?

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
- B) No it does not.
  Why: other reasons.
"#;

#[test]
fn ledger_append_only_violation_is_detected_after_a_commit() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = "# Terms\n\n## pass\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A clear yes.\naliases: passes\n";
    std::fs::write(dir.join("adrdom/terms.md"), terms).unwrap();
    approve_terms_via_ledger(&dir, terms);
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    // Commit the ledger as it stands.
    run_git(&dir, &["add", "adrdom"]);
    run_git(&dir, &["commit", "-q", "-m", "ask"]);

    // A clean check should still pass the ledger check (no divergence yet).
    let out = adr(&dir, &["check"], &[], "");
    let out_text = stdout(&out);
    assert!(
        !out_text.contains("ledger:"),
        "unexpected ledger failure before tampering: {}",
        out_text
    );

    // Now tamper with an already-committed line instead of only appending.
    // Change the detail column so the row stays well-formed (still a valid
    // `asked` row with a real hash) but the working copy is no longer a
    // byte-for-byte prefix match of what's on HEAD.
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let content = std::fs::read_to_string(&ledger_path).unwrap();
    let mut lines: Vec<&str> = content.lines().collect();
    let asked_idx = lines
        .iter()
        .position(|l| l.contains("\tasked\t"))
        .expect("an asked row");
    let tampered = lines[asked_idx].replace("Does this pass?", "rewritten after the fact");
    lines[asked_idx] = &tampered;
    let new_content = lines.join("\n") + "\n";
    std::fs::write(&ledger_path, new_content).unwrap();

    let out2 = adr(&dir, &["check"], &[], "");
    assert!(!out2.status.success());
    let out2_text = stdout(&out2);
    assert!(out2_text.contains("ledger:"), "{}", out2_text);
    assert!(out2_text.contains("not append-only"), "{}", out2_text);
}

#[test]
fn ledger_accepts_new_event_kinds() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let mut content = std::fs::read_to_string(&ledger_path).unwrap();
    for event in ["withdrawn", "approved", "context", "terms", "unbanned"] {
        content.push_str(&format!(
            "2024-01-01T00:00:00Z\t{}\t-\t-\tsomedetail\n",
            event
        ));
    }
    std::fs::write(&ledger_path, content).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    let out_text = stdout(&out);
    assert!(
        !out_text.contains("malformed row"),
        "new event kinds should not be treated as malformed: {}",
        out_text
    );
}
