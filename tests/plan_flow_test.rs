// End-to-end walk of the whole plan lifecycle through the built binary:
// approve derives and caps tiers, ask enforces
// the derived tier order plus mandatory answer sets and writes
// `assumes` rows, answer accepts a refusal, check freezes a dependent's
// answer set and reports done, and a plan change (dropping
// questions, adding one) withdraws the dropped id at the next approve.
//
// One shared fixture walks the whole story rather than one test per
// behaviour, since each step's file state depends on the ids/rows the
// previous step minted.
//
// `approve` -- not `ask` -- mints every heading's id now, all at
// once, skeletons included, and rewrites a heading-shaped `depends:` line
// to that id. Every step below that "grows" a heading (skeleton into a
// full body, or a set onto a depends line) does so by editing the file as
// it currently stands on disk (already carrying every heading's id from
// the first approve), never by overwriting it with a separate id-less text
// constant -- that would make `approve`/`ask` see a genuine plan change
// (an id vanishing) and refuse for the wrong reason. `extract_ids` also no
// longer tracks "how many questions have been asked" (every heading gets
// an id up front): a heading's id is looked up by its own heading text
// instead.
mod common;

use common::*;

const REL: &str = "adrdom/decisions/widget.md";

// --- state 0: three questions, every body written, every dependent with an
// answer set (approve refuses a body-less heading and a set-less dependent).
const STATE_0: &str = r#"# Widget

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
depends: paint, 1.1 What color is the widget?=A

Problem: the paint might not match.

Standard: plain words.

- A) Yes.
  Why: reasons.
  Costs: costs.
- B) No.
  Why: other reasons.
  Costs: other costs.

### 1.3 Does the paint need a second coat?
depends: paint, 1.1 What color is the widget?=*

Problem: second coat need is undecided.

Standard: plain words.

- A) Yes.
  Why: reasons.
  Costs: costs.
- B) No.
  Why: other reasons.
  Costs: other costs.
"#;

// `approve_terms_with(&dir, &["widget", "paint"])` (tests/common/mod.rs)
// assigns each word a fixed id in array order: "widget" is index 0,
// "paint" is index 1.
const PAINT_ID: &str = "99999991-9999-9999-9999-999999999999";

const Q4_BLOCK: &str = "\n### 1.4 Does the paint have a glossy finish?\n\
depends: paint, 1.1 What color is the widget?\n\n\
Problem: the paint's finish is undecided.\n\n\
Standard: plain words.\n\n\
- A) Yes.\n  Why: reasons.\n  Costs: costs.\n\
- B) No.\n  Why: other reasons.\n  Costs: other costs.\n";

/// The `id: <uuid>` value under the first heading in `content` starting
/// with `heading_prefix` -- a heading's id looked up by its own text, since
/// `approve` now gives every heading (skeleton included) one up front, so
/// counting or positionally indexing `extract_ids` no longer tells "was
/// this one asked" apart from "does it merely exist".
fn id_for_heading(content: &str, heading_prefix: &str) -> String {
    content
        .lines()
        .skip_while(|l| !l.starts_with(heading_prefix))
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap_or_else(|| {
            panic!(
                "no id found for heading {:?} in: {}",
                heading_prefix, content
            )
        })
        .to_string()
}

//
// Walk: approve one plan with every body and answer set written; ask asks
// the whole file and freezes the sets; the next ask has nothing to ask; after
// the owner answers B, the dependent on =A leaves the file and the =*
// dependent stays; a frozen set edited after ask fails `check`; a
// plan change drops Q3 and adds Q4, withdrawing the asked Q3.
// The tier the tool derives for each question of the topic file at `path`,
// in file order.
fn tiers_of(path: &std::path::Path) -> Vec<usize> {
    let rf = adr::record::load(path).unwrap();
    (0..rf.sections.len())
        .map(|i| rf.tier_of(i).unwrap())
        .collect()
}

