mod common;

use common::*;

/// A record with the full shape: `# topic`, context prose, two `##`
/// groups, and `### q?` bodies with Problem/Standard/options.
const TWO_GROUP_RECORD: &str = r#"# Topic

Context prose line one.
Context prose line two.

### 1.1 First question?
depends: decision

Problem: the decision is unclear.

Standard: s1.

- A) Answer one.
  Why: reasons.
  Costs: costs.
- B) Answer two.
  Why: other reasons.
  Costs: other costs.
"#;

const GROUP_TWO_ADDITION: &str = r#"
### 2.1 Second question?
depends: 1.1 First question?, decision

Problem: p2, another decision.

Standard: s2.

- A) Second answer one.
  Why: reasons.
  Costs: costs.
- B) Second answer two.
  Why: other reasons.
  Costs: other costs.
"#;

const ONE_QUESTION_RECORD: &str = r#"# Topic

Context prose.

### 1.1 Only question?
depends: decision

Problem: the decision at hand.

Standard: s.

- A) Answer A.
  Why: reasons.
  Costs: costs.
- B) Answer B.
  Why: other reasons.
  Costs: other costs.
"#;

const HEADING_REMOVED: &str = "# Topic\n\nContext prose.\n";

// --- ask refusals naming `adr approve` -------------------------------

#[test]
fn ask_refuses_an_unapproved_file_naming_adr_approve() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "question");
    let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP_RECORD);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("adr approve"), "{}", err);
    assert!(err.contains("adrdom/decisions/example.md"), "{}", err);

    // Nothing written.
    let content = std::fs::read_to_string(&record).unwrap();
    assert_eq!(content, TWO_GROUP_RECORD);
}

// --- approve writes approved/context rows, idempotently --------------

// adr: b741b573-a2ee-498e-98fd-3a7e7d319470
//
// "approve writes one event for the headings ('approved') and one for the
// prose ('context'), each with its own hash": exactly one of each row for
// one approve, and rerunning with nothing changed writes neither again --
// which only holds if each hash is compared and stored on its own, not
// folded into the other's.
#[test]
fn approve_writes_approved_and_context_rows_then_is_idempotent() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP_RECORD);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(ledger.lines().filter(|l| l.contains("\tapproved\t")).count(), 1);
    assert_eq!(ledger.lines().filter(|l| l.contains("\tcontext\t")).count(), 1);

    // Rerun: no new rows.
    let out2 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out2.status.success());
    let ledger2 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(ledger, ledger2, "rerunning approve with nothing changed must be a no-op");
    assert!(stdout(&out2).contains("plan unchanged"), "{}", stdout(&out2));
    assert!(stdout(&out2).contains("prose unchanged"), "{}", stdout(&out2));
}

// --- approve refuses on an option-count shape error --------------------

#[test]
fn approve_refuses_a_question_with_one_option() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    let content = "# Example\n\n### 1.1 One option?\n\nProblem: p.\n\nStandard: s.\n\n- A) Only choice.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("1 options; at least two"), "{}", err);
    assert!(err.contains("agent fixes the topic file"), "{}", err);

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, content, "approve must not modify the file when it refuses");
    assert!(extract_ids(&after).is_empty(), "no id should have been assigned: {}", after);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tapproved\t") || l.contains("\tcontext\t")),
        "no approved/context row should have been written: {}",
        ledger
    );
}

#[test]
fn approve_refuses_a_question_with_four_options() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    let content = "# Example\n\n### 1.1 Four options?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n- D) Four.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("4 options; at most three"), "{}", err);
    assert!(err.contains("agent fixes the topic file"), "{}", err);

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, content, "approve must not modify the file when it refuses");
    assert!(extract_ids(&after).is_empty(), "no id should have been assigned: {}", after);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tapproved\t") || l.contains("\tcontext\t")),
        "no approved/context row should have been written: {}",
        ledger
    );
}

// --- editing context prose requires re-approval -----------------------

// adr: b741b573-a2ee-498e-98fd-3a7e7d319470
//
// "each with its own hash; ask stops a file where either part is
// unapproved (e.g. the prose edited after approve)": editing only the
// context prose leaves the plan hash (and its `approved` row) untouched
// while the context hash moves, so `ask` stops on "prose not approved"
// even though the heading itself is still approved.
#[test]
fn editing_context_prose_requires_reapproval_with_one_new_context_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());

    // Edited on the file as `approve` actually left it (with the heading's
    // id already minted) -- not the original text-form constant, which
    // would wipe that id back out and make the *plan* look unapproved too
    // (an id is part of the plan).
    let approved_content = std::fs::read_to_string(&record).unwrap();
    let edited = approved_content.replace(
        "Context prose line one.",
        "Context prose line one, edited.",
    );
    std::fs::write(&record, &edited).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("prose not approved"), "{}", stderr(&out));

    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let approved_before = ledger_before.lines().filter(|l| l.contains("\tapproved\t")).count();
    let context_before = ledger_before.lines().filter(|l| l.contains("\tcontext\t")).count();

    let out2 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out2.status.success(), "approve failed: {}", stderr(&out2));

    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let approved_after = ledger_after.lines().filter(|l| l.contains("\tapproved\t")).count();
    let context_after = ledger_after.lines().filter(|l| l.contains("\tcontext\t")).count();

    assert_eq!(approved_after, approved_before, "plan hash did not change; no new approved row expected");
    assert_eq!(context_after, context_before + 1, "exactly one new context row expected");

    // ask now succeeds.
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
}

