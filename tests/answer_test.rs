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

    let out = adr_answer(&dir, question_id, "A");
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

// `answer` is an owner command; this test runs it the way an agent runs it
// -- no interactive terminal, through the plain `adr(...)` helper rather
// than `adr_owner` -- and shows it is refused and writes nothing (no
// `**Answer:**` line, no `answered` ledger row).
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

    let answer_out = adr_answer(&dir, &old_id, "A");
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

    let answer_out = adr_answer(&dir, &new_id, "A");
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

// adr: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d
// adr: cd83e277-ebf0-4458-a77a-c526ab4cfeca
// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// core (0): "an answered decision never changes; it leaves force only when
// a newer decision supersedes it". Shows both halves: (1) editing an
// answered question's text makes `check` fail on it, naming the failure
// with the "text changed since being answered" message and pointing the
// agent at restoring it from git or superseding it -- never at re-asking
// it (which can't clear an answered question) and never at removing the
// answer line (which is genuine, not hand-written) -- and (2) a later
// answered question naming `supersedes: <old id>` is what takes the old
// one out of force -- `adr show` stops printing it, exactly as
// `answer_with_supersedes_writes_superseded_row_and_show_hides_old` already
// shows for the ledger/show side, but here reached through an edit that
// would otherwise have let the answered text drift.
#[test]
fn answered_question_text_is_immutable_until_a_later_decision_supersedes_it() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let original = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&original)[0].clone();

    let answer_out = adr_answer(&dir, &old_id, "A");
    assert!(
        answer_out.status.success(),
        "answer A failed: {}",
        stderr(&answer_out)
    );
    let answered = std::fs::read_to_string(&record).unwrap();

    // 1) An answered question's text never changes: editing its body (here,
    // the Problem line) after the answer makes `check` fail on it, even
    // though nothing touched the `**Answer:**` line itself.
    let tampered = answered.replacen(
        "Problem: names leaked into decisions.",
        "Problem: names leaked into decisions, reworded after the answer.",
        1,
    );
    assert_ne!(answered, tampered, "fixture edit should have changed something");
    std::fs::write(&record, &tampered).unwrap();

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(
        !check_out.status.success(),
        "check should fail on an answered question edited after the answer: {}",
        stdout(&check_out)
    );
    assert!(
        stdout(&check_out).contains("answers:"),
        "{}",
        stdout(&check_out)
    );
    assert!(
        stdout(&check_out).contains("text changed since being answered"),
        "{}",
        stdout(&check_out)
    );
    assert!(
        stdout(&check_out).contains(&format!(
            "agent restores it from git (e.g. git checkout -- adrdom/decisions/example.md), or writes a new question that supersedes {}",
            old_id
        )),
        "next move must be restore-from-git or supersede, never re-ask or remove the (genuine) answer line: {}",
        stdout(&check_out)
    );
    assert!(
        !stdout(&check_out).contains("answer line without ledger row"),
        "a genuine answer line must never be reported as hand-written: {}",
        stdout(&check_out)
    );
    assert!(
        !stdout(&check_out).contains("re-asks"),
        "an answered question can never be re-asked: {}",
        stdout(&check_out)
    );

    // Restore the answered text exactly as the owner answered it, then
    // check the second half: a later answered question naming
    // `supersedes: <old id>` is the only thing that takes the old decision
    // out of force.
    std::fs::write(&record, &answered).unwrap();
    let restored_check = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&restored_check).contains("answers:"),
        "the answers check should be silent once the answered text is restored: {}",
        stdout(&restored_check)
    );

    let mut content = answered.clone();
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
    let new_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id)
        .expect("new id assigned");

    let answer_out = adr_answer(&dir, &new_id, "A");
    assert!(
        answer_out.status.success(),
        "answer on the superseding question failed: {}",
        stderr(&answer_out)
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

// adr: f4f667ad-8714-4e65-897b-c7002e423c28
//
// "a supersedes line may name several decisions": the third question's
// single `supersedes:` line names both old_id_1 and old_id_2, and
// answering it writes one `superseded` row per named id.
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
    let answer_out = adr_answer(&dir, &old_id_1, "A");
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
    let answer_out = adr_answer(&dir, &old_id_2, "A");
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

    let answer_out = adr_answer(&dir, &new_id, "A");
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

    let first = adr_answer(&dir, &question_id, "A");
    assert!(first.status.success(), "first answer failed: {}", stderr(&first));

    // Answering the same, now-answered id again must refuse with the same
    // "already answered" message the pre-refactor command produced, and
    // must not append a second `answered` row.
    let second = adr_answer(&dir, &question_id, "A");
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

// (E3) A `supersedes:` naming two ids, one current and one merely asked
// (never answered, so never current at all): `ask` now refuses the
// candidate outright, since a supersedes target must be current the
// moment the question naming it is asked. This used to be caught by
// `answer`; the gate moved to `ask`.
//
// adr: f4f667ad-8714-4e65-897b-c7002e423c28
//
// "each target must be current when the question is asked (ask stops a
// non-current target)": 1.2 is only ever asked, never answered, so it is
// not current when 1.3's supersedes line names it -- `ask` refuses the
// whole candidate rather than letting the current target alone through.
#[test]
fn ask_refuses_a_two_id_supersedes_when_one_target_is_not_current() {
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
    let answer_out = adr_answer(&dir, &old_id_1, "A");
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
    let before_ask = std::fs::read_to_string(&record).unwrap();

    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        !ask_out.status.success(),
        "ask should have refused: {}",
        stdout(&ask_out)
    );
    let err = stderr(&ask_out);
    assert!(err.contains(old_id_2.as_str()), "{}", err);
    assert!(err.contains("which is not a current decision"), "{}", err);
    assert!(err.contains("agent fixes the supersedes line"), "{}", err);

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after, before_ask,
        "a refused ask must change nothing in the topic file"
    );
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tsuperseded\t")),
        "ask must not have written any superseded rows: {}",
        ledger
    );
}

