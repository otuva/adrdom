mod common;

use common::*;

const FIRST_QUESTION: &str = r#"# Example

## Group

### 1.1 What is the unit of authority called?

Problem: names leaked into decisions.

Standard: plain words.

- A) The authoritative sentence is a decision, and the question that produced it is a question.
  Why: plainest word.
  Costs: two nouns.
- B) The whole unit is a block.
  Why: matches predecessors.
  Costs: names a region.
"#;

const SECOND_QUESTION: &str = r#"
### 1.2 What is the second unit of authority called?

Problem: naming the second unit.

Standard: plain words.

- A) The second thing is a widget.
  Why: reasons.
- B) The second thing is a gadget.
  Why: other reasons.
"#;

const THIRD_QUESTION: &str = r#"
### 1.3 What is the third unit of authority called?

Problem: naming the third unit.

Standard: plain words.

- A) The third thing is a gizmo.
  Why: reasons.
- B) The third thing is a doohickey.
  Why: other reasons.
"#;

#[test]
fn lock_writes_answer_line_and_ledger_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    let question_id = &ids[0];

    let out = adr_owner(&dir, &["lock", question_id, "A"], "");
    assert!(out.status.success(), "lock failed: {}", stderr(&out));
    assert!(stdout(&out).contains("The authoritative sentence is a decision"));

    let after = std::fs::read_to_string(&record).unwrap();
    assert!(after.contains("**Answer:** A"));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger
            .lines()
            .any(|l| l.contains("\tanswered\t") && l.contains(question_id.as_str())),
        "ledger missing answered row: {}",
        ledger
    );
}

#[test]
fn lock_refuses_when_claudecode_is_set() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    let question_id = &ids[0];

    // CLAUDECODE set, no bypass: must be refused with exit code 2, and
    // nothing should be written.
    let out = adr(
        &dir,
        &["lock", question_id, "A"],
        &[("CLAUDECODE", "1")],
        "",
    );
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));

    let after = std::fs::read_to_string(&record).unwrap();
    assert!(
        !after.contains("**Answer:**"),
        "lock must not have written anything: {}",
        after
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.contains("\tanswered\t"),
        "lock must not have appended a ledger row: {}",
        ledger
    );
}

#[test]
fn lock_with_supersedes_writes_superseded_row_and_show_hides_old() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    let old_id = ids[0].clone();

    let lock_out = adr_owner(&dir, &["lock", &old_id, "A"], "");
    assert!(
        lock_out.status.success(),
        "lock A failed: {}",
        stderr(&lock_out)
    );

    // Add a second question that supersedes the first, with `supersedes:`
    // directly under its heading.
    let mut content = std::fs::read_to_string(&record).unwrap();
    let second_with_supersedes = SECOND_QUESTION.replacen(
        "### 1.2 What is the second unit of authority called?\n",
        &format!(
            "### 1.2 What is the second unit of authority called?\n\nsupersedes: {}\n",
            old_id
        ),
        1,
    );
    content.push_str(&second_with_supersedes);
    std::fs::write(&record, &content).unwrap();
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);

    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    let new_id = ids
        .iter()
        .find(|id| *id != &old_id)
        .expect("new id assigned")
        .clone();

    let lock_out = adr_owner(&dir, &["lock", &new_id, "A"], "");
    assert!(
        lock_out.status.success(),
        "lock B failed: {}",
        stderr(&lock_out)
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger
            .lines()
            .any(|l| l.contains("\tsuperseded\t") && l.contains(old_id.as_str())),
        "ledger missing superseded row: {}",
        ledger
    );

    let show = adr(&dir, &["show"], &[], "");
    assert!(show.status.success());
    let shown = stdout(&show);
    assert!(
        !shown.contains("The authoritative sentence is a decision"),
        "superseded sentence still shown: {}",
        shown
    );
    assert!(
        shown.contains("The second thing is a widget."),
        "new sentence missing: {}",
        shown
    );
}