// adr: b741b573-a2ee-498e-98fd-3a7e7d319470
//
// "approve writes one event for the headings and one for the prose, each
// with its own hash; ask stops a file where either part is unapproved":
// the other direction from `editing_context_prose_requires_reapproval...`
// above -- here the context prose is left untouched (so the `context`
// hash, and its row, never move) while a heading's own text is edited
// after approval, which moves only the plan hash. `ask` stops the file on
// "plan not approved" even though the context is still approved, and
// re-approving writes exactly one new `approved` row and no new `context`
// row.
#[test]
fn editing_a_heading_requires_reapproval_with_one_new_approved_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());

    // Edited on the file as `approve` actually left it (with the heading's
    // id already minted), renaming the heading text only -- the context
    // prose lines above it are untouched.
    let approved_content = std::fs::read_to_string(&record).unwrap();
    let edited = approved_content.replace(
        "### 1.1 First question?",
        "### 1.1 First question, renamed?",
    );
    assert_ne!(edited, approved_content, "the heading text must exist to rename");
    std::fs::write(&record, &edited).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("plan not approved"), "{}", stderr(&out));

    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let approved_before = ledger_before.lines().filter(|l| l.contains("\tapproved\t")).count();
    let context_before = ledger_before.lines().filter(|l| l.contains("\tcontext\t")).count();

    let out2 = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out2.status.success(), "approve failed: {}", stderr(&out2));
    assert!(
        stdout(&out2).contains("prose unchanged"),
        "{}",
        stdout(&out2)
    );

    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let approved_after = ledger_after.lines().filter(|l| l.contains("\tapproved\t")).count();
    let context_after = ledger_after.lines().filter(|l| l.contains("\tcontext\t")).count();

    assert_eq!(approved_after, approved_before + 1, "exactly one new approved row expected");
    assert_eq!(context_after, context_before, "context hash did not change; no new context row expected");

    // ask now succeeds.
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
}

// --- tier rule: a later group waits on an earlier one ------------------

#[test]
fn ask_refuses_group_two_while_group_one_unanswered_then_passes_after_answer() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let first_id = extract_ids(&content)[0].clone();

    // Add group two; group one's question is still unanswered.
    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(GROUP_TWO_ADDITION);
    std::fs::write(&record, &content2).unwrap();
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("First question?"), "{}", err);

    // Nothing newly asked: `approve` already gave group two's heading an
    // id too, so `extract_ids` no longer tells asked apart from
    // merely approved -- the ledger's `asked` row count does.
    let ledger_still = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(asked_row_count(&dir), 1, "{}", ledger_still);

    // Answer group one, then ask succeeds for group two.
    let answer_out = adr_answer(&dir, &first_id, "A");
    assert!(answer_out.status.success(), "answer failed: {}", stderr(&answer_out));

    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "ask failed: {}", stderr(&out2));
    let ledger_final = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(asked_row_count(&dir), 2, "{}", ledger_final);
}

// --- a struck asked heading is withdrawn on approve ---------------------

// adr: 21ae8ae0-6884-4d3c-8464-024a8a5aae2d
//
// "when the owner approves a plan that no longer holds an asked question,
// approve writes a withdrawn event for it": the heading naming `id` is
// struck from the file (never answered, never superseded), and the very
// `approve` that accepts the edit writes the `withdrawn` row for it.
#[test]
fn approve_withdraws_a_struck_asked_heading() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    std::fs::write(&record, HEADING_REMOVED).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger.lines().any(|l| l.contains("\twithdrawn\t") && l.contains(id.as_str())),
        "ledger missing withdrawn row: {}",
        ledger
    );
}

// --- approve withdraws a struck refused heading --------------------------

// adr: 6b3d8e59-e145-49e1-860d-a79d753d745c
//
// "when the owner approves a plan that drops it the section is gone
// (approve accepts the drop and writes a withdrawn event for it)": `id` is
// refused (never answered, never superseded) and its section stays in the
// file, marked refused, until its heading is struck; the very `approve`
// that accepts that edit writes the `withdrawn` row for it, exactly the
// way `approve_withdraws_a_struck_asked_heading` shows for a merely-asked
// heading above.
#[test]
fn approve_withdraws_a_struck_refused_heading() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/topic.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "refuse").status.success());

    // While the refused section still sits in the file, marked refused,
    // check is clean over it -- no decision, no test, nothing owed.
    let check_before = adr(&dir, &["check"], &[], "");
    assert!(
        check_before.status.success(),
        "check should be clean with the refused section still in the file: {}",
        stdout(&check_before)
    );

    std::fs::write(&record, HEADING_REMOVED).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger.lines().any(|l| l.contains("\twithdrawn\t") && l.contains(id.as_str())),
        "ledger missing withdrawn row for the dropped refused id: {}",
        ledger
    );

    // check stays clean once the drop is approved.
    let check_after = adr(&dir, &["check"], &[], "");
    assert!(
        check_after.status.success(),
        "check should stay clean once the refused heading's drop is approved: {}",
        stdout(&check_after)
    );
}

// --- approve refuses when a current heading vanishes without a supersede --

// adr: 21ae8ae0-6884-4d3c-8464-024a8a5aae2d
//
// "approve stops when an answered question is missing from every topic
// file": `id` is answered and current, never superseded, yet its heading
// is struck; `approve` refuses (naming `id`) instead of withdrawing it,
// and writes no `withdrawn` row.
#[test]
fn approve_refuses_when_answered_heading_gone_and_not_superseded() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());

    std::fs::write(&record, HEADING_REMOVED).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains(id.as_str()), "{}", err);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.lines().any(|l| l.contains("\twithdrawn\t")));
}