// (E3) `ask` now refuses a candidate whose `supersedes:` names an id that
// is not current at ask time, so a target already stale by the time a
// question is asked can no longer be driven through a real `ask` call the
// way this test used to reach `answer`'s own refusal. That "stale at ask"
// path is covered directly against `ask` below
// (`ask_refuses_a_candidate_whose_supersedes_target_is_stale`). This test
// keeps its original meaning -- a supersedes target already superseded
// *before* the question naming it was asked still refuses at `answer` --
// by asking the third question for real without `supersedes:` (so `ask`'s
// new gate has nothing to refuse), then hand-editing the file to add the
// `supersedes:` line and hand-writing the matching `asked` ledger row
// after old_id's `superseded` row, the way
// `answer_refuses_a_section_with_no_terms_line` bypasses `ask` for a
// hand-crafted ledger state elsewhere in this file.
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
    let answer_out = adr_answer(&dir, &old_id, "A");
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
    let answer_out = adr_answer(&dir, &second_id, "A");
    assert!(answer_out.status.success(), "answer 1.2 failed: {}", stderr(&answer_out));

    // Add and really ask the third question with no `supersedes:` line yet
    // -- old_id is already superseded, so a real ask naming it here would
    // now be refused by `ask`'s own gate; that is exactly what the next
    // test below checks. This test is about `answer`'s side of the freeze.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(THIRD_QUESTION);
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

    // Hand-add the `supersedes:` line naming the already-superseded old_id,
    // and hand-write a matching `asked` row for the edited section -- its
    // position lands after old_id's existing `superseded` row, so the
    // target was superseded *before* this (hand-crafted) `asked` row, not
    // after.
    let edited = content.replacen(
        "### 1.3 What is the third unit of authority called?\n",
        &format!(
            "### 1.3 What is the third unit of authority called?\n\nsupersedes: {}\n",
            old_id
        ),
        1,
    );
    std::fs::write(&record, &edited).unwrap();
    let heading_idx = edited.find("### 1.3 What is the third unit of authority called?").unwrap();
    let section_text = edited[heading_idx..].trim_end_matches('\n');
    let new_hash = mirror_section_hash(section_text);
    append_ledger_row(
        &dir,
        "asked",
        &third_id,
        &new_hash,
        "1.3 What is the third unit of authority called?",
    );

    let answer_out = adr_answer(&dir, &third_id, "A");
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

