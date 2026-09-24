// Wave 2, ask (spec §4): the derived tier rule, the cap, the two
// refused-dependency refusals (generic blocker + specific "stands on
// refused"), the moot refusal, the mandatory-set refusals, and the
// `assumes` rows/tier print that come out of a successful ask.
//
// 9.1 A: every fixture now goes through a real `adr approve` (`approve_record`)
// before `ask` touches it, since `approve` -- not `ask` any more -- is what
// mints a heading's id, and `ask` refuses outright on a section with none.
// Every heading here needs its own `depends:` line for `approve` to accept
// it (batch 8's pre-existing rule); `depends: widget` (a term
// `approve_minimal_terms` already sets up) covers a root-tier heading that
// has no other natural dependency. Because every heading already has an id
// the moment it is approved, `extract_ids(...).len()` no longer tracks
// "how many questions have been asked" -- a count of `\tasked\t` ledger
// rows, or a before/after file-content comparison, is what these tests use
// instead.
mod common;

use common::*;
use sha2::{Digest, Sha256};

fn sha256_hex(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

fn asked_row_count(dir: &std::path::Path) -> usize {
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default();
    ledger.lines().filter(|l| l.contains("\tasked\t")).count()
}

const Q1: &str = r#"# Example

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
"#;

fn q2_depends_on_q1(set: &str) -> String {
    format!(
        r#"
### 1.2 Does the widget's paint match its color{set_note}?

depends: 1.1 What color is the widget?{set}

Problem: the paint might not match.

Standard: plain words.

- A) The paint matches.
  Why: reasons.
  Costs: costs.
- B) The paint does not match.
  Why: other reasons.
  Costs: other costs.
"#,
        set = set,
        set_note = ""
    )
}

// --- tier order: blocked, then allowed once the blocker is answered -------

// adr: aec860fd-22ad-44a7-99f7-222ea19537e7
//
// "ask asks the lowest tier with an unanswered question (and stops a
// higher one)": Q2 (tier 2, derived from its `depends:` line, never
// written as a tier by hand) is refused while Q1's tier is still open, and
// only succeeds once Q1 is answered.
#[test]
fn ask_blocks_a_higher_tier_candidate_naming_the_open_tier_blocker() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let q1_id = extract_ids(&content)[0].clone();

    // Q1 is asked but not yet answered; add Q2, which stands on it with an
    // explicit set. Q2 is a candidate (never asked); Q1 is not (already
    // asked at its current hash), so Q1's tier alone is what's open.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    let before_refused_ask = std::fs::read_to_string(&record).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert_eq!(
        err.trim_end(),
        format!(
            "ask: adrdom/decisions/example.md: \"1.2 Does the widget's paint match its color?\" is in tier 2; tier 1 is open, waiting on \"1.1 What color is the widget?\"; owner runs: adr answer {} <letter> on it first",
            q1_id
        )
    );

    // Nothing written: ask wrote no new content and asked nothing new.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, before_refused_ask, "ask must not modify the file when it refuses");
    assert_eq!(asked_row_count(&dir), 1, "only Q1 should be asked so far");

    // Now answer Q1: the same ask on Q2 succeeds.
    let answer_out = adr_owner(&dir, &["answer", &q1_id, "A"], "");
    assert!(answer_out.status.success(), "{}", stderr(&answer_out));

    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "ask should now succeed: {}", stderr(&out2));
    assert_eq!(asked_row_count(&dir), 2, "Q2 should now be asked too");
}

#[test]
fn ask_prefers_the_specific_stands_on_refused_message_over_the_generic_blocker() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let q1_id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["answer", &q1_id, "refuse"], "")
        .status
        .success());

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    // Q2's own heading dep names Q1 directly, so the specific
    // "stands on refused" check (which runs ahead of the generic tier-order
    // blocker) is what fires here.
    assert_eq!(
        err.trim_end(),
        "ask: adrdom/decisions/example.md: \"1.2 Does the widget's paint match its color?\" stands on refused \"1.1 What color is the widget?\"; agent re-plans, owner runs: adr approve adrdom/decisions/example.md"
    );
}

