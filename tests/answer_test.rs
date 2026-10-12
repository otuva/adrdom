mod common;

use common::*;
use std::path::Path;

// Every heading needs a `depends:` line for the real `adr approve`
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

const FOURTH_QUESTION: &str = r#"
### 1.4 What is the fourth unit of authority called?
depends: authority

Problem: naming the fourth unit.

Standard: plain words.

- A) The fourth thing is a gadget.
  Why: reasons.
  Costs: costs.
- B) The fourth thing is a gimmick.
  Why: other reasons.
  Costs: other costs.
"#;

/// The plan lines of `question` (its heading and `depends:` line) with a
/// `supersedes:` line of `ids` directly under `depends:`, and no body.
fn plan_lines(question: &str, ids: &[&str]) -> String {
    let (head, _) = question.split_once("\n\nProblem:").unwrap();
    format!("{}\nsupersedes: {}\n", head, ids.join(", "))
}

/// The body of `question`: everything from its Problem line on.
fn body_lines(question: &str) -> String {
    let (_, rest) = question.split_once("\n\nProblem:").unwrap();
    format!("\nProblem:{}", rest)
}

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

    // The listing with no file named prints every current decision with
    // the option picked.
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
        stdout(&check_out).contains("an answered question's text stays as it was answered"),
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
            "agent restores adrdom/decisions/example.md from git, or writes a question that supersedes {}",
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
        !stdout(&restored_check).contains("text changed since being answered"),
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

    // The listing with no file named prints every current decision with
    // the option picked.
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

