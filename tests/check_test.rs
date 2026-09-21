mod common;

use common::*;

const ONE_QUESTION: &str = r#"# Example

## Group

### 3.1 Does this pass?

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
- B) No it does not.
  Why: other reasons.
"#;

/// Writes `ONE_QUESTION`, approves its plan/context via a hand-written
/// ledger row (so `ask` succeeds even once that command starts refusing
/// unapproved files, and so `check` doesn't flag its body as living under
/// an unapproved heading), asks, then owner-locks answer A.
fn lock_one_question(dir: &std::path::Path) -> (std::path::PathBuf, String) {
    let record = write_record(dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_via_ledger(dir, "adrdom/decisions/example.md", ONE_QUESTION);
    assert!(adr(dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    let out = adr_owner(dir, &["lock", &id, "A"], "");
    assert!(out.status.success(), "lock failed: {}", stderr(&out));
    (record, id)
}

#[test]
fn check_passes_on_a_clean_repo() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let (_record, id) = lock_one_question(&dir);
    write_tie(&dir, &id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(stdout(&out).contains("ties: 1 tied"));
}

#[test]
fn check_parse_failure() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/bad.md",
        "### 4.1 Bad option test?\n\n- A) no terminator here\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(stdout(&out).contains("parse:"), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("no decision sentence"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_ids_failure_missing_id() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    // No `adr ask` run, so the section has no id.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(stdout(&out).contains("ids:"), "{}", stdout(&out));
    assert!(stdout(&out).contains("missing id"), "{}", stdout(&out));
}

#[test]
fn check_ledger_malformed_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());

    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let mut content = std::fs::read_to_string(&ledger_path).unwrap();
    content.push_str("this-is-not-a-valid-row\n");
    std::fs::write(&ledger_path, content).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(stdout(&out).contains("ledger:"), "{}", stdout(&out));
    assert!(stdout(&out).contains("malformed row"), "{}", stdout(&out));
}

#[test]
fn check_answers_failure_hand_written_answer_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str("\n**Answer:** A\n");
    std::fs::write(&record, content).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(stdout(&out).contains("answers:"), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("answer line without ledger row"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_banned_default_and_message_modes_name_the_word() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());

    let ban_out = adr(&dir, &["ban"], &[], "forbiddenword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));
    assert!(!stdout(&ban_out).to_lowercase().contains("forbiddenword"));

    // default mode: a tracked file containing the banned word.
    let tracked = dir.join("notes.txt");
    std::fs::write(&tracked, "this contains forbiddenword right here\n").unwrap();
    run_git(&dir, &["add", "notes.txt"]);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let out_text = stdout(&out);
    assert!(
        out_text.contains("banned: notes.txt:1: banned word \"forbiddenword\""),
        "{}",
        out_text
    );

    // --message mode: a commit message file containing the banned word.
    let msg_path = dir.join("COMMIT_EDITMSG");
    std::fs::write(&msg_path, "fix: mentions forbiddenword in the subject\n").unwrap();
    let out2 = adr(
        &dir,
        &["check", "--message", msg_path.to_str().unwrap()],
        &[],
        "",
    );
    assert!(!out2.status.success());
    let out2_text = stdout(&out2);
    assert!(
        out2_text.contains("COMMIT_EDITMSG:1: banned word \"forbiddenword\""),
        "{}",
        out2_text
    );
}

#[test]
fn check_ties_failure_when_no_tie() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let (_record, _id) = lock_one_question(&dir);
    // No tie recorded anywhere for the id.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let out_text = stdout(&out);
    assert!(out_text.contains("ties:"), "{}", out_text);
    assert!(out_text.contains("no tie"), "{}", out_text);
}

