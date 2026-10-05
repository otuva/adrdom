// `ask`: the derived tier rule, the cap, the two refused-dependency
// refusals (generic blocker + specific "stands on refused"), the moot
// refusal, the mandatory-set refusals, and the `assumes` rows/tier print
// that come out of a successful ask.
//
// Every fixture now goes through a real `adr approve` (`approve_record`)
// before `ask` touches it, since `approve` -- not `ask` -- is what mints a
// heading's id, and `ask` refuses outright on a section with none.
// Every heading here needs its own `depends:` line for `approve` to accept
// it; `depends: widget` (a term `approve_minimal_terms` already sets up)
// covers a root-tier heading that has no other natural dependency. Because
// every heading already has an id
// the moment it is approved, `extract_ids(...).len()` no longer tracks
// "how many questions have been asked" -- a count of `\tasked\t` ledger
// rows, or a before/after file-content comparison, is what these tests use
// instead.
mod common;

use common::*;

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

- A) Yes.
  Why: reasons.
  Costs: costs.
- B) No.
  Why: other reasons.
  Costs: other costs.
"#,
        set = set,
        set_note = ""
    )
}

// What `ask` prints after its first line ("ask: nothing to ask in <file>"):
// the `next` lines, one per move.
fn after_first_line(printed: &str) -> String {
    printed.lines().skip(1).collect::<Vec<_>>().join("\n")
}

// The plan change the `next` line asks for when Q2 stands on a refused
// question: Q2's depends line names the term only, and the owner approves.
fn unhook_q2_from_the_refused_question(dir: &std::path::Path, record: &std::path::Path) {
    let text = std::fs::read_to_string(record).unwrap();
    let start = text.find("### 1.2").unwrap();
    let dep = start + text[start..].find("\ndepends: ").unwrap() + 1;
    let end = dep + text[dep..].find('\n').unwrap();
    let changed = format!("{}depends: widget{}", &text[..dep], &text[end..]);
    std::fs::write(record, changed).unwrap();
    approve_record(dir, "adrdom/decisions/example.md");
}

// --- every refusal in one run ----------------------------------------------

// One run of ask names every fault of every question it was about to ask:
// two loaded words in the first question, one in the second, and the
// missing answer set of the question that stands on the first. Nothing is
// written and nothing is asked.
#[test]
fn ask_names_every_fault_in_one_run() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let plan = "# Example\n\n### 1.1 What color is the widget?\ndepends: widget\n\n\
                ### 1.2 Does the widget have a lid?\ndepends: widget\n\n\
                ### 1.3 Does the widget's paint match its color?\ndepends: 1.1 What color is the widget?\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", plan);
    approve_record(&dir, "adrdom/decisions/example.md");

    let approved = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&approved);
    let body_1 = "\nProblem: the widget is already red and currently unpainted.\n\n\
                  Standard: plain words.\n\n\
                  - A) The widget is red.\n  Why: reasons.\n  Costs: costs.\n\
                  - B) The widget is blue.\n  Why: other reasons.\n  Costs: other costs.\n";
    let body_2 = "\nProblem: the widget has no lid today.\n\n\
                  Standard: plain words.\n\n\
                  - A) Yes.\n  Why: reasons.\n  Costs: costs.\n\
                  - B) No.\n  Why: other reasons.\n  Costs: other costs.\n";
    let with_bodies = approved
        .replacen(&format!("id: {}\n", ids[0]), &format!("id: {}\n{}", ids[0], body_1), 1)
        .replacen(&format!("id: {}\n", ids[1]), &format!("id: {}\n{}", ids[1], body_2), 1);
    assert_ne!(with_bodies, approved, "the id lines must be found");
    std::fs::write(&record, &with_bodies).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    let err = stderr(&out);
    let lines: Vec<&str> = err.lines().collect();
    assert_eq!(lines.len(), 4, "{}", err);
    assert!(lines.iter().all(|l| l.starts_with("ask: adrdom/decisions/example.md")), "{}", err);
    assert_eq!(lines.iter().filter(|l| l.contains("without an answer set")).count(), 1, "{}", err);
    for word in ["already", "currently", "today"] {
        assert_eq!(
            lines.iter().filter(|l| l.contains(&format!("\"{}\"", word))).count(),
            1,
            "one line for {:?}: {}",
            word,
            err
        );
    }

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, with_bodies, "ask must not modify the file when it refuses");
    assert_eq!(asked_row_count(&dir), 0, "nothing is asked while a refusal stands");
}

