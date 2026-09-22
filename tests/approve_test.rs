mod common;

use common::*;

/// A record with the full shape: `# topic`, context prose, two `##`
/// groups, and `### q?` bodies with Problem/Standard/options.
const TWO_GROUP_RECORD: &str = r#"# Topic

Context prose line one.
Context prose line two.

## Group One

### 1.1 First question?

Problem: p1.

Standard: s1.

- A) Answer one.
  Why: reasons.
  Costs: costs.
- B) Answer two.
  Why: other reasons.
  Costs: other costs.
"#;

const GROUP_TWO_ADDITION: &str = r#"
## Group Two

### 2.1 Second question?

Problem: p2.

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

## Group One

### 1.1 Only question?

Problem: p.

Standard: s.

- A) Answer A.
  Why: reasons.
  Costs: costs.
- B) Answer B.
  Why: other reasons.
  Costs: other costs.
"#;

const HEADING_REMOVED: &str = "# Topic\n\nContext prose.\n\n## Group One\n";

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

#[test]
fn approve_writes_approved_and_context_rows_then_is_idempotent() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
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
    assert!(stdout(&out2).contains("context unchanged"), "{}", stdout(&out2));
}

// --- approve refuses on an option-count shape error --------------------

#[test]
fn approve_refuses_a_question_with_one_option() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let content = "# Example\n\n## Group\n\n### 1.1 One option?\n\nProblem: p.\n\nStandard: s.\n\n- A) Only choice.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("1 options; at least two"), "{}", err);
    assert!(err.contains("owner fixes the topic file"), "{}", err);

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
    let content = "# Example\n\n## Group\n\n### 1.1 Four options?\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n- D) Four.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("4 options; at most three"), "{}", err);
    assert!(err.contains("owner fixes the topic file"), "{}", err);

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

#[test]
fn editing_context_prose_requires_reapproval_with_one_new_context_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "question");
    let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());

    let edited = TWO_GROUP_RECORD.replace(
        "Context prose line one.",
        "Context prose line one, edited.",
    );
    std::fs::write(&record, &edited).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("context not approved"), "{}", stderr(&out));

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

// --- tier rule: a later group waits on an earlier one ------------------

#[test]
fn ask_refuses_group_two_while_group_one_unanswered_then_passes_after_answer() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "question");
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

    // Nothing changed: group two got no id.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(extract_ids(&after).len(), 1);

    // Answer group one, then ask succeeds for group two.
    let answer_out = adr_owner(&dir, &["answer", &first_id, "A"], "");
    assert!(answer_out.status.success(), "answer failed: {}", stderr(&answer_out));

    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "ask failed: {}", stderr(&out2));
    let after2 = std::fs::read_to_string(&record).unwrap();
    assert_eq!(extract_ids(&after2).len(), 2);
}

// --- a struck asked heading is withdrawn on approve ---------------------

#[test]
fn approve_withdraws_a_struck_asked_heading() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "question");
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

// --- approve refuses when a current heading vanishes without a supersede --

#[test]
fn approve_refuses_when_answered_heading_gone_and_not_superseded() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "question");
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["answer", &id, "A"], "").status.success());

    std::fs::write(&record, HEADING_REMOVED).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains(id.as_str()), "{}", err);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.lines().any(|l| l.contains("\twithdrawn\t")));
}

// --- approve allows a vanished heading once it is superseded ------------

#[test]
fn approve_allows_a_vanished_heading_once_superseded() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "question");
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["answer", &old_id, "A"], "").status.success());

    let superseding = format!(
        "\n### 1.2 Replacement question?\n\nsupersedes: {}\n\nProblem: p2.\n\nStandard: s2.\n\n- A) New answer.\n  Why: reasons.\n  Costs: costs.\n- B) Other new answer.\n  Why: other reasons.\n  Costs: other costs.\n",
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
    assert!(adr_owner(&dir, &["answer", &new_id, "A"], "").status.success());

    // Now drop the (superseded) 1.1 heading entirely, keeping only 1.2.
    let content4 = std::fs::read_to_string(&record).unwrap();
    let idx = content4.find("### 1.2").expect("1.2 present");
    let rebuilt = format!("# Topic\n\nContext prose.\n\n## Group One\n\n{}", &content4[idx..]);
    std::fs::write(&record, &rebuilt).unwrap();

    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve should have allowed it: {}", stderr(&out));
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.lines().any(|l| l.contains("\twithdrawn\t") && l.contains(old_id.as_str())));
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
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());

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

## Group One

### 1.1 What is the decision?

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

## Group One

### 1.1 What is the choice?

Problem: the choice is unclear.

Standard: plain words.

- A) The choice is final.
  Why: reasons.
  Costs: costs.
- B) The choice is provisional.
  Why: other reasons.
  Costs: other costs.
"#;

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
    assert!(adr_owner(&dir, &["answer", &id, "A"], "").status.success());

    // Move the "choice" alias from `decision` to `verdict`.
    std::fs::write(&terms, TERMS_ALIAS_MOVED).unwrap();

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(
            "word \"choice\" moved off term \"decision\", carried by adrdom/decisions/example.md: \"1.1 What is the choice?\""
        ),
        "{}",
        err
    );

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

