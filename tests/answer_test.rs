mod common;

use common::*;
use std::path::Path;

// 9.1 A: every heading needs a `depends:` line for the real `adr approve`
// these tests now go through (see `approve_record` in tests/common/mod.rs);
// `depends: authority` reuses the term `approve_minimal_terms(&dir,
// "authority")` sets up in every test below.
const FIRST_QUESTION: &str = r#"# Example

### 1.1 What is the unit of authority called?
depends: authority

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
depends: authority

Problem: naming the second unit.

Standard: plain words.

- A) The second thing is a widget.
  Why: reasons.
  Costs: costs.
- B) The second thing is a gadget.
  Why: other reasons.
  Costs: other costs.
"#;

const THIRD_QUESTION: &str = r#"
### 1.3 What is the third unit of authority called?
depends: authority

Problem: naming the third unit.

Standard: plain words.

- A) The third thing is a gizmo.
  Why: reasons.
  Costs: costs.
- B) The third thing is a doohickey.
  Why: other reasons.
  Costs: other costs.
"#;

#[test]
fn answer_writes_answer_line_and_ledger_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    let question_id = &ids[0];

    let out = adr_owner(&dir, &["answer", question_id, "A"], "");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
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
fn answer_refuses_without_an_interactive_terminal() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    let question_id = &ids[0];

    // No owner bypass: stdin is piped (never a TTY) in this harness, so the
    // gate (9.2 A: stdin is an interactive terminal, no env var) refuses
    // with exit code 2 and nothing should be written.
    let out = adr(&dir, &["answer", question_id, "A"], &[], "");
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("this is the owner's command"),
        "stderr: {}",
        stderr(&out)
    );

    let after = std::fs::read_to_string(&record).unwrap();
    assert!(
        !after.contains("**Answer:**"),
        "answer must not have written anything: {}",
        after
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.contains("\tanswered\t"),
        "answer must not have appended a ledger row: {}",
        ledger
    );
}

#[test]
fn answer_with_supersedes_writes_superseded_row_and_show_hides_old() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    let old_id = ids[0].clone();

    let answer_out = adr_owner(&dir, &["answer", &old_id, "A"], "");
    assert!(
        answer_out.status.success(),
        "answer A failed: {}",
        stderr(&answer_out)
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
    approve_record(&dir, "adrdom/decisions/example.md");

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

    let answer_out = adr_owner(&dir, &["answer", &new_id, "A"], "");
    assert!(
        answer_out.status.success(),
        "answer B failed: {}",
        stderr(&answer_out)
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
fn answer_with_two_id_supersedes_writes_a_row_per_id() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id_1 = extract_ids(&content)[0].clone();
    let answer_out = adr_owner(&dir, &["answer", &old_id_1, "A"], "");
    assert!(answer_out.status.success(), "answer 1.1 failed: {}", stderr(&answer_out));

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(SECOND_QUESTION);
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id_2 = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id_1)
        .expect("second id assigned");
    let answer_out = adr_owner(&dir, &["answer", &old_id_2, "A"], "");
    assert!(answer_out.status.success(), "answer 1.2 failed: {}", stderr(&answer_out));

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
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id_1 && id != &old_id_2)
        .expect("third id assigned");

    let answer_out = adr_owner(&dir, &["answer", &new_id, "A"], "");
    assert!(answer_out.status.success(), "answer 1.3 failed: {}", stderr(&answer_out));

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
fn answer_refuses_a_second_time_with_already_answered_message() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let question_id = extract_ids(&content)[0].clone();

    let first = adr_owner(&dir, &["answer", &question_id, "A"], "");
    assert!(first.status.success(), "first answer failed: {}", stderr(&first));

    // Answering the same, now-answered id again must refuse with the same
    // "already answered" message the pre-refactor command produced, and
    // must not append a second `answered` row.
    let second = adr_owner(&dir, &["answer", &question_id, "A"], "");
    assert!(!second.status.success(), "second answer should have refused");
    assert!(
        stderr(&second).contains("is already answered"),
        "{}",
        stderr(&second)
    );
    assert!(
        stderr(&second).starts_with("answer: "),
        "refusal should keep the answer: prefix: {}",
        stderr(&second)
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger.lines().filter(|l| l.contains("\tanswered\t")).count(),
        1,
        "second answer must not have written another answered row: {}",
        ledger
    );
}