#[test]
fn check_install_fails_when_settings_deny_entry_removed_and_passes_after_init() {
    let dir = init_repo();
    assert!(adr(&dir, &["init", "--claude"], &[], "").status.success());

    std::fs::remove_file(dir.join(".claude/settings.json")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let out_text = stdout(&out);
    assert!(out_text.contains("install:"), "{}", out_text);
    assert!(
        out_text.contains(".claude/settings.json lacks deny entry"),
        "{}",
        out_text
    );
    assert!(out_text.contains("adr init --claude"), "{}", out_text);

    assert!(adr(&dir, &["init", "--claude"], &[], "").status.success());

    let out2 = adr(&dir, &["check"], &[], "");
    let out2_text = stdout(&out2);
    assert!(
        !out2_text.contains("install:"),
        "install check should pass after re-init: {}",
        out2_text
    );
}

#[test]
fn check_install_passes_when_no_claude_directory() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());

    assert!(
        !dir.join(".claude").exists(),
        "plain init must not create .claude"
    );

    let out = adr(&dir, &["check"], &[], "");
    let out_text = stdout(&out);
    assert!(
        !out_text.contains("install:"),
        "no .claude directory should not trigger the install check: {}",
        out_text
    );
}

#[test]
fn check_ties_ignores_adr_prefix_that_is_not_a_uuid() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let (_record, id) = lock_one_question(&dir);
    write_tie(&dir, &id);
    // Prose that happens to contain "adr:" followed by a non-uuid token,
    // like a doc comment in the tool's own source, is not a tie.
    std::fs::write(
        dir.join("notes.txt"),
        "/// adr: a control layer\nsee adr: not-a-uuid here\n",
    )
    .unwrap();
    run_git(&dir, &["add", "notes.txt"]);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(stdout(&out).contains("ties: 1 tied"));
}

#[test]
fn check_parse_reports_a_file_shape_error() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    // No h1 at all: a file-level shape error.
    write_record(
        &dir,
        "adrdom/decisions/bad.md",
        "### 4.1 A question with no h1 above it?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("parse:"), "{}", text);
    assert!(text.contains("missing h1"), "{}", text);
}

#[test]
fn check_passes_a_skeleton_heading_with_no_id_or_body() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/skeleton.md",
        "# Skeleton\n\n## Group\n\n### 1.1 Not written yet?\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(!text.contains("ids:"), "{}", text);
    assert!(!text.contains("parse:"), "{}", text);
}

#[test]
fn check_fails_a_body_under_an_unapproved_heading() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Example\n\n## Group\n\n### 1.1 Does this need approval?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
    );
    // No `approved` ledger row written: the plan is unapproved.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("plan:"), "{}", text);
    assert!(
        text.contains("body under unapproved heading \"1.1 Does this need approval?\""),
        "{}",
        text
    );
}

#[test]
fn check_prints_awaiting_approval_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/skeleton.md",
        "# Skeleton\n\n## Group\n\n### 1.1 Not written yet?\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        text.contains("plan: 1 file(s) awaiting approval: adrdom/decisions/skeleton.md"),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_question_heading_duplicated_across_two_files() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/a.md",
        "# File A\n\n## Group\n\n### 9.9 Same heading?\n",
    );
    write_record(
        &dir,
        "adrdom/decisions/b.md",
        "# File B\n\n## Group\n\n### 9.9 Same heading?\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "plan: duplicate heading \"9.9 Same heading?\" in adrdom/decisions/a.md and adrdom/decisions/b.md"
        ),
        "{}",
        text
    );
}