// A `supersedes:` naming two ids, both current when the plan is approved;
// one of them is superseded by another question after the question naming
// both was asked (one run of ask asks both together). `answer` takes the
// question and supersedes both targets: the sibling replacer answered first.
//
// adr: f4f667ad-8714-4e65-897b-c7002e423c28
// adr: f19e7914-ad97-4f6b-87dd-9301e53cd068
//
// "each target must be current when the question is asked (ask stops a
// non-current target)".
#[test]
fn answer_takes_a_two_id_supersedes_when_a_sibling_replaced_one_target_after_the_ask() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let rel = "adrdom/decisions/example.md";
    let record = write_record(&dir, rel, &format!("{}{}", FIRST_QUESTION, SECOND_QUESTION));
    approve_record(&dir, rel);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    let (old_id_1, old_id_2) = (ids[0].clone(), ids[1].clone());
    assert!(adr_answer(&dir, &old_id_1, "A").status.success());
    assert!(adr_answer(&dir, &old_id_2, "A").status.success());

    // 1.4 (with its body) supersedes old_id_2; 1.3 supersedes both and waits
    // on 1.4. Both targets are current, so approve accepts the plan.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&format!(
        "{}{}",
        FOURTH_QUESTION.replacen(
            "depends: authority\n",
            &format!("depends: authority\nsupersedes: {}\n", old_id_2),
            1
        ),
        format!(
            "{}{}",
            plan_lines(THIRD_QUESTION, &[&old_id_1, &old_id_2]).replacen(
                "depends: authority\n",
                "depends: authority, 1.4 What is the fourth unit of authority called?=A\n",
                1
            ),
            body_lines(THIRD_QUESTION)
        ),
    ));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, rel);
    // One ask puts 1.4 and 1.3 to the owner together; 1.3's targets are both
    // current then. Answering 1.4 supersedes old_id_2.
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    let fourth_id = ids[2].clone();
    assert!(adr_answer(&dir, &fourth_id, "A").status.success());

    // 1.3 stands on 1.4 answered A, and old_id_2 was superseded after 1.3
    // was asked: a sibling replacer answered first (the target stands as
    // `supersedes_refusals` reads it), so `answer` takes 1.3 and supersedes
    // both targets. One run asked 1.3 while both were current.
    let third_id = ids[3].clone();
    let out = adr_answer(&dir, &third_id, "A");
    assert!(out.status.success(), "{}", stderr(&out));
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    for old in [&old_id_1, &old_id_2] {
        assert!(
            ledger.lines().any(|l| l.contains("\tsuperseded\t") && l.contains(old.as_str()) && l.contains(third_id.as_str())),
            "{} is superseded by 1.3: {}",
            old,
            ledger
        );
    }
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
    // Later than old_id's real `superseded` row (today's actual clock
    // time, from the real `adr answer` above), not the fixed 2024 date
    // `append_ledger_row` uses: this row's own position -- which
    // `latest_asked_pos`/`latest_superseded_pos` now derive from time, per
    // "Does the tool take the order of ledger rows from their time?" --
    // has to land after that `superseded` row for this test's point (old_id
    // was already superseded before this hand-crafted ask) to hold.
    append_ledger_row_at(
        &dir,
        "asked",
        &third_id,
        &new_hash,
        "1.3 What is the third unit of authority called?",
        "2099-01-01T00:00:00Z",
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

// A stale target: X is superseded, then a later question Z names X in
// `supersedes:`. `approve` refuses Z's plan outright, since the target is
// not a current decision.
//
// adr: f19e7914-ad97-4f6b-87dd-9301e53cd068
#[test]
fn approve_refuses_a_heading_whose_supersedes_target_is_stale() {
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

    // The third question names the already-superseded old_id: `approve`
    // refuses it, before the plan holds it.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&plan_lines(THIRD_QUESTION, &[&old_id]));
    std::fs::write(&record, &content).unwrap();
    plan_record(&dir, "adrdom/decisions/example.md");
    let before = std::fs::read_to_string(&record).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(!err.starts_with("approve: "), "{}", err);
    assert!(err.contains("1.3 What is the third unit of authority called?"), "{}", err);
    assert!(err.contains(old_id.as_str()), "{}", err);
    assert!(err.contains("which is not a current decision; a question supersedes only a current decision"), "{}", err);
    assert!(err.contains("agent rewrites the supersedes entry"), "{}", err);

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, before, "a refused approve must change nothing in the topic file");
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
        "adrdom/terms.md missing; the terms file exists once a topic file does; agent writes adrdom/terms.md, owner runs: adr approve adrdom/terms.md"
    );
}

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// "Does the tool stop while the terms file waits for approval?" (A): a
// draft with its own shape error (an added term with no id yet, minted
// only by `approve`) refuses answer the same as any other unapproved
// terms.md -- `require_terms`'s shape gate catches it before approval is
// even checked. Supersedes the cd450764 B behavior this test used to show
// (answer falling back to the last approved snapshot and succeeding).
#[test]
fn answer_refuses_when_terms_md_holds_an_unapproved_added_term_with_a_shape_error() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = write_record(
        &dir,
        "adrdom/terms.md",
        "# Terms\n\n## authority\ndefinition: A fixture term for tests.\naliases: \n",
    );
    let terms_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(terms_out.status.success(), "approve terms failed: {}", stderr(&terms_out));

    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // Add a brand-new term with no `id:` line at all: only approve writes
    // ids, so the entry waits for the owner's approval of the terms file,
    // the one move named, as check and show name it (no agent fix).
    let mut terms = std::fs::read_to_string(&terms_path).unwrap();
    terms.push_str("\n## widget\ndefinition: An unrelated fixture term.\naliases: \n");
    std::fs::write(&terms_path, &terms).unwrap();

    let out = adr_answer(&dir, &id, "A");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("adrdom/terms.md is changed; commands read only the approved terms file; owner runs: adr approve adrdom/terms.md"), "{}", err);
    assert!(!err.contains("agent fixes"), "{}", err);
}

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// "Does the tool stop while the terms file waits for approval?" (A): an
// asked question can be neither answered nor refused while terms.md holds
// a term added but never approved -- supersedes the cd450764 B behavior
// this test used to show (both succeeding regardless).
#[test]
fn answer_and_refuse_stop_while_terms_md_holds_an_unapproved_added_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = write_record(
        &dir,
        "adrdom/terms.md",
        "# Terms\n\n## authority\ndefinition: A fixture term for tests.\naliases: \n",
    );
    let terms_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(terms_out.status.success(), "approve terms failed: {}", stderr(&terms_out));

    let mut content = FIRST_QUESTION.to_string();
    content.push_str(SECOND_QUESTION);
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let asked = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&asked);
    let (id1, id2) = (ids[0].clone(), ids[1].clone());

    // A brand-new, unapproved term added on top of the real approval above.
    let mut terms = std::fs::read_to_string(&terms_path).unwrap();
    terms.push_str(
        "\n## widget\nid: 55555555-5555-5555-5555-555555555555\ndefinition: An unrelated fixture term.\naliases: \n",
    );
    std::fs::write(&terms_path, &terms).unwrap();

    let answer_out = adr_answer(&dir, &id1, "A");
    assert!(!answer_out.status.success(), "answer should have refused: {}", stdout(&answer_out));
    assert!(stderr(&answer_out).contains("adrdom/terms.md is changed"), "{}", stderr(&answer_out));

    let refuse_out = adr_answer(&dir, &id2, "refuse");
    assert!(!refuse_out.status.success(), "refuse should have refused: {}", stdout(&refuse_out));
    assert!(stderr(&refuse_out).contains("adrdom/terms.md is changed"), "{}", stderr(&refuse_out));
}

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// "Does the tool stop while the terms file waits for approval?" (A): a
// term retired in the terms file's latest real approval, with a further,
// unapproved draft edit sitting on top of that retirement, now refuses
// answer with the generic "terms not approved" -- the hash-mismatch gate
// fires before answer ever reaches the terms-line-specific retired check.
// Supersedes the cd450764 B behavior this test used to show (answer
// judging the terms line against the approved snapshot and refusing with
// the more specific "names retired term" message).
#[test]
fn answer_refuses_while_a_draft_sits_on_top_of_a_real_retirement() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = write_record(
        &dir,
        "adrdom/terms.md",
        "# Terms\n\n## authority\ndefinition: A fixture term for tests.\naliases: \n\n## gadget\ndefinition: A second fixture term no question names.\naliases: \n",
    );
    let terms_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(terms_out.status.success(), "approve terms failed: {}", stderr(&terms_out));

    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // Retire "gadget" for real, with its own approve: no question names
    // it, so the retire is not refused (per "Does the owner retire a term
    // that a question names?").
    let mut retired = std::fs::read_to_string(&terms_path).unwrap();
    retired.push_str("retired: yes\n");
    std::fs::write(&terms_path, &retired).unwrap();
    let retire_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(
        retire_out.status.success(),
        "approve retired terms failed: {}",
        stderr(&retire_out)
    );

    // A further, unapproved draft edit on top of that retirement.
    let mut draft = std::fs::read_to_string(&terms_path).unwrap();
    draft.push_str(
        "\n## widget\nid: 55555555-5555-5555-5555-555555555555\ndefinition: An unrelated fixture term.\naliases: \n",
    );
    std::fs::write(&terms_path, &draft).unwrap();

    let out = adr_answer(&dir, &id, "A");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        "adrdom/decisions/example.md: adrdom/terms.md is changed; commands read only the approved terms file; owner runs: adr approve adrdom/terms.md"
    );
}

