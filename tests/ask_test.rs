mod common;

use common::*;

#[test]
fn example_record_round_trips_and_ask_assigns_ids() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    // `approve`, not `ask`, mints the heading's id now.
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

    let asked_before = asked_row_count(&dir);

    // Edit the question's Problem text (inside the hashed range, before any Answer).
    let content = std::fs::read_to_string(&record).unwrap();
    let edited = content.replace(
        "names leaked into decisions.",
        "names leaked into decisions, badly.",
    );
    std::fs::write(&record, edited).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));
    assert!(
        stdout(&out).contains("re-asked"),
        "expected a re-ask in output: {}",
        stdout(&out)
    );

    let asked_after = asked_row_count(&dir);
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

    let answer_arg = id_at_version(&dir, question_id);
    let answer_out = adr_owner(&dir, &["answer", &answer_arg, "A"], "");
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
    let never_approved = write_record(
        &dir,
        "adrdom/decisions/never.md",
        // A heading of its own: one heading never stands in two topic files
        // (23f21671 "Does one question heading stand in two topic files?").
        &Q1_ONLY.replace("What color is the widget?", "What shape is the widget?"),
    );
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

//
// The decision "What stands under a question's heading?" (178a232e), its "two or three lettered options" clause, the refusal path:
// one option is too few.
#[test]
fn ask_refuses_a_question_with_one_option() {
    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
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

//
// The decision "What stands under a question's heading?" (178a232e), its "two or three lettered options" clause, the other refusal
// path: four options is too many.
#[test]
fn ask_refuses_a_question_with_four_options() {
    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
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
fn ask_refuses_on_a_banned_word_naming_the_heading_and_term() {
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
    assert!(err.contains("banned word (term authority)"), "{}", err);
    assert!(
        !err.to_lowercase().contains("predecessors"),
        "the matched word must never reach stderr: {}",
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
    assert!(err.contains("banned word"), "{}", err);
    assert!(!err.contains("(term"), "{}", err);
    assert!(
        !err.to_lowercase().contains("predecessors"),
        "the matched word must never reach stderr: {}",
        err
    );
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
    approve_terms_via_ledger(&dir, terms);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 What is the unit of authority called?\" uses no term"),
        "{}",
        err
    );
}

//
// "Does a retired term hold its words?" -- no, so retiring frees a term's
// words and a new term may take them: "authority"
// is retired below, "power" is a new current term whose alias is
// "authority", and a brand new question's use of the word "authority"
// pairs "power"'s id -- not the retired term's -- proving the freed word
// really did pass to the new concept.
#[test]
fn ask_pairs_a_word_to_the_current_term_that_took_it_from_a_retired_one() {
    // adr: 5577f646-a9d4-4785-9193-2317b9aa502d
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

// adr: 711a7636-9f4d-402d-ae62-265208adf0ac
// `ask refuses a question none of whose words pair to a term`: no word of
// the heading, problem, standard or options below pairs to the one term
// approved ("authority"), so ask refuses instead of writing an empty terms
// line, and the refusal names the agent's next move.
#[test]
fn ask_refuses_a_candidate_that_pairs_no_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let content = "# Example\n\n### 1.1 Is this pass?\n\nProblem: whether it passes.\n\nStandard: clear yes or no.\n\n- A) Yes it passes.\n  Why: reasons.\n  Costs: costs.\n- B) No it does not.\n  Why: other reasons.\n  Costs: other costs.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", content);
    approve_minimal_terms(&dir, "authority");
    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("\"1.1 Is this pass?\" uses no term"),
        "{}",
        err
    );
    // The refusal names the next move, and it is the agent's to make.
    assert!(err.contains("adds the term to adrdom/terms.md"), "{}", err);
    assert!(err.contains("agent "), "{}", err);

    // No terms line written to the record...
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after, content);
    assert!(!after.lines().any(|l| l.starts_with("terms:")));

    // ...and no `asked` row written to the ledger.
    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger_before, ledger_after,
        "ask should write nothing to the ledger on refusal"
    );
}

#[test]
fn ask_writes_the_terms_line_with_an_explicit_alias_plural() {
    // adr: 521e6211-344a-4b56-a1aa-07be956b3b29 -- "decisions" pairs only
    // because it is written as an explicit alias of "decision"; there is no
    // implicit "s"/"es" plural match any more.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // The term must exist before `approve` runs, so it can resolve the
    // heading's own `depends: decision` line.
    let terms_content = "# Terms\n\n## decision\nid: 99999999-9999-9999-9999-999999999999\ndefinition: A fixture term for tests.\naliases: decisions\n";
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();
    approve_terms_via_ledger(&dir, terms_content);
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));

    let content = std::fs::read_to_string(&record).unwrap();
    assert!(
        content
            .lines()
            .any(|l| l.starts_with("terms: ") && l.contains("decisions=99999999-9999-9999-9999-999999999999")),
        "expected the explicit alias to pair on the terms line: {}",
        content
    );
}

