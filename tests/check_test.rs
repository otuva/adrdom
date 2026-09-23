mod common;

use common::*;

const ONE_QUESTION: &str = r#"# Example

### 3.1 Does this pass?

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.
"#;

/// Writes `ONE_QUESTION`, approves its plan/context via a hand-written
/// ledger row (so `ask` succeeds even once that command starts refusing
/// unapproved files, and so `check` doesn't flag its body as living under
/// an unapproved heading), asks, then owner-answers answer A.
fn answer_one_question(dir: &std::path::Path) -> (std::path::PathBuf, String) {
    // A term named "pass" (matching the word already in ONE_QUESTION's
    // heading) so a real `adr ask` writes a genuine, non-empty terms line
    // -- required now that an asked/answered section must carry one.
    let terms_content =
        "# Terms\n\n## pass\nid: 11111111-1111-1111-1111-111111111111\ndefinition: Whether something is accepted.\naliases: \n";
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();
    approve_terms_via_ledger(dir, terms_content);
    let record = write_record(dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_via_ledger(dir, "adrdom/decisions/example.md", ONE_QUESTION);
    assert!(adr(dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    let out = adr_owner(dir, &["answer", &id, "A"], "");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    (record, id)
}

#[test]
fn check_passes_on_a_clean_repo() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (_record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(stdout(&out).contains("tested: 1"));
}

#[test]
fn check_parse_failure() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
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
    assert!(adr(&dir, &["install"], &[], "").status.success());
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
    assert!(adr(&dir, &["install"], &[], "").status.success());

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
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "pass");
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
    assert!(adr(&dir, &["install"], &[], "").status.success());

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
fn check_tests_failure_when_untested() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (_record, _id) = answer_one_question(&dir);
    // Nothing recorded anywhere for the id.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let out_text = stdout(&out);
    assert!(out_text.contains("tested:"), "{}", out_text);
    assert!(out_text.contains("untested"), "{}", out_text);
}

#[test]
fn check_install_fails_when_settings_deny_entry_removed_and_passes_after_install() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());

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
    assert!(out_text.contains("adr install --claude"), "{}", out_text);

    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());

    let out2 = adr(&dir, &["check"], &[], "");
    let out2_text = stdout(&out2);
    assert!(
        !out2_text.contains("install:"),
        "install check should pass after re-install: {}",
        out2_text
    );
}

#[test]
fn check_install_passes_when_no_claude_directory() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    assert!(
        !dir.join(".claude").exists(),
        "plain install must not create .claude"
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
fn check_install_fails_when_agents_md_is_missing() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::remove_file(dir.join("AGENTS.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("install: AGENTS.md lacks the adrdom brief; run: adr install"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_install_fails_when_agents_md_has_no_marked_section() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::write(dir.join("AGENTS.md"), "# Notes\n\nNo markers here.\n").unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("install: AGENTS.md lacks the adrdom brief; run: adr install"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_install_fails_when_agents_md_brief_differs() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let content = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    let edited = content.replacen(
        "This repository is governed by `adr`.",
        "This repository is governed by adr, edited.",
        1,
    );
    assert_ne!(content, edited, "fixture edit should have changed something");
    std::fs::write(dir.join("AGENTS.md"), edited).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out)
            .contains("install: AGENTS.md brief differs from this binary's; run: adr install"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_install_fails_when_claude_md_does_not_import_agents_md() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::write(dir.join("CLAUDE.md"), "# Notes\n\nNo import here.\n").unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("install: CLAUDE.md does not import AGENTS.md; run: adr install"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_install_passes_right_after_install_with_a_claude_md_present() {
    let dir = init_repo();
    std::fs::write(dir.join("CLAUDE.md"), "# Notes\n\nOwner text.\n").unwrap();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&out).contains("install:"),
        "check should pass right after install: {}",
        stdout(&out)
    );
}

#[test]
fn check_tests_ignores_adr_prefix_that_is_not_a_uuid() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (_record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);
    // Prose that happens to contain "adr:" followed by a non-uuid token,
    // like a doc comment in the tool's own source, is not a test.
    std::fs::write(
        dir.join("notes.txt"),
        "/// adr: a control layer\nsee adr: not-a-uuid here\n",
    )
    .unwrap();
    run_git(&dir, &["add", "notes.txt"]);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(stdout(&out).contains("tested: 1"));
}

