// End-to-end walk of the whole plan lifecycle (plan-build-spec.md §1-§6)
// through the built binary: approve derives and caps tiers, ask enforces
// the derived tier order plus mandatory/moot answer sets and writes
// `assumes` rows, answer accepts a refusal, check freezes a dependent's
// answer set and reports moot notes/done, and a plan change (dropping
// questions, adding one) withdraws the dropped id at the next approve.
//
// One shared fixture walks the whole story rather than one test per
// behaviour, since each step's file state depends on the ids/rows the
// previous step minted.
mod common;

use common::*;

const REL: &str = "adrdom/decisions/widget.md";

// --- state 0: three questions, all skeleton (heading + depends only) ------
const STATE_0: &str = r#"# Widget

### 1.1 What color is the widget?
depends: widget

### 1.2 Does the paint match the widget's color?
depends: paint
depends: 1.1 What color is the widget?

### 1.3 Does the paint need a second coat?
depends: paint
depends: 1.1 What color is the widget?
"#;

// --- state 1: Q1 gets a body; Q2/Q3 still skeleton, no sets yet -----------
const STATE_1: &str = r#"# Widget

### 1.1 What color is the widget?
depends: widget

Problem: the widget's color is undecided.

Standard: plain words.

- A) The widget is red.
  Why: reasons.
  Costs: costs.
- B) The widget is blue.
  Why: other reasons.
  Costs: other costs.

### 1.2 Does the paint match the widget's color?
depends: paint
depends: 1.1 What color is the widget?

### 1.3 Does the paint need a second coat?
depends: paint
depends: 1.1 What color is the widget?
"#;

// --- state 2: Q2 gets =A, Q3 gets =* (sets only, target unchanged) -------
const STATE_2: &str = r#"# Widget

### 1.1 What color is the widget?
depends: widget

Problem: the widget's color is undecided.

Standard: plain words.

- A) The widget is red.
  Why: reasons.
  Costs: costs.
- B) The widget is blue.
  Why: other reasons.
  Costs: other costs.

### 1.2 Does the paint match the widget's color?
depends: paint
depends: 1.1 What color is the widget?=A

### 1.3 Does the paint need a second coat?
depends: paint
depends: 1.1 What color is the widget?=*
"#;

const Q2_MARKER: &str = "depends: 1.1 What color is the widget?=A\n\n### 1.3";
const Q2_BODY_INSERT: &str = "depends: 1.1 What color is the widget?=A\n\n\
Problem: the paint might not match.\n\n\
Standard: plain words.\n\n\
- A) The paint matches.\n  Why: reasons.\n  Costs: costs.\n\
- B) The paint does not match.\n  Why: other reasons.\n  Costs: other costs.\n\n\
### 1.3";

const Q3_TAIL_MARKER: &str = "depends: 1.1 What color is the widget?=*\n";
const Q3_BODY_INSERT: &str = "depends: 1.1 What color is the widget?=*\n\n\
Problem: second coat need is undecided.\n\n\
Standard: plain words.\n\n\
- A) A second coat of paint is needed.\n  Why: reasons.\n  Costs: costs.\n\
- B) A second coat of paint is not needed.\n  Why: other reasons.\n  Costs: other costs.\n";

const Q4_BLOCK: &str = "\n### 1.4 Does the paint have a glossy finish?\n\
depends: paint\n\
depends: 1.1 What color is the widget?=B\n\n\
Problem: the paint's finish is undecided.\n\n\
Standard: plain words.\n\n\
- A) The finish is glossy.\n  Why: reasons.\n  Costs: costs.\n\
- B) The finish is matte.\n  Why: other reasons.\n  Costs: other costs.\n";

