mod common;

use common::*;

const ONE_QUESTION: &str = r#"### 3.1 Does this pass?

- A) Yes it passes.
  Why: reasons.
- B) No it does not.
  Why: other reasons.
"#;

fn lock_one_question(dir: &std::path::Path) -> (std::path::PathBuf, String) {
    let record = write_record(dir, "adrdom/decisions/example.md", ONE_QUESTION);
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
    write_unchecked(&dir, &id, "not wired to a test yet");

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(stdout(&out).contains("ties: 0 tied, 1 unchecked"));
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
fn check_banned_default_and_message_modes_never_print_the_text() {
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
    assert!(out_text.contains("banned:"), "{}", out_text);
    assert!(out_text.contains("notes.txt:1"), "{}", out_text);
    assert!(
        !out_text.to_lowercase().contains("forbiddenword"),
        "leaked banned text: {}",
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
    assert!(out2_text.contains("banned:"), "{}", out2_text);
    assert!(out2_text.contains("COMMIT_EDITMSG:1"), "{}", out2_text);
    assert!(
        !out2_text.to_lowercase().contains("forbiddenword"),
        "leaked banned text: {}",
        out2_text
    );
}

#[test]
fn check_ties_failure_and_override() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let (_record, id) = lock_one_question(&dir);
    // No tie line and no unchecked.tsv entry for `id`.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let out_text = stdout(&out);
    assert!(out_text.contains("ties:"), "{}", out_text);
    assert!(
        out_text.contains("no tie and not listed as unchecked"),
        "{}",
        out_text
    );
    assert!(
        !out_text.contains("ties: 0 tied"),
        "success line must not print alongside a real failure"
    );

    let override_out = adr_owner(
        &dir,
        &["override", "ties", &id, "--reason", "reviewing this week"],
        "",
    );
    assert!(
        override_out.status.success(),
        "override failed: {}",
        stderr(&override_out)
    );

    let out2 = adr(&dir, &["check"], &[], "");
    assert!(
        out2.status.success(),
        "check should pass after override: {}",
        stdout(&out2)
    );
    let out2_text = stdout(&out2);
    assert!(out2_text.contains("OVERRIDDEN"), "{}", out2_text);
    assert!(out2_text.contains("reviewing this week"), "{}", out2_text);
}

#[test]
fn check_ties_ignores_adr_prefix_that_is_not_a_uuid() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let (_record, id) = lock_one_question(&dir);
    write_unchecked(&dir, &id, "not wired to a test yet");
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
    assert!(stdout(&out).contains("ties: 0 tied, 1 unchecked"));
}
