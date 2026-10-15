// "Does the tool stop while the terms file waits for approval?" (aa475aa3
// A, supersedes "Does check fail a terms file that differs from its
// latest approval?" and "Does answer wait for terms approval?"): ask,
// answer, refuse, and check all stop while adrdom/terms.md differs from
// its latest approval, and the owner approves it first. There is no
// tolerant path any more, and no "waiting" note either -- an unapproved
// terms file is always a failure/refusal, never a mere draft.
mod common;

use common::*;

const PASS_RECORD: &str = r#"# Example

### 1.1 Is this pass?
depends: pass

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.
"#;

/// A second, first-tier question (also depending directly on "pass"), so a
/// single `adr ask` picks up both ids at once -- lets a test answer one and
/// refuse the other.
const SECOND_QUESTION: &str = r#"
### 1.2 Is this also pass?
depends: pass

Problem: whether the second one passes too.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.
"#;

const TERMS_PASS_V1: &str = "# Terms\n\n\
## pass\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: Whether something is accepted.\n\
aliases: \n";

/// A draft edit on top of an already-approved `TERMS_PASS_V1`: a brand-new,
/// unused term, added exactly the way an agent would while proposing a
/// term -- nothing here pairs anything current, which is the point: even a
/// harmless-looking draft now stops every command below.
const TERMS_PASS_V1_PLUS_DRAFT_TERM: &str = "# Terms\n\n\
## pass\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: Whether something is accepted.\n\
aliases: \n\
\n\
## spare\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: An unused fixture term.\n\
aliases: \n";

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// `check` fails outright while adrdom/terms.md differs from its latest
// approval, naming the one move that clears it -- no "waiting" note, and
// no dependency on anything current reaching any particular term.
#[test]
fn check_fails_naming_the_owners_move_while_terms_is_unapproved() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/terms.md", TERMS_PASS_V1);
    // No `terms` ledger row: unapproved.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(
        text.contains("adrdom/terms.md: \"pass\" is new; owner runs: adr approve adrdom/terms.md pass"),
        "{}",
        text
    );
    assert!(
        text.contains("not yet: commit, after the owner approves adrdom/terms.md"),
        "{}",
        text
    );
}

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// `ask` refuses outright while adrdom/terms.md is unapproved, even for a
// candidate that pairs a term nobody touched.
#[test]
fn ask_refuses_while_terms_is_unapproved() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/terms.md", TERMS_PASS_V1);
    approve_record(&dir, "adrdom/terms.md");
    let record = write_record(&dir, "adrdom/decisions/example.md", PASS_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");

    let terms_path = dir.join("adrdom/terms.md");
    std::fs::write(&terms_path, TERMS_PASS_V1_PLUS_DRAFT_TERM).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("adrdom/terms.md: \"spare\" is new; owner runs: adr approve adrdom/terms.md spare"),
        "{}",
        stderr(&out)
    );
}

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// `answer` and `refuse` stop the same way once a question is already
// asked: judged against the file's approval state at answer time, not at
// ask time -- unlike the superseded cd450764 B, there is no fallback that
// lets either one proceed while a draft sits on top of the last approval.
#[test]
fn answer_and_refuse_stop_while_terms_is_unapproved() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/terms.md", TERMS_PASS_V1);
    approve_record(&dir, "adrdom/terms.md");

    let mut content = PASS_RECORD.to_string();
    content.push_str(SECOND_QUESTION);
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let asked = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&asked);
    let (id1, id2) = (ids[0].clone(), ids[1].clone());

    let terms_path = dir.join("adrdom/terms.md");
    std::fs::write(&terms_path, TERMS_PASS_V1_PLUS_DRAFT_TERM).unwrap();

    let answer_out = adr_answer(&dir, &id1, "A");
    assert!(!answer_out.status.success(), "{}", stdout(&answer_out));
    assert!(
        stderr(&answer_out).contains("adrdom/terms.md: \"spare\" is new; owner runs: adr approve adrdom/terms.md spare"),
        "{}",
        stderr(&answer_out)
    );

    let refuse_out = adr_answer(&dir, &id2, "refuse");
    assert!(!refuse_out.status.success(), "{}", stdout(&refuse_out));
    assert!(
        stderr(&refuse_out).contains("adrdom/terms.md: \"spare\" is new; owner runs: adr approve adrdom/terms.md spare"),
        "{}",
        stderr(&refuse_out)
    );
}

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// The owner running `adr approve adrdom/terms.md` is the one move that
// clears every one of the above: `check`, `answer`/`refuse` on already-
// asked questions, and `ask` on a fresh candidate all proceed right after.
#[test]
fn approving_the_terms_file_clears_check_answer_refuse_and_ask() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/terms.md", TERMS_PASS_V1);
    approve_record(&dir, "adrdom/terms.md");

    let mut content = PASS_RECORD.to_string();
    content.push_str(SECOND_QUESTION);
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let asked = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&asked);
    let (id1, id2) = (ids[0].clone(), ids[1].clone());

    let terms_path = dir.join("adrdom/terms.md");
    std::fs::write(&terms_path, TERMS_PASS_V1_PLUS_DRAFT_TERM).unwrap();

    // Sanity: everything is blocked right up until the approve below.
    assert!(!adr(&dir, &["check"], &[], "").status.success());
    assert!(!adr_answer(&dir, &id1, "A").status.success());

    let approve_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(approve_out.status.success(), "{}", stderr(&approve_out));

    assert!(adr(&dir, &["check"], &[], "").status.success());

    let answer_out = adr_answer(&dir, &id1, "A");
    assert!(answer_out.status.success(), "{}", stderr(&answer_out));
    write_test_comment(&dir, &id1);

    let refuse_out = adr_answer(&dir, &id2, "refuse");
    assert!(refuse_out.status.success(), "{}", stderr(&refuse_out));

    // A fresh candidate can be asked too, now that the wait is over.
    let mut with_third = std::fs::read_to_string(&record).unwrap();
    with_third.push_str(
        "\n### 1.3 Does a third pass?\ndepends: pass\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n  Costs: z.\n- B) w.\n  Why: y.\n  Costs: z.\n",
    );
    std::fs::write(&record, &with_third).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
}
