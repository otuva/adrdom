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
- B) Answer two.
  Why: other reasons.
"#;

const GROUP_TWO_ADDITION: &str = r#"
## Group Two

### 2.1 Second question?

Problem: p2.

Standard: s2.

- A) Second answer one.
  Why: reasons.
- B) Second answer two.
  Why: other reasons.
"#;

const ONE_QUESTION_RECORD: &str = r#"# Topic

Context prose.

## Group One

### 1.1 Only question?

Problem: p.

Standard: s.

- A) Answer A.
  Why: reasons.
- B) Answer B.
  Why: other reasons.
"#;

const HEADING_REMOVED: &str = "# Topic\n\nContext prose.\n\n## Group One\n";

// --- ask refusals naming `adr approve` -------------------------------

#[test]
fn ask_refuses_an_unapproved_file_naming_adr_approve() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
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
    assert!(adr(&dir, &["init"], &[], "").status.success());
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

// --- editing context prose requires re-approval -----------------------

#[test]
fn editing_context_prose_requires_reapproval_with_one_new_context_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
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
fn ask_refuses_group_two_while_group_one_unanswered_then_passes_after_lock() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
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
    let lock_out = adr_owner(&dir, &["lock", &first_id, "A"], "");
    assert!(lock_out.status.success(), "lock failed: {}", stderr(&lock_out));

    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success(), "ask failed: {}", stderr(&out2));
    let after2 = std::fs::read_to_string(&record).unwrap();
    assert_eq!(extract_ids(&after2).len(), 2);
}

// --- a struck asked heading is withdrawn on approve ---------------------

#[test]
fn approve_withdraws_a_struck_asked_heading() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
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

// --- approve refuses when an in-force heading vanishes without a supersede --

#[test]
fn approve_refuses_when_answered_heading_gone_and_not_superseded() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["lock", &id, "A"], "").status.success());

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
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION_RECORD);
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["lock", &old_id, "A"], "").status.success());

    let superseding = format!(
        "\n### 1.2 Replacement question?\n\nsupersedes: {}\n\nProblem: p2.\n\nStandard: s2.\n\n- A) New answer.\n  Why: reasons.\n",
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
    assert!(adr_owner(&dir, &["lock", &new_id, "A"], "").status.success());

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
definition: A choice locked by the owner.\n\
test: Does the owner have to choose among options?\n\
owner: adrdom/decisions/example.md\n\
aliases: \n\
banned: \n";

#[test]
fn ask_refuses_while_terms_unapproved() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
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
definition: A choice locked by the owner.\n\
test: Does the owner have to choose among options?\n\
owner: adrdom/decisions/example.md\n\
aliases: \n\
banned: \n";

#[test]
fn approve_fills_missing_term_ids() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
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

### 1.1 What counts as a decision?

Problem: whether a decision is well defined.

Standard: plain words.

- A) A decision is one commitment.
  Why: reasons.
- B) A decision is a block.
  Why: other reasons.
"#;

#[test]
fn ask_writes_terms_line_binding_word_to_term_id() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
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

// --- approve refuses a terms edit that moves an alias bound in force ----

const TERMS_WITH_ALIAS: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice locked by the owner.\n\
test: Does the owner have to choose among options?\n\
owner: adrdom/decisions/example.md\n\
aliases: choice\n\
banned: \n\
\n\
## verdict\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A ruling reached after deliberation.\n\
test: Is it the outcome of a formal process?\n\
owner: adrdom/decisions/example.md\n\
aliases: \n\
banned: \n";

const TERMS_ALIAS_MOVED: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice locked by the owner.\n\
test: Does the owner have to choose among options?\n\
owner: adrdom/decisions/example.md\n\
aliases: \n\
banned: \n\
\n\
## verdict\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A ruling reached after deliberation.\n\
test: Is it the outcome of a formal process?\n\
owner: adrdom/decisions/example.md\n\
aliases: choice\n\
banned: \n";

const CHOICE_QUESTION_RECORD: &str = r#"# Topic

Context prose.

## Group One

### 1.1 What is the choice?

Problem: the choice is unclear.

Standard: plain words.

- A) The choice is final.
  Why: reasons.
- B) The choice is provisional.
  Why: other reasons.
"#;

#[test]
fn approve_refuses_a_terms_edit_that_moves_an_alias_bound_by_an_in_force_decision() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
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
    assert!(adr_owner(&dir, &["lock", &id, "A"], "").status.success());

    // Move the "choice" alias from `decision` to `verdict`.
    std::fs::write(&terms, TERMS_ALIAS_MOVED).unwrap();

    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("What is the choice?"), "{}", err);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(ledger.lines().filter(|l| l.contains("\tterms\t")).count(), 1);
}

// --- approve refuses a newly banned word appearing in a record ---------

const TERMS_NO_BAN: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice locked by the owner.\n\
test: Does the owner have to choose among options?\n\
owner: adrdom/decisions/example.md\n\
aliases: \n\
banned: \n";

const TERMS_BANS_VERDICT: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice locked by the owner.\n\
test: Does the owner have to choose among options?\n\
owner: adrdom/decisions/example.md\n\
aliases: \n\
banned: verdict\n";

const RECORD_WITH_VERDICT: &str = r#"# Topic

This is the verdict on naming.

## Group One

### 1.1 Q?

Problem: p.

Standard: s.

- A) x.
  Why: y.
"#;

#[test]
fn approve_refuses_a_new_banned_word_appearing_in_a_record_naming_file_and_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    write_record(&dir, "adrdom/decisions/example.md", RECORD_WITH_VERDICT);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_NO_BAN);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());

    std::fs::write(&terms, TERMS_BANS_VERDICT).unwrap();
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should have refused: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("adrdom/decisions/example.md:3"), "{}", err);
    assert!(err.contains("verdict"), "{}", err);
}

// --- approve writes a terms row on a clean edit -------------------------

const TERMS_V2: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice locked by the owner, in writing.\n\
test: Does the owner have to choose among options?\n\
owner: adrdom/decisions/example.md\n\
aliases: \n\
banned: \n";

#[test]
fn approve_writes_a_terms_row_on_a_clean_edit() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
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
