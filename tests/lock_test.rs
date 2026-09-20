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
fn lock_with_two_id_supersedes_refuses_when_one_is_not_in_force() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
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