#[test]
fn answer_with_two_id_supersedes_refuses_when_one_is_not_current() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id_1 = extract_ids(&content)[0].clone();
    let answer_out = adr_owner(&dir, &["answer", &old_id_1, "A"], "");
    assert!(answer_out.status.success(), "answer 1.1 failed: {}", stderr(&answer_out));

    // 1.2 is asked but never answered, so it is not current.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(SECOND_QUESTION);
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
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
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id_1 && id != &old_id_2)
        .expect("third id assigned");

    let answer_out = adr_owner(&dir, &["answer", &new_id, "A"], "");
    assert!(
        !answer_out.status.success(),
        "answer should have refused: {}",
        stdout(&answer_out)
    );
    assert!(stderr(&answer_out).contains(old_id_2.as_str()), "{}", stderr(&answer_out));

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after.matches("**Answer:**").count(),
        1,
        "answer must not have written an answer for 1.3: {}",
        after
    );
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tsuperseded\t")),
        "answer must not have written any superseded rows: {}",
        ledger
    );
}

#[test]
fn answer_refuses_a_supersedes_target_already_superseded() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&content)[0].clone();
    let answer_out = adr_owner(&dir, &["answer", &old_id, "A"], "");
    assert!(answer_out.status.success(), "answer 1.1 failed: {}", stderr(&answer_out));

    // The second question supersedes the first and is answered: the first
    // stops being current here.
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
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let second_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id)
        .expect("second id assigned");
    let answer_out = adr_owner(&dir, &["answer", &second_id, "A"], "");
    assert!(answer_out.status.success(), "answer 1.2 failed: {}", stderr(&answer_out));

    // The third question names the same, now superseded, first id: a
    // supersedes target must be current (4.14 A), so this refuses instead
    // of being accepted as lineage.
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
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let third_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id && id != &second_id)
        .expect("third id assigned");
    let answer_out = adr_owner(&dir, &["answer", &third_id, "A"], "");
    assert!(
        !answer_out.status.success(),
        "answer should have refused: {}",
        stdout(&answer_out)
    );
    let err = stderr(&answer_out);
    assert!(err.contains(old_id.as_str()), "{}", err);
    assert!(err.contains("is not a current decision"), "{}", err);

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after.matches("**Answer:**").count(),
        2,
        "answer must not have written an answer for 1.3: {}",
        after
    );
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let rows = ledger
        .lines()
        .filter(|l| l.contains("\tsuperseded\t") && l.contains(old_id.as_str()))
        .count();
    assert_eq!(rows, 1, "expected exactly one superseded row for the first id: {}", ledger);
}

#[test]
fn answer_accepts_a_supersedes_target_that_is_current() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&content)[0].clone();
    let answer_out = adr_owner(&dir, &["answer", &old_id, "A"], "");
    assert!(answer_out.status.success(), "answer 1.1 failed: {}", stderr(&answer_out));

    // The second question supersedes the first while it is still current
    // (latest state `answered`): accepted.
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
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let second_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id)
        .expect("second id assigned");
    let answer_out = adr_owner(&dir, &["answer", &second_id, "A"], "");
    assert!(answer_out.status.success(), "answer 1.2 failed: {}", stderr(&answer_out));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger
            .lines()
            .any(|l| l.contains("\tsuperseded\t") && l.contains(old_id.as_str())),
        "expected a superseded row for the first id: {}",
        ledger
    );
}

// --- answer's own terms.md gates (6) ---------------------------------------

#[test]
fn answer_refuses_when_terms_md_is_missing() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // Remove terms.md entirely after the id was asked.
    std::fs::remove_file(dir.join("adrdom/terms.md")).unwrap();

    let out = adr_owner(&dir, &["answer", &id, "A"], "");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        "answer: adrdom/terms.md missing; agent writes it, owner runs: adr approve adrdom/terms.md"
    );
}