#[test]
fn check_parse_reports_a_file_shape_error() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
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
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    write_record(
        &dir,
        "adrdom/decisions/skeleton.md",
        "# Skeleton\n\n### 1.1 Not written yet?\n",
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
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Example\n\n### 1.1 Does this need approval?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
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
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/skeleton.md",
        "# Skeleton\n\n### 1.1 Not written yet?\n",
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
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/a.md",
        "# File A\n\n### 9.9 Same heading?\n",
    );
    write_record(
        &dir,
        "adrdom/decisions/b.md",
        "# File B\n\n### 9.9 Same heading?\n",
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
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Example\n\n### 1.1 Some question?\n",
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
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let wd_id = "88888888-8888-8888-8888-888888888888";
    let content = format!(
        "# Example\n\n### 1.1 Some question?\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
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
fn check_prints_done_for_a_fully_answered_approved_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (_record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("done: adrdom/decisions/example.md"),
        "{}",
        stdout(&out)
    );
}

// ---------------------------------------------------------------------
// terms (section 5): missing/unapproved-with-and-without-records, an
// answered/asked section's terms line (missing uuid, retired uuid, absent
// line), no re-lookup of the underlying text, the unused-terms note,
// staged reading, and circular definitions.
// ---------------------------------------------------------------------

#[test]
fn check_fails_on_a_terms_file_shape_error() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // Missing the "# Terms" h1: a shape error.
    std::fs::write(
        dir.join("adrdom/terms.md"),
        "## decision\nid: 11111111-1111-1111-1111-111111111111\ndefinition: x.\naliases: \n",
    )
    .unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("terms: adrdom/terms.md:"), "{}", text);
    assert!(text.contains("missing h1"), "{}", text);
}

#[test]
fn check_fails_when_terms_missing_and_a_record_exists() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Example\n\n### 1.1 Some question?\n",
    );
    // No adrdom/terms.md at all.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("terms: adrdom/terms.md missing"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_passes_when_terms_missing_and_no_records_exist() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // No records, no terms.md.

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(!stdout(&out).contains("terms:"), "{}", stdout(&out));
}

#[test]
fn check_fails_when_terms_unapproved_and_a_record_exists() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Example\n\n### 1.1 Some question?\n",
    );
    std::fs::write(
        dir.join("adrdom/terms.md"),
        "# Terms\n\n## decision\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A choice.\naliases: \n",
    )
    .unwrap();
    // No `terms` ledger row: unapproved.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "terms: adrdom/terms.md not approved; owner runs: adr approve adrdom/terms.md"
        ),
        "{}",
        text
    );
}

#[test]
fn check_notes_awaiting_approval_when_terms_unapproved_and_no_records() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::write(
        dir.join("adrdom/terms.md"),
        "# Terms\n\n## decision\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A choice.\naliases: \n",
    )
    .unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("terms: awaiting approval"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_fails_an_asked_section_whose_terms_line_names_a_missing_uuid() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let term_id = "11111111-1111-1111-1111-111111111111";
    let terms_content = format!(
        "# Terms\n\n## decision\nid: {}\ndefinition: A choice answered by the owner.\naliases: \n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), &terms_content).unwrap();
    approve_terms_via_ledger(&dir, &terms_content);

    let question_id = "22222222-2222-2222-2222-222222222222";
    let missing_uuid = "99999999-9999-9999-9999-999999999999";
    let record_content = format!(
        "# Example\n\n### 1.1 Some question?\n\nid: {}\n\nterms: decision={}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
        question_id, missing_uuid
    );
    write_record(&dir, "adrdom/decisions/example.md", &record_content);
    append_ledger_row(&dir, "asked", question_id, "deadbeef", "1.1 Some question?");

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(&format!(
            "adrdom/decisions/example.md: 1.1 Some question?: term {} not found",
            missing_uuid
        )),
        "{}",
        text
    );
}

#[test]
fn check_fails_an_asked_section_bound_to_a_retired_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let term_id = "33333333-3333-3333-3333-333333333333";
    let terms_content = format!(
        "# Terms\n\n## decision\nid: {}\ndefinition: A choice answered by the owner.\naliases: \nretired: yes\n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), &terms_content).unwrap();
    approve_terms_via_ledger(&dir, &terms_content);

    let question_id = "44444444-4444-4444-4444-444444444444";
    let record_content = format!(
        "# Example\n\n### 1.1 Some question?\n\nid: {}\n\nterms: decision={}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
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
}

#[test]
fn check_fails_an_asked_section_with_no_terms_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_content = "# Terms\n\n## decision\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A choice answered by the owner.\naliases: \n";
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();
    approve_terms_via_ledger(&dir, terms_content);

    let question_id = "55555555-5555-5555-5555-555555555555";
    let record_content = format!(
        "# Example\n\n### 1.1 Some question?\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
        question_id
    );
    write_record(&dir, "adrdom/decisions/example.md", &record_content);
    append_ledger_row(&dir, "asked", question_id, "deadbeef", "1.1 Some question?");

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("adrdom/decisions/example.md: 1.1 Some question?: no terms line"),
        "{}",
        text
    );
}

