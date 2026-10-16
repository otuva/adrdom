// `ask` and `answer`: the derived tiers, the cap, the two refused-dependency
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

// A second `ask` with nothing left to ask writes nothing and prints
// nothing (per "What does a command that writes print?", 069a6b4a A); the
// moves that remain are the ones `show` prints. Asserts both, and returns
// what `show` printed for the file.
fn nothing_to_ask_and_show(dir: &std::path::Path, record: &std::path::Path) -> String {
    let again = adr(dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(again.status.success(), "{}", stderr(&again));
    assert_eq!(stdout(&again), "", "{}", stdout(&again));
    stdout(&adr(dir, &["show"], &[], ""))
}

// Every move line `shown` holds, in the order printed: the whole list a
// test compares, so an extra, repeated or reordered move fails it.
fn moves_of(shown: &str) -> Vec<String> {
    shown
        .lines()
        .map(str::trim)
        .filter(|l| {
            l.starts_with("agent ") || l.starts_with("owner runs:") || l.starts_with("not yet")
        })
        .map(str::to_string)
        .collect()
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
        .replace(
            "is red and unpainted.",
            "is already red and currently unpainted.",
        )
        .replace("no lid.", "no lid today.");
    std::fs::write(&record, &loaded).unwrap();
    let rf = adr::record::load(&record).unwrap();
    let rel = "adrdom/decisions/example.md";
    let later = "2099-01-01T00:00:00Z";
    append_ledger_row_at(
        &dir,
        "approved",
        "-",
        &adr::record::plan_hash(&rf),
        rel,
        later,
    );
    append_ledger_row_at(
        &dir,
        "context",
        "-",
        &adr::record::context_hash(&rf),
        rel,
        later,
    );
    let with_bodies = loaded;

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        !out.status.success(),
        "ask should have refused: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    let lines: Vec<&str> = err.lines().collect();
    assert_eq!(lines.len(), 3, "{}", err);
    assert!(
        lines
            .iter()
            .all(|l| l.starts_with("adrdom/decisions/example.md:")),
        "{}",
        err
    );
    for word in ["already", "currently", "today"] {
        assert_eq!(
            lines
                .iter()
                .filter(|l| l.contains(&format!("\"{}\"", word)))
                .count(),
            1,
            "one line for {:?}: {}",
            word,
            err
        );
    }

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after, with_bodies,
        "ask must not modify the file when it refuses"
    );
    assert_eq!(
        asked_row_count(&dir),
        0,
        "nothing is asked while a refusal stands"
    );
}

// --- one run asks every tier ------------------------------------------------

// adr: 1d7c758d-1fd5-4e0c-8559-517315b15aaa
//
// "What does ask put to the owner at one run?": every unanswered question of
// the file at once, whatever its tier. Q1 is asked and open; Q2 (tier 2,
// standing on Q1) is asked by the next run beside it. `answer` is what keeps
// Q2 from being taken first (`answer_takes_a_question_in_the_order_it_stands_on`).
#[test]
fn ask_asks_a_later_tier_candidate_while_the_lower_tier_is_open() {
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
    // asked at its current hash).
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));
    assert!(
        stdout(&out)
            .lines()
            .any(|l| l == "asked: \"1.2 Does the widget's paint match its color?\""),
        "{}",
        stdout(&out)
    );
    assert_eq!(asked_row_count(&dir), 2, "Q1 and Q2 are both asked");

    // Nothing is left to ask; show names the owner's answer to Q1 and
    // leaves Q2 under `not yet`: `answer` takes Q1 first.
    let printed = nothing_to_ask_and_show(&dir, &record);
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
        printed.contains(
            "not yet: \"1.2 Does the widget's paint match its color?\" is asked and waits for \"1.1 What color is the widget?\""
        ),
        "{}",
        printed
    );
    assert_eq!(asked_row_count(&dir), 2);
}