// Regression: the missing-answered-id check used to run after `record::save`
// and the `approved`/`context` ledger rows had already been written, so a
// refusal here left a rewritten topic file and a moved ledger behind --
// contradicting `do_approve_record`'s own doc comment that nothing is
// written when a check refuses. The plan hash of the headless file differs
// from what's on the ledger, so a pre-fix `approve` would have appended a
// fresh `approved` row before ever reaching this refusal.
#[test]
fn approve_leaves_file_and_ledger_unchanged_when_missing_answered_id_refuses() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());

    std::fs::write(&record, HEADING_REMOVED).unwrap();
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let record_before = std::fs::read(&record).unwrap();
    let ledger_before = std::fs::read(&ledger_path).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    assert!(stderr(&out).contains(id.as_str()), "{}", stderr(&out));

    let record_after = std::fs::read(&record).unwrap();
    let ledger_after = std::fs::read(&ledger_path).unwrap();
    assert_eq!(
        record_before, record_after,
        "topic file must be untouched when approve refuses"
    );
    assert_eq!(
        ledger_before, ledger_after,
        "ledger must be untouched when approve refuses"
    );
}

// --- approve allows a vanished heading once it is superseded ------------

// adr: 1b181f10-97f4-4bbe-8939-d5842192126c
//
// "once superseded, a question's section may leave the file as a plan
// change the owner approves, and approve allows that only for superseded
// questions": old_id's section is dropped from the file entirely once it
// is superseded, and `approve` accepts the edit (contrast
// `approve_refuses_when_answered_heading_gone_and_not_superseded`, where
// the same removal on a current, non-superseded id refuses). "the old
// text stays in git history and its hash in the ledger": old_id's
// `answered` row, carrying the section's hash, is still in the ledger
// after its text has left the file.
#[test]
fn approve_allows_a_vanished_heading_once_superseded() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &old_id, "A").status.success());
    let old_answered_row = {
        let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
        ledger
            .lines()
            .find(|l| l.contains("\tanswered\t") && l.contains(old_id.as_str()))
            .expect("answered row for old_id")
            .to_string()
    };

    let superseding = format!(
        "\n### 1.2 Replacement question?\ndepends: decision\n\nsupersedes: {}\n\nProblem: the decision needs revisiting.\n\nStandard: s2.\n\n- A) New answer.\n  Why: reasons.\n  Costs: costs.\n- B) Other new answer.\n  Why: other reasons.\n  Costs: other costs.\n",
        old_id
    );
    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(&superseding);
    std::fs::write(&record, &content2).unwrap();
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content3 = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content3)
        .into_iter()
        .find(|i| i != &old_id)
        .expect("new id assigned");
    assert!(adr_answer(&dir, &new_id, "A").status.success());

    // Now drop the (superseded) 1.1 heading entirely, keeping only 1.2.
    let content4 = std::fs::read_to_string(&record).unwrap();
    let idx = content4.find("### 1.2").expect("1.2 present");
    let rebuilt = format!("# Topic\n\nContext prose.\n\n{}", &content4[idx..]);
    std::fs::write(&record, &rebuilt).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve should have allowed it: {}", stderr(&out));
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.lines().any(|l| l.contains("\twithdrawn\t") && l.contains(old_id.as_str())));
    // The old text is gone from the file, but its ledger row -- hash and
    // all -- is untouched: nothing rewrites or drops a ledger row when the
    // section it describes leaves the file.
    assert!(
        ledger.lines().any(|l| l == old_answered_row),
        "old_id's answered row must still be in the ledger unchanged: {}",
        ledger
    );
}

// --- a topic file with nothing current or open left may be deleted -------

// adr: 1b181f10-97f4-4bbe-8939-d5842192126c
//
// "a topic file with no current or open question left may be deleted the
// same way": once `a.md`'s only question is superseded and its section
// removed by an approved edit, the file holds no current or open question
// at all; deleting it outright leaves `check` clean, exactly as removing
// just the section did above -- no special act beyond the plan change
// already approved.
#[test]
fn check_stays_clean_once_an_emptied_topic_file_is_deleted() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");

    let record_a = write_record(&dir, "adrdom/decisions/a.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record_a.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record_a.to_str().unwrap()], &[], "")
        .status
        .success());
    let content_a = std::fs::read_to_string(&record_a).unwrap();
    let old_id = extract_ids(&content_a)[0].clone();
    assert!(adr_answer(&dir, &old_id, "A").status.success());
    let old_answered_row = {
        let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
        ledger
            .lines()
            .find(|l| l.contains("\tanswered\t") && l.contains(old_id.as_str()))
            .expect("answered row for old_id")
            .to_string()
    };

    // A second, unrelated topic file supersedes old_id: a.md's only
    // question is now superseded, not answered by anything a.md still
    // holds.
    let superseding = format!(
        "# B\n\n### 2.1 Replacement in another file?\ndepends: decision\n\nsupersedes: {}\n\nProblem: the decision needs revisiting from elsewhere.\n\nStandard: s2.\n\n- A) New answer.\n  Why: reasons.\n  Costs: costs.\n- B) Other new answer.\n  Why: other reasons.\n  Costs: other costs.\n",
        old_id
    );
    let record_b = write_record(&dir, "adrdom/decisions/b.md", &superseding);
    assert!(adr_owner(&dir, &["approve", record_b.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record_b.to_str().unwrap()], &[], "")
        .status
        .success());
    let content_b = std::fs::read_to_string(&record_b).unwrap();
    let new_id = extract_ids(&content_b)[0].clone();
    assert!(adr_answer(&dir, &new_id, "A").status.success());

    // a.md's only heading is superseded now: an approved edit may drop it,
    // leaving a.md with no current or open question left at all.
    std::fs::write(&record_a, HEADING_REMOVED).unwrap();
    assert!(adr_owner(&dir, &["approve", record_a.to_str().unwrap()], "")
        .status
        .success());

    // Delete the emptied file outright.
    std::fs::remove_file(&record_a).unwrap();

    // Tie new_id so the only thing left for `check` to possibly complain
    // about is old_id and the deleted file, not an unrelated "untested"
    // note on b.md's own, still-current decision.
    write_test_comment(&dir, &new_id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should stay clean: {}", stdout(&out));
    let text = stdout(&out);
    assert!(
        !text.contains(old_id.as_str()),
        "a deleted, fully-superseded file's id must earn no failure: {}",
        text
    );

    // The old text is gone from disk entirely, but its hash lives on in
    // the ledger, unaffected by the file's deletion.
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger.lines().any(|l| l == old_answered_row),
        "old_id's answered row must survive the file's deletion: {}",
        ledger
    );
}