// --- tier order: blocked, then allowed once the blocker is answered -------

// adr: aec860fd-22ad-44a7-99f7-222ea19537e7
//
// "ask asks the lowest tier with an unanswered question (and stops a
// higher one)": Q2 (tier 2, derived from its `depends:` line, never
// written as a tier by hand) is refused while Q1's tier is still open, and
// only succeeds once Q1 is answered.
#[test]
fn ask_prints_nothing_to_ask_naming_the_owners_answer_when_a_later_tier_candidate_waits() {
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

    // Q2 is in tier 2 and tier 1 is open: ask leaves it for a later run,
    // prints that there is nothing to ask, and the `next` line names the
    // owner's answer to Q1.
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask should print nothing to ask: {}", stderr(&out));
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));
    let printed = stdout(&out);
    assert!(
        printed.starts_with("ask: nothing to ask in ") && printed.lines().next().unwrap().ends_with("example.md"),
        "{}",
        printed
    );
    assert!(
        printed.contains(&format!(
            "owner runs: adr answer {}@{} <letter>  # \"1.1 What color is the widget?\"",
            q1_id,
            current_version(&dir, &q1_id)
        )),
        "{}",
        printed
    );
    assert!(
        printed.contains("not yet, after the questions named above: \"1.2 Does the widget's paint match its color?\""),
        "{}",
        printed
    );

    // Nothing written: ask wrote no new content and asked nothing new.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, before_refused_ask, "ask must not modify the file when it has nothing to ask");
    assert_eq!(asked_row_count(&dir), 1, "only Q1 should be asked so far");

    // Now answer Q1: the same ask on Q2 succeeds.
    let answer_out = adr_answer(&dir, &q1_id, "A");
    assert!(answer_out.status.success(), "{}", stderr(&answer_out));

    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "ask should now succeed: {}", stderr(&out2));
    assert_eq!(asked_row_count(&dir), 2, "Q2 should now be asked too");
}

#[test]
fn ask_leaves_a_candidate_standing_on_a_refused_question_and_the_plan_change_works() {
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
    assert!(adr_answer(&dir, &q1_id, "refuse")
        .status
        .success());

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    // Q2 stands on refused Q1, which holds tier 1 open: Q2 is left for a
    // later run, so ask prints nothing to ask, and the `next` lines name
    // the refused question and the plan change.
    let before = std::fs::read_to_string(&record).unwrap();
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        after_first_line(&stdout(&out)),
        "  agent changes the plan: \"1.1 What color is the widget?\" is refused and holds the questions that come after it\n  not yet, after the questions named above: \"1.2 Does the widget's paint match its color?\""
    );
    assert_eq!(std::fs::read_to_string(&record).unwrap(), before);
    assert_eq!(asked_row_count(&dir), 1, "Q2 is not asked");

    // The move it names works: Q2 no longer stands on Q1, the plan is
    // approved again, and ask asks Q2.
    unhook_q2_from_the_refused_question(&dir, &record);
    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert!(stdout(&out2).contains("asked: \"1.2 Does the widget's paint match its color?\" (tier 1)"), "{}", stdout(&out2));
    assert_eq!(asked_row_count(&dir), 2);
}