// adr: 1d7c758d-1fd5-4e0c-8559-517315b15aaa
//
// One run of ask on a file that holds three tiers asks all of its
// questions: an `asked` row for each, and one line for each row.
#[test]
fn one_ask_run_asks_every_question_of_a_multi_tier_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let body = "\nProblem: the widget's state is undecided.\n\nStandard: plain words.\n\n- A) Yes.\n  Why: reasons.\n  Costs: costs.\n- B) No.\n  Why: other reasons.\n  Costs: other costs.\n";
    let content = format!(
        "# Example\n\n### Is alpha ready?\ndepends: widget\n{body}\n### Is beta ready?\ndepends: widget, Is alpha ready?=*\n{body}\n### Is gamma ready?\ndepends: widget, Is beta ready?=*\n{body}"
    );
    let rel = "adrdom/decisions/example.md";
    let record = write_record(&dir, rel, &content);
    approve_record(&dir, rel);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let printed = stdout(&out);
    for heading in ["Is alpha ready?", "Is beta ready?", "Is gamma ready?"] {
        assert!(
            printed
                .lines()
                .any(|l| l == format!("asked: \"{}\"", heading)),
            "{}",
            printed
        );
    }
    assert_eq!(
        asked_row_count(&dir),
        3,
        "every question is asked in one run"
    );
    let text = std::fs::read_to_string(&record).unwrap();
    for id in extract_ids(&text) {
        current_version(&dir, &id);
    }
}

#[test]
fn ask_asks_a_candidate_standing_on_a_refused_question_and_the_plan_change_works() {
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
    assert!(adr_answer(&dir, &q1_id, "refuse").status.success());

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    // Q2 stands on refused Q1: ask asks it (every question, whatever its
    // tier), and `answer` refuses it until the plan changes.
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out)
            .lines()
            .any(|l| l == "asked: \"1.2 Does the widget's paint match its color?\""),
        "{}",
        stdout(&out)
    );
    assert_eq!(asked_row_count(&dir), 2);

    // What the owner would read next: the refused question, the plan
    // change, and Q2 under `not yet`.
    let shown = nothing_to_ask_and_show(&dir, &record);
    assert_eq!(
        moves_of(&shown),
        vec![
            "agent writes the plan change that drops or replaces \"1.1 What color is the widget?\"".to_string(),
            "not yet: \"1.2 Does the widget's paint match its color?\" is asked and waits for \"1.1 What color is the widget?\"".to_string(),
        ],
        "{}",
        shown
    );

    // The move it names works: Q2 no longer stands on Q1, the plan is
    // approved again, and Q2 asked at tier 1 is taken after the next ask.
    unhook_q2_from_the_refused_question(&dir, &record);
    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert!(
        stdout(&out2)
            .lines()
            .any(|l| l == "asked: \"1.2 Does the widget's paint match its color?\""),
        "{}",
        stdout(&out2)
    );
    let q2_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[1].clone();
    assert!(adr_answer(&dir, &q2_id, "A").status.success());
}

// adr: 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9
//
// "a refused question counts as unanswered for its tier, so the next tier
// waits (answer refuses it, naming the refused question) until the owner
// approves a plan change that drops or replaces it, after which the next
// tier can be answered": Qx (tier 1, refused) holds the tiers above it even
// while Q2 (tier 2) depends only on Q1, never on Qx directly -- answer
// refuses Q2 and the `next` lines name refused Qx. Once Qx's heading is
// struck and the plan re-approved, answer takes Q2.
#[test]
fn answer_refuses_a_tier_above_a_refused_question_until_the_plan_changes() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");

    // Qx (refused) precedes Q1 (left plainly unanswered) in file order.
    // Q2 depends only on Q1, never on Qx, so the generic tier-order
    // blocker (picked by file order within the lowest open tier) is what
    // `answer` names, as refused.
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
    assert!(adr_answer(&dir, &qx_id, "refuse").status.success());

    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content2).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    // Ask asks Q2 as well. `answer` takes Q1 (tier 1, beside the refused
    // Qx), but refuses Q2 even though Q2 does not stand on Qx: it is in a
    // tier above the refused question's.
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(asked_row_count(&dir), 3, "Qx, Q1 and Q2 are asked");
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    let (q1_id, q2_id) = (ids[1].clone(), ids[2].clone());
    assert!(adr_answer(&dir, &q1_id, "A").status.success());

    let refused = adr_answer(&dir, &q2_id, "A");
    assert!(!refused.status.success());
    let err = stderr(&refused);
    assert!(
        err.contains("\"1.2 Does the widget's paint match its color?\" waits: \"1.0 Is the widget on backorder?\" is refused and holds the tiers above it"),
        "{}",
        err
    );
    assert!(
        err.contains("agent writes the plan change that drops or replaces \"1.0 Is the widget on backorder?\", owner runs: adr approve adrdom/decisions/example.md"),
        "{}",
        err
    );
    let next = nothing_to_ask_and_show(&dir, &record);
    assert!(next.contains("agent writes the plan change that drops or replaces \"1.0 Is the widget on backorder?\""), "{}", next);
    assert!(
        next.contains("not yet: \"1.2 Does the widget's paint match its color?\" is asked and waits for \"1.0 Is the widget on backorder?\""),
        "{}",
        next
    );

    // Drop Qx's heading and re-approve: the refused question no longer
    // holds any tier, so `answer` now takes Q2.
    let after_refusal = std::fs::read_to_string(&record).unwrap();
    let q1_start = after_refusal.find("### 1.1").expect("1.1 heading present");
    let dropped = format!("# Example\n\n{}", &after_refusal[q1_start..]);
    std::fs::write(&record, &dropped).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let taken = adr_answer(&dir, &q2_id, "A");
    assert!(
        taken.status.success(),
        "answer should take Q2 once the refused question is dropped from the plan: {}",
        stderr(&taken)
    );
}