// --- the frozen-depends check also covers a superseded section ----------
//
// The same check already refuses a `Text`-shaped `depends:` entry rewritten
// inside a section whose latest ledger state is `answered` or `refused`
// (`approve_terms_refuses_before_writing_leaving_the_file_unchanged` and
// friends exercise the terms half of this; there was no coverage at all,
// for either code path, of the third closed state a section can be in:
// `superseded`). A superseded decision is exactly as closed as a refused
// one, so an edit there must refuse the same way.

#[test]
fn approve_refuses_a_text_depends_edit_in_a_superseded_section() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &old_id, "A").status.success());

    let superseding = format!(
        "\n### 1.2 Replacement question?\ndepends: decision\n\nsupersedes: {}\n\nProblem: the decision needs revisiting.\n\nStandard: s2.\n\n- A) New answer.\n  Why: reasons.\n  Costs: costs.\n- B) Other new answer.\n  Why: other reasons.\n  Costs: other costs.\n",
        old_id
    );
    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(&superseding);
    std::fs::write(&record, &content2).unwrap();
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content3 = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content3)
        .into_iter()
        .find(|i| i != &old_id)
        .expect("new id assigned");
    // Answering 1.2 (which carries `supersedes: <old_id>`) is what writes
    // the `superseded` ledger row for 1.1's id.
    assert!(adr_answer(&dir, &new_id, "A").status.success());

    // 1.1's heading is kept in the file (unlike
    // `approve_allows_a_vanished_heading_once_superseded`), but its
    // already-resolved `depends: <term id>` line is hand-edited back to
    // plain text, exactly as an agent might carry an unrelated edit
    // forward. 1.1's id is now `superseded`, not `answered`/`refused`.
    let content4 = std::fs::read_to_string(&record).unwrap();
    let rewritten = content4.replacen(
        "depends: 99999999-9999-9999-9999-999999999999\n",
        "depends: decision\n",
        1,
    );
    assert_ne!(
        content4, rewritten,
        "expected 1.1's depends line to already be rewritten to the term id"
    );
    std::fs::write(&record, &rewritten).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Only question?\": answered; agent removes the edit"),
        "{}",
        err
    );
}

// --- terms: ask waits on terms approval ---------------------------------

const TERMS_BASIC: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice answered by the owner.\n\
aliases: \n";

#[test]
fn ask_refuses_while_terms_unapproved() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);
    write_record(&dir, "adrdom/terms.md", TERMS_BASIC);
    // A real `adr approve` on the record now refuses up front while
    // `adrdom/terms.md` itself is unapproved: it used to succeed here since
    // the hand-written id in `TERMS_BASIC` let depends resolution go
    // through despite there being no `terms` ledger row at all.
    // `approve_via_ledger` gets the record "looks approved" to `ask` the
    // same way real approve's write step would have, without depending on
    // the now-refusing terms gate -- `ask`'s own terms gate fires before it
    // ever needs the section's id.
    approve_via_ledger(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("terms not approved"), "{}", err);
    assert!(err.contains("adr approve adrdom/terms.md"), "{}", err);
}

// --- approve fills missing term ids -------------------------------------

const TERMS_NO_IDS: &str = "# Terms\n\n\
## decision\n\
definition: A choice answered by the owner.\n\
aliases: \n";

#[test]
fn approve_fills_missing_term_ids() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/decisions/example.md", "# Example\n");
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_NO_IDS);

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));

    let content = std::fs::read_to_string(&terms).unwrap();
    assert!(content.lines().any(|l| l.starts_with("id: ")), "{}", content);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(ledger.lines().any(|l| l.contains("\tterms\t")));
}

// --- ask writes a terms: line binding a word to the term id ------------

const DECISION_QUESTION_RECORD: &str = r#"# Topic

Context prose.

### 1.1 What is the decision?
depends: decision

Problem: whether a decision is well defined.

Standard: plain words.

- A) A decision is one commitment.
  Why: reasons.
  Costs: costs.
- B) A decision is a block.
  Why: other reasons.
  Costs: other costs.
"#;

#[test]
fn ask_writes_terms_line_binding_word_to_term_id() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", DECISION_QUESTION_RECORD);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_BASIC);

    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    assert!(
        content
            .lines()
            .any(|l| l.starts_with("terms: ") && l.contains("decision=11111111-1111-1111-1111-111111111111")),
        "{}",
        content
    );
}

// --- approve refuses a terms edit that moves a bound word off a term --

// `decision`'s definition deliberately avoids the word "choice" itself
// (its own alias): using it there would make the term's definition pair
// with itself, a self-cycle refused by step 3, unrelated to what these
// fixtures are testing.
const TERMS_WITH_ALIAS: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A pick answered by the owner.\n\
aliases: choice\n\
\n\
## verdict\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A ruling reached after deliberation.\n\
aliases: \n";

const TERMS_ALIAS_MOVED: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A pick answered by the owner.\n\
aliases: \n\
\n\
## verdict\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A ruling reached after deliberation.\n\
aliases: choice\n";

const CHOICE_QUESTION_RECORD: &str = r#"# Topic

Context prose.

### 1.1 What is the choice?
depends: decision

Problem: the choice is unclear.

Standard: plain words.

- A) The choice is final.
  Why: reasons.
  Costs: costs.
- B) The choice is provisional.
  Why: other reasons.
  Costs: other costs.
"#;