#[test]
fn plan_lifecycle_end_to_end() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget", "paint"]);

    // --- 1. the tool derives the tiers from the depends lines ---------------
    let record = write_record(&dir, REL, STATE_0);
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    assert_eq!(tiers_of(&record), vec![1, 2, 2]);

    // approve gave every heading (all three) an id.
    let state_0_approved = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        extract_ids(&state_0_approved).len(),
        3,
        "{}",
        state_0_approved
    );
    let q1_id = id_for_heading(&state_0_approved, "### 1.1");
    let q2_id = id_for_heading(&state_0_approved, "### 1.2");
    let q3_id = id_for_heading(&state_0_approved, "### 1.3");
    let _ = &q2_id;

    // --- 2. ask asks the file and writes the assumes rows --------------------
    let check_out = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&check_out).contains("awaiting approval"),
        "{}",
        stdout(&check_out)
    );

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask should succeed: {}", stderr(&out));
    let printed = stdout(&out);
    // One line per row written: three asked rows and two assumes rows.
    for line in [
        "asked: \"1.1 What color is the widget?\"",
        "assumes: \"1.2 Does the paint match the widget's color?\" stands on \"1.1 What color is the widget?\" under its answer set",
        "assumes: \"1.3 Does the paint need a second coat?\" stands on \"1.1 What color is the widget?\" under its answer set",
    ] {
        assert!(printed.lines().any(|l| l == line), "{}\n{}", line, printed);
    }
    assert_eq!(printed.lines().count(), 5, "{}", printed);

    let state_3 = std::fs::read_to_string(&record).unwrap();
    assert_eq!(id_for_heading(&state_3, "### 1.1"), q1_id);
    let ledger_after_ask1 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        printed
            .lines()
            .any(|l| l == "asked: \"1.2 Does the paint match the widget's color?\"")
            && printed
                .lines()
                .any(|l| l == "asked: \"1.3 Does the paint need a second coat?\""),
        "one run asks every tier: {}",
        printed
    );
    assert_eq!(asked_row_count(&dir), 3, "{}", ledger_after_ask1);
    assert_eq!(
        ledger_after_ask1
            .lines()
            .filter(|l| l.contains("\tassumes\t"))
            .count(),
        2,
        "{}",
        ledger_after_ask1
    );

    // --- 3. what is left names the owner's answer and what waits for it ----
    // ask with nothing to ask writes nothing and prints nothing; show names
    // the moves.
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        out.status.success(),
        "ask should have nothing to ask: {}",
        stderr(&out)
    );
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "", "{}", stdout(&out));
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    for mv in [
        format!(
            "owner runs: adr answer {}@{} <letter>  # \"1.1 What color is the widget?\"",
            q1_id,
            current_version(&dir, &q1_id)
        ),
        "not yet: \"1.2 Does the paint match the widget's color?\" is asked and waits for \"1.1 What color is the widget?\"".to_string(),
        "not yet: \"1.3 Does the paint need a second coat?\" is asked and waits for \"1.1 What color is the widget?\"".to_string(),
    ] {
        assert!(shown.lines().any(|l| l.trim() == mv), "{}\n{}", mv, shown);
    }
    assert_eq!(asked_row_count(&dir), 3, "nothing new is asked");

    // --- 5. changing the frozen set after ask is a check failure ----------
    let set_changed = state_3.replacen(
        &format!("depends: {}, {}=A", PAINT_ID, q1_id),
        &format!("depends: {}, {}=B", PAINT_ID, q1_id),
        1,
    );
    assert_ne!(set_changed, state_3, "Q2's =A line must be found");
    std::fs::write(&record, &set_changed).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        !out.status.success(),
        "check should have failed: {}",
        stdout(&out)
    );
    assert!(
        stdout(&out).contains(
            "answer set on 1.1 What color is the widget? changed after ask; agent restores the answer set from git"
        ),
        "{}",
        stdout(&out)
    );

    // Restore =A.
    std::fs::write(&record, &state_3).unwrap();

    // --- 4. owner answers Q1; the asked Q2 (set A) leaves with the answer; Q3 stays --
    let out = adr_answer(&dir, &q1_id, "B");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    write_test_comment(&dir, &q1_id);

    // The answer removed Q2 (asked, its set lacks B); the file stays approved
    // and check passes.
    let state_5 = std::fs::read_to_string(&record).unwrap();
    assert!(
        !state_5.contains("### 1.2 Does the paint match the widget's color?"),
        "{}",
        state_5
    );
    let check_out = adr(&dir, &["check"], &[], "");
    assert!(
        check_out.status.success(),
        "{}{}",
        stdout(&check_out),
        stderr(&check_out)
    );
    assert!(
        !stdout(&check_out).contains("no case left"),
        "{}",
        stdout(&check_out)
    );

    // Q3 was asked in the first run: nothing is left to ask.
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        out.status.success(),
        "ask should have nothing to ask: {}",
        stderr(&out)
    );
    assert_eq!(stdout(&out), "", "{}", stdout(&out));

    let state_7 = std::fs::read_to_string(&record).unwrap();
    assert_eq!(extract_ids(&state_7).len(), 2, "{}", state_7);
    assert_eq!(id_for_heading(&state_7, "### 1.1"), q1_id);
    assert_eq!(id_for_heading(&state_7, "### 1.3"), q3_id);
    assert_eq!(
        asked_row_count(&dir),
        3,
        "Q1, Q2 and Q3 were asked; Q2 was removed"
    );

    // --- 6. owner refuses Q3 ------------------------------------------------
    let out = adr_answer(&dir, &q3_id, "refuse");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));

    let state_8 = std::fs::read_to_string(&record).unwrap();
    assert!(state_8.contains("**Answer:** refused"), "{}", state_8);
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger
            .lines()
            .any(|l| l.contains("\trefused\t") && l.contains(q3_id.as_str())),
        "{}",
        ledger
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        out.status.success(),
        "check should pass: {}",
        stderr(&out).to_string() + &stdout(&out)
    );
    // The file is not finished while Q2 is open: show still names a move.
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert!(
        shown.lines().any(|l| l.starts_with("agent ")
            || l.starts_with("owner runs:")
            || l.starts_with("not yet")),
        "a move is left while Q2 is open: {}",
        shown
    );

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(
        stdout(&out).contains("[refused] 1.3 Does the paint need a second coat?"),
        "{}",
        stdout(&out)
    );

    // --- 7. plan change: drop Q3, add Q4 ------------------------------------
    let cut = state_8
        .find("### 1.3 Does the paint need a second coat?")
        .unwrap();
    let mut state_9 = state_8[..cut].trim_end().to_string();
    state_9.push('\n');
    state_9.push_str(Q4_BLOCK);
    std::fs::write(&record, &state_9).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    let printed = stdout(&out);
    assert!(
        printed
            .lines()
            .any(|l| l == "withdrawn: \"1.3 Does the paint need a second coat?\""),
        "{}",
        printed
    );
    assert!(
        !printed.contains("1.2 Does the paint match the widget's color?"),
        "Q2 was removed by an answer, nothing to withdraw: {}",
        printed
    );
    assert_eq!(tiers_of(&record), vec![1, 2]);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        out.status.success(),
        "ask should succeed on Q4: {}",
        stderr(&out)
    );
    assert!(
        stdout(&out)
            .lines()
            .any(|l| l == "asked: \"1.4 Does the paint have a glossy finish?\""),
        "{}",
        stdout(&out)
    );

    // --- 8. owner answers Q4; tie tests; the file is done -------------------
    let state_10 = std::fs::read_to_string(&record).unwrap();
    // Only Q1 and Q4 remain in the file at all now, so exactly two ids remain.
    let ids_10 = extract_ids(&state_10);
    assert_eq!(ids_10.len(), 2, "{}", state_10);
    assert_eq!(id_for_heading(&state_10, "### 1.1"), q1_id);
    let q4_id = id_for_heading(&state_10, "### 1.4");

    let out = adr_answer(&dir, &q4_id, "A");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));

    write_test_comment(&dir, &q1_id);
    write_test_comment(&dir, &q4_id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    // Every question answered and tested: a pass prints nothing (bd83aa22).
    assert_eq!(stdout(&out), "", "{}", stdout(&out));
}