// --- cap -------------------------------------------------------------------

//
// core: "four questions per tier, a value set in the tool; approve and ask
// stop a plan with a tier over four". Five root questions (one tier) is one
// past TIER_CAP: `approve` refuses naming the exact cap ("5 questions would
// be put to the owner together, the most is 4") and does not modify the
// file. This test shows only the approve side: `ask` on this never-approved
// file stops on "plan not approved" before it reads the cap, so the ask
// side of the cap is shown by `ask_stops_a_tier_that_a_refused_replacement_
// in_another_file_took_over_the_cap` in supersede_cap_test.rs.
// adr: 0e38f001-0226-4927-9a78-2e9fc1ec18d4
#[test]
fn approve_refuses_a_tier_over_the_cap() {
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

    let approve_out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!approve_out.status.success());
    assert!(
        stderr(&approve_out).contains("5 questions in one tier, the most is 4"),
        "{}",
        stderr(&approve_out)
    );
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after, content,
        "approve may not modify the file when it refuses"
    );
}

// --- stands on refused: a specific dependency, not the generic blocker ----

#[test]
fn next_names_every_open_move_when_a_candidate_stands_on_a_refused_question() {
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
    assert!(adr_answer(&dir, &q1_id, "refuse").status.success());

    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(&q2_depends_on_q1("=A"));
    std::fs::write(&record, &content2).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    // Ask asks Q2 beside the others; a second run has nothing to ask, and
    // show names every open move: the owner's answer to Qx, the plan change
    // for refused Q1, and Q2 under `not yet`.
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(asked_row_count(&dir), 3, "Q2 is asked");
    let shown = nothing_to_ask_and_show(&dir, &record);
    let expected = vec![
        format!(
            "owner runs: adr answer {}@{} <letter>  # \"1.0 Is the widget in stock?\"",
            qx_id,
            current_version(&dir, &qx_id)
        ),
        "agent writes the plan change that drops or replaces \"1.1 What color is the widget?\"".to_string(),
        "not yet: \"1.2 Does the widget's paint match its color?\" is asked and waits for \"1.1 What color is the widget?\"".to_string(),
    ];
    assert_eq!(moves_of(&shown), expected, "{}", shown);

    // The move it names works: Q2 no longer stands on Q1, the plan is
    // approved again, and ask asks Q2 again (in tier 1, beside the open Qx).
    unhook_q2_from_the_refused_question(&dir, &record);
    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert!(
        stdout(&out2)
            .lines()
            .any(|l| l == "asked: \"1.2 Does the widget's paint match its color?\""),
        "{}",
        stdout(&out2)
    );
    assert_eq!(asked_row_count(&dir), 4);
}

// --- open tier holds a written question --------------------------------------