// adr: 7cf81ded-be48-4777-b83e-f8a2998c1ab5
// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// `approve stops a terms edit that takes a word off an entry while a
// current decision pairs the word to it, and names those decisions`:
// CHOICE_QUESTION_RECORD's 1.1 is answered A (current) with a terms line
// pairing the word "choice" to term "decision"; moving the "choice" alias
// off "decision" onto "verdict" is refused, naming exactly that current
// decision by heading text, and the terms.md ledger row count below shows
// nothing was written on the refusal. The decision behind it is answered
// (current), so it can never be re-asked -- the next move named is
// restoring the wording in adrdom/terms.md or superseding the decision by
// id, never "re-asks" (per "Does a refusal name the command that clears
// it?": a refusal about an answered question never tells the agent to
// re-ask it, since that can't clear it).
#[test]
fn approve_refuses_a_terms_edit_that_moves_a_word_off_a_term_carried_by_a_current_decision() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", CHOICE_QUESTION_RECORD);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_WITH_ALIAS);

    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());

    // Move the "choice" alias from `decision` to `verdict`.
    std::fs::write(&terms, TERMS_ALIAS_MOVED).unwrap();

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(
            "word \"choice\" no longer names term \"decision\", used by adrdom/decisions/example.md: \"1.1 What is the choice?\""
        ),
        "{}",
        err
    );
    assert!(
        err.contains(&format!(
            "agent restores the wording in adrdom/terms.md so \"choice\" still names \"decision\", or writes a question that supersedes {}",
            id
        )),
        "an answered decision's next move must be restore-or-supersede, never re-ask: {}",
        err
    );
    assert!(!err.contains("re-asks"), "an answered decision can never be re-asked: {}", err);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(ledger.lines().filter(|l| l.contains("\tterms\t")).count(), 1);
}

#[test]
fn approve_allows_moving_a_word_that_nothing_carries() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_WITH_ALIAS);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());

    std::fs::write(&terms, TERMS_ALIAS_MOVED).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(out.status.success(), "approve should have allowed it: {}", stderr(&out));
}

// --- approve writes a terms row on a clean edit -------------------------

const TERMS_V2: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice answered by the owner, in writing.\n\
aliases: \n";

// `approve writes one event over its hash`: each distinct content of
// adrdom/terms.md that `approve` accepts gets exactly one `terms` ledger
// row, keyed to that content's hash -- the first approve writes row 1
// (asserted just above via
// `approve_terms_reruns_as_already_approved_with_no_new_row`'s sibling
// check), and approving the edited `TERMS_V2` content writes exactly one
// more row (2 total), not one per file or per term inside it.
#[test]
fn approve_writes_a_terms_row_on_a_clean_edit() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/decisions/example.md", "# Example\n");
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_BASIC);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());

    std::fs::write(&terms, TERMS_V2).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    assert!(stdout(&out).contains("approved"), "{}", stdout(&out));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(ledger.lines().filter(|l| l.contains("\tterms\t")).count(), 2);
}

// --- approve terms (4): refuses before writing, circular, already approved

const TERMS_MISSING_DEFINITION: &str = "# Terms\n\n## decision\naliases: \n";

#[test]
fn approve_terms_refuses_before_writing_leaving_the_file_unchanged() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_MISSING_DEFINITION);
    let before = std::fs::read_to_string(&terms).unwrap();

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    assert!(
        stderr(&out).contains("missing field \"definition\""),
        "{}",
        stderr(&out)
    );

    let after = std::fs::read_to_string(&terms).unwrap();
    assert_eq!(
        before, after,
        "approve must not write anything to adrdom/terms.md on refusal, even though minting an id for the id-less term above happens in memory"
    );
}

const TERMS_CIRCULAR: &str = "# Terms\n\n\
## alpha\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: See beta.\n\
aliases: \n\
\n\
## beta\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: See alpha.\n\
aliases: \n";

// adr: fcc1c1ec-d8e8-428e-9731-9817cdeb8e76
#[test]
fn approve_refuses_a_circular_definition() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_CIRCULAR);
    let before = std::fs::read_to_string(&terms).unwrap();

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("circular definition: alpha -> beta -> alpha"),
        "{}",
        err
    );

    let after = std::fs::read_to_string(&terms).unwrap();
    assert_eq!(before, after, "approve must not write anything on refusal");
}

// E1.1: two independent problems (a self-cycle on `alpha`, a missing
// `aliases` field on `beta`) must both come back from one `approve` call,
// not just the first found.
const TERMS_CYCLE_AND_SHAPE_ERROR: &str = "# Terms\n\n\
## alpha\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A loop that refers to alpha.\n\
aliases: \n\
\n\
## beta\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: Something else.\n";

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// `approve` (which reads the file directly) reports both problems; `check`
// (per "Does the tool stop while the terms file waits for approval?", A)
// fails on the same terms.md and reports the same two problems, so the
// agent reads them before the owner runs `approve`. It does not name the
// owner's move while `approve` would refuse the file.
#[test]
fn approve_and_check_report_a_cycle_and_a_shape_error() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_CYCLE_AND_SHAPE_ERROR);

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("circular definition: alpha -> alpha"), "{}", err);
    assert!(err.contains("missing field \"aliases\""), "{}", err);

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(!check_out.status.success());
    let text = stdout(&check_out);
    assert!(
        text.contains("terms: adrdom/terms.md: circular definition: alpha -> alpha"),
        "{}",
        text
    );
    assert!(text.contains("missing field \"aliases\""), "{}", text);
    assert!(!text.contains("owner runs: adr approve adrdom/terms.md"), "{}", text);
}

#[test]
fn approve_terms_reruns_as_already_approved_with_no_new_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_BASIC);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    assert!(stdout(&out).contains("already approved"), "{}", stdout(&out));

    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger_before, ledger_after,
        "an already-approved rerun must not append a ledger row"
    );
}

// --- approve terms (4): dependents ---------------------------------------

const TERMS_TWO_ENTRIES: &str = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A choice answered by the owner.\n\
aliases: \n\
\n\
## verdict\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A ruling reached after deliberation.\n\
aliases: \n";