// adr: f571a454-3e6c-46c1-b754-0822a5d1d667
//
// `ask` pairs every word of the question -- heading, body lines and every
// option -- to terms, not only the heading or a chosen answer. In
// `AUTHORITY_QUESTION`, "leaked" sits only in the Problem line ("names
// leaked into decisions") and "predecessors" sits only in option B's Why
// line ("matches predecessors"); the heading and option A never use either
// word. Both still show up on the one terms line `ask` writes.
#[test]
fn ask_pairs_a_word_from_the_problem_line_and_a_word_from_a_single_option() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // "decision" is needed for the heading's own `depends: decision` line
    // to resolve at approve time; "leaked" and "predecessors" are the
    // words under test here.
    approve_terms_with(&dir, &["decision", "leaked", "predecessors"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", AUTHORITY_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));

    let content = std::fs::read_to_string(&record).unwrap();
    let terms_line = content
        .lines()
        .find(|l| l.starts_with("terms: "))
        .unwrap_or_else(|| panic!("no terms line written: {}", content));
    assert!(
        terms_line.contains("leaked=99999991-9999-9999-9999-999999999999"),
        "Problem-line-only word missing from terms line: {}",
        terms_line
    );
    assert!(
        terms_line.contains("predecessors=99999992-9999-9999-9999-999999999999"),
        "option-B-only word missing from terms line: {}",
        terms_line
    );
}

// --- core.md: "What does a depends line name?" -- the "does not resolve"
// half, reached through ask on a file approve already accepted --------

// adr: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf
//
// "every question carries [a depends] line ...; approve and ask stop a
// line that is missing or names anything that does not resolve": the
// path this test reaches is exactly the one the owner's workflow can
// actually produce -- approve already accepted the file (minting an id
// and rewriting the depends line to it), and an agent then edits the
// depends line on disk without re-approving, either retargeting it to a
// uuid that resolves to nothing at all or deleting the line outright.
// Either edit is a `depends:` edit, so it moves the file's plan hash out
// from under the last `approved` ledger row -- `ask` does stop the file,
// but with the generic "plan not approved" refusal, before it ever
// reaches a per-entry resolution check (src/depends.rs's own doc comment
// names only `approve`'s validation and `check`'s plan check as callers
// of `resolve_dep`; `ask` is not one of them). `adr check`, run on the
// same broken file right after, is what actually names the specific
// thing wrong with the line ("... which is not a current decision" /
// "has no depends line") -- confirming ask's refusal, though real, never
// gets that specific.
#[test]
fn ask_refuses_a_depends_entry_that_no_longer_resolves_or_is_missing_after_approval() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1_ONLY);
    approve_record(&dir, "adrdom/decisions/example.md");
    let approved = std::fs::read_to_string(&record).unwrap();

    // approve_minimal_terms's fixed term id.
    let widget_term_id = "99999999-9999-9999-9999-999999999999";
    assert!(
        approved.contains(&format!("depends: {}", widget_term_id)),
        "expected approve to have rewritten the depends line to the term id: {}",
        approved
    );

    // Case 1: retargeted to a uuid naming nothing at all.
    let garbage = approved.replace(
        &format!("depends: {}", widget_term_id),
        "depends: 00000000-0000-0000-0000-000000000000",
    );
    std::fs::write(&record, &garbage).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "ask should have refused: {}", stdout(&out));
    assert!(
        stderr(&out).contains("plan not approved"),
        "{}",
        stderr(&out)
    );
    let check_out = adr(&dir, &["check"], &[], "");
    // The edited file awaits approval again: check prints the finding as a note (912f4f3b-1213-435a-af0d-7617017fcff4).
    assert!(check_out.status.success());
    assert!(
        stdout(&check_out).contains("which is not a current decision"),
        "{}",
        stdout(&check_out)
    );

    // Case 2: the depends line removed outright.
    let missing: String = approved
        .lines()
        .filter(|l| !l.starts_with("depends:"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(&record, &missing).unwrap();

    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out2.status.success(), "ask should have refused: {}", stdout(&out2));
    assert!(
        stderr(&out2).contains("plan not approved"),
        "{}",
        stderr(&out2)
    );
    let check_out2 = adr(&dir, &["check"], &[], "");
    assert!(check_out2.status.success());
    assert!(
        stdout(&check_out2).contains("has no depends line"),
        "{}",
        stdout(&check_out2)
    );
}