// (E3) A split: two questions asked together (same `ask` call), both
// naming the same current target X in `supersedes:`. Answering one
// supersedes X; answering the other afterward must still be accepted,
// since X was current when both were asked -- the freeze is at ask time,
// not answer time -- and each answer writes its own `superseded` row for
// X (one per answer, never merged).
//
// adr: f4f667ad-8714-4e65-897b-c7002e423c28
//
// "several newer decisions may name the same older one ... the older one
// leaves force when the first replacement is answered (a second
// replacement asked before that is still accepted at answer)": second_id
// and third_id both name old_id and were both asked while it was still
// current; answering second_id first makes old_id leave force, and
// answering third_id afterward -- asked before that, at a time old_id was
// current -- is still accepted, not refused as stale.
#[test]
fn answer_accepts_a_split_where_two_questions_asked_together_name_the_same_target() {
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
    let answer_out = adr_answer(&dir, &old_id, "A");
    assert!(answer_out.status.success(), "answer 1.1 failed: {}", stderr(&answer_out));

    // Both the second and third questions supersede old_id while it is
    // still current, and both get asked in the same `ask` run.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&SECOND_QUESTION.replacen(
        "### 1.2 What is the second unit of authority called?\n",
        &format!(
            "### 1.2 What is the second unit of authority called?\n\nsupersedes: {}\n",
            old_id
        ),
        1,
    ));
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
    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(ask_out.status.success(), "ask failed: {}", stderr(&ask_out));

    let content = std::fs::read_to_string(&record).unwrap();
    let ids: Vec<String> = extract_ids(&content).into_iter().filter(|id| id != &old_id).collect();
    assert_eq!(ids.len(), 2, "expected the second and third ids: {:?}", ids);
    let second_id = ids[0].clone();
    let third_id = ids[1].clone();

    let first_answer = adr_answer(&dir, &second_id, "A");
    assert!(
        first_answer.status.success(),
        "first half of the split failed: {}",
        stderr(&first_answer)
    );
    let second_answer = adr_answer(&dir, &third_id, "A");
    assert!(
        second_answer.status.success(),
        "second half of the split failed: {}",
        stderr(&second_answer)
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let superseded_rows: Vec<&str> = ledger
        .lines()
        .filter(|l| l.contains("\tsuperseded\t") && l.contains(old_id.as_str()))
        .collect();
    assert_eq!(
        superseded_rows.len(),
        2,
        "expected one superseded row per answer naming old_id: {:?}",
        superseded_rows
    );
    assert!(superseded_rows.iter().any(|l| l.contains(second_id.as_str())));
    assert!(superseded_rows.iter().any(|l| l.contains(third_id.as_str())));
}

// (E3) A stale target: X is superseded, then a later question Z names X
// in `supersedes:`. `ask` refuses Z outright, since the target is not
// current at the moment Z is asked.
#[test]
fn ask_refuses_a_candidate_whose_supersedes_target_is_stale() {
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
    let answer_out = adr_answer(&dir, &old_id, "A");
    assert!(answer_out.status.success(), "answer 1.1 failed: {}", stderr(&answer_out));

    // The second question supersedes old_id and is answered: old_id is
    // now superseded, no longer current.
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
    let answer_out = adr_answer(&dir, &second_id, "A");
    assert!(answer_out.status.success(), "answer 1.2 failed: {}", stderr(&answer_out));

    // The third question names the already-superseded old_id: `ask`
    // refuses it directly, before it ever becomes askable.
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
    // Re-read: approve minted the third heading's id and rewrote its
    // depends entry, so this is the exact pre-ask content `ask` must leave
    // untouched below.
    let before_ask = std::fs::read_to_string(&record).unwrap();

    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        !ask_out.status.success(),
        "ask should have refused: {}",
        stdout(&ask_out)
    );
    let err = stderr(&ask_out);
    assert!(err.contains("1.3 What is the third unit of authority called?"), "{}", err);
    assert!(err.contains(old_id.as_str()), "{}", err);
    assert!(err.contains("which is not a current decision"), "{}", err);
    assert!(err.contains("agent fixes the supersedes line"), "{}", err);

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after, before_ask,
        "a refused ask must change nothing in the topic file"
    );
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tasked\t") && l.contains("1.3 What is the third unit of authority called?")),
        "ask must not have written an asked row for the refused third question: {}",
        ledger
    );
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
    let answer_out = adr_answer(&dir, &old_id, "A");
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
    let answer_out = adr_answer(&dir, &second_id, "A");
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

    let out = adr_answer(&dir, &id, "A");
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

    let out = adr_answer(&dir, &id, "A");
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

    let out = adr_answer(&dir, &id, "A");
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

    let out = adr_answer(&dir, &id, "A");
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

    let out = adr_answer(&dir, &id, "A");
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

    let out = adr_answer(&dir, &id, "A");
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

    let out = adr_answer(&dir, &id, "A");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        format!(
            "answer: {}: terms line names retired term \"authority\"; agent re-asks",
            id
        )
    );
}