const TERMS_DECISION_REMOVED: &str = "# Terms\n\n\
## verdict\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A ruling reached after deliberation.\n\
aliases: \n";

const TERMS_DECISION_RETIRED: &str = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A choice answered by the owner.\n\
aliases: \n\
retired: yes\n\
\n\
## verdict\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A ruling reached after deliberation.\n\
aliases: \n";

const DECISION_RECORD_FOR_DEPENDENTS: &str = r#"# Topic

Context prose.

### 1.1 What is the decision?
depends: decision

Problem: the decision is unclear.

Standard: plain words.

- A) The decision is final.
  Why: reasons.
  Costs: costs.
- B) The decision is provisional.
  Why: other reasons.
  Costs: other costs.
"#;

#[test]
fn approve_refuses_removing_a_term_carried_by_a_current_decision() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", DECISION_RECORD_FOR_DEPENDENTS);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_TWO_ENTRIES);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());

    std::fs::write(&terms, TERMS_DECISION_REMOVED).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("term 33333333-3333-3333-3333-333333333333 (\"decision\") removed"),
        "{}",
        err
    );
    assert!(err.contains("What is the decision?"), "{}", err);
    assert!(
        err.contains("agent restores the term in adrdom/terms.md, marked retired: yes, owner runs: adr approve adrdom/terms.md"),
        "{}",
        err
    );
}

#[test]
fn approve_refuses_retiring_a_term_carried_by_a_current_decision() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", DECISION_RECORD_FOR_DEPENDENTS);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_TWO_ENTRIES);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());

    std::fs::write(&terms, TERMS_DECISION_RETIRED).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("term \"decision\" retired but used by"),
        "{}",
        err
    );
    assert!(err.contains("What is the decision?"), "{}", err);
    assert!(
        err.contains(&format!("agent supersedes {} with a new question first", id)),
        "{}",
        err
    );
}

// Two terms: "flag" (this test retires it) and "step" (stays current, and
// is the only one the heading's prose actually mentions -- `ask` refuses a
// heading whose terms line pairs no term at all, so the prose needs *some*
// term, just never "flag").
const TERMS_FLAG_AND_STEP: &str = "# Terms\n\n\
## flag\n\
id: 66666666-6666-6666-6666-666666666666\n\
definition: A marker set on something.\n\
aliases: \n\
\n\
## step\n\
id: 77777777-7777-7777-7777-777777777777\n\
definition: One move in a sequence.\n\
aliases: \n";

const TERMS_FLAG_RETIRED_STEP_CURRENT: &str = "# Terms\n\n\
## flag\n\
id: 66666666-6666-6666-6666-666666666666\n\
definition: A marker set on something.\n\
aliases: \n\
retired: yes\n\
\n\
## step\n\
id: 77777777-7777-7777-7777-777777777777\n\
definition: One move in a sequence.\n\
aliases: \n";

// This heading's prose never mentions "flag" at all -- only its `depends:`
// line stands on the term -- so its `terms:` line carries no pairing for
// it (only for "step", which the prose does mention). Before this
// decision, `terms_refusals`' dependents came from a `terms:`/definition
// line only, so retiring a term named solely by a current decision's
// `depends:` line went unrefused.
const DECISION_RECORD_DEPENDS_ONLY: &str = r#"# Topic

Context prose.

### 1.1 What comes next?
depends: flag

Problem: the next step is unclear.

Standard: plain words.

- A) Take the step now.
  Why: reasons.
  Costs: costs.
- B) Hold the step.
  Why: other reasons.
  Costs: other costs.
"#;

#[test]
fn approve_refuses_retiring_a_term_named_only_by_a_current_decisions_depends_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", DECISION_RECORD_DEPENDS_ONLY);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_FLAG_AND_STEP);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    // Its `terms:` line, once written, pairs "step" but has no pairing for
    // "flag" at all: the only thing standing on "flag" is the `depends:`
    // line.
    let terms_line = std::fs::read_to_string(&record)
        .unwrap()
        .lines()
        .find(|l| l.starts_with("terms: "))
        .map(|l| l.to_string())
        .expect("1.1 has a terms line");
    assert!(
        terms_line.contains("step=77777777-7777-7777-7777-777777777777"),
        "{}",
        terms_line
    );
    assert!(!terms_line.contains("66666666-6666-6666-6666-666666666666"), "{}", terms_line);

    std::fs::write(&terms, TERMS_FLAG_RETIRED_STEP_CURRENT).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("term \"flag\" retired but used by"), "{}", err);
    assert!(err.contains("What comes next?"), "{}", err);
    assert!(
        err.contains(&format!("agent supersedes {} with a new question first", id)),
        "{}",
        err
    );
}

const TERMS_DECISION_RETIRED_WORD_TAKEN: &str = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A choice answered by the owner.\n\
aliases: \n\
retired: yes\n\
\n\
## verdict\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A ruling reached after deliberation.\n\
aliases: \n\
\n\
## commitment\n\
id: 55555555-5555-5555-5555-555555555555\n\
definition: A pledge that stands until met.\n\
aliases: decision\n";

// `depends: decision` (the term this test goes on to retire), on purpose:
// once 1.1 is superseded it is no longer open, so `check`'s `plan` category
// resolving this depends entry to the now-retired term is exactly the case
// that used to fail forever (unconditionally, with no exception for a
// superseded heading, unlike the `terms:` line, which this decision
// covers) and must now pass.
const DECISION_RECORD_DEPENDS_ELSEWHERE: &str = r#"# Topic

Context prose.

### 1.1 What is the decision?
depends: decision

Problem: the decision is unclear.

Standard: plain words.

- A) The decision is final.
  Why: reasons.
  Costs: costs.
- B) The decision is provisional.
  Why: other reasons.
  Costs: other costs.
"#;