#[test]
fn lock_with_two_id_supersedes_writes_a_row_per_id() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id_1 = extract_ids(&content)[0].clone();
    let lock_out = adr_owner(&dir, &["lock", &old_id_1, "A"], "");
    assert!(lock_out.status.success(), "lock 1.1 failed: {}", stderr(&lock_out));

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(SECOND_QUESTION);
    std::fs::write(&record, &content).unwrap();
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id_2 = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id_1)
        .expect("second id assigned");
    let lock_out = adr_owner(&dir, &["lock", &old_id_2, "A"], "");
    assert!(lock_out.status.success(), "lock 1.2 failed: {}", stderr(&lock_out));

    // Add a third question that supersedes both prior ids at once,
    // comma-separated on a single `supersedes:` line (4.14).
    let mut content = std::fs::read_to_string(&record).unwrap();
    let third_with_supersedes = THIRD_QUESTION.replacen(
        "### 1.3 What is the third unit of authority called?\n",
        &format!(
            "### 1.3 What is the third unit of authority called?\n\nsupersedes: {}, {}\n",
            old_id_1, old_id_2
        ),
        1,
    );
    content.push_str(&third_with_supersedes);
    std::fs::write(&record, &content).unwrap();
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id_1 && id != &old_id_2)
        .expect("third id assigned");

    let lock_out = adr_owner(&dir, &["lock", &new_id, "A"], "");
    assert!(lock_out.status.success(), "lock 1.3 failed: {}", stderr(&lock_out));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    for old in [&old_id_1, &old_id_2] {
        assert!(
            ledger
                .lines()
                .any(|l| l.contains("\tsuperseded\t") && l.contains(old.as_str())),
            "ledger missing superseded row for {}: {}",
            old,
            ledger
        );
    }
}

#[test]
fn lock_refuses_a_second_time_with_already_answered_message() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let question_id = extract_ids(&content)[0].clone();

    let first = adr_owner(&dir, &["lock", &question_id, "A"], "");
    assert!(first.status.success(), "first lock failed: {}", stderr(&first));

    // Locking the same, now-answered id again must refuse with the same
    // "already answered" message the pre-refactor command produced, and
    // must not append a second `answered` row.
    let second = adr_owner(&dir, &["lock", &question_id, "A"], "");
    assert!(!second.status.success(), "second lock should have refused");
    assert!(
        stderr(&second).contains("is already answered"),
        "{}",
        stderr(&second)
    );
    assert!(
        stderr(&second).starts_with("lock: "),
        "refusal should keep the lock: prefix: {}",
        stderr(&second)
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger.lines().filter(|l| l.contains("\tanswered\t")).count(),
        1,
        "second lock must not have written another answered row: {}",
        ledger
    );
}

#[test]
fn lock_with_two_id_supersedes_refuses_when_one_is_not_in_force() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id_1 = extract_ids(&content)[0].clone();
    let lock_out = adr_owner(&dir, &["lock", &old_id_1, "A"], "");
    assert!(lock_out.status.success(), "lock 1.1 failed: {}", stderr(&lock_out));

    // 1.2 is asked but never locked, so it is not in force.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(SECOND_QUESTION);
    std::fs::write(&record, &content).unwrap();
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id_2 = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id_1)
        .expect("second id assigned");

    let mut content = std::fs::read_to_string(&record).unwrap();
    let third_with_supersedes = THIRD_QUESTION.replacen(
        "### 1.3 What is the third unit of authority called?\n",
        &format!(
            "### 1.3 What is the third unit of authority called?\n\nsupersedes: {}, {}\n",
            old_id_1, old_id_2
        ),
        1,
    );
    content.push_str(&third_with_supersedes);
    std::fs::write(&record, &content).unwrap();
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id_1 && id != &old_id_2)
        .expect("third id assigned");

    let lock_out = adr_owner(&dir, &["lock", &new_id, "A"], "");
    assert!(
        !lock_out.status.success(),
        "lock should have refused: {}",
        stdout(&lock_out)
    );
    assert!(stderr(&lock_out).contains(old_id_2.as_str()), "{}", stderr(&lock_out));

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after.matches("**Answer:**").count(),
        1,
        "lock must not have written an answer for 1.3: {}",
        after
    );
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tsuperseded\t")),
        "lock must not have written any superseded rows: {}",
        ledger
    );
}