// adr: 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9
//
// "a refused question counts as unanswered for its tier, so the next tier
// waits (ask stops it, naming the refused question) until the owner
// approves a plan change that drops or replaces it, after which the next
// tier can be asked": Qx (tier 1, refused) holds tier 1 open even while
// Q2 (tier 2) depends only on Q1, never on Qx directly -- ask leaves Q2
// for a later run and the `next` lines it prints name refused Qx. Once
// Qx's heading is struck and the plan re-approved, Qx no longer holds any
// tier open: answering Q1 opens tier 2, and ask now succeeds for Q2.
#[test]
fn ask_names_a_refused_question_in_the_open_tier_and_the_plan_change_works() {
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
    assert!(adr_answer(&dir, &qx_id, "refuse")
        .status
        .success());

    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content2).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    // Q2 is left for a later run; ask prints nothing to ask and the `next`
    // lines name refused Qx and the plan change.
    let q1_id_now = std::fs::read_to_string(&record)
        .unwrap()
        .lines()
        .skip_while(|l| !l.starts_with("### 1.1 What color"))
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        after_first_line(&stdout(&out)),
        format!(
            "  owner runs: adr answer {}@{} <letter>  # \"1.1 What color is the widget?\"\n  agent changes the plan: \"1.0 Is the widget on backorder?\" is refused and holds the questions that come after it\n  not yet, after the questions named above: \"1.2 Does the widget's paint match its color?\"",
            q1_id_now,
            current_version(&dir, &q1_id_now)
        )
    );
    assert_eq!(asked_row_count(&dir), 2, "Q2 is not asked");

    // Drop Qx's heading and re-approve: the refused question no longer
    // holds any tier open, so answering Q1 opens tier 2 and ask can now
    // reach Q2.
    let after_refusal = std::fs::read_to_string(&record).unwrap();
    let q1_start = after_refusal.find("### 1.1").expect("1.1 heading present");
    let dropped = format!("# Example\n\n{}", &after_refusal[q1_start..]);
    std::fs::write(&record, &dropped).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let q1_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &q1_id, "A").status.success());

    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        out2.status.success(),
        "ask should succeed once the refused blocker is dropped from the plan: {}",
        stderr(&out2)
    );
    assert_eq!(
        asked_row_count(&dir),
        3,
        "Qx, Q1, and now Q2 should all have been asked"
    );
}

// --- cap -------------------------------------------------------------------

// adr: 15d82e53-e344-4347-b639-3bf6eb9ef1b4
//
// core: "four questions per tier, a value set in the tool; approve and ask
// stop a plan with a tier over four". Five root questions (one tier) is one
// past TIER_CAP: `approve` refuses naming the exact cap ("5 questions would
// be put to the owner together, the most is 4"), and since the file was
// never approved, `ask` independently refuses it too ("plan not approved") -- neither command
// lets the over-cap plan through, and neither writes to the file on
// refusal.
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
        stderr(&approve_out).contains("5 questions would be put to the owner together, the most is 4"),
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
fn ask_names_every_open_move_when_a_candidate_stands_on_a_refused_question() {
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
    let qx_id = ids[0].clone();
    // Q1 ("What color...") is the second heading in the file; refuse it.
    let q1_id = after
        .lines()
        .skip_while(|l| !l.starts_with("### 1.1 What color"))
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();
    assert!(adr_answer(&dir, &q1_id, "refuse")
        .status
        .success());

    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content2).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        after_first_line(&stdout(&out)),
        format!(
            "  owner runs: adr answer {}@{} <letter>  # \"1.0 Is the widget in stock?\"\n  agent changes the plan: \"1.1 What color is the widget?\" is refused and holds the questions that come after it\n  not yet, after the questions named above: \"1.2 Does the widget's paint match its color?\"",
            qx_id,
            current_version(&dir, &qx_id)
        )
    );
    assert_eq!(asked_row_count(&dir), 2, "Q2 is not asked");

    // The move it names works: Q2 no longer stands on Q1, the plan is
    // approved again, and ask asks Q2 (in tier 1, beside the open Qx).
    unhook_q2_from_the_refused_question(&dir, &record);
    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert!(stdout(&out2).contains("asked: \"1.2 Does the widget's paint match its color?\" (tier 1)"), "{}", stdout(&out2));
    assert_eq!(asked_row_count(&dir), 3);
}

// --- open tier holds a skeleton --------------------------------------