#[test]
fn plan_lifecycle_end_to_end() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget", "paint"]);

    // --- 1. approve derives and caps the tiers -----------------------------
    let record = write_record(&dir, REL, STATE_0);
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    assert!(
        stdout(&out).contains("approve: tiers: 1 (1) 2 (2)"),
        "{}",
        stdout(&out)
    );

    // --- 2. ask refuses without answer sets; sets alone need no re-approve -
    write_record(&dir, REL, STATE_1);
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/widget.md: \"1.2 Does the paint match the widget's color?\" depends on \"1.1 What color is the widget?\" without an answer set; agent writes =A, =A|B or =* on that depends line"
    );
    // Nothing written on refusal.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, STATE_1, "ask must not modify the file when it refuses");

    write_record(&dir, REL, STATE_2);
    let check_out = adr(&dir, &["check"], &[], "");
    let check_text = stdout(&check_out);
    assert!(
        !check_text.contains("awaiting approval"),
        "changing only the answer sets must not require re-approval: {}",
        check_text
    );

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask should now succeed: {}", stderr(&out));
    let printed = stdout(&out);
    assert!(
        printed.contains("asked: \"1.1 What color is the widget?\" (tier 1)"),
        "{}",
        printed
    );
    assert!(
        printed.contains("  assumes: \"1.2 Does the paint match the widget's color?\"=A"),
        "{}",
        printed
    );
    assert!(
        printed.contains("  assumes: \"1.3 Does the paint need a second coat?\"=*"),
        "{}",
        printed
    );

    let state_3 = std::fs::read_to_string(&record).unwrap();
    let q1_id = extract_ids(&state_3)[0].clone();
    let ledger_after_ask1 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger_after_ask1.lines().filter(|l| l.contains("\tasked\t")).count(),
        1,
        "{}",
        ledger_after_ask1
    );
    assert_eq!(
        ledger_after_ask1.lines().filter(|l| l.contains("\tassumes\t")).count(),
        2,
        "{}",
        ledger_after_ask1
    );

    // --- 3. a candidate outside the open tier is refused, blocker named ---
    let with_q2_body = state_3.replacen(Q2_MARKER, Q2_BODY_INSERT, 1);
    assert_ne!(with_q2_body, state_3, "Q2 marker must be found");
    std::fs::write(&record, &with_q2_body).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        format!(
            "ask: adrdom/decisions/widget.md: \"1.2 Does the paint match the widget's color?\" is in tier 2; tier 1 is open, waiting on \"1.1 What color is the widget?\"; owner runs: adr answer {} <letter> on it first",
            q1_id
        )
    );

    // Remove that body again.
    std::fs::write(&record, &state_3).unwrap();

    // --- 4. owner answers Q1; Q2's frozen set A is now moot; Q3's * asks --
    let out = adr_owner(&dir, &["answer", &q1_id, "B"], "");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    write_test_comment(&dir, &q1_id);

    let state_5 = std::fs::read_to_string(&record).unwrap();
    let check_out = adr(&dir, &["check"], &[], "");
    assert!(
        stdout(&check_out).contains(
            "moot: adrdom/decisions/widget.md: \"1.2 Does the paint match the widget's color?\" stands on \"1.1 What color is the widget?\"=A, answered B"
        ),
        "{}",
        stdout(&check_out)
    );

    let with_q2_body_2 = state_5.replacen(Q2_MARKER, Q2_BODY_INSERT, 1);
    assert_ne!(with_q2_body_2, state_5, "Q2 marker must be found");
    std::fs::write(&record, &with_q2_body_2).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/widget.md: \"1.2 Does the paint match the widget's color?\" is moot: stands on \"1.1 What color is the widget?\"=A, answered B; agent drops or replaces the heading, owner runs: adr approve adrdom/decisions/widget.md"
    );

    // Revert Q2 to skeleton again, then give Q3 a body: it asks fine.
    std::fs::write(&record, &state_5).unwrap();
    let with_q3_body = state_5.replacen(Q3_TAIL_MARKER, Q3_BODY_INSERT, 1);
    assert_ne!(with_q3_body, state_5, "Q3 marker must be found");
    std::fs::write(&record, &with_q3_body).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask should now succeed: {}", stderr(&out));
    assert!(
        stdout(&out).contains("asked: \"1.3 Does the paint need a second coat?\" (tier 2)"),
        "{}",
        stdout(&out)
    );

    let state_7 = std::fs::read_to_string(&record).unwrap();
    let ids_7 = extract_ids(&state_7);
    assert_eq!(ids_7.len(), 2, "{}", state_7);
    assert_eq!(ids_7[0], q1_id);
    let q3_id = ids_7[1].clone();

    // --- 5. changing the frozen set after ask is a check failure ----------
    let set_changed = state_7.replacen(
        "depends: 1.1 What color is the widget?=A",
        "depends: 1.1 What color is the widget?=B",
        1,
    );
    assert_ne!(set_changed, state_7, "Q2's =A line must be found");
    std::fs::write(&record, &set_changed).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "check should have failed: {}", stdout(&out));
    assert!(
        stdout(&out).contains(
            "answers: adrdom/decisions/widget.md: \"1.2 Does the paint match the widget's color?\" answer set on \"1.1 What color is the widget?\" changed after ask; agent restores it, a change is a plan change"
        ),
        "{}",
        stdout(&out)
    );

    // Restore =A.
    std::fs::write(&record, &state_7).unwrap();

    // --- 6. owner refuses Q3 ------------------------------------------------
    let out = adr_owner(&dir, &["answer", &q3_id, "refuse"], "");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));

    let state_8 = std::fs::read_to_string(&record).unwrap();
    assert!(state_8.contains("**Answer:** refused"), "{}", state_8);
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger.lines().any(|l| l.contains("\trefused\t") && l.contains(q3_id.as_str())),
        "{}",
        ledger
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stderr(&out).to_string() + &stdout(&out));
    assert!(
        !stdout(&out).contains(&format!("done: {}", REL)),
        "file must not be done while Q2 is unanswered: {}",
        stdout(&out)
    );

    let out = adr(&dir, &["show"], &[], "");
    assert!(
        stdout(&out).contains("[refused] 1.3 Does the paint need a second coat?"),
        "{}",
        stdout(&out)
    );

    // --- 7. plan change: drop Q2 and Q3, add Q4 -----------------------------
    let cut = state_8.find("### 1.2 Does the paint match the widget's color?").unwrap();
    let mut state_9 = state_8[..cut].trim_end().to_string();
    state_9.push('\n');
    state_9.push_str(Q4_BLOCK);
    std::fs::write(&record, &state_9).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    let printed = stdout(&out);
    assert!(
        printed.contains("approve: withdrawn: 1.3 Does the paint need a second coat?"),
        "{}",
        printed
    );
    assert!(
        !printed.contains("1.2 Does the paint match the widget's color?"),
        "Q2 never had an id, nothing to withdraw: {}",
        printed
    );
    assert!(printed.contains("approve: tiers: 1 (1) 2 (1)"), "{}", printed);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask should succeed on Q4: {}", stderr(&out));
    assert!(
        stdout(&out).contains("asked: \"1.4 Does the paint have a glossy finish?\" (tier 2)"),
        "{}",
        stdout(&out)
    );

    // --- 8. owner answers Q4; tie tests; the file is done -------------------
    let state_10 = std::fs::read_to_string(&record).unwrap();
    let ids_10 = extract_ids(&state_10);
    assert_eq!(ids_10.len(), 2, "{}", state_10);
    assert_eq!(ids_10[0], q1_id);
    let q4_id = ids_10[1].clone();

    let out = adr_owner(&dir, &["answer", &q4_id, "A"], "");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));

    write_test_comment(&dir, &q1_id);
    write_test_comment(&dir, &q4_id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(
        stdout(&out).contains(&format!("done: {}", REL)),
        "{}",
        stdout(&out)
    );
}
