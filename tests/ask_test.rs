mod common;

use common::*;

#[test]
fn example_record_round_trips_and_ask_assigns_ids() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    // 9.1 A: `approve`, not `ask`, mints the heading's id now.
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    assert_eq!(ids.len(), 1, "expected an id for the question: {}", content);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_rows: Vec<&str> = ledger.lines().filter(|l| l.contains("\tasked\t")).collect();
    assert_eq!(asked_rows.len(), 1, "ledger: {}", ledger);
    for id in &ids {
        assert!(asked_rows.iter().any(|r| r.contains(id.as_str())));
    }

    // Running ask again with nothing changed should not add more rows.
    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success());
    let ledger2 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger, ledger2,
        "re-running ask with no changes should be a no-op"
    );
}

#[test]
fn ask_reasks_on_edit() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_before = ledger_before
        .lines()
        .filter(|l| l.contains("\tasked\t"))
        .count();

    // Edit the question's Problem text (inside the hashed range, before any Answer).
    let content = std::fs::read_to_string(&record).unwrap();
    let edited = content.replace(
        "names leaked into decisions.",
        "names leaked into decisions, badly.",
    );
    std::fs::write(&record, edited).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));
    assert!(
        stdout(&out).contains("re-asked"),
        "expected a re-ask in output: {}",
        stdout(&out)
    );

    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_after = ledger_after
        .lines()
        .filter(|l| l.contains("\tasked\t"))
        .count();
    assert_eq!(
        asked_after,
        asked_before + 1,
        "expected exactly one new asked row"
    );
}

#[test]
fn ask_refuses_to_edit_an_answered_section() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    let question_id = &ids[0];

    let answer_out = adr_owner(&dir, &["answer", question_id, "A"], "");
    assert!(
        answer_out.status.success(),
        "answer failed: {}",
        stderr(&answer_out)
    );

    // Now edit the answered question's Problem text.
    let content = std::fs::read_to_string(&record).unwrap();
    let edited = content.replace("names leaked into decisions.", "something else entirely.");
    std::fs::write(&record, edited.clone()).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        !out.status.success(),
        "ask should have refused: {}",
        stdout(&out)
    );
    assert_eq!(out.status.code(), Some(1));

    // Nothing should have changed: the file is exactly what we wrote.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after, edited,
        "ask must not modify the file when it refuses"
    );
}

const Q1_ONLY: &str = r#"# Example

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

const Q2_ADDED_AFTER_APPROVAL: &str = r#"
### 1.2 What color is the second widget?
depends: widget

Problem: the second widget's color is undecided.

Standard: plain words.

- A) The second widget is red.
  Why: reasons.
  Costs: costs.
- B) The second widget is blue.
  Why: other reasons.
  Costs: other costs.
"#;

// adr: da59b448-1afb-4469-b40e-69eda8f253c2
//
// core (0): "ask stops any question whose heading is not in the plan the
// owner approved". Two ways a heading ends up outside the approved plan:
// the file was never approved at all, and a heading appended after an
// approval that covered only the headings before it. In both, `ask`
// refuses "plan not approved" and writes nothing -- no id minted, no
// `asked` ledger row -- rather than asking the heading that was in the
// approved plan and silently ignoring the one that was not.
#[test]
fn ask_refuses_a_heading_added_after_approval_and_a_never_approved_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");

    // A file never approved at all.
    let never_approved = write_record(&dir, "adrdom/decisions/never.md", Q1_ONLY);
    let out = adr(&dir, &["ask", never_approved.to_str().unwrap()], &[], "");
    assert!(
        !out.status.success(),
        "ask should refuse an unapproved file: {}",
        stdout(&out)
    );
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/never.md: plan not approved; owner runs: adr approve adrdom/decisions/never.md"
    );

    // A heading added after approval: the owner approved only 1.1, then the
    // agent appends 1.2 without asking the owner to re-approve.
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1_ONLY);
    approve_record(&dir, "adrdom/decisions/example.md");
    let approved = std::fs::read_to_string(&record).unwrap();

    let mut with_new_heading = approved.clone();
    with_new_heading.push_str(Q2_ADDED_AFTER_APPROVAL);
    std::fs::write(&record, &with_new_heading).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        !out.status.success(),
        "ask should refuse a heading outside the approved plan: {}",
        stdout(&out)
    );
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/decisions/example.md: plan not approved; owner runs: adr approve adrdom/decisions/example.md"
    );

    // Nothing written: no id minted for the new heading, no ledger row.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after, with_new_heading,
        "ask must not modify the file when it refuses"
    );
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tasked\t")),
        "ask must not have asked anything: {}",
        ledger
    );
}