// adr: aec860fd-22ad-44a7-99f7-222ea19537e7
//
// "the tool derives the tiers from the depends lines ... ask asks the
// lowest tier with an unanswered question": a skeleton heading (approved,
// with an id, but no body yet) still holds its tier open. A later tier's
// candidate is left for a later run, and the `next` lines give the agent's
// move ("writes the body"), not the owner's -- there is nothing yet for
// the owner to answer. Once the skeleton gets a body, is asked, and is
// answered, the tier it was blocking opens and the later candidate can be
// asked.
#[test]
fn ask_leaves_a_later_tier_candidate_while_the_open_tier_holds_a_skeleton() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");

    // Q1 starts as a bare skeleton: heading + depends only, no body.
    let content = format!(
        "# Example\n\n### 1.1 What color is the widget?\ndepends: widget\n{}",
        q2_depends_on_q1("=A")
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");

    // Q1 is a skeleton, so there is nothing to ask: Q2 is left for a later
    // run and the `next` lines name the agent's move on Q1.
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        after_first_line(&stdout(&out)),
        "  agent writes the body of \"1.1 What color is the widget?\"\n  not yet, after the questions named above: \"1.2 Does the widget's paint match its color?\""
    );
    assert_eq!(asked_row_count(&dir), 0, "nothing is asked");

    // The move it names works: Q1 gets its body, and ask asks Q1 alone;
    // Q2 waits for the answer.
    let before = std::fs::read_to_string(&record).unwrap();
    let split_at = before.find("### 1.2").unwrap();
    let mut with_q1_body = before[..split_at].to_string();
    with_q1_body.push_str("\nProblem: the widget's color is undecided.\n\nStandard: plain words.\n\n- A) The widget is red.\n  Why: reasons.\n  Costs: costs.\n- B) The widget is blue.\n  Why: other reasons.\n  Costs: other costs.\n\n");
    with_q1_body.push_str(&before[split_at..]);
    std::fs::write(&record, &with_q1_body).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert!(
        stdout(&out2).contains("asked: \"1.1 What color is the widget?\" (tier 1)"),
        "{}",
        stdout(&out2)
    );
    assert!(!stdout(&out2).contains("(tier 2)"), "{}", stdout(&out2));
    assert_eq!(asked_row_count(&dir), 1, "Q2 waits");
    let q1_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &q1_id, "A").status.success());

    // Q1 is answered, tier 2 is open, and ask asks Q2.
    let out3 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out3.status.success(), "{}", stderr(&out3));
    assert!(
        stdout(&out3).contains("asked: \"1.2 Does the widget's paint match its color?\" (tier 2)"),
        "{}",
        stdout(&out3)
    );
}

// The open tier holds a fully written, approved question that nobody has
// run `ask` on yet. `ask` asks it in this run and leaves the later tier's
// candidate for a later run, without a refusal; the owner's answer opens
// tier 2 and the next run asks it.
#[test]
fn ask_asks_the_written_question_and_leaves_the_later_tier_candidate() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");

    let content = format!("{}{}", Q1, q2_depends_on_q1("=A"));
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "asked: \"1.1 What color is the widget?\" (tier 1)\n  stood on by: \"1.2 Does the widget's paint match its color?\" under A\n"
    );
    assert_eq!(asked_row_count(&dir), 1, "only Q1 is asked");

    let q1_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &q1_id, "A").status.success());
    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert!(
        stdout(&out2).contains("asked: \"1.2 Does the widget's paint match its color?\" (tier 2)"),
        "{}",
        stdout(&out2)
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
    assert!(adr_answer(&dir, &q1_id, "B").status.success());

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/example.md: \"1.2 Does the widget's paint match its color?\" has no case left: it stands on \"1.1 What color is the widget?\" under A, and the answer was B; agent drops or replaces the heading, owner runs: adr approve adrdom/decisions/example.md"
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

//
// "when a tier's bodies are written, every depends entry naming one of its
// questions gets the letters": the set is frozen with the question being
// asked. A question written and answered after its target was answered
// never needed a set, and when a new question later joins the target's
// tier, ask asks the new one without demanding a set on that old,
// answered line.
#[test]
fn ask_asks_a_new_sibling_without_a_set_on_an_answered_dependents_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let q1_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &q1_id, "A").status.success());

    // Q2 arrives after Q1 is answered: its entry carries no set, and ask
    // and answer take it as it is.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1(""));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let q2_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[1].clone();
    assert!(adr_answer(&dir, &q2_id, "A").status.success());

    // Q3 joins Q1's tier. Q2's line naming Q1 still has no set; Q1 is
    // answered, so no set is asked for, and Q3 is asked.
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(
        "\n### 1.3 Does the widget need a lid?\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
    );
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("asked: \"1.3 Does the widget need a lid?\" (tier 1)"), "{}", stdout(&out));
}