#[test]
fn answer_refuses_a_section_with_no_terms_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let content = "# Example\n\n### 1.1 What is the authority?\ndepends: authority\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n  Costs: z.\n- B) w.\n  Why: y.\n  Costs: z.\n";
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
    // Later than the real `asked` row `adr ask` just wrote (today's actual
    // clock time, not the fixed 2024 date `append_ledger_row` uses): per
    // "Does the tool take the order of ledger rows from their time?", a
    // row's place in `rows` comes from its time, so this hand-written row
    // has to carry a time after "now" to stand as the latest one, not just
    // sit later in the file.
    append_ledger_row_at(&dir, "asked", &id, &new_hash, "1.1 What is the authority?", "2099-01-01T00:00:00Z");

    let out = adr_answer(&dir, &id, "A");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        format!("{} (\"1.1 What is the authority?\"): no terms line; an asked question keeps the terms it was asked with; agent runs: adr ask adrdom/decisions/example.md", id)
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
    // Later than the real `asked` row `adr ask` already wrote, same reason
    // as `answer_refuses_a_section_with_no_terms_line`'s own hand-written
    // row above.
    append_ledger_row_at(dir, "asked", &id, &new_hash, detail, "2099-01-01T00:00:00Z");
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
        format!("{} (\"1.1 What is the unit of authority called?\"): malformed terms line; an answered question keeps the meaning it was asked with; agent runs: adr ask adrdom/decisions/example.md", id)
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
            "{} (\"1.1 What is the unit of authority called?\"): terms line names term 00000000-0000-0000-0000-000000000000 (\"authority\") that no longer exists; an answered question keeps the meaning it was asked with; agent runs: adr ask adrdom/decisions/example.md",
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
            "{} (\"1.1 What is the unit of authority called?\"): terms line names retired term \"authority\"; an answered question keeps the meaning it was asked with; agent runs: adr ask adrdom/decisions/example.md",
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
    let content = "# Example\n\n### 1.1 Q?\ndepends: widget\n\nProblem: p of the widget.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
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
        plan_record(&dir, "adrdom/decisions/example.md");
        let cmd = format!(
            "env -u CLAUDECODE {} approve {}",
            adr_bin().display(),
            record.to_str().unwrap()
        );
        let run = || {
            scrubbed(std::process::Command::new("script"))
                .current_dir(&dir)
                .args(["-q", "-e", "-c", &cmd, "/dev/null"])
                .output()
                .expect("`script` (util-linux) is needed to open a real terminal")
        };
        // Opening a pty under load can fail before adr runs at all; one
        // retry keeps that outside adr's own outcome. approve is
        // idempotent on an approved file, so a second run is safe.
        let mut out = run();
        if !out.status.success() {
            out = run();
        }
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
        err.contains(&format!(
            "{} (\"1.1 What is the unit of authority called?\"): no version given; owner runs: adr answer {}@",
            question_id, question_id
        )),
        "expected the next move to name adr answer with a version: {}",
        err
    );
    // The move the refusal names, as printed, with a letter in place of
    // `<letter>`: made at the end of this test, and it answers the question.
    let command = err.trim_end().split("owner runs: adr ").nth(1).unwrap().to_string();

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

    let args: Vec<&str> = command
        .split(' ')
        .map(|a| if a == "<letter>" { "A" } else { a })
        .collect();
    let out = adr_owner(&dir, &args, "");
    assert!(out.status.success(), "{:?}: {}", args, stderr(&out));

    // The question is answered now: a bare id names no command that
    // would answer it, and the owner is sent to show.
    let out = adr_owner(&dir, &["answer", question_id, "A"], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        format!("answer: {}: no version given; owner runs: adr show", question_id)
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
    approve_record(&dir, "adrdom/decisions/example.md");
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

// adr: 21ae8ae0-6884-4d3c-8464-024a8a5aae2d
// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// An answer for a question whose latest ledger state is `withdrawn` (its
// heading was struck, then the section came back by hand) is told so, not
// sent to `ask`, which never picks a withdrawn question up.
#[test]
fn answer_on_a_withdrawn_question_says_withdrawn_not_edited() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", FIRST_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let full = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&full)[0].clone();
    let version = current_version(&dir, &id);

    std::fs::write(&record, "# Example\n\nContext prose.\n").unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    std::fs::write(&record, &full).unwrap();

    let out = adr_owner(&dir, &["answer", &format!("{}@{}", id, version), "A"], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("is already withdrawn; show prints"), "{}", err);
    assert!(err.contains("owner runs: adr show"), "{}", err);
    assert!(!err.contains("adr ask"), "{}", err);
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// The version is judged before the letter: an answer with both a stale
// version and a bad letter hears about the version, whose move names the
// current one, so the letter refusal it may meet next names a command that
// runs as printed.
#[test]
fn answer_with_a_bad_letter_and_a_stale_version_is_refused_for_the_version() {
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
    let stale_version = current_version(&dir, &question_id);
    let edited = content.replace(
        "names leaked into decisions.",
        "names leaked into decisions, badly.",
    );
    std::fs::write(&record, edited).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    let reask = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(reask.status.success(), "reask failed: {}", stderr(&reask));
    assert_ne!(current_version(&dir, &question_id), stale_version);

    let stale_arg = format!("{}@{}", question_id, stale_version);
    let out = adr_owner(&dir, &["answer", &stale_arg, "Z"], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("has moved since that version was read"), "{}", err);
    assert!(!err.contains("letter Z does not name an option"), "{}", err);
    // With the current version the letter is the one fault left.
    let current = format!("{}@{}", question_id, current_version(&dir, &question_id));
    let out = adr_owner(&dir, &["answer", &current, "Z"], "");
    assert!(stderr(&out).contains("letter Z does not name an option"), "{}", stderr(&out));
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

// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
//
// "answer takes that version from the terminal ... and a version that no
// longer matches refuses the answer": the version is only ever checked
// once the letter itself parses. A letter that isn't a single uppercase
// letter (a lowercase "a", or a multi-character "abc") is refused with
// `invalid_letter` regardless of the version given, and writes nothing.
#[test]
fn answer_with_an_invalid_letter_is_refused_and_writes_nothing() {
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

    for bad_letter in ["a", "abc"] {
        let arg = id_at_version(&dir, question_id);
        let out = adr_owner(&dir, &["answer", &arg, bad_letter], "");
        assert!(
            !out.status.success(),
            "an invalid letter must be refused: {}",
            bad_letter
        );
        let err = stderr(&out);
        assert!(
            err.contains(&format!("invalid letter {}", bad_letter)),
            "bad_letter={}: {}",
            bad_letter,
            err
        );
        assert!(
            err.contains(&format!("owner runs: adr answer {} <letter>", arg)),
            "bad_letter={}: {}",
            bad_letter,
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