#[test]
fn lock_accepts_a_supersedes_target_already_superseded_as_lineage() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&content)[0].clone();
    let lock_out = adr_owner(&dir, &["lock", &old_id, "A"], "");
    assert!(lock_out.status.success(), "lock 1.1 failed: {}", stderr(&lock_out));

    // The second question supersedes the first and is locked: the first
    // leaves force here.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&SECOND_QUESTION.replacen(
        "### 1.2 What is the second unit of authority called?\n",
        &format!(
            "### 1.2 What is the second unit of authority called?\n\nsupersedes: {}\n",
            old_id
        ),
        1,
    ));
    std::fs::write(&record, &content).unwrap();
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let second_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id)
        .expect("second id assigned");
    let lock_out = adr_owner(&dir, &["lock", &second_id, "A"], "");
    assert!(lock_out.status.success(), "lock 1.2 failed: {}", stderr(&lock_out));

    // The third question names the same, now superseded, first id: the
    // lock is accepted as lineage and writes a second superseded row.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&THIRD_QUESTION.replacen(
        "### 1.3 What is the third unit of authority called?\n",
        &format!(
            "### 1.3 What is the third unit of authority called?\n\nsupersedes: {}\n",
            old_id
        ),
        1,
    ));
    std::fs::write(&record, &content).unwrap();
    approve_via_ledger(&dir, "adrdom/decisions/example.md", &content);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let third_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id && id != &second_id)
        .expect("third id assigned");
    let lock_out = adr_owner(&dir, &["lock", &third_id, "A"], "");
    assert!(lock_out.status.success(), "lineage lock failed: {}", stderr(&lock_out));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let rows = ledger
        .lines()
        .filter(|l| l.contains("\tsuperseded\t") && l.contains(old_id.as_str()))
        .count();
    assert_eq!(rows, 2, "expected two superseded rows for the first id: {}", ledger);
}

// --- lock's own terms.md gates (6) ---------------------------------------

#[test]
fn lock_refuses_when_terms_md_is_missing() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // Remove terms.md entirely after the id was asked.
    std::fs::remove_file(dir.join("adrdom/terms.md")).unwrap();

    let out = adr_owner(&dir, &["lock", &id, "A"], "");
    assert!(!out.status.success(), "lock should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        "lock: adrdom/terms.md missing; agent writes it, owner runs: adr approve adrdom/terms.md"
    );
}

#[test]
fn lock_refuses_when_terms_md_has_a_shape_error() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // Corrupt terms.md with an unknown line after it was approved.
    let mut terms = std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap();
    terms.push_str("notes: extra\n");
    std::fs::write(dir.join("adrdom/terms.md"), terms).unwrap();

    let out = adr_owner(&dir, &["lock", &id, "A"], "");
    assert!(!out.status.success(), "lock should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.starts_with("lock: adrdom/terms.md:"), "{}", err);
    assert!(err.contains("unknown line"), "{}", err);
}

#[test]
fn lock_refuses_when_terms_md_is_not_approved() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // Edit terms.md's definition without re-approving it.
    let terms_path = dir.join("adrdom/terms.md");
    let terms = std::fs::read_to_string(&terms_path).unwrap();
    let edited = terms.replace("A fixture term for tests.", "A fixture term, edited.");
    std::fs::write(&terms_path, edited).unwrap();

    let out = adr_owner(&dir, &["lock", &id, "A"], "");
    assert!(!out.status.success(), "lock should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("terms not approved"), "{}", err);
    assert!(err.contains("adr approve adrdom/terms.md"), "{}", err);
}

#[test]
fn lock_refuses_a_section_with_no_terms_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let content = "# Topic\n\n## Group\n\n### 1.1 What is the authority?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", content);
    approve_minimal_terms(&dir, "authority");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let asked = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&asked)[0].clone();

    // Strip the `terms:` line ask just wrote, restoring the section's text
    // to exactly what it was before that line existed (`ask`'s id
    // insertion leaves two blank lines where the original single blank
    // line was, so this is derived from the real file rather than
    // hand-assembled), and hand-write a matching `asked` row for that
    // text so lock sees the id in the asked state at its current hash,
    // but with no terms line at all.
    let without_terms: String = asked
        .lines()
        .filter(|l| !l.starts_with("terms: "))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(&record, &without_terms).unwrap();

    let heading_idx = without_terms.find("### 1.1 What is the authority?").unwrap();
    let section_text = without_terms[heading_idx..].trim_end_matches('\n');
    let new_hash = mirror_section_hash(section_text);
    append_ledger_row(&dir, "asked", &id, &new_hash, "1.1 What is the authority?");

    let out = adr_owner(&dir, &["lock", &id, "A"], "");
    assert!(!out.status.success(), "lock should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        format!("lock: {}: no terms line; agent re-asks", id)
    );
}