#[test]
fn answer_refuses_when_terms_md_has_a_shape_error() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // Corrupt terms.md with an unknown line after it was approved.
    let mut terms = std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap();
    terms.push_str("notes: extra\n");
    std::fs::write(dir.join("adrdom/terms.md"), terms).unwrap();

    let out = adr_owner(&dir, &["answer", &id, "A"], "");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.starts_with("answer: adrdom/terms.md:"), "{}", err);
    assert!(err.contains("unknown line"), "{}", err);
}

#[test]
fn answer_refuses_when_terms_md_is_not_approved() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
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

    let out = adr_owner(&dir, &["answer", &id, "A"], "");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("terms not approved"), "{}", err);
    assert!(err.contains("adr approve adrdom/terms.md"), "{}", err);
}

#[test]
fn answer_refuses_a_section_with_no_terms_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let content = "# Topic\n\n### 1.1 What is the authority?\ndepends: authority\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n  Costs: z.\n- B) w.\n  Why: y.\n  Costs: z.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let asked = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&asked)[0].clone();

    // Strip the `terms:` line ask just wrote, restoring the section's text
    // to exactly what it was before that line existed (`approve`'s id
    // insertion leaves two blank lines where the original single blank
    // line was, so this is derived from the real file rather than
    // hand-assembled), and hand-write a matching `asked` row for that
    // text so answer sees the id in the asked state at its current hash,
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

    let out = adr_owner(&dir, &["answer", &id, "A"], "");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        format!("answer: {}: no terms line; agent re-asks", id)
    );
}

/// Replaces the `terms: ` line `ask` wrote with `new_terms_line`, then
/// hand-writes a matching `asked` ledger row for the section's new hash
/// (mirrors `answer_refuses_a_section_with_no_terms_line`'s approach, but
/// keeps the terms line present instead of removing it).
fn corrupt_terms_line(dir: &Path, record: &Path, heading: &str, new_terms_line: &str) {
    let asked = std::fs::read_to_string(record).unwrap();
    let id = extract_ids(&asked)[0].clone();
    let edited: String = asked
        .lines()
        .map(|l| if l.starts_with("terms: ") { new_terms_line } else { l })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(record, &edited).unwrap();

    let heading_idx = edited.find(heading).unwrap();
    let section_text = edited[heading_idx..].trim_end_matches('\n');
    let new_hash = mirror_section_hash(section_text);
    let detail = heading.trim_start_matches("### ");
    append_ledger_row(dir, "asked", &id, &new_hash, detail);
}

#[test]
fn answer_refuses_a_malformed_terms_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // `terms: authority` with no `=uuid` at all: `parse_terms_line` rejects it.
    corrupt_terms_line(
        &dir,
        &record,
        "### 1.1 What is the unit of authority called?",
        "terms: authority",
    );

    let out = adr_owner(&dir, &["answer", &id, "A"], "");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        format!("answer: {}: malformed terms line; agent re-asks", id)
    );
}

#[test]
fn answer_refuses_a_terms_line_naming_a_nonexistent_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // A well-formed pairing, but naming a uuid no term in terms.md has.
    corrupt_terms_line(
        &dir,
        &record,
        "### 1.1 What is the unit of authority called?",
        "terms: authority=00000000-0000-0000-0000-000000000000",
    );

    let out = adr_owner(&dir, &["answer", &id, "A"], "");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        format!(
            "answer: {}: terms line names term 00000000-0000-0000-0000-000000000000 that no longer exists; agent re-asks",
            id
        )
    );
}

#[test]
fn answer_refuses_a_terms_line_naming_a_retired_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // Retire the term the record's terms line already names, and
    // hand-approve that retired terms.md content: the record's terms line
    // and its ledger `asked` row are untouched, only terms.md changes.
    let retired_terms =
        "# Terms\n\n## authority\nid: 99999999-9999-9999-9999-999999999999\ndefinition: A fixture term for tests.\naliases: \nretired: yes\n";
    std::fs::write(dir.join("adrdom/terms.md"), retired_terms).unwrap();
    approve_terms_via_ledger(&dir, retired_terms);

    let out = adr_owner(&dir, &["answer", &id, "A"], "");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        format!(
            "answer: {}: terms line names retired term \"authority\"; agent re-asks",
            id
        )
    );
}
