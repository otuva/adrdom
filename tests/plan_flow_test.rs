// End-to-end walk of the whole plan lifecycle through the built binary:
// approve derives and caps tiers, ask enforces
// the derived tier order plus mandatory/moot answer sets and writes
// `assumes` rows, answer accepts a refusal, check freezes a dependent's
// answer set and reports moot notes/done, and a plan change (dropping
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

// --- state 0: three questions, all skeleton (heading + depends only) ------
const STATE_0: &str = r#"# Widget

### 1.1 What color is the widget?
depends: widget

### 1.2 Does the paint match the widget's color?
depends: paint, 1.1 What color is the widget?

### 1.3 Does the paint need a second coat?
depends: paint, 1.1 What color is the widget?
"#;

// `approve_terms_with(&dir, &["widget", "paint"])` (tests/common/mod.rs)
// assigns each word a fixed id in array order: "widget" is index 0,
// "paint" is index 1.
const WIDGET_ID: &str = "99999990-9999-9999-9999-999999999999";
const PAINT_ID: &str = "99999991-9999-9999-9999-999999999999";

/// `approve` rewrites every `Text` depends entry to the id it resolves to
/// -- a term entry to the term's id, a heading entry to the target
/// heading's own id -- so every marker/insert pair below that edits an
/// already-approved file is built from those ids, never the original words.
/// Each marker also carries the heading's own id line, which `approve` now
/// writes directly after the (single, comma-separated) depends line rather
/// than before it, so every marker below needs that
/// section's own id too, not just the ids its depends line names.
fn q1_marker(q1_id: &str) -> String {
    format!("depends: {}\nid: {}\n\n### 1.2", WIDGET_ID, q1_id)
}
fn q1_body_insert(q1_id: &str) -> String {
    format!(
        "depends: {}\nid: {}\n\n\
Problem: the widget's color is undecided.\n\n\
Standard: plain words.\n\n\
- A) The widget is red.\n  Why: reasons.\n  Costs: costs.\n\
- B) The widget is blue.\n  Why: other reasons.\n  Costs: other costs.\n\n\
### 1.2",
        WIDGET_ID, q1_id
    )
}

fn q2_set_marker(q1_id: &str, q2_id: &str) -> String {
    format!("depends: {}, {}\nid: {}\n\n### 1.3", PAINT_ID, q1_id, q2_id)
}
fn q2_set_insert(q1_id: &str, q2_id: &str) -> String {
    format!("depends: {}, {}=A\nid: {}\n\n### 1.3", PAINT_ID, q1_id, q2_id)
}

fn q3_set_marker(q1_id: &str, q3_id: &str) -> String {
    format!("depends: {}, {}\nid: {}\n", PAINT_ID, q1_id, q3_id)
}
fn q3_set_insert(q1_id: &str, q3_id: &str) -> String {
    format!("depends: {}, {}=*\nid: {}\n", PAINT_ID, q1_id, q3_id)
}

fn q2_marker(q1_id: &str, q2_id: &str) -> String {
    format!("depends: {}, {}=A\nid: {}\n\n### 1.3", PAINT_ID, q1_id, q2_id)
}
fn q2_body_insert(q1_id: &str, q2_id: &str) -> String {
    format!(
        "depends: {}, {}=A\nid: {}\n\n\
Problem: the paint might not match.\n\n\
Standard: plain words.\n\n\
- A) The paint matches.\n  Why: reasons.\n  Costs: costs.\n\
- B) The paint does not match.\n  Why: other reasons.\n  Costs: other costs.\n\n\
### 1.3",
        PAINT_ID, q1_id, q2_id
    )
}

fn q3_tail_marker(q1_id: &str, q3_id: &str) -> String {
    format!("depends: {}, {}=*\nid: {}\n", PAINT_ID, q1_id, q3_id)
}
fn q3_body_insert(q1_id: &str, q3_id: &str) -> String {
    format!(
        "depends: {}, {}=*\nid: {}\n\n\
Problem: second coat need is undecided.\n\n\
Standard: plain words.\n\n\
- A) A second coat of paint is needed.\n  Why: reasons.\n  Costs: costs.\n\
- B) A second coat of paint is not needed.\n  Why: other reasons.\n  Costs: other costs.\n",
        PAINT_ID, q1_id, q3_id
    )
}

const Q4_BLOCK: &str = "\n### 1.4 Does the paint have a glossy finish?\n\
depends: paint, 1.1 What color is the widget?=B\n\n\
Problem: the paint's finish is undecided.\n\n\
Standard: plain words.\n\n\
- A) The finish is glossy.\n  Why: reasons.\n  Costs: costs.\n\
- B) The finish is matte.\n  Why: other reasons.\n  Costs: other costs.\n";

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
        .unwrap_or_else(|| panic!("no id found for heading {:?} in: {}", heading_prefix, content))
        .to_string()
}