// adr: da80931c-68cf-4c51-acce-060307bbd973
//
// "Does answer take a question standing on a question that has no decision?"
// No: one run of ask asks Q1 and Q2 (standing on Q1) together, and `answer`,
// a letter or a refusal, refuses Q2 while Q1 has no decision. The refusal
// names Q1 and the owner's command for it; answering in order passes.
#[test]
fn answer_takes_a_question_in_the_order_it_stands_on() {
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
        "asked: \"1.1 What color is the widget?\"\nassumes: \"1.2 Does the widget's paint match its color?\" stands on \"1.1 What color is the widget?\" under its answer set\nasked: \"1.2 Does the widget's paint match its color?\"\n"
    );
    assert_eq!(asked_row_count(&dir), 2, "both are asked in one run");

    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    let (q1_id, q2_id) = (ids[0].clone(), ids[1].clone());
    let before = std::fs::read_to_string(&record).unwrap();
    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    for letter in ["A", "refuse"] {
        let out = adr_answer(&dir, &q2_id, letter);
        assert!(!out.status.success(), "{}", letter);
        assert_eq!(
            stderr(&out).trim_end(),
            format!(
                "\"1.2 Does the widget's paint match its color?\" waits: \"1.1 What color is the widget?\" has no decision yet; owner runs: adr answer {}@{} <letter>",
                q1_id,
                current_version(&dir, &q1_id)
            )
        );
    }
    assert_eq!(
        std::fs::read_to_string(&record).unwrap(),
        before,
        "a refused answer writes nothing"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap(),
        ledger_before
    );

    assert!(adr_answer(&dir, &q1_id, "A").status.success());
    assert!(adr_answer(&dir, &q2_id, "A").status.success());
}

// adr: da80931c-68cf-4c51-acce-060307bbd973
//
// A target that is refused has no decision either: `answer` (letter and
// refusal) refuses the question standing on it, naming the plan change and
// the owner's approval. The page's answer route goes through the same gate.
#[test]
fn answer_refuses_a_question_standing_on_a_refused_question() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = format!("{}{}", Q1, q2_depends_on_q1("=A"));
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    let (q1_id, q2_id) = (ids[0].clone(), ids[1].clone());
    assert!(adr_answer(&dir, &q1_id, "refuse").status.success());

    for letter in ["A", "refuse"] {
        let out = adr_answer(&dir, &q2_id, letter);
        assert!(!out.status.success(), "{}", letter);
        assert_eq!(
            stderr(&out).trim_end(),
            "\"1.2 Does the widget's paint match its color?\" waits: \"1.1 What color is the widget?\" is refused and the question stands on it; agent writes the plan change that drops or replaces \"1.1 What color is the widget?\", owner runs: adr approve adrdom/decisions/example.md"
        );
    }
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
    assert!(
        stderr(&out).contains("answered, under an answer set"),
        "{}",
        stderr(&out)
    );
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
    assert!(
        stderr(&out).contains("without an answer set"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn approve_refuses_a_set_missing_on_an_asked_but_unanswered_target() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&q2_depends_on_q1(""));
    std::fs::write(&record, &content).unwrap();
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("without an answer set"),
        "{}",
        stderr(&out)
    );
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
        stderr(&out).contains("\"1.2 Does the widget's paint match its color?\" depends on \"1.1 What color is the widget?\" under an answer set naming option C, which it does not have"),
        "{}",
        stderr(&out)
    );
}

// --- assumes rows and the tier print --------------------------------------

#[test]
fn ask_prints_a_line_per_row_and_writes_an_assumes_row_per_dependent() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    // Q2 is a skeleton (heading + depends only): it must not itself become
    // a candidate for ask, but it still counts as a
    // dependent of Q1 for the `assumes` row this ask writes.
    let content = format!("{}{}", Q1, q2_depends_on_q1("=B|A"));
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let printed = stdout(&out);
    // One line per row: each asked row and the assumes row.
    assert_eq!(
        printed,
        "asked: \"1.1 What color is the widget?\"\nassumes: \"1.2 Does the widget's paint match its color?\" stands on \"1.1 What color is the widget?\" under its answer set\nasked: \"1.2 Does the widget's paint match its color?\"\n"
    );

    let after = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&after);
    assert_eq!(
        ids.len(),
        2,
        "both headings should have an id from approve: {}",
        after
    );
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
        .unwrap_or_else(|| panic!("no assumes row: {}", ledger));
    let fields: Vec<&str> = row.split('\t').collect();
    assert_eq!(fields[1], "assumes");
    assert_eq!(
        fields[2], q1_id,
        "assumes row id should be the asked question's id"
    );
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
    assert_ne!(
        refused_content, edited,
        "the fixture should have that Problem line"
    );
    std::fs::write(&record, &edited).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        !out.status.success(),
        "ask should have refused: {}",
        stdout(&out)
    );
    // The finding check and show print for the same state, under its own
    // rule, with its move.
    assert_eq!(
        stderr(&out).trim_end(),
        "adrdom/decisions/example.md: 1.1 What color is the widget?: text changed since being refused; agent restores adrdom/decisions/example.md from git"
    );
    let checked = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(checked.contains("adrdom/decisions/example.md: 1.1 What color is the widget?: text changed since being refused;"), "{}", checked);
    let shown = stdout(&adr(
        &dir,
        &["show", "adrdom/decisions/example.md"],
        &[],
        "",
    ));
    assert!(shown.contains("adrdom/decisions/example.md: 1.1 What color is the widget?: text changed since being refused;"), "{}", shown);

    // Nothing changed: the file is exactly as edited, no new ledger rows.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, edited);
    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger_before
            .lines()
            .filter(|l| l.contains("\trefused\t"))
            .count(),
        1,
        "{}",
        ledger_before
    );
}