#[test]
fn check_passes_when_a_new_term_would_now_match_words_already_answered_in_text() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let widget_id = "66666666-6666-6666-6666-666666666666";
    let terms_v1 = format!(
        "# Terms\n\n## widget\nid: {}\ndefinition: A small thing.\naliases: \n",
        widget_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), &terms_v1).unwrap();
    approve_terms_via_ledger(&dir, &terms_v1);

    let question_id = "77777777-7777-7777-7777-777777777777";
    let record_content = format!(
        "# Example\n\n### 1.1 Some question?\n\nid: {}\n\nterms: widget={}\n\nProblem: the decision needs a widget.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
        question_id, widget_id
    );
    write_record(&dir, "adrdom/decisions/example.md", &record_content);
    append_ledger_row(&dir, "asked", question_id, "deadbeef", "1.1 Some question?");

    // A new term "decision" would now match a word already sitting in
    // the answered prose, but `check` never re-resolves that text against
    // today's terms, so this must not surface as a failure.
    let decision_id = "88888888-8888-8888-8888-888888888888";
    let terms_v2 = format!(
        "{}\n## decision\nid: {}\ndefinition: A choice answered by the owner.\naliases: \n",
        terms_v1, decision_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), &terms_v2).unwrap();
    approve_terms_via_ledger(&dir, &terms_v2);

    // No re-lookup: the existing `terms: widget=<id>` line still names a
    // term that exists and is not retired, so the `terms` check reports
    // nothing about this section, regardless of the new "decision" term
    // now also matching a word already sitting in its (untouched) prose.
    // (Other checks, like `answers`/`plan`, still fire on this
    // hand-crafted fixture -- irrelevant to what this test is about.)
    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    let terms_failure_on_section = text
        .lines()
        .any(|l| l.starts_with("terms:") && l.contains("Some question?"));
    assert!(!terms_failure_on_section, "{}", text);
}

#[test]
fn check_notes_a_term_paired_by_nothing() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let term_id = "99999999-1111-1111-1111-111111111111";
    let terms_content = format!(
        "# Terms\n\n## orphan\nid: {}\ndefinition: Nothing refers to this.\naliases: \n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), &terms_content).unwrap();
    approve_terms_via_ledger(&dir, &terms_content);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("terms: 1 terms paired by nothing: orphan"),
        "{}",
        stdout(&out)
    );
}

const GOOD_TERMS: &str = "# Terms\n\n## decision\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A choice.\naliases: \n";
const BAD_TERMS: &str = "not a terms file at all\n";

#[test]
fn check_staged_reads_terms_from_the_index() {
    // A bad staged copy is caught even though the working tree looks
    // fine: `check --staged` must read the index, not the working file.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());

        std::fs::write(dir.join("adrdom/terms.md"), BAD_TERMS).unwrap();
        run_git(&dir, &["add", "adrdom/terms.md"]);
        // The working tree copy is fixed, but that fix was never staged.
        std::fs::write(dir.join("adrdom/terms.md"), GOOD_TERMS).unwrap();

        let out = adr(&dir, &["check", "--staged"], &[], "");
        let text = stdout(&out);
        assert!(text.contains("missing h1"), "{}", text);
    }

    // A good copy already committed -- not part of this commit's staged
    // diff at all -- is still read from the index: a dirty, unstaged
    // working-tree edit must not fail `check --staged` for a commit that
    // does not include it.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());

        std::fs::write(dir.join("adrdom/terms.md"), GOOD_TERMS).unwrap();
        run_git(&dir, &["add", "adrdom/terms.md"]);
        run_git(&dir, &["commit", "--no-verify", "-q", "-m", "approve terms"]);

        // Dirty, unstaged edit: not part of any staged diff.
        std::fs::write(dir.join("adrdom/terms.md"), BAD_TERMS).unwrap();

        let out = adr(&dir, &["check", "--staged"], &[], "");
        let text = stdout(&out);
        assert!(!text.contains("missing h1"), "{}", text);
    }
}

#[test]
fn check_fails_a_circular_definition() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms_content = "# Terms\n\n\
## alpha\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: See beta.\n\
aliases: \n\
\n\
## beta\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: See alpha.\n\
aliases: \n";
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("terms: adrdom/terms.md: circular definition: alpha -> beta -> alpha"),
        "{}",
        text
    );
}