#[test]
fn ask_refuses_a_question_with_one_option() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\n### 1.1 One option?\n\nProblem: p.\n\nStandard: s.\n\n- A) Only choice.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("1 options; at least two"), "{}", err);
    assert!(err.contains("agent fixes the topic file"), "{}", err);

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, content, "ask must not modify the file when it refuses");
    assert!(extract_ids(&after).is_empty(), "no id should have been assigned: {}", after);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.contains("\tasked\t"), "no asked row should have been written: {}", ledger);
}

#[test]
fn ask_refuses_a_question_with_four_options() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let content = "# Example\n\n### 1.1 Four options?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n- D) Four.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("4 options; at most three"), "{}", err);
    assert!(err.contains("agent fixes the topic file"), "{}", err);

    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, content, "ask must not modify the file when it refuses");
    assert!(extract_ids(&after).is_empty(), "no id should have been assigned: {}", after);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.contains("\tasked\t"), "no asked row should have been written: {}", ledger);
}

#[test]
fn ask_leaves_a_skeleton_heading_unasked_though_approve_gave_it_an_id() {
    // 9.1 A point 1: `approve` gives every heading an id, skeletons
    // included -- but `ask` still never asks a skeleton, so it gets no
    // ledger row for it.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    let content = format!(
        "{}\n### 1.2 Where does the plan live?\ndepends: example\n",
        EXAMPLE_RECORD
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");
    let after_approve = std::fs::read_to_string(&record).unwrap();
    let idx = after_approve.find("### 1.2 Where does the plan live?").unwrap();
    assert!(
        after_approve[idx..].contains("id: "),
        "approve should have given the skeleton an id too: {}",
        &after_approve[idx..]
    );

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));
    let after_ask = std::fs::read_to_string(&record).unwrap();
    // `ask` does rewrite the other (real) question's `terms:` line, but the
    // skeleton's own slice of the file -- from its heading onward, since it
    // is the last heading -- must stay exactly as `approve` left it.
    let skeleton_before = &after_approve[idx..];
    let skeleton_after = &after_ask[after_ask.find("### 1.2 Where does the plan live?").unwrap()..];
    assert_eq!(skeleton_after, skeleton_before, "ask must not touch the skeleton section");
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.contains("Where does the plan live?"));
}

// --- terms wiring (3): missing/shape/approval refusals, banned, retired,
// zero pairings, and the plural terms-line write --------------------------

const AUTHORITY_QUESTION: &str = r#"# Example

### 1.1 What is the unit of authority called?
depends: decision

Problem: names leaked into decisions.

Standard: plain words.

- A) The authoritative sentence is a decision.
  Why: plainest word.
  Costs: two nouns.
- B) The whole unit is a block.
  Why: matches predecessors.
  Costs: names a region.
"#;

#[test]
fn ask_refuses_when_terms_md_is_missing() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        "ask: adrdom/terms.md missing; agent writes it, owner runs: adr approve adrdom/terms.md"
    );

    // Nothing written.
    let content = std::fs::read_to_string(&record).unwrap();
    assert_eq!(content, AUTHORITY_QUESTION);
}

#[test]
fn ask_refuses_on_a_terms_md_shape_error() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    // Missing the "# Terms" h1: a shape error.
    write_record(
        &dir,
        "adrdom/terms.md",
        "## authority\nid: 11111111-1111-1111-1111-111111111111\ndefinition: x.\naliases: \n",
    );

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.starts_with("ask: adrdom/terms.md:"), "{}", err);
    assert!(err.contains("missing h1"), "{}", err);
}