#[test]
fn check_passes_when_a_superseded_id_is_missing_from_every_record() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Example\n\n## Group\n\n### 1.1 Some question?\n",
    );
    let sup_id = "99999999-9999-9999-9999-999999999999";
    append_ledger_row(&dir, "asked", sup_id, "abc123", "old heading");
    append_ledger_row(&dir, "superseded", sup_id, "-", "new-id");
    // `sup_id` appears in no record at all.

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains(&format!("ledger id {} not found", sup_id)),
        "{}",
        text
    );
    assert!(
        !text.contains(&format!("asked row id {} not found", sup_id)),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_withdrawn_id_still_present_in_a_record() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let wd_id = "88888888-8888-8888-8888-888888888888";
    let content = format!(
        "# Example\n\n## Group\n\n### 1.1 Some question?\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
        wd_id
    );
    write_record(&dir, "adrdom/decisions/example.md", &content);
    append_ledger_row(&dir, "withdrawn", wd_id, "-", "1.1 Some question?");

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("ids:"), "{}", text);
    assert!(
        text.contains(&format!("withdrawn id {} still present in", wd_id)),
        "{}",
        text
    );
}

#[test]
fn check_fails_on_a_terms_file_shape_error() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    // Missing the "# Terms" h1: a shape error.
    std::fs::write(
        dir.join("adrdom/terms.md"),
        "## decision\nid: 11111111-1111-1111-1111-111111111111\ndefinition: x.\ntest: y?\nowner: adrdom/decisions/example.md\naliases: \nbanned: \n",
    )
    .unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("terms: adrdom/terms.md:"), "{}", text);
    assert!(text.contains("missing h1"), "{}", text);
}

#[test]
fn check_fails_when_a_terms_binding_no_longer_resolves() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());

    let term_id = "11111111-1111-1111-1111-111111111111";
    let question_id = "22222222-2222-2222-2222-222222222222";

    let terms_content = format!(
        "# Terms\n\n## decision\nid: {}\ndefinition: A choice locked by the owner.\ntest: Does the owner have to choose among options?\nowner: adrdom/decisions/example.md\naliases: \nbanned: \n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();

    // The `terms:` line claims a binding to "decision", but nothing in the
    // heading/id/Problem/Standard/options text mentions it any more.
    let record_content = format!(
        "# Example\n\n## Group\n\n### 1.1 Some question?\n\nid: {}\n\nterms: decision={}\n\nProblem: nothing about the word here.\n\nStandard: still nothing about it.\n\n- A) fine.\n  Why: ok.\n",
        question_id, term_id
    );
    write_record(&dir, "adrdom/decisions/example.md", &record_content);
    append_ledger_row(&dir, "asked", question_id, "deadbeef", "1.1 Some question?");

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("terms:"), "{}", text);
    assert!(
        text.contains(
            "adrdom/decisions/example.md: 1.1 Some question?: binding changed (decision now none)"
        ),
        "{}",
        text
    );
}

#[test]
fn check_fails_a_bound_term_that_is_retired() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());

    let term_id = "33333333-3333-3333-3333-333333333333";
    let question_id = "44444444-4444-4444-4444-444444444444";

    let terms_content = format!(
        "# Terms\n\n## decision\nid: {}\ndefinition: A choice locked by the owner.\ntest: Does the owner have to choose among options?\nowner: adrdom/decisions/example.md\naliases: \nbanned: \nretired: yes\n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();

    // "decision" still appears in the Problem line, so the binding itself
    // is unchanged; the term behind it is retired.
    let record_content = format!(
        "# Example\n\n## Group\n\n### 1.1 Some question?\n\nid: {}\n\nterms: decision={}\n\nProblem: the decision needs revisiting.\n\nStandard: still needs review.\n\n- A) fine.\n  Why: ok.\n",
        question_id, term_id
    );
    write_record(&dir, "adrdom/decisions/example.md", &record_content);
    append_ledger_row(&dir, "asked", question_id, "deadbeef", "1.1 Some question?");

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "adrdom/decisions/example.md: 1.1 Some question?: bound term \"decision\" is retired"
        ),
        "{}",
        text
    );
    assert!(!text.contains("binding changed (decision"), "{}", text);
}


#[test]
fn check_prints_done_for_a_fully_answered_approved_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let (_record, id) = lock_one_question(&dir);
    write_tie(&dir, &id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("done: adrdom/decisions/example.md"),
        "{}",
        stdout(&out)
    );
}