// adr: aa275fac-def5-42a2-8a61-660142acad8e
// adr: 98d5d389-8315-41da-9970-666e31f3a94d
//
// The tool guards against an agent that runs an owner command because it
// looks helpful, not one that works to get past it: the plain agent run is
// stopped, and a run that opens a terminal on purpose goes through.
//
// "a sign of a person at the keyboard, an interactive terminal; without it
// the command stops, whatever the agent" is what every owner command
// (`approve`, `answer`, `unban`, `uninstall`) looks for before it runs.
// Shows both halves: with no interactive terminal on stdin -- and no agent
// env var set either, since the gate reads none of them -- every one of
// those commands refuses without writing anything, and the refusal names
// the command as the owner's, never the terminal it is actually keyed on;
// then, with a real interactive terminal (a pty allocated by `script`, no
// `ADR_OWNER_BYPASS` or test feature involved), the same gate lets
// `approve` run for real.
#[test]
fn owner_commands_refuse_without_a_tty_and_run_with_one() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = "# Example\n\n### 1.1 Q?\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    // No TTY (the test harness always pipes stdin) and CLAUDECODE cleared
    // (the `adr` helper already removes it): every owner command refuses,
    // exit code 2, and none of the refusals mention "terminal".
    let approve_out = adr(&dir, &["approve", record.to_str().unwrap()], &[], "");
    assert_eq!(approve_out.status.code(), Some(2), "{}", stderr(&approve_out));
    let approve_err = stderr(&approve_out);
    assert!(approve_err.contains("this is the owner's command"), "{}", approve_err);
    assert!(!approve_err.to_lowercase().contains("terminal"), "{}", approve_err);

    let answer_out = adr(
        &dir,
        &["answer", "11111111-1111-1111-1111-111111111111", "A"],
        &[],
        "",
    );
    assert_eq!(answer_out.status.code(), Some(2), "{}", stderr(&answer_out));
    let answer_err = stderr(&answer_out);
    assert!(answer_err.contains("this is the owner's command"), "{}", answer_err);
    assert!(!answer_err.to_lowercase().contains("terminal"), "{}", answer_err);

    let unban_out = adr(&dir, &["unban"], &[], "somebannedword\n");
    assert_eq!(unban_out.status.code(), Some(2), "{}", stderr(&unban_out));
    let unban_err = stderr(&unban_out);
    assert!(unban_err.contains("this is the owner's command"), "{}", unban_err);
    assert!(!unban_err.to_lowercase().contains("terminal"), "{}", unban_err);

    let uninstall_out = adr(&dir, &["uninstall"], &[], "");
    assert_eq!(uninstall_out.status.code(), Some(2), "{}", stderr(&uninstall_out));
    let uninstall_err = stderr(&uninstall_out);
    assert!(
        uninstall_err.contains("this is the owner's command"),
        "{}",
        uninstall_err
    );
    assert!(!uninstall_err.to_lowercase().contains("terminal"), "{}", uninstall_err);

    // Nothing any of the refused commands could have written landed.
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tapproved\t") || l.contains("\tcontext\t")),
        "no approve row should have been written on refusal: {}",
        ledger
    );

    // A real interactive terminal -- via `script`, which allocates a pty
    // for the child's stdin -- is not an agent env var and is not
    // `ADR_OWNER_BYPASS`: the same gate lets `approve` run for real. A
    // Linux machine without `script` fails the test, so this half is never
    // skipped where CI runs; other systems' `script` takes other flags.
    #[cfg(target_os = "linux")]
    {
        let cmd = format!(
            "env -u CLAUDECODE {} approve {}",
            adr_bin().display(),
            record.to_str().unwrap()
        );
        let out = std::process::Command::new("script")
            .current_dir(&dir)
            .args(["-q", "-e", "-c", &cmd, "/dev/null"])
            .output()
            .expect("`script` (util-linux) is needed to open a real terminal");
        assert!(
            out.status.success(),
            "approve should run under a real tty: stdout={} stderr={}",
            stdout(&out),
            stderr(&out)
        );
        assert!(
            stdout(&out).contains("approved"),
            "expected approve's own output under a real tty: {}",
            stdout(&out)
        );
    }
}

// --- version on the answer (11be073c-ce91-44d8-833a-ef0b5e0b1672) ---------
//
// "show and the page print a short version of each asked question, answer
// takes that version from the terminal and the page alike, and a version
// that no longer matches refuses the answer": the three cases below cover
// the terminal half (the page's own POST is covered in tests/serve_test.rs).

// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
//
// The happy path: `<id>@<version>`, with the version the owner would read
// off `adr show` right now, answers exactly like a bare id used to.
#[test]
fn answer_with_the_current_version_succeeds() {
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
    let arg = id_at_version(&dir, question_id);

    let out = adr_owner(&dir, &["answer", &arg, "A"], "");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));

    let after = std::fs::read_to_string(&record).unwrap();
    assert!(after.contains("**Answer:** A"), "{}", after);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger
            .lines()
            .any(|l| l.contains("\tanswered\t") && l.contains(question_id.as_str())),
        "ledger missing answered row: {}",
        ledger
    );
}

// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
//
// A bare id -- no `@version` at all -- is refused before it is even
// looked up, and writes nothing: not the topic file, not the ledger.
#[test]
fn answer_with_a_bare_id_is_refused_and_writes_nothing() {
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
    assert!(!out.status.success(), "a bare id must be refused");
    let err = stderr(&out);
    assert!(err.contains("no version given"), "{}", err);
    assert!(
        err.contains("owner runs: adr show"),
        "expected the next move to name adr show: {}",
        err
    );

    let after = std::fs::read_to_string(&record).unwrap();
    assert!(
        !after.contains("**Answer:**"),
        "nothing should be written to the topic file: {}",
        after
    );
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tanswered\t")),
        "nothing should be written to the ledger: {}",
        ledger
    );
}

// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
//
// "a version that no longer matches refuses the answer": editing the
// question's text and re-asking it moves its hash (and so its version) --
// the version read before the edit is refused, and nothing is written.
#[test]
fn answer_with_a_stale_version_is_refused_and_writes_nothing() {
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
    let question_id = ids[0].clone();
    let stale_version = current_version(&dir, &question_id);

    // Edit the question's Problem text (inside the hashed range, before any
    // Answer) and re-ask: the section's hash -- and so its version -- moves.
    let edited = content.replace(
        "names leaked into decisions.",
        "names leaked into decisions, badly.",
    );
    std::fs::write(&record, edited).unwrap();
    let reask = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(reask.status.success(), "reask failed: {}", stderr(&reask));
    assert_ne!(
        current_version(&dir, &question_id),
        stale_version,
        "the re-ask should have moved the version"
    );

    let stale_arg = format!("{}@{}", question_id, stale_version);
    let out = adr_owner(&dir, &["answer", &stale_arg, "A"], "");
    assert!(!out.status.success(), "a stale version must be refused");
    let err = stderr(&out);
    assert!(
        err.contains("has moved since that version was read"),
        "{}",
        err
    );

    let after = std::fs::read_to_string(&record).unwrap();
    assert!(
        !after.contains("**Answer:**"),
        "nothing should be written to the topic file: {}",
        after
    );
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tanswered\t")),
        "nothing should be written to the ledger: {}",
        ledger
    );
}

// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
//
// The version comparison is case-insensitive: an uppercased version (hex
// case shouldn't matter) still answers successfully.
#[test]
fn answer_with_an_uppercased_version_still_succeeds() {
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
    let version = current_version(&dir, question_id);
    let arg = format!("{}@{}", question_id, version.to_uppercase());

    let out = adr_owner(&dir, &["answer", &arg, "A"], "");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));

    let after = std::fs::read_to_string(&record).unwrap();
    assert!(after.contains("**Answer:** A"), "{}", after);
}

// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
//
// A version that is not exactly 8 hex characters is a malformed argument,
// not a question that moved since it was read: it gets its own message,
// distinct from `stale_version`, and writes nothing.
#[test]
fn answer_with_a_malformed_version_is_refused_with_its_own_message_and_writes_nothing() {
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

    for bad_version in ["zzzzzzzz", "abc", "1234567", "123456789"] {
        let arg = format!("{}@{}", question_id, bad_version);
        let out = adr_owner(&dir, &["answer", &arg, "A"], "");
        assert!(
            !out.status.success(),
            "a malformed version must be refused: {}",
            bad_version
        );
        let err = stderr(&out);
        assert!(
            err.contains("is not a valid version"),
            "bad_version={}: {}",
            bad_version,
            err
        );
        assert!(
            !err.contains("has moved since that version was read"),
            "a malformed version is not the same failure as a stale one: {}",
            err
        );
    }

    let after = std::fs::read_to_string(&record).unwrap();
    assert!(
        !after.contains("**Answer:**"),
        "nothing should be written to the topic file: {}",
        after
    );
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tanswered\t")),
        "nothing should be written to the ledger: {}",
        ledger
    );
}