// --- ask takes every move show names ------------------------------------------

// adr: f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
//
// core: "ask puts every unanswered question of the file to the owner in one
// run"; workflow: a tier is built once its questions are answered. Alpha
// (tier 1) and delta (tier 2, standing on alpha) both have bodies and
// nothing is asked. `show` names `ask` for both, so `ask` must take it: it
// asks alpha and delta together. `show` then names the owner's answer to
// alpha and holds delta under `not yet`; once alpha is answered it names the
// owner's answer to delta.
#[test]
fn ask_asks_every_tier_show_named_and_the_owners_answers_come_in_order() {
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
    assert!(
        shown.lines().any(|l| l.contains("agent runs: adr ask")
            && l.contains("Is alpha ready?")
            && l.contains("Is delta ready?")),
        "{}",
        shown
    );
    assert!(!shown.contains("not yet"), "{}", shown);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        out.status.success(),
        "ask must take the move show named: {}",
        stderr(&out)
    );
    assert!(
        stdout(&out)
            .lines()
            .any(|l| l == "asked: \"Is alpha ready?\""),
        "{}",
        stdout(&out)
    );
    assert!(
        stdout(&out)
            .lines()
            .any(|l| l == "asked: \"Is delta ready?\""),
        "{}",
        stdout(&out)
    );
    assert_eq!(asked_row_count(&dir), 2, "alpha and delta are asked");
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));

    let text = std::fs::read_to_string(&record).unwrap();
    let id_of = |heading: &str| {
        text.lines()
            .skip_while(|l| !l.starts_with(heading))
            .find_map(|l| l.strip_prefix("id: "))
            .unwrap()
            .to_string()
    };
    let alpha_id = id_of("### Is alpha ready?");
    let delta_id = id_of("### Is delta ready?");

    // What show prints now: the owner answers alpha, delta is not yet.
    let shown = stdout(&adr(&dir, &["show", rel], &[], ""));
    assert!(shown.contains("owner runs: adr answer"), "{}", shown);
    assert!(shown.contains(&alpha_id), "{}", shown);
    assert!(
        !shown.contains(&format!("adr answer {}@", delta_id)),
        "{}",
        shown
    );
    assert!(
        shown
            .lines()
            .any(|l| l.contains("not yet") && l.contains("Is delta ready?")),
        "{}",
        shown
    );

    // A run with nothing to ask prints nothing; show names the same moves.
    let shown_again = nothing_to_ask_and_show(&dir, &record);
    assert!(
        shown_again.contains("owner runs: adr answer"),
        "{}",
        shown_again
    );
    assert_eq!(asked_row_count(&dir), 2);

    // The owner's move; then show names the answer to delta.
    assert!(adr_answer(&dir, &alpha_id, "A").status.success());
    let shown = stdout(&adr(&dir, &["show", rel], &[], ""));
    assert!(
        shown.contains(&format!("owner runs: adr answer {}@", delta_id)),
        "{}",
        shown
    );
    assert!(!shown.contains("not yet: \"Is delta"), "{}", shown);
}