// adr: 05af74a6-36be-4923-b66f-1467b323816a
//
// `answered questions keep the retired term's identifier on their terms
// line, so their meaning does not move (check stays clean for them)`.
// 1.1's answered terms line pairs "decision"=33333333-..., and its
// `depends:` line resolves to that same id; once 1.2 supersedes 1.1 (so
// 1.1 is no longer a *current* decision), retiring "decision" -- and
// handing its freed word to the new term "commitment" -- is accepted, and
// `adr check` stays entirely clean even though 1.1's terms line and
// depends line both still name the now-retired id: its meaning does not
// move just because the word moved on.
#[test]
fn approve_allows_retiring_a_term_once_its_only_current_decision_is_superseded_and_check_stays_clean(
) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(
        &dir,
        "adrdom/decisions/topic.md",
        DECISION_RECORD_DEPENDS_ELSEWHERE,
    );
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_TWO_ENTRIES);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &old_id, "A").status.success());
    // No test tie for `old_id`: once it is superseded below it is no
    // longer a *current* decision, and the `tested` check only requires
    // (and only tolerates) a tie for a current one.
    let old_terms_line = std::fs::read_to_string(&record)
        .unwrap()
        .lines()
        .find(|l| l.starts_with("terms: "))
        .expect("1.1 has a terms line")
        .to_string();
    assert!(
        old_terms_line.contains("decision=33333333-3333-3333-3333-333333333333"),
        "{}",
        old_terms_line
    );

    // Supersede 1.1 with a new question that also stands on "verdict", so
    // retiring "decision" afterwards has no current dependent left to
    // refuse it.
    let superseding = format!(
        "\n### 1.2 What is the verdict now?\ndepends: verdict\n\nsupersedes: {}\n\nProblem: whether the verdict still holds.\n\nStandard: plain words.\n\n- A) The verdict stands.\n  Why: reasons.\n  Costs: costs.\n- B) The verdict is reversed.\n  Why: other reasons.\n  Costs: other costs.\n",
        old_id
    );
    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(&superseding);
    std::fs::write(&record, &content2).unwrap();
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content3 = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content3)
        .into_iter()
        .find(|i| i != &old_id)
        .expect("new id assigned");
    assert!(adr_answer(&dir, &new_id, "A").status.success());
    write_test_comment(&dir, &new_id);

    // 1.1 is now superseded, not current: retiring "decision" (and giving
    // its freed word to the brand new "commitment" term) is accepted.
    std::fs::write(&terms, TERMS_DECISION_RETIRED_WORD_TAKEN).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve should have allowed retiring a term no longer carried by a current decision: {}",
        stderr(&out)
    );

    // The old (superseded) 1.1 section still carries "decision"'s retired
    // id on its terms line -- untouched -- and `check` reports nothing
    // about it.
    let final_content = std::fs::read_to_string(&record).unwrap();
    assert!(
        final_content.lines().any(|l| l == old_terms_line),
        "1.1's terms line must be unchanged: {}",
        final_content
    );
    let check_out = adr(&dir, &["check"], &[], "");
    assert!(check_out.status.success(), "check should stay clean: {}", stdout(&check_out));
    assert!(
        !stdout(&check_out).contains("retired"),
        "{}",
        stdout(&check_out)
    );
}

const TERMS_BEFORE_RENAME: &str = "# Terms\n\n\
## decision\n\
id: 55555555-5555-5555-5555-555555555555\n\
definition: A choice answered by the owner.\n\
aliases: \n";

const TERMS_AFTER_RENAME_ALIAS_KEPT: &str = "# Terms\n\n\
## ruling\n\
id: 55555555-5555-5555-5555-555555555555\n\
definition: A choice answered by the owner.\n\
aliases: decision\n";

// adr: 7c306235-6243-4d79-b9aa-08ed19662779
#[test]
fn approve_allows_a_rename_that_keeps_the_old_name_as_an_alias() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", DECISION_RECORD_FOR_DEPENDENTS);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_BEFORE_RENAME);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());

    // The question's own terms line, written by `ask` before the rename,
    // pairs the word "decision" to the term's identifier -- not its name.
    let term_uuid = "55555555-5555-5555-5555-555555555555";
    let before_rename = std::fs::read_to_string(&record).unwrap();
    assert!(
        before_rename.contains(&format!("decision={}", term_uuid)),
        "expected the question's terms line to pair by identifier: {}",
        before_rename
    );

    std::fs::write(&terms, TERMS_AFTER_RENAME_ALIAS_KEPT).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve should have allowed the rename: {}",
        stderr(&out)
    );

    // The identifier carries the meaning: the answered question's stored
    // terms line is untouched by the rename (still `decision=<uuid>`, the
    // old name, now an alias rather than the term's `## ` heading), and
    // `check` reads that identifier, not the name, so the pairing still
    // holds.
    let after_rename = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        before_rename, after_rename,
        "a terms.md rename must not touch the answered question's own file"
    );

    let check_out = adr(&dir, &["check"], &[], "");
    let text = stdout(&check_out);
    assert!(
        !text
            .lines()
            .any(|l| l.starts_with("terms:") && l.contains("What is the decision?")),
        "a rename that keeps the old name as an alias must not break the answered question's pairing: {}",
        text
    );
}

const TERMS_ONE_RETIRED_ONE_CURRENT_SAME_WORD: &str = "# Terms\n\n\
## decision\n\
id: 66666666-6666-6666-6666-666666666666\n\
definition: An old fixture term.\n\
aliases: \n\
retired: yes\n\
\n\
## verdict\n\
id: 77777777-7777-7777-7777-777777777777\n\
definition: A ruling reached after deliberation.\n\
aliases: decision\n";