#[test]
fn ask_refuses_on_a_banned_word_naming_the_word_and_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    // Ban the word before it appears anywhere: `ban` itself refuses an
    // already-in-use word, so the record has to come after.
    assert!(adr(&dir, &["ban", "--term", "authority"], &[], "predecessors\n")
        .status
        .success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("banned word \"predecessors\" (term authority)"),
        "{}",
        err
    );
}

#[test]
fn ask_refuses_on_a_banned_word_with_no_term_name() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    assert!(adr(&dir, &["ban"], &[], "predecessors\n").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("banned word \"predecessors\""), "{}", err);
    assert!(!err.contains("(term"), "{}", err);
}

#[test]
fn ask_pairs_nothing_for_a_word_only_on_a_retired_term() {
    // "authority" only appears on a retired term, and the question's own
    // text pairs no other term: `resolve` never matches a retired term
    // (9.3 B), so the word pairs nothing at all, and the existing
    // zero-pairing refusal fires -- there is no separate retired-match
    // refusal any more.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    let terms = "# Terms\n\n## authority\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A fixture term.\naliases: \nretired: yes\n";
    write_record(&dir, "adrdom/terms.md", terms);
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let hash = {
        // Mirror terms::file_hash for this fixed content: each line
        // right-trimmed before joining (e.g. the "aliases: " line's
        // trailing space is dropped).
        use sha2::{Digest, Sha256};
        let trimmed = terms
            .split('\n')
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        let mut h = Sha256::new();
        h.update(trimmed.as_bytes());
        h.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>()
    };
    let mut existing = std::fs::read_to_string(&ledger_path).unwrap();
    existing.push_str(&format!("2024-01-01T00:00:00Z\tterms\t-\t{}\tadrdom/terms.md\n", hash));
    std::fs::write(&ledger_path, existing).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 What is the unit of authority called?\" pairs no term"),
        "{}",
        err
    );
}

#[test]
fn ask_pairs_a_word_to_the_current_term_that_took_it_from_a_retired_one() {
    // A retired term and a current term share the same word ("authority"):
    // `duplicate_word_errors` allows this since the retired entry is
    // ignored, and a new pairing (ask's terms line) sees current terms
    // only, so the word binds to the current term's id.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // The "decision" term must exist and be approved before `approve` runs
    // on the record, so it can resolve the heading's own `depends:
    // decision` line to an id.
    let terms = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A fixture term.\n\
aliases: \n\
\n\
## authority\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: The old fixture term.\n\
aliases: \n\
retired: yes\n\
\n\
## power\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: The new fixture term.\n\
aliases: authority\n";
    write_record(&dir, "adrdom/terms.md", terms);
    approve_terms_via_ledger(&dir, terms);
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let content = std::fs::read_to_string(&record).unwrap();
    assert!(
        content.contains("terms: authority=22222222-2222-2222-2222-222222222222"),
        "{}",
        content
    );
}

#[test]
fn ask_refuses_a_candidate_that_pairs_no_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let content = "# Example\n\n### 1.1 Does this pass?\n\nProblem: whether it passes.\n\nStandard: clear yes or no.\n\n- A) Yes it passes.\n  Why: reasons.\n  Costs: costs.\n- B) No it does not.\n  Why: other reasons.\n  Costs: other costs.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", content);
    approve_minimal_terms(&dir, "authority");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Does this pass?\" pairs no term"),
        "{}",
        err
    );
    assert!(err.contains("adds the term to adrdom/terms.md"), "{}", err);

    // Nothing written.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, content);
}

#[test]
fn ask_writes_the_terms_line_with_a_plural_word() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // The term must exist before `approve` runs, so it can resolve the
    // heading's own `depends: decision` line.
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));

    let content = std::fs::read_to_string(&record).unwrap();
    assert!(
        content
            .lines()
            .any(|l| l.starts_with("terms: ") && l.contains("decisions=99999999-9999-9999-9999-999999999999")),
        "expected a plural match on the terms line: {}",
        content
    );
}