//
// The mandatory-set gate must also catch a target that was asked in an
// earlier run and is still unanswered, not only a target being asked for
// the first time this run: Q1 is asked and left unanswered; Q2 (tier 2)
// then joins depending on Q1 with no set, alongside a new tier-1 Q3. Q1
// itself is not a candidate this run (already asked, unchanged), but Q3
// is, so the gate runs -- and must still refuse on Q2's line naming Q1.
#[test]
fn ask_refuses_a_set_missing_on_an_asked_but_unanswered_target() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    // Q1 is now asked, tier 1, left unanswered.

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1(""));
    content.push_str(
        "\n### 1.3 Does the widget need a lid?\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
    );
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/example.md: \"1.2 Does the widget's paint match its color?\" depends on \"1.1 What color is the widget?\" without an answer set; agent writes =A, =A|B or =* on that depends line"
    );
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
        printed.contains("  stood on by: \"1.2 Does the widget's paint match its color?\" under A|B"),
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
    // an `assumes` row keys on the dependent's own heading
    // id, not its heading text.
    let expected_hash = assumes_hash(&q2_id, "A|B");
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

// adr: b782664f-7465-4e1e-99e0-13f6dd2de774
//
// "its identifier never returns, and a replacement is a new question":
// rewording the refused section under its own id does not bring it back
// askable -- `ask` refuses and points the agent at reverting the edit or
// writing a new question (a new id), never at re-asking the old one.
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
    assert!(adr_answer(&dir, &id, "refuse").status.success());

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

// --- ask leaves a later-tier candidate for a later run ----------------------

// adr: aec860fd-22ad-44a7-99f7-222ea19537e7
// adr: f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
//
// core: "ask asks the lowest tier with an unanswered question"; workflow: a
// tier is built once its questions are answered. Alpha (tier 1) and delta
// (tier 2, standing on alpha) both have bodies and nothing is asked. `show`
// names `ask` as alpha's move, so `ask` must take it: it asks alpha, leaves
// delta for a later run without refusing, and after the owner answers alpha
// the same command asks delta.
#[test]
fn ask_asks_the_open_tier_and_leaves_a_later_tier_candidate_for_a_later_run() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let body = "\nProblem: the widget's state is undecided.\n\nStandard: plain words.\n\n- A) Yes.\n  Why: reasons.\n  Costs: costs.\n- B) No.\n  Why: other reasons.\n  Costs: other costs.\n";
    let content = format!(
        "# Example\n\n### Is alpha ready?\ndepends: widget\n{body}\n### Is delta ready?\ndepends: widget, Is alpha ready?=*\n{body}"
    );
    let rel = "adrdom/decisions/example.md";
    let record = write_record(&dir, rel, &content);
    approve_record(&dir, rel);

    let shown = stdout(&adr(&dir, &["show", rel], &[], ""));
    assert!(shown.contains("agent runs adr ask"), "{}", shown);
    assert!(shown.contains("not yet"), "{}", shown);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask must take the move show named: {}", stderr(&out));
    assert!(stdout(&out).contains("asked: \"Is alpha ready?\" (tier 1)"), "{}", stdout(&out));
    assert!(!stdout(&out).contains("Is delta ready?\" (tier"), "{}", stdout(&out));
    assert_eq!(asked_row_count(&dir), 1, "only alpha is asked");
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));

    let text = std::fs::read_to_string(&record).unwrap();
    let alpha_id = text
        .lines()
        .skip_while(|l| !l.starts_with("### Is alpha ready?"))
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();

    // What show prints now: the owner answers alpha, delta is not yet.
    let shown = stdout(&adr(&dir, &["show", rel], &[], ""));
    assert!(shown.contains("owner runs: adr answer"), "{}", shown);
    assert!(shown.contains(&alpha_id), "{}", shown);
    assert!(
        shown.lines().any(|l| l.contains("not yet") && l.contains("Is delta ready?")),
        "{}",
        shown
    );

    // A run with nothing to ask says so and names the same next lines.
    let again = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(again.status.success(), "{}", stderr(&again));
    assert!(stdout(&again).contains("nothing to ask in"), "{}", stdout(&again));
    assert!(stdout(&again).contains("owner runs: adr answer"), "{}", stdout(&again));
    assert_eq!(asked_row_count(&dir), 1);

    // The owner's move; then ask asks delta.
    assert!(adr_answer(&dir, &alpha_id, "A").status.success());
    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert!(stdout(&out2).contains("asked: \"Is delta ready?\" (tier 2)"), "{}", stdout(&out2));
    assert_eq!(asked_row_count(&dir), 2);
}
