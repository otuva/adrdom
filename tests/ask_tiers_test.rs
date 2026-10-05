// `ask`: the derived tier rule, the cap, the two refused-dependency
// refusals (generic blocker + specific "stands on refused"),
// the mandatory-set refusals, and the `assumes` rows/tier print
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
// two loaded words in the first question and one in the second. Nothing is
// written and nothing is asked.
#[test]
fn ask_names_every_fault_in_one_run() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let body_1 = "\nProblem: the widget is red and unpainted.\n\n\
                  Standard: plain words.\n\n\
                  - A) The widget is red.\n  Why: reasons.\n  Costs: costs.\n\
                  - B) The widget is blue.\n  Why: other reasons.\n  Costs: other costs.\n";
    let body_2 = "\nProblem: the widget has no lid.\n\n\
                  Standard: plain words.\n\n\
                  - A) Yes.\n  Why: reasons.\n  Costs: costs.\n\
                  - B) No.\n  Why: other reasons.\n  Costs: other costs.\n";
    let body_3 = "\nProblem: the paint might not match.\n\n\
                  Standard: plain words.\n\n\
                  - A) Yes.\n  Why: reasons.\n  Costs: costs.\n\
                  - B) No.\n  Why: other reasons.\n  Costs: other costs.\n";
    let plan = format!(
        "# Example\n\n### 1.1 What color is the widget?\ndepends: widget\n{}\n\
         ### 1.2 Does the widget have a lid?\ndepends: widget\n{}\n\
         ### 1.3 Does the widget's paint match its color?\ndepends: 1.1 What color is the widget?=A\n{}",
        body_1, body_2, body_3
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &plan);
    approve_record(&dir, "adrdom/decisions/example.md");
    // Approve cannot take the loaded words, so they are put back after it
    // and the approval rows are written by hand: ask is the second stop.
    let loaded = std::fs::read_to_string(&record)
        .unwrap()
        .replace("is red and unpainted.", "is already red and currently unpainted.")
        .replace("no lid.", "no lid today.");
    std::fs::write(&record, &loaded).unwrap();
    let rf = adr::record::load(&record).unwrap();
    let rel = "adrdom/decisions/example.md";
    let later = "2099-01-01T00:00:00Z";
    append_ledger_row_at(&dir, "approved", "-", &adr::record::plan_hash(&rf), rel, later);
    append_ledger_row_at(&dir, "context", "-", &adr::record::context_hash(&rf), rel, later);
    let with_bodies = loaded;

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    let err = stderr(&out);
    let lines: Vec<&str> = err.lines().collect();
    assert_eq!(lines.len(), 3, "{}", err);
    assert!(lines.iter().all(|l| l.starts_with("ask: adrdom/decisions/example.md")), "{}", err);
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
    // The agent's planning moves come first; the refusal is the cap's.
    plan_record(&dir, "adrdom/decisions/example.md");
    let content = std::fs::read_to_string(&record).unwrap();

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

// --- open tier holds a written question --------------------------------------

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

// --- answered targets --------------------------------------------------

// `approve` refuses an answer set on an entry naming an answered target.
#[test]
fn approve_refuses_a_set_on_an_answered_target() {
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
    assert!(adr_answer(&dir, &q1_id, "B").status.success());

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content).unwrap();
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("answered, under an answer set"), "{}", stderr(&out));
}

// --- answer sets mandatory ------------------------------------------------

#[test]
fn approve_refuses_a_tier_n_dependent_with_no_answer_set() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = format!("{}{}", Q1, q2_depends_on_q1(""));
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("without an answer set"), "{}", stderr(&out));
}

#[test]
fn approve_refuses_a_set_missing_on_an_asked_but_unanswered_target() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1(""));
    std::fs::write(&record, &content).unwrap();
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("without an answer set"), "{}", stderr(&out));
}

// A set naming an option the target lacks never reaches `ask`: approve refuses
// it first (the same message, found before approval).
#[test]
fn approve_refuses_a_set_naming_an_option_the_target_does_not_have() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = format!("{}{}", Q1, q2_depends_on_q1("=C"));
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("\"1.2 Does the widget's paint match its color?\" depends on \"1.1 What color is the widget?\" names option C which \"1.1 What color is the widget?\" does not have"),
        "{}",
        stderr(&out)
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
    let content = format!("{}{}", Q1, q2_depends_on_q1("=B|A"));
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