#[test]
fn ask_names_the_generic_refused_blocker_when_the_candidate_does_not_depend_on_it() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");

    // Qx (refused) precedes Q1 (left plainly unanswered) in file order.
    // Q2 depends only on Q1, never on Qx, so the generic tier-order
    // blocker (picked by file order within the open tier) is what must
    // fire, naming Qx as refused.
    let content = r#"# Example

### 1.0 Is the widget on backorder?
depends: widget

Problem: stock is unknown.

Standard: plain words.

- A) It is on backorder.
  Why: reasons.
  Costs: costs.
- B) It is in stock.
  Why: other reasons.
  Costs: other costs.

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
"#;
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let after = std::fs::read_to_string(&record).unwrap();
    let qx_id = after
        .lines()
        .skip_while(|l| !l.starts_with("### 1.0 Is the widget on backorder?"))
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();
    assert!(adr_owner(&dir, &["answer", &qx_id, "refuse"], "")
        .status
        .success());

    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content2).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert_eq!(
        err.trim_end(),
        "ask: adrdom/decisions/example.md: \"1.2 Does the widget's paint match its color?\" is in tier 2; tier 1 is open, waiting on refused \"1.0 Is the widget on backorder?\"; agent re-plans, owner runs: adr approve adrdom/decisions/example.md"
    );
}

// --- cap -------------------------------------------------------------------

#[test]
fn ask_refuses_a_tier_over_the_cap() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let mut content = String::from("# Example\n");
    for i in 1..=5 {
        content.push_str(&format!(
            "\n### 1.{n} Is widget option {n} chosen?\ndepends: widget\n\nProblem: widget option {n}.\n\nStandard: plain words.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
            n = i
        ));
    }
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);

    // The cap refuses at `approve` itself now (approve checks the same cap
    // before ever minting an id), so `ask` never even gets a chance to see
    // this file approved; assert the refusal there directly instead of
    // faking approval around it.
    let approve_out = adr_owner(
        &dir,
        &["approve", record.to_str().unwrap()],
        "",
    );
    assert!(!approve_out.status.success());
    assert!(
        stderr(&approve_out).contains("tier 1 holds 5 questions, the cap is 4"),
        "{}",
        stderr(&approve_out)
    );

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/example.md: plan not approved; owner runs: adr approve adrdom/decisions/example.md"
    );
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, content, "neither approve nor ask may modify the file when they refuse");
}

// --- stands on refused: a specific dependency, not the generic blocker ----

#[test]
fn ask_names_the_specific_refused_dependency_not_the_file_order_blocker() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");

    // Qx precedes Q1 in file order and stays plainly unanswered (never
    // refused); Q2 depends on Q1 alone, which is the one that gets
    // refused. If the generic tier-order blocker (picked by file order)
    // fired here it would name Qx, not Q1; the specific check must win.
    let content = r#"# Example

### 1.0 Is the widget in stock?
depends: widget

Problem: stock is unknown.

Standard: plain words.

- A) It is in stock.
  Why: reasons.
  Costs: costs.
- B) It is out of stock.
  Why: other reasons.
  Costs: other costs.

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
"#;
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let after = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&after);
    assert_eq!(ids.len(), 2, "{}", after);
    // Q1 ("What color...") is the second heading in the file; refuse it.
    let q1_id = after
        .lines()
        .skip_while(|l| !l.starts_with("### 1.1 What color"))
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();
    assert!(adr_owner(&dir, &["answer", &q1_id, "refuse"], "")
        .status
        .success());

    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content2).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert_eq!(
        err.trim_end(),
        "ask: adrdom/decisions/example.md: \"1.2 Does the widget's paint match its color?\" stands on refused \"1.1 What color is the widget?\"; agent re-plans, owner runs: adr approve adrdom/decisions/example.md"
    );
    assert!(
        !err.contains("1.0 Is the widget in stock?"),
        "must not name the unrelated file-order blocker: {}",
        err
    );
}

// --- moot --------------------------------------------------------------

#[test]
fn ask_refuses_a_moot_candidate() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let q1_id = extract_ids(&content)[0].clone();
    // Answer B, but Q2 will only stand on A.
    assert!(adr_owner(&dir, &["answer", &q1_id, "B"], "").status.success());

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/example.md: \"1.2 Does the widget's paint match its color?\" is moot: stands on \"1.1 What color is the widget?\"=A, answered B; agent drops or replaces the heading, owner runs: adr approve adrdom/decisions/example.md"
    );
}

