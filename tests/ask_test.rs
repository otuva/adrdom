mod common;

use common::*;

#[test]
fn example_record_round_trips_and_ask_assigns_ids() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    assert_eq!(
        ids.len(),
        2,
        "expected an id for the question and the principle: {}",
        content
    );
    assert_ne!(ids[0], ids[1]);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_rows: Vec<&str> = ledger.lines().filter(|l| l.contains("\tasked\t")).collect();
    assert_eq!(asked_rows.len(), 2, "ledger: {}", ledger);
    for id in &ids {
        assert!(asked_rows.iter().any(|r| r.contains(id.as_str())));
    }

    // Running ask again with nothing changed should not add more rows.
    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success());
    let ledger2 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger, ledger2,
        "re-running ask with no changes should be a no-op"
    );
}

#[test]
fn ask_reasks_on_edit() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_before = ledger_before
        .lines()
        .filter(|l| l.contains("\tasked\t"))
        .count();

    // Edit the question's Problem text (inside the hashed range, before any Answer).
    let content = std::fs::read_to_string(&record).unwrap();
    let edited = content.replace(
        "names leaked into decisions.",
        "names leaked into decisions, badly.",
    );
    std::fs::write(&record, edited).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));
    assert!(
        stdout(&out).contains("re-asked"),
        "expected a re-ask in output: {}",
        stdout(&out)
    );

    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_after = ledger_after
        .lines()
        .filter(|l| l.contains("\tasked\t"))
        .count();
    assert_eq!(
        asked_after,
        asked_before + 1,
        "expected exactly one new asked row"
    );
}

#[test]
fn ask_refuses_to_edit_an_answered_section() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    let question_id = &ids[0];

    let lock_out = adr_owner(&dir, &["lock", question_id, "A"], "");
    assert!(
        lock_out.status.success(),
        "lock failed: {}",
        stderr(&lock_out)
    );

    // Now edit the answered question's Problem text.
    let content = std::fs::read_to_string(&record).unwrap();
    let edited = content.replace("names leaked into decisions.", "something else entirely.");
    std::fs::write(&record, edited.clone()).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        !out.status.success(),
        "ask should have refused: {}",
        stdout(&out)
    );
    assert_eq!(out.status.code(), Some(1));

    // Nothing should have changed: the file is exactly what we wrote.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after, edited,
        "ask must not modify the file when it refuses"
    );
}

#[test]
fn ask_leaves_a_skeleton_heading_without_id_or_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let content = format!("{}\n### 1.2 Where does the plan live?\n", EXAMPLE_RECORD);
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));
    let after = std::fs::read_to_string(&record).unwrap();
    let idx = after.find("### 1.2 Where does the plan live?").unwrap();
    assert!(
        !after[idx..].contains("id: "),
        "skeleton heading got an id: {}",
        &after[idx..]
    );
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.contains("Where does the plan live?"));
}

// --- terms wiring (3): missing/shape/approval refusals, banned, retired,
// zero pairings, and the plural terms-line write --------------------------

const AUTHORITY_QUESTION: &str = r#"# Example

## Group

### 1.1 What is the unit of authority called?

Problem: names leaked into decisions.

Standard: plain words.

- A) The authoritative sentence is a decision.
  Why: plainest word.
- B) The whole unit is a block.
  Why: matches predecessors.
"#;

#[test]
fn ask_refuses_when_terms_md_is_missing() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/terms.md missing; agent writes it, owner runs: adr approve adrdom/terms.md"
    );

    // Nothing written.
    let content = std::fs::read_to_string(&record).unwrap();
    assert_eq!(content, AUTHORITY_QUESTION);
}

#[test]
fn ask_refuses_on_a_terms_md_shape_error() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    // Missing the "# Terms" h1: a shape error.
    write_record(
        &dir,
        "adrdom/terms.md",
        "## authority\nid: 11111111-1111-1111-1111-111111111111\ndefinition: x.\naliases: \n",
    );

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.starts_with("ask: adrdom/terms.md:"), "{}", err);
    assert!(err.contains("missing h1"), "{}", err);
}

#[test]
fn ask_refuses_on_a_banned_word_naming_the_word_and_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    // Ban the word before it appears anywhere: `ban` itself refuses an
    // already-in-use word, so the record has to come after.
    assert!(adr(&dir, &["ban", "--term", "authority"], &[], "predecessors\n")
        .status
        .success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("banned word \"predecessors\" (term authority)"),
        "{}",
        err
    );
}

#[test]
fn ask_refuses_on_a_banned_word_with_no_term_name() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    assert!(adr(&dir, &["ban"], &[], "predecessors\n").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("banned word \"predecessors\""), "{}", err);
    assert!(!err.contains("(term"), "{}", err);
}

#[test]
fn ask_refuses_a_retired_match() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    let terms = "# Terms\n\n## authority\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A fixture term.\naliases: \nretired: yes\n";
    write_record(&dir, "adrdom/terms.md", terms);
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let hash = {
        // Mirror terms::file_hash for this fixed content.
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(terms.as_bytes());
        h.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>()
    };
    let mut existing = std::fs::read_to_string(&ledger_path).unwrap();
    existing.push_str(&format!("2024-01-01T00:00:00Z\tterms\t-\t{}\tadrdom/terms.md\n", hash));
    std::fs::write(&ledger_path, existing).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("resolves \"authority\" to retired term \"authority\""),
        "{}",
        err
    );
}

#[test]
fn ask_refuses_a_candidate_that_pairs_no_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let content = "# Example\n\n## Group\n\n### 1.1 Does this pass?\n\nProblem: whether it passes.\n\nStandard: clear yes or no.\n\n- A) Yes it passes.\n  Why: reasons.\n- B) No it does not.\n  Why: other reasons.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", content);
    approve_minimal_terms(&dir, "authority");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Does this pass?\" pairs no term"),
        "{}",
        err
    );
    assert!(err.contains("adds the term to adrdom/terms.md"), "{}", err);

    // Nothing written.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, content);
}

#[test]
fn ask_writes_the_terms_line_with_a_plural_word() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_minimal_terms(&dir, "decision");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));

    let content = std::fs::read_to_string(&record).unwrap();
    assert!(
        content
            .lines()
            .any(|l| l.starts_with("terms: ") && l.contains("decisions=99999999-9999-9999-9999-999999999999")),
        "expected a plural match on the terms line: {}",
        content
    );
}