// adr: f01e20ae-00fc-4ec9-a25b-88a90dffc624
//
// "one approval covers the headings and their depends lines; the question
// bodies are written after it, one tier at a time, with no further
// approval". Step 1 below approves STATE_0 -- headings and depends lines
// only, every body still a skeleton -- in a single `approve` call; step 2
// then grows Q1's skeleton into a full body and Q2/Q3's depends lines into
// answer sets, and `check` confirms nothing is "awaiting approval" (no
// second approve happened), and the very next `ask` succeeds straight
// through on that body -- accepted with no new approve in between.
//
// adr: fa90692a-150e-4a89-91e2-1d575cb59d0f
//
// core: "when a tier's bodies are written, every depends entry naming one
// of its questions gets the letters it holds under (=A, =A|B or =*); ask
// stops the tier until every such set is written and freezes the sets with
// the bodies ...; once the tier is answered, a question whose set lacks the
// answer is moot: ask stops it, check reports it, and it leaves at the next
// plan change (approve accepts dropping it)". This single walk shows every
// clause: step 2 has `ask` refuse Q2 for missing an answer set, then freeze
// Q2's `=A` and Q3's `=*` the moment `ask` asks Q1; step 5 has `check` fail
// once Q2's frozen `=A` is edited to `=B` after that ask (freezing holds);
// step 4 has `check` report "moot: ... stands on ... =A, answered B" and
// `ask` refuse the same moot Q2 outright; step 7 drops the still-moot Q2
// (and the answered-but-refused Q3) at the next `approve`, which accepts it
// and prints "approve: withdrawn: 1.3 ...".
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

    // approve gave every heading (all three, skeletons included) an id.
    let state_0_approved = std::fs::read_to_string(&record).unwrap();
    assert_eq!(extract_ids(&state_0_approved).len(), 3, "{}", state_0_approved);
    let q1_id = id_for_heading(&state_0_approved, "### 1.1");
    let q2_id = id_for_heading(&state_0_approved, "### 1.2");
    let q3_id = id_for_heading(&state_0_approved, "### 1.3");

    // --- 2. ask refuses without answer sets; sets alone need no re-approve -
    // Grow Q1's skeleton into a full body, editing the id-bearing file in
    // place (not overwriting it with a separate id-less constant, which
    // would make the id vanish and look like a plan change).
    let state_1 = state_0_approved.replacen(&q1_marker(&q1_id), &q1_body_insert(&q1_id), 1);
    assert_ne!(state_1, state_0_approved, "Q1 marker must be found");
    std::fs::write(&record, &state_1).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/widget.md: \"1.2 Does the paint match the widget's color?\" depends on \"1.1 What color is the widget?\" without an answer set; agent writes =A, =A|B or =* on that depends line\n\
         ask: adrdom/decisions/widget.md: \"1.3 Does the paint need a second coat?\" depends on \"1.1 What color is the widget?\" without an answer set; agent writes =A, =A|B or =* on that depends line"
    );
    // Nothing written on refusal.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, state_1, "ask must not modify the file when it refuses");

    // Q2 gets =A, Q3 gets =* (sets only, target unchanged).
    let state_2 = state_1.replacen(&q2_set_marker(&q1_id, &q2_id), &q2_set_insert(&q1_id, &q2_id), 1);
    assert_ne!(state_2, state_1, "Q2 set marker must be found");
    let state_2 = state_2.replacen(&q3_set_marker(&q1_id, &q3_id), &q3_set_insert(&q1_id, &q3_id), 1);
    std::fs::write(&record, &state_2).unwrap();

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
    assert_eq!(id_for_heading(&state_3, "### 1.1"), q1_id);
    let ledger_after_ask1 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(asked_row_count(&dir), 1, "{}", ledger_after_ask1);
    assert_eq!(
        ledger_after_ask1.lines().filter(|l| l.contains("\tassumes\t")).count(),
        2,
        "{}",
        ledger_after_ask1
    );

    // --- 3. a candidate outside the open tier is refused, blocker named ---
    let with_q2_body = state_3.replacen(&q2_marker(&q1_id, &q2_id), &q2_body_insert(&q1_id, &q2_id), 1);
    assert_ne!(with_q2_body, state_3, "Q2 marker must be found");
    std::fs::write(&record, &with_q2_body).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        format!(
            "ask: adrdom/decisions/widget.md: \"1.2 Does the paint match the widget's color?\" is in tier 2; tier 1 is open, waiting on \"1.1 What color is the widget?\"; owner runs: adr answer {}@{} <letter> on it first",
            q1_id,
            current_version(&dir, &q1_id)
        )
    );

    // Remove that body again.
    std::fs::write(&record, &state_3).unwrap();

    // --- 4. owner answers Q1; Q2's frozen set A is now moot; Q3's * asks --
    let out = adr_answer(&dir, &q1_id, "B");
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

    let with_q2_body_2 = state_5.replacen(&q2_marker(&q1_id, &q2_id), &q2_body_insert(&q1_id, &q2_id), 1);
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
    let with_q3_body = state_5.replacen(&q3_tail_marker(&q1_id, &q3_id), &q3_body_insert(&q1_id, &q3_id), 1);
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
    // All three headings (Q1 asked/answered, Q2 still a skeleton, Q3 just
    // asked) already carry an id from the very first approve.
    assert_eq!(extract_ids(&state_7).len(), 3, "{}", state_7);
    assert_eq!(id_for_heading(&state_7, "### 1.1"), q1_id);
    assert_eq!(id_for_heading(&state_7, "### 1.3"), q3_id);
    assert_eq!(asked_row_count(&dir), 2, "Q1 and Q3 should be asked; Q2 stays a skeleton");

    // --- 5. changing the frozen set after ask is a check failure ----------
    let set_changed = state_7.replacen(
        &format!("depends: {}, {}=A", PAINT_ID, q1_id),
        &format!("depends: {}, {}=B", PAINT_ID, q1_id),
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
    let out = adr_answer(&dir, &q3_id, "refuse");
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

    let out = adr(&dir, &["show", "--all"], &[], "");
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
        "Q2 was never asked, nothing to withdraw: {}",
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
    // Only Q1 and Q4 remain in the file at all now (Q2/Q3 were dropped, not
    // merely left unasked), so exactly two ids remain.
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
    assert!(
        stdout(&out).contains(&format!("done: {}", REL)),
        "{}",
        stdout(&out)
    );
}