// --- answer sets mandatory ------------------------------------------------

#[test]
fn ask_refuses_a_tier_n_dependent_with_no_answer_set() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = format!(
        "{}\n### 1.2 Does the widget's paint match its color?\ndepends: 1.1 What color is the widget?\n",
        Q1
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");
    let before = std::fs::read_to_string(&record).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/example.md: \"1.2 Does the widget's paint match its color?\" depends on \"1.1 What color is the widget?\" without an answer set; agent writes =A, =A|B or =* on that depends line"
    );
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, before);
}

#[test]
fn ask_refuses_a_set_naming_an_option_the_target_does_not_have() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = format!(
        "{}\n### 1.2 Does the widget's paint match its color?\ndepends: 1.1 What color is the widget?=C\n",
        Q1
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/example.md: \"1.2 Does the widget's paint match its color?\" depends on \"1.1 What color is the widget?\" names option C which \"1.1 What color is the widget?\" does not have; agent fixes the answer set to only name options the target has"
    );
}

// --- assumes rows and the tier print --------------------------------------

#[test]
fn ask_prints_the_tier_and_writes_an_assumes_row_per_dependent() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    // Q2 is a skeleton (heading + depends only): it must not itself become
    // a candidate blocked by the tier rule, but it still counts as a
    // dependent of Q1 for the `assumes` row this ask writes.
    let content = format!(
        "{}\n### 1.2 Does the widget's paint match its color?\ndepends: 1.1 What color is the widget?=B|A\n",
        Q1
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let printed = stdout(&out);
    assert!(
        printed.contains("asked: \"1.1 What color is the widget?\" (tier 1)"),
        "{}",
        printed
    );
    // The set is printed sorted, regardless of how it was written (=B|A).
    assert!(
        printed.contains("  assumes: \"1.2 Does the widget's paint match its color?\"=A|B"),
        "{}",
        printed
    );

    let after = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&after);
    assert_eq!(ids.len(), 2, "both headings should have an id from approve: {}", after);
    let q1_id = after
        .lines()
        .skip_while(|l| !l.starts_with("### 1.1 What color"))
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();
    let q2_id = after
        .lines()
        .skip_while(|l| !l.starts_with("### 1.2 Does the widget's paint"))
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    // 9.1 A point 3: an `assumes` row keys on the dependent's own heading
    // id, not its heading text.
    let expected_hash = sha256_hex(&format!("{}=A|B", q2_id));
    let row = ledger
        .lines()
        .find(|l| l.contains("\tassumes\t"))
        .expect(&format!("no assumes row: {}", ledger));
    let fields: Vec<&str> = row.split('\t').collect();
    assert_eq!(fields[1], "assumes");
    assert_eq!(fields[2], q1_id, "assumes row id should be the asked question's id");
    assert_eq!(fields[3], expected_hash, "row: {}", row);
    assert_eq!(fields[4], q2_id);
}

// --- edited-after-refused (mirrors the pre-existing edited-after-answered
// guard) -------------------------------------------------------------------

#[test]
fn ask_refuses_when_a_refused_section_was_edited_since_refusal() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["answer", &id, "refuse"], "").status.success());

    // Edit the section's body after it was refused: the Problem line's text
    // moves the section hash while `**Answer:** refused` stays untouched.
    // Neither heading nor context prose changed, so plan/context approval
    // (recorded from Q1 above) still holds.
    let refused_content = std::fs::read_to_string(&record).unwrap();
    let edited = refused_content.replace(
        "Problem: the widget's color is undecided.",
        "Problem: the widget's color is still undecided, reworded.",
    );
    assert_ne!(refused_content, edited, "the fixture should have that Problem line");
    std::fs::write(&record, &edited).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        format!(
            "ask: {}: \"1.1 What color is the widget?\" was edited after being refused; agent reverts the edit or writes a new question",
            record.display()
        )
    );

    // Nothing changed: the file is exactly as edited, no new ledger rows.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, edited);
    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger_before.lines().filter(|l| l.contains("\trefused\t")).count(),
        1,
        "{}",
        ledger_before
    );
}