// adr: da80931c-68cf-4c51-acce-060307bbd973
// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// The refusal names the move `show` names for the question Q2 waits on. Q1 is
// edited after it was asked: while the plan awaits approval the move is the
// owner's approve (ask would refuse); once approved it is the agent's ask.
#[test]
fn answer_names_the_move_show_names_for_an_edited_target() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let rel = "adrdom/decisions/example.md";
    let record = write_record(&dir, rel, &format!("{}{}", Q1, q2_depends_on_q1("=A")));
    approve_record(&dir, rel);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let q2_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[1].clone();

    let text = std::fs::read_to_string(&record).unwrap();
    std::fs::write(
        &record,
        text.replacen("is undecided.", "is undecided, reworded.", 1),
    )
    .unwrap();

    let refused = adr_answer(&dir, &q2_id, "A");
    assert!(!refused.status.success());
    assert!(
        stderr(&refused)
            .trim_end()
            .ends_with(&format!("owner runs: adr approve {}", rel)),
        "{}",
        stderr(&refused)
    );
    let shown = stdout(&adr(&dir, &["show", rel], &[], ""));
    assert!(
        shown.contains(&format!("owner runs: adr approve {}", rel)),
        "{}",
        shown
    );

    approve_record(&dir, rel);
    let refused = adr_answer(&dir, &q2_id, "A");
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains(&format!("agent runs: adr ask {}", rel)),
        "{}",
        stderr(&refused)
    );
    let shown = stdout(&adr(&dir, &["show", rel], &[], ""));
    assert!(
        shown.contains(&format!("agent runs: adr ask {}", rel)),
        "{}",
        shown
    );
}

// adr: da80931c-68cf-4c51-acce-060307bbd973
//
// Two questions edited since they were asked, and a third standing on both:
// the refusal of `answer` on the third names one `adr ask` clause listing
// both headings, as `show` prints it.
#[test]
fn answer_names_one_ask_clause_for_every_edited_heading() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let body = "\nProblem: the widget's state is undecided.\n\nStandard: plain words.\n\n- A) Yes.\n  Why: reasons.\n  Costs: costs.\n- B) No.\n  Why: other reasons.\n  Costs: other costs.\n";
    let content = format!(
        "# Example\n\n### Is alpha ready?\ndepends: widget\n{body}\n### Is beta ready?\ndepends: widget\n{body}\n### Is delta ready?\ndepends: widget, Is alpha ready?=*, Is beta ready?=*\n{body}"
    );
    let rel = "adrdom/decisions/example.md";
    let record = write_record(&dir, rel, &content);
    approve_record(&dir, rel);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let q3_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[2].clone();
    let version = current_version(&dir, &q3_id);

    let text = std::fs::read_to_string(&record).unwrap();
    let text = text.replacen("is undecided.", "is undecided, reworded.", 2);
    std::fs::write(&record, text).unwrap();
    approve_record(&dir, rel);

    let refused = adr_owner(
        &dir,
        &["answer", &format!("{}@{}", q3_id, version), "A"],
        "",
    );
    assert!(!refused.status.success());
    let err = stderr(&refused);
    assert_eq!(err.matches("agent runs: adr ask").count(), 1, "{}", err);
    // The refusal names both headings; its ask clause carries no `#`
    // comment, so no clause after it is read as part of one.
    assert!(
        err.contains(
            "\"Is alpha ready?\" has no decision yet, \"Is beta ready?\" has no decision yet"
        ),
        "{}",
        err
    );
    assert!(!err.contains('#'), "{}", err);
}

// adr: da80931c-68cf-4c51-acce-060307bbd973
// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// A plan `approve` would refuse: `next` names what clears the refusal, and so
// does the refusal of `answer`; neither tells the owner to approve.
#[test]
fn answer_names_the_plan_refusal_next_names_when_approve_would_refuse() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let rel = "adrdom/decisions/example.md";
    let record = write_record(&dir, rel, &format!("{}{}", Q1, q2_depends_on_q1("=A")));
    approve_record(&dir, rel);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let q2_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[1].clone();

    // Q1 is edited and a heading with no depends line is added: approve
    // would refuse the plan.
    let text = std::fs::read_to_string(&record).unwrap();
    let text = text.replacen("is undecided.", "is undecided, reworded.", 1);
    std::fs::write(
        &record,
        format!("{}\n### 1.9 Is the widget plain?\n", text.trim_end()),
    )
    .unwrap();

    let refused = adr_answer(&dir, &q2_id, "A");
    assert!(!refused.status.success());
    let err = stderr(&refused);
    assert!(!err.contains("owner runs: adr approve"), "{}", err);
    assert!(err.contains("1.9 Is the widget plain?"), "{}", err);
    let shown = stdout(&adr(&dir, &["show", rel], &[], ""));
    assert!(shown.contains("1.9 Is the widget plain?"), "{}", shown);
    assert!(!shown.contains("owner runs: adr approve"), "{}", shown);
}