/// 9.3 B: a retired entry's name/alias no longer belongs to it; a current
/// entry may take it, and `approve` (which parses and shape-checks the
/// file) does not refuse over the shared word.
#[test]
fn approve_allows_a_current_term_to_take_a_retired_terms_name_or_alias() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(
        &dir,
        "adrdom/terms.md",
        TERMS_ONE_RETIRED_ONE_CURRENT_SAME_WORD,
    );

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve should have allowed the retired term's word to be reused: {}",
        stderr(&out)
    );
}

const TERMS_TWO_CURRENT_SAME_WORD: &str = "# Terms\n\n\
## decision\n\
id: 88888888-8888-8888-8888-888888888888\n\
definition: A choice answered by the owner.\n\
aliases: verdict\n\
\n\
## verdict\n\
id: 99999999-9999-9999-9999-999999999999\n\
definition: A ruling reached after deliberation.\n\
aliases: \n";

/// Unlike a retired entry, two current entries sharing a word still
/// refuse (9.3 B only excuses a retired entry).
#[test]
fn approve_refuses_two_current_entries_sharing_a_word() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_TWO_CURRENT_SAME_WORD);

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("word \"verdict\""), "{}", err);
}

const TERMS_CROSS_REF: &str = "# Terms\n\n\
## decision\n\
id: 66666666-6666-6666-6666-666666666666\n\
definition: A choice bound by a question.\n\
aliases: \n\
\n\
## question\n\
id: 77777777-7777-7777-7777-777777777777\n\
definition: A thing the owner is asked to decide.\n\
aliases: \n";

#[test]
fn approve_writes_a_terms_line_per_term_from_its_definition() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_CROSS_REF);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());

    let content = std::fs::read_to_string(&terms).unwrap();
    assert!(
        content.contains("terms: question=77777777-7777-7777-7777-777777777777"),
        "{}",
        content
    );
    // The term whose definition pairs nothing gets no terms: line.
    let question_block = &content[content.find("## question").unwrap()..];
    assert!(!question_block.contains("terms: "), "{}", question_block);
}

// --- approve terms (4): a term's own terms: line is the recorded pairing,
// read as a dependent exactly like a decision's frozen line -- as it stood
// in the file before this approve, never re-resolved from today's
// definition text.

const TERMS_ENTRY_WITH_BOGUS_LINE: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice answered by the owner.\n\
aliases: \n\
\n\
## question\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A thing bound to a decision.\n\
aliases: \n\
terms: bogus=99999999-9999-9999-9999-999999999999\n";

#[test]
fn approve_refuses_an_agent_written_bogus_terms_line_in_a_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_ENTRY_WITH_BOGUS_LINE);

    // "question"'s terms: line is read as its recorded pairing (9.3 B):
    // term 99999999-... doesn't exist, so approve refuses "removed but
    // carried by term \"question\"", the same as a decision naming a
    // removed term would. Nothing is written.
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(
            "term 99999999-9999-9999-9999-999999999999 (\"bogus\") removed but used by term \"question\""
        ),
        "{}",
        err
    );

    let content = std::fs::read_to_string(&terms).unwrap();
    assert_eq!(content, TERMS_ENTRY_WITH_BOGUS_LINE);
}

const TERMS_ENTRY_PAIRS_ANOTHER_V1: &str = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A choice answered by the owner.\n\
aliases: \n\
\n\
## question\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A thing bound to a decision.\n\
aliases: \n";

// "question" keeps the machine-written `terms:` line the first approve
// gave it (pairing "decision" to its id): that recorded line, not a fresh
// re-resolve, is what the second approve's dependents check reads.
const TERMS_ENTRY_PAIRS_ANOTHER_RETIRED: &str = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A choice answered by the owner.\n\
aliases: \n\
retired: yes\n\
\n\
## question\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A thing bound to a decision.\n\
aliases: \n\
terms: decision=33333333-3333-3333-3333-333333333333\n";

#[test]
fn approve_refuses_retiring_a_term_carried_by_another_terms_definition() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_ENTRY_PAIRS_ANOTHER_V1);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());

    // "question"'s definition is untouched and still says "a decision";
    // only "decision" is retired. (The recorded terms: line below is
    // exactly what that first approve wrote for "question".)
    std::fs::write(&terms, TERMS_ENTRY_PAIRS_ANOTHER_RETIRED).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(
            "term \"decision\" retired but used by term \"question\"; agent rewords the definition to drop the retired word, owner runs: adr approve adrdom/terms.md"
        ),
        "{}",
        err
    );
}

// Retiring "decision" and, in the same approve, adding a new current term
// that takes its freed word: "question" still carries the retired id via
// its own recorded terms: line, so this refuses exactly as it does without
// the new term -- taking the word does not make the retirement any safer,
// since it is "question"'s *old* pairing, not today's word lookup, that is
// being protected.
const TERMS_ENTRY_PAIRS_ANOTHER_RETIRED_WORD_TAKEN: &str = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A choice answered by the owner.\n\
aliases: \n\
retired: yes\n\
\n\
## verdict\n\
id: 55555555-5555-5555-5555-555555555555\n\
definition: A ruling reached after deliberation.\n\
aliases: decision\n\
\n\
## question\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A thing bound to a decision.\n\
aliases: \n\
terms: decision=33333333-3333-3333-3333-333333333333\n";

#[test]
fn approve_refuses_retiring_a_term_while_a_new_term_takes_its_word_and_another_still_carries_it() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_ENTRY_PAIRS_ANOTHER_V1);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());

    // "decision" retires, "verdict" is a brand new current term taking the
    // freed word "decision" as an alias, and "question" keeps the recorded
    // terms: line the first approve wrote for it, still naming "decision"'s
    // id.
    std::fs::write(&terms, TERMS_ENTRY_PAIRS_ANOTHER_RETIRED_WORD_TAKEN).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(
            "term \"decision\" retired but used by term \"question\"; agent rewords the definition to drop the retired word, owner runs: adr approve adrdom/terms.md"
        ),
        "{}",
        err
    );
}