## Group One

### 1.1 What is the decision?

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
    assert!(adr_owner(&dir, &["answer", &id, "A"], "").status.success());

    std::fs::write(&terms, TERMS_DECISION_REMOVED).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("term 33333333-3333-3333-3333-333333333333 removed"),
        "{}",
        err
    );
    assert!(err.contains("What is the decision?"), "{}", err);
    assert!(err.contains("owner retires instead"), "{}", err);
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
    assert!(adr_owner(&dir, &["answer", &id, "A"], "").status.success());

    std::fs::write(&terms, TERMS_DECISION_RETIRED).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("term \"decision\" retired but carried by"),
        "{}",
        err
    );
    assert!(err.contains("What is the decision?"), "{}", err);
    assert!(err.contains("owner supersedes first"), "{}", err);
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
    assert!(adr_owner(&dir, &["answer", &id, "A"], "").status.success());

    std::fs::write(&terms, TERMS_AFTER_RENAME_ALIAS_KEPT).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve should have allowed the rename: {}",
        stderr(&out)
    );
}

#[test]
fn approve_prints_dependents_when_a_definition_changes() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", DECISION_RECORD_FOR_DEPENDENTS);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_BASIC);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    // Commit the approved terms.md so HEAD matches the ledger's latest
    // terms row: only then does approve have a trustworthy "before" text
    // to diff definitions against.
    // `--no-verify`: the repo's own pre-commit hook runs `adr check
    // --staged`, which would fail here purely because the record hasn't
    // been asked yet at this point in the test; that governance gate is
    // not what this test is about.
    run_git(&dir, &["add", "adrdom/terms.md"]);
    run_git(&dir, &["commit", "--no-verify", "-q", "-m", "approve terms"]);

    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["answer", &id, "A"], "").status.success());

    let terms_v2 = "# Terms\n\n## decision\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A choice answered by the owner, revised.\naliases: \n";
    std::fs::write(&terms, terms_v2).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("term \"decision\" definition changed"), "{}", text);
    assert!(
        text.contains("current: adrdom/decisions/example.md: 1.1 What is the decision?"),
        "{}",
        text
    );
}

const TERMS_ENTRY_DEPENDENT_V1: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice answered by the owner.\n\
aliases: \n\
\n\
## question\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A thing bound to a decision.\n\
aliases: \n";

/// The spec's dependents (step 7) also include a non-retired term whose
/// own terms line carries the changed term's id, not only current
/// decisions. Here "question"'s definition pairs "decision", so approving a
/// changed "decision" definition should print a `term: question` line.
#[test]
fn approve_prints_a_term_dependent_when_a_definition_changes() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_ENTRY_DEPENDENT_V1);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    // Commit the approved terms.md (with its machine-written "question"
    // terms line intact) so HEAD matches the ledger's latest terms row.
    run_git(&dir, &["add", "adrdom/terms.md"]);
    run_git(&dir, &["commit", "--no-verify", "-q", "-m", "approve terms"]);

    // Revise only "decision"'s definition text; leave "question"'s block
    // (and its terms line) untouched, as an owner hand-edit would.
    let content = std::fs::read_to_string(&terms).unwrap();
    let terms_v2 = content.replace(
        "definition: A choice answered by the owner.",
        "definition: A choice answered by the owner, revised.",
    );
    std::fs::write(&terms, &terms_v2).unwrap();

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("term \"decision\" definition changed"), "{}", text);
    assert!(text.contains("  term: question"), "{}", text);
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

// --- approve terms (4): a term's own terms: line is machine state, not
// prior state -- it is always overwritten from the fresh definition, never
// read as a dependent in its own right.

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
fn approve_overwrites_an_agent_written_bogus_terms_line_in_a_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_ENTRY_WITH_BOGUS_LINE);

    // Under the old rule this line was read as prior state: term
    // 99999999-... doesn't exist, so approve refused with "removed but
    // carried by term \"question\"". It's agent-written, unapproved
    // state; approve overwrites it from "question"'s actual definition
    // (which pairs "decision") instead of refusing on it.
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(out.status.success(), "approve should have succeeded: {}", stderr(&out));

    let content = std::fs::read_to_string(&terms).unwrap();
    assert!(!content.contains("bogus=99999999"), "{}", content);
    let question_block = &content[content.find("## question").unwrap()..];
    assert!(
        question_block.contains("terms: decision=11111111-1111-1111-1111-111111111111"),
        "{}",
        question_block
    );
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
aliases: \n";

#[test]
fn approve_refuses_retiring_a_term_carried_by_another_terms_definition() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_ENTRY_PAIRS_ANOTHER_V1);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());

    // "question"'s definition is untouched and still says "a decision";
    // only "decision" is retired.
    std::fs::write(&terms, TERMS_ENTRY_PAIRS_ANOTHER_RETIRED).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("term \"decision\" retired but carried by term \"question\"; owner supersedes first"),
        "{}",
        err
    );
}
