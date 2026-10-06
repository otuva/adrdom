mod common;

use common::*;

const ONE_QUESTION: &str = r#"# Example

### 3.1 Is this pass?
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
    approve_record(dir, "adrdom/decisions/example.md");
    assert!(adr(dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    let arg = id_at_version(dir, &id);
    let out = adr_owner(dir, &["answer", &arg, "A"], "");
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
    // a missing id means the plan was never approved; an unapproved file
    // is only a count note, never a failure.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    // No `adr approve` run, so the section has no id.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!stdout(&out).contains("ids:"), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("awaiting approval: adrdom/decisions/example.md"),
        "{}",
        stdout(&out)
    );
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
    approve_record(&dir, "adrdom/decisions/example.md");
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
fn check_passes_a_skeleton_that_got_an_id_from_approve_but_was_never_asked() {
    // `approve` gives every heading an id, skeletons included --
    // `check`'s answers check must not mistake "has an id but was never
    // asked" for "edited after asking" just because a ledger `asked` row
    // for that id doesn't exist yet.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "pass");
    let content = format!(
        "{}\n### 3.2 Not asked yet?\ndepends: pass\n\nProblem: p of the pass.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n  Costs: z.\n- B) w.\n  Why: y.\n  Costs: z.\n",
        ONE_QUESTION
    );
    let record = write_record(&dir, "adrdom/decisions/example.md", &content);
    approve_record(&dir, "adrdom/decisions/example.md");

    let after = std::fs::read_to_string(&record).unwrap();
    let idx = after.find("### 3.2 Not asked yet?").unwrap();
    assert!(after[idx..].contains("id: "), "{}", &after[idx..]);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("edited after asking"), "{}", text);
}

// adr: d8a9dfff-de89-442c-a08c-8276efab3f36
// The scan runs from three places: the installed pre-commit hook (staged
// files), the installed commit-msg hook (the commit message), and this
// repository's own CI workflow (every tracked file, on push). A hit names
// the file and line, never the matched text.
#[test]
fn check_banned_default_and_message_modes_name_the_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let pre_commit = std::fs::read_to_string(dir.join(".githooks/pre-commit")).unwrap();
    assert!(
        pre_commit.contains("adr check --staged"),
        "pre-commit hook must run the staged-files scan: {}",
        pre_commit
    );
    let commit_msg = std::fs::read_to_string(dir.join(".githooks/commit-msg")).unwrap();
    assert!(
        commit_msg.contains("adr check --message"),
        "commit-msg hook must run the message scan: {}",
        commit_msg
    );

    let workflow_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/check.yml");
    let workflow = std::fs::read_to_string(&workflow_path).unwrap();
    assert!(
        workflow.contains("adr check"),
        "CI workflow must run `adr check` over every tracked file: {}",
        workflow
    );

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
        out_text.contains("banned: notes.txt:1: banned word"),
        "{}",
        out_text
    );
    assert!(
        !out_text.to_lowercase().contains("forbiddenword"),
        "the matched word must never reach stdout: {}",
        out_text
    );
    assert!(
        !stderr(&out).to_lowercase().contains("forbiddenword"),
        "the matched word must never reach stderr: {}",
        stderr(&out)
    );

    // --message mode: a commit message file containing the banned word.
    // Everything else staged first (adrdom/state/banned.sha256 included):
    // by the time the commit-msg hook runs for real, the pre-commit hook
    // has already run against this same staged tree, so `--message` reads
    // it from the index too (34e81f57 A) -- with nothing staged at all,
    // the ban list itself wouldn't exist in the index yet and this check
    // would have nothing to compare the message against.
    run_git(&dir, &["add", "-A"]);
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
        out2_text.contains("COMMIT_EDITMSG:1: banned word"),
        "{}",
        out2_text
    );
    assert!(
        !out2_text.to_lowercase().contains("forbiddenword"),
        "the matched word must never reach stdout: {}",
        out2_text
    );
    assert!(
        !stderr(&out2).to_lowercase().contains("forbiddenword"),
        "the matched word must never reach stderr: {}",
        stderr(&out2)
    );
}

// adr: 735a32bd-db0c-428e-b65e-2acc750671b0
// A banned word fails wherever it appears, quoted or not: the scanner
// tokenizes on runs of alphanumerics, so punctuation around the word --
// enclosing double quotes or a backtick code span -- never shields it.
#[test]
fn check_banned_scanner_catches_word_in_quotes_and_backticks() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let ban_out = adr(&dir, &["ban"], &[], "forbiddenword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));

    std::fs::write(
        dir.join("notes.txt"),
        "she called it \"forbiddenword\" out loud\nthe code spells it `forbiddenword` in the span\n",
    )
    .unwrap();
    run_git(&dir, &["add", "notes.txt"]);

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        !out.status.success(),
        "check should have failed: {}",
        stdout(&out)
    );
    let text = stdout(&out);
    assert!(
        text.contains("banned: notes.txt:1: banned word"),
        "word inside double quotes must still be caught: {}",
        text
    );
    assert!(
        text.contains("banned: notes.txt:2: banned word"),
        "word inside a backtick code span must still be caught: {}",
        text
    );
    assert!(
        !text.to_lowercase().contains("forbiddenword"),
        "the matched word must never reach stdout: {}",
        text
    );
    assert!(
        !stderr(&out).to_lowercase().contains("forbiddenword"),
        "the matched word must never reach stderr: {}",
        stderr(&out)
    );
}

// adr: d8a9dfff-de89-442c-a08c-8276efab3f36
//
// "What does the ban scan read?": `adrdom/terms.md` is a tracked file like
// any other, so a banned word inside its content is caught by the same
// scan and reported once, prefixed `banned:` -- `check_terms`'s own
// section/term terms-line reads never re-scan for banned words
// (`scan_banned: false`), so there is no second, `terms:`-prefixed hit for
// the same line. Both the default (working-tree) scan and `--staged`
// catch it, once each.
#[test]
fn check_reports_a_banned_word_inside_terms_md_exactly_once() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let ban_out = adr(&dir, &["ban"], &[], "forbiddenword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));

    let terms_content = "# Terms\n\n## widget\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A gadget also known as forbiddenword.\naliases: \n";
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();
    // Stage everything, not just terms.md: `--staged` reads the ban list
    // itself (`adrdom/state/banned.sha256`, written by `adr ban` above)
    // through the index too, so it has to be staged for the scan to have
    // anything to compare against (mirrors `check_banned_default_and_message_modes_name_the_line`'s
    // `git add -A` before its own `--message` scan).
    run_git(&dir, &["add", "-A"]);

    // Default (working-tree) scan.
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("banned: adrdom/terms.md:5: banned word"),
        "{}",
        text
    );
    // One failure line for the location, not one per category: count
    // lines that name this location at all (the message itself says
    // "banned word" twice -- once in "what happened", once in the next
    // move -- so counting that phrase would over-count a single hit).
    assert_eq!(
        text.lines().filter(|l| l.contains("adrdom/terms.md:5:")).count(),
        1,
        "the hit must be reported exactly once, no terms:-prefixed duplicate: {}",
        text
    );
    assert!(
        !text.contains("terms: adrdom/terms.md:5: banned word"),
        "check_terms must not re-scan and duplicate the banned-word hit: {}",
        text
    );

    // `--staged`, with the same content staged.
    let out2 = adr(&dir, &["check", "--staged"], &[], "");
    assert!(!out2.status.success());
    let text2 = stdout(&out2);
    assert!(
        text2.contains("banned: adrdom/terms.md:5: banned word"),
        "{}",
        text2
    );
    assert_eq!(
        text2.lines().filter(|l| l.contains("adrdom/terms.md:5:")).count(),
        1,
        "the staged hit must also be reported exactly once: {}",
        text2
    );
}

// adr: 3460d220-e523-448b-aeff-baa8051b7d06
//
// An untested current decision is a note: check names it on every run and
// exits 0, then stops naming it once an `adr: <id>` comment exists for it.
#[test]
fn check_tests_note_when_untested() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (_record, id) = answer_one_question(&dir);
    // Nothing recorded anywhere for the id.

    let out = adr(&dir, &["check"], &[], "");
    let out_text = stdout(&out);
    assert!(out.status.success(), "{}", out_text);
    let untested_line = |t: &str| t.lines().any(|l| l.starts_with("untested:") && l.contains(&format!("adr: {}", id)));
    assert!(untested_line(&out_text), "check must name the untested decision by id: {}", out_text);
    assert!(out_text.contains("tested: 0"), "{}", out_text);

    write_test_comment(&dir, &id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        out.status.success(),
        "check should pass once the decision is tied to a test: {}",
        stdout(&out)
    );
    assert!(stdout(&out).contains("tested: 1"), "{}", stdout(&out));
    assert!(!untested_line(&stdout(&out)), "{}", stdout(&out));
}

// adr: cd83e277-ebf0-4458-a77a-c526ab4cfeca
// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// "the ledger holds a hash of each answered question's text, and check
// fails any answered question whose text no longer matches it". Answers a
// question for real, so the `answered` ledger row carries a real sha256
// hash of the text as answered (not a placeholder), then edits the
// section's Problem line afterward: the section's current hash no longer
// matches the hash the ledger recorded, and check fails naming it with the
// "text changed since being answered" message -- never "answer line
// without ledger row" (the answer line is genuine) and never a re-ask
// (an answered question can't be re-asked); the fix named is restoring the
// text from git or superseding the decision.
#[test]
fn check_fails_an_answered_question_whose_text_changed_since_answering() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let answered_row = ledger
        .lines()
        .find(|l| l.contains("\tanswered\t") && l.contains(id.as_str()))
        .unwrap_or_else(|| panic!("no answered row for {}: {}", id, ledger));
    let hash_field = answered_row.split('\t').nth(3).unwrap();
    assert_ne!(
        hash_field, "-",
        "the answered row should carry a real hash of the text: {}",
        answered_row
    );
    assert_eq!(
        hash_field.len(),
        64,
        "expected a sha256 hex hash on the answered row: {}",
        answered_row
    );

    // Untouched, check still passes.
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass before any edit: {}", stdout(&out));

    // Edit the answered section's Problem line after the fact -- the
    // `**Answer:** A` line and the id are left alone.
    let content = std::fs::read_to_string(&record).unwrap();
    let edited = content.replacen(
        "Problem: whether it passes.",
        "Problem: whether it truly passes.",
        1,
    );
    assert_ne!(content, edited, "fixture edit should have changed something");
    std::fs::write(&record, &edited).unwrap();

    let out2 = adr(&dir, &["check"], &[], "");
    assert!(!out2.status.success());
    let text = stdout(&out2);
    assert!(text.contains("answers:"), "{}", text);
    assert!(text.contains("text changed since being answered"), "{}", text);
    assert!(
        text.contains(&format!(
            "agent restores it from git (e.g. git checkout -- adrdom/decisions/example.md), or writes a new question that supersedes {}",
            id
        )),
        "{}",
        text
    );
    assert!(
        !text.contains("answer line without ledger row"),
        "a genuine answer line must never be reported as hand-written: {}",
        text
    );
    assert!(!text.contains("re-asks"), "an answered question can never be re-asked: {}", text);
    assert!(text.contains("3.1 Is this pass?"), "{}", text);
}

// adr: cd83e277-ebf0-4458-a77a-c526ab4cfeca
// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// Same standard as the Problem-line edit above, but the edit lands on the
// option letters themselves: option A (the answered one) is renamed to C
// after the fact, while the `**Answer:** A` line is left untouched. The
// section's `answer_letter()` no longer resolves (there is no longer an
// option A to point at), which must never be read as "no genuine answer
// exists" -- the `answered` ledger row is still there for this id, so the
// text moved since answering, same as any other post-answer edit.
#[test]
fn check_fails_on_a_renamed_answered_option_never_as_answer_without_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass before any edit: {}", stdout(&out));

    // Rename option A to C -- the `**Answer:** A` line still says A, but
    // there is no longer an option A in the section.
    let content = std::fs::read_to_string(&record).unwrap();
    let edited = content.replacen("- A) Yes it passes.", "- C) Yes it passes.", 1);
    assert_ne!(content, edited, "fixture edit should have changed something");
    std::fs::write(&record, &edited).unwrap();

    let out2 = adr(&dir, &["check"], &[], "");
    assert!(!out2.status.success());
    let text = stdout(&out2);
    assert!(text.contains("answers:"), "{}", text);
    assert!(text.contains("text changed since being answered"), "{}", text);
    assert!(
        !text.contains("answer line without ledger row"),
        "a genuine answer line must never be reported as hand-written just because its option letter moved: {}",
        text
    );
}

// "a comment `adr: <id>` inside the test, found by text search in every
// tracked file outside adrdom, ties a test to a decision". A tie written
// in a tracked, non-Rust file outside adrdom/ counts (also relied on by
// every other test here through `write_test_comment`); the same comment
// placed under adrdom/, or left untracked, does not, so `check` still
// notes the decision untested until it is tracked outside adrdom/.
// adr: 4319a653-6901-446c-94ab-6420ef472134
#[test]
fn check_tests_ignores_a_tie_under_adrdom_and_an_untracked_tie() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (_record, id) = answer_one_question(&dir);

    // Tracked, but under adrdom/: the scan skips it.
    std::fs::write(dir.join("adrdom/note.txt"), format!("adr: {}\n", id)).unwrap();
    run_git(&dir, &["add", "adrdom/note.txt"]);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains(&format!("no tracked file holds \"adr: {}\"", id)),
        "a tie under adrdom/ must not count: {}",
        stdout(&out)
    );

    // The same comment outside adrdom/, but not yet tracked.
    std::fs::write(dir.join("outside.txt"), format!("adr: {}\n", id)).unwrap();

    let out2 = adr(&dir, &["check"], &[], "");
    assert!(out2.status.success(), "{}", stdout(&out2));
    assert!(
        stdout(&out2).contains(&format!("no tracked file holds \"adr: {}\"", id)),
        "an untracked tie must not count: {}",
        stdout(&out2)
    );

    // Tracking a non-Rust file outside adrdom/ is what makes it count.
    run_git(&dir, &["add", "outside.txt"]);
    let out3 = adr(&dir, &["check"], &[], "");
    assert!(out3.status.success(), "{}", stdout(&out3));
    assert!(stdout(&out3).contains("tested: 1"), "{}", stdout(&out3));
}

// adr: 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
// check fails when an agent's installed block (here, .claude/settings.json's
// deny entries) differs from what install writes, and the message's move is
// re-running install.
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
        stdout(&out).contains("install: AGENTS.md lacks the adrdom brief; agent runs: adr install"),
        "{}",
        stdout(&out)
    );
}

// adr: 34e81f57-9b3c-4dc4-a2ba-ca227523177d
//
// The install check is judged like every other file `check` reads: `adr
// install` writes AGENTS.md straight to the working tree, but nothing
// stages it. `check --staged` reads the index, where AGENTS.md does not
// exist yet, so it refuses exactly as it would if the file were missing
// outright; plain `adr check` reads the working tree, where the freshly
// installed brief is fine.
#[test]
fn check_staged_refuses_an_unstaged_agents_md_from_install() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // AGENTS.md sits on disk from install, but nothing has been staged.

    let out = adr(&dir, &["check", "--staged"], &[], "");
    let text = stdout(&out);
    assert!(!out.status.success(), "{}", text);
    assert!(
        text.contains("install: AGENTS.md lacks the adrdom brief; agent runs: adr install"),
        "an unstaged AGENTS.md from install must not satisfy a staged check: {}",
        text
    );

    let out2 = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&out2).contains("install:"),
        "plain check reads the working tree, where AGENTS.md is fine: {}",
        stdout(&out2)
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
        stdout(&out).contains("install: AGENTS.md lacks the adrdom brief; agent runs: adr install"),
        "{}",
        stdout(&out)
    );
}

// adr: 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
// check fails when the AGENTS.md brief section differs from what install
// writes; the message's move is running install again.
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
            .contains("install: AGENTS.md brief differs from this binary's; agent runs: adr install"),
        "{}",
        stdout(&out)
    );
}

// adr: 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
// check fails when the CLAUDE.md import (an agent's installed block, checked
// once .claude/ exists) differs from what install writes; the move is
// running install again.
#[test]
fn check_install_fails_when_claude_md_does_not_import_agents_md() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());
    std::fs::write(dir.join("CLAUDE.md"), "# Notes\n\nNo import here.\n").unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("install: CLAUDE.md does not import AGENTS.md; agent runs: adr install --claude"),
        "{}",
        stdout(&out)
    );
}

// adr: 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
// check fails when the pre-commit hook differs from what install writes
// (an agent's installed block, extended by the owner to cover the git
// hooks); the move is running install again, which restores it.
#[test]
fn check_install_fails_when_pre_commit_hook_differs_and_passes_after_reinstall() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    std::fs::write(dir.join(".githooks/pre-commit"), "#!/bin/sh\necho edited\n").unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains(
            "install: .githooks/pre-commit differs from what install writes; agent runs: adr install"
        ),
        "{}",
        stdout(&out)
    );

    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert_eq!(
        std::fs::read_to_string(dir.join(".githooks/pre-commit")).unwrap(),
        "#!/bin/sh\nadr check --staged --quiet\nexit $?\n",
        "reinstall should restore the hook"
    );

    let out2 = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&out2).contains("install:"),
        "install check should pass after re-install: {}",
        stdout(&out2)
    );
}

// adr: 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
// check fails when the pre-commit hook's content is current but the file
// is not executable, since git skips such a hook without a word; install
// repairs the mode without rewriting the content.
#[cfg(unix)]
#[test]
fn check_install_fails_when_pre_commit_hook_is_not_executable_and_install_repairs_it() {
    use std::os::unix::fs::PermissionsExt;
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let hook = dir.join(".githooks/pre-commit");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o644)).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains(
            "install: .githooks/pre-commit is not executable, git skips it; agent runs: adr install"
        ),
        "{}",
        stdout(&out)
    );

    let out = adr(&dir, &["install"], &[], "");
    assert!(out.status.success());
    assert!(
        stdout(&out).contains(".githooks/pre-commit"),
        "install must report the repaired hook, not up to date: {}",
        stdout(&out)
    );
    assert!(
        std::fs::metadata(&hook).unwrap().permissions().mode() & 0o100 != 0,
        "install should restore the execute bit"
    );

    let out2 = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&out2).contains("install:"),
        "install check should pass after re-install: {}",
        stdout(&out2)
    );
}

// adr: 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
// check fails when the commit-msg hook is deleted outright, and install
// recreates it.
#[test]
fn check_install_fails_when_commit_msg_hook_missing_and_passes_after_reinstall() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    std::fs::remove_file(dir.join(".githooks/commit-msg")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains(
            "install: .githooks/commit-msg is missing; agent runs: adr install"
        ),
        "{}",
        stdout(&out)
    );

    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert!(dir.join(".githooks/commit-msg").is_file());

    let out2 = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&out2).contains("install:"),
        "install check should pass after re-install: {}",
        stdout(&out2)
    );
}

// adr: 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
// check --staged judges the staged hook, not the working tree: the good
// hook is staged, then the working-tree copy is corrupted, and the staged
// check still passes (per 34e81f57 A, the same source rule as AGENTS.md).
#[test]
fn check_staged_judges_the_staged_hook_not_the_working_tree_copy() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    run_git(&dir, &["add", "-A"]);

    std::fs::write(dir.join(".githooks/pre-commit"), "#!/bin/sh\necho edited\n").unwrap();

    let out = adr(&dir, &["check", "--staged"], &[], "");
    assert!(
        out.status.success(),
        "staged check should judge the staged (good) hook, not the corrupted working tree: {}",
        stdout(&out)
    );

    let out2 = adr(&dir, &["check"], &[], "");
    assert!(
        !out2.status.success(),
        "plain check reads the working tree, where the hook is now bad"
    );
    assert!(
        stdout(&out2).contains(
            "install: .githooks/pre-commit differs from what install writes; agent runs: adr install"
        ),
        "{}",
        stdout(&out2)
    );
}

// adr: f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
// The installed brief tells the agent to build a decision when `show`
// names it and not before; `show` names it once every question beside it
// is answered (tests/show_next_test.rs holds that half).
#[test]
fn installed_brief_tells_the_agent_to_build_only_a_fully_answered_tier() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(
        agents.contains(
            "Build a decision when `next`\n   names it, and not before."
        ),
        "{}",
        agents
    );
}

// adr: 23d53545-c238-48cc-ae5e-21a2c7aab83e
// The installed brief tells the agent that the owner may strike, add or
// reorder headings before approving, or write the plan itself.
#[test]
fn installed_brief_tells_the_owner_may_edit_or_write_the_plan() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    let flat = agents.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains(
            "The owner may strike, add or reorder headings before approving, or write the plan itself and approve it the same way."
        ),
        "{}",
        agents
    );
}

// adr: c3127af2-4c96-48d5-a164-bac49a9a09ec
//
// "Does the agent build what no current decision covers?": the tool
// cannot enforce this itself (there is no build step it runs), so the
// decision is implemented entirely as the installed brief's instruction to
// the agent -- checked here the same way `check` verifies the rest of the
// brief's text landed in AGENTS.md verbatim. Three things the brief must
// say: don't build without a covering decision, write the question and
// stop, and say so and stop when a current decision looks wrong.
#[test]
fn installed_brief_tells_the_agent_not_to_build_without_a_covering_decision() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(
        agents.contains(
            "If no current decision covers what you are about to build, do not build\n   it. Write the question (below) and stop until the owner answers."
        ),
        "{}",
        agents
    );
    assert!(
        agents.contains(
            "If a current decision looks wrong, say so in chat and stop. Never edit an\n   answered question and never work around a decision."
        ),
        "{}",
        agents
    );
}

// adr: 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
// A CLAUDE.md the owner wrote by hand, with no .claude/ directory: plain
// install (no --claude) leaves it untouched (still missing the import), and
// check does not hold CLAUDE.md's import to any standard while .claude/
// does not exist.
#[test]
fn check_install_passes_right_after_install_with_a_claude_md_present() {
    let dir = init_repo();
    std::fs::write(dir.join("CLAUDE.md"), "# Notes\n\nOwner text.\n").unwrap();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert_eq!(
        std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap(),
        "# Notes\n\nOwner text.\n",
        "plain install must not touch an existing CLAUDE.md"
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&out).contains("install:"),
        "check should pass right after install: {}",
        stdout(&out)
    );
}

#[test]
fn check_install_passes_right_after_install_claude_created_claude_md() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());
    assert!(dir.join("CLAUDE.md").is_file());

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&out).contains("install:"),
        "check should pass right after install --claude: {}",
        stdout(&out)
    );
}

#[test]
fn check_install_passes_when_no_codex_directory() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    assert!(
        !dir.join(".codex").exists(),
        "plain install must not create .codex"
    );

    let out = adr(&dir, &["check"], &[], "");
    let out_text = stdout(&out);
    assert!(
        !out_text.contains("install:"),
        "no .codex directory should not trigger the install check: {}",
        out_text
    );
}

#[test]
fn check_install_passes_right_after_install_codex() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--codex"], &[], "").status.success());
    assert!(dir.join(".codex/rules/adrdom.rules").is_file());

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&out).contains("install:"),
        "check should pass right after install --codex: {}",
        stdout(&out)
    );
}

#[test]
fn check_install_fails_when_codex_rules_missing_and_passes_after_reinstall() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--codex"], &[], "").status.success());
    std::fs::remove_file(dir.join(".codex/rules/adrdom.rules")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let out_text = stdout(&out);
    assert!(
        out_text.contains(".codex/rules/adrdom.rules missing"),
        "{}",
        out_text
    );
    assert!(out_text.contains("adr install --codex"), "{}", out_text);

    assert!(adr(&dir, &["install", "--codex"], &[], "").status.success());
    let out2 = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&out2).contains("install:"),
        "install check should pass after re-install --codex: {}",
        stdout(&out2)
    );
}

#[test]
fn check_install_fails_when_codex_rules_edited() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--codex"], &[], "").status.success());
    std::fs::write(dir.join(".codex/rules/adrdom.rules"), "edited\n").unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains(".codex/rules/adrdom.rules differs from this binary's"),
        "{}",
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

// adr: f97e62a4-45ab-46c6-b4b1-a674792a4493
//
// "a fixed shape: one first-level heading naming the topic, optional prose
// under it, then the questions as third-level headings ending in \"?\";
// any other text, such as a note between questions, fails check". A line
// sitting between two `### ` questions is neither of those things, so it
// fails as a stray line in the first question's own section (everything
// through the next heading belongs to it) -- exactly the failure the
// decision describes for text nobody picked.
#[test]
fn check_fails_a_note_sitting_between_two_questions() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    let content = "# Example\n\n\
### 1.1 First question?\n\n\
Problem: p.\n\n\
Standard: s.\n\n\
- A) x.\n  Why: y.\n  Costs: c.\n\
- B) y.\n  Why: y.\n  Costs: c.\n\n\
A note nobody asked for.\n\n\
### 1.2 Second question?\n\n\
Problem: p2.\n\n\
Standard: s2.\n\n\
- A) x.\n  Why: y.\n  Costs: c.\n\
- B) y.\n  Why: y.\n  Costs: c.\n";
    write_record(&dir, "adrdom/decisions/example.md", content);

    let stray_line_no = content
        .lines()
        .position(|l| l == "A note nobody asked for.")
        .unwrap()
        + 1;

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "check should have failed: {}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("parse:"), "{}", text);
    assert!(
        text.contains(&format!("stray line {}", stray_line_no)),
        "{}",
        text
    );
}

// adr: f97e62a4-45ab-46c6-b4b1-a674792a4493
//
// A skeleton heading -- one with a `depends:` line but no id and no body --
// is never a failure: check passes clean, with no `ids:` or `parse:`
// failure reported for it.
#[test]
fn check_passes_a_skeleton_heading_with_no_id_or_body() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    write_record(
        &dir,
        "adrdom/decisions/skeleton.md",
        "# Skeleton\n\n### 1.1 Not written yet?\ndepends: widget\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(!text.contains("ids:"), "{}", text);
    assert!(!text.contains("parse:"), "{}", text);
}

//
// A question body under an unapproved heading no longer fails check: the
// file is only counted as awaiting approval.
#[test]
fn check_passes_a_body_under_an_unapproved_heading() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Example\n\n### 1.1 Does this need approval?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("body under unapproved heading"), "{}", text);
    assert!(text.contains("awaiting approval"), "{}", text);
}

//
// "check reports a file awaiting approval as a count ... and passes
// (exit 0)": a plan with no body under any heading (a bare skeleton) is
// unapproved, so it lands in the count, but nothing in it fails check.
#[test]
fn check_prints_awaiting_approval_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // A depends line and approved terms, so the only thing outstanding is
    // approval itself -- otherwise the missing-depends-line and
    // missing-terms failures would fail check for unrelated reasons and
    // the exit-0 clause couldn't be checked cleanly.
    approve_minimal_terms(&dir, "widget");
    write_record(
        &dir,
        "adrdom/decisions/skeleton.md",
        "# Skeleton\n\n### 1.1 Not written yet?\ndepends: widget\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        out.status.success(),
        "a skeleton awaiting approval must pass check: {}",
        stdout(&out)
    );
    let text = stdout(&out);
    assert!(
        text.contains("plan: 1 file(s) awaiting approval: adrdom/decisions/skeleton.md"),
        "{}",
        text
    );
}

// adr: b741b573-a2ee-498e-98fd-3a7e7d319470
//
// "check reports it as awaiting approval" for the case the option names:
// the prose edited after approve, with the heading (and its `approved`
// row) untouched. `check` still passes -- the heading itself is approved,
// so no body lives under an unapproved heading -- but the file is counted
// as awaiting approval because its `context` row is stale.
#[test]
fn check_counts_a_file_awaiting_approval_when_only_its_context_prose_is_stale() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Example\n\nContext prose.\n\n### 1.1 Only question?\ndepends: decision\n\nProblem: p of the decision.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n  Costs: c.\n- B) y.\n  Why: y.\n  Costs: c.\n",
    );
    approve_record(&dir, "adrdom/decisions/example.md");

    // Edit only the context prose, on the file as approve left it (id
    // already minted), so the heading and its `approved` row stay valid.
    let approved_content = std::fs::read_to_string(&record).unwrap();
    let edited = approved_content.replace("Context prose.", "Context prose, edited.");
    std::fs::write(&record, &edited).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        out.status.success(),
        "a heading-approved file with stale context must still pass check: {}",
        stdout(&out)
    );
    let text = stdout(&out);
    assert!(
        text.contains("plan: 1 file(s) awaiting approval: adrdom/decisions/example.md"),
        "{}",
        text
    );
    assert!(
        !text.contains("body under unapproved heading"),
        "the heading itself is approved, so its body must not fail: {}",
        text
    );
}

// adr: 76bd7ef0-f5be-40d8-b5ba-0bd4bbd047e4
//
// "a question heading that appears in two topic files fails check": shown
// here with files named "a.md"/"b.md", themselves valid names under the
// same decision's naming half (a single lowercase-letter word needs no
// hyphen); `check_fails_a_bad_topic_file_name` and
// `check_passes_a_good_topic_file_name`, below, tie that half.
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
fn check_fails_a_question_heading_duplicated_within_one_file() {
    // Regression: the cross-file dedup above only compares paths, so two
    // identical `### ` headings inside the SAME file collapsed to one path
    // and never surfaced -- and `derive_tiers`' heading index binds a
    // depends line to the first of the two only (`.or_insert`), silently.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/dup.md",
        "# File\n\n### 9.9 Same heading?\n\n### 9.9 Same heading?\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "plan: duplicate heading \"9.9 Same heading?\" twice in adrdom/decisions/dup.md"
        ),
        "{}",
        text
    );
}

// adr: 76bd7ef0-f5be-40d8-b5ba-0bd4bbd047e4
//
// "each file holds one topic and is named by it in lowercase words joined
// by hyphens, with no number or prefix": an uppercase letter and an
// underscore both fail the name, and check names the file and the message
// `repo::valid_topic_file_name`/`msg::bad_topic_file_name` share.
#[test]
fn check_fails_a_bad_topic_file_name() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/Bad_Name.md",
        "# File\n\n### 1.1 Q?\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "plan: adrdom/decisions/Bad_Name.md: file name is not lowercase words joined by single hyphens"
        ),
        "{}",
        text
    );
}

// adr: 76bd7ef0-f5be-40d8-b5ba-0bd4bbd047e4
//
// The other side of the same rule: lowercase words joined by single
// hyphens is a name check accepts without complaint -- and, with an
// otherwise-clean skeleton heading, check passes outright.
#[test]
fn check_passes_a_good_topic_file_name() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    write_record(
        &dir,
        "adrdom/decisions/good-name.md",
        "# Good Name\n\n### 1.1 Not written yet?\ndepends: widget\n",
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        out.status.success(),
        "a validly named skeleton file must pass check: {}",
        stdout(&out)
    );
    let text = stdout(&out);
    assert!(
        !text.contains("file name is not lowercase words joined by single hyphens"),
        "{}",
        text
    );
}

// adr: 76bd7ef0-f5be-40d8-b5ba-0bd4bbd047e4
//
// "is named by it": a well-formed name that is not the h1 topic,
// lowercased and hyphenated, fails check; the same file under the name
// its topic gives passes.
#[test]
fn check_fails_a_topic_file_name_that_does_not_match_its_topic() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let body = "# Two Words\n\n### 1.1 Not written yet?\ndepends: widget\n";
    write_record(&dir, "adrdom/decisions/other-name.md", body);

    let out = adr(&dir, &["check"], &[], "");
    // A draft: the finding is a note (912f4f3b-1213-435a-af0d-7617017fcff4).
    assert!(out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "plan: adrdom/decisions/other-name.md: file name does not match its topic \"Two Words\"; agent renames the file to two-words.md"
        ),
        "{}",
        text
    );

    std::fs::remove_file(dir.join("adrdom/decisions/other-name.md")).unwrap();
    write_record(&dir, "adrdom/decisions/two-words.md", body);
    let out = adr(&dir, &["check"], &[], "");
    assert!(
        out.status.success(),
        "a file named by its topic must pass check: {}",
        stdout(&out)
    );
}

// adr: 76bd7ef0-f5be-40d8-b5ba-0bd4bbd047e4
//
// "with no number": a topic whose text holds a digit has no name any file
// can carry, so check refuses the topic text itself, whatever the file is
// called; no rename clears it, and rewording the topic does.
#[test]
fn check_fails_a_topic_whose_text_no_file_name_can_carry() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let body = "# Rule 2\n\n### 1.1 Not written yet?\ndepends: widget\n";
    write_record(&dir, "adrdom/decisions/rule.md", body);

    let out = adr(&dir, &["check"], &[], "");
    // A draft: the finding is a note (912f4f3b-1213-435a-af0d-7617017fcff4).
    assert!(out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "plan: adrdom/decisions/rule.md: topic \"Rule 2\" has no file name: a name holds lowercase words only, no number; agent rewords the topic line"
        ),
        "{}",
        text
    );
    assert!(
        !text.contains("does not match its topic"),
        "no rename is offered for a topic that cannot be named: {}",
        text
    );

    write_record(
        &dir,
        "adrdom/decisions/rule.md",
        "# Rule\n\n### 1.1 Not written yet?\ndepends: widget\n",
    );
    let out = adr(&dir, &["check"], &[], "");
    assert!(
        out.status.success(),
        "the reworded topic under its own name must pass check: {}",
        stdout(&out)
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

// adr: 21ae8ae0-6884-4d3c-8464-024a8a5aae2d
//
// "a withdrawn identifier never returns": once `wd_id` carries a
// `withdrawn` ledger row, its reappearing in a topic file's `id:` line is
// itself a failure, not a tolerated reuse.
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
        text.contains(&format!("withdrawn id {} (\"1.1 Some question?\") still present in", wd_id)),
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

// adr: d981f6b0-df0d-428f-8c0e-e220b1651fa1
//
// "no close: a topic file is done when every question in its plan is
// answered, check reports it as done, and a heading added later is a plan
// change like any other (after approve, the file is no longer reported
// done until the new question is answered)". No owner command ever closes
// or reopens the file -- done is recomputed by `check` from scratch every
// time, so a plain plan change (a new heading, approved like any other)
// is enough to undo it, and answering that heading is enough to redo it.
#[test]
fn adding_a_heading_undoes_done_until_it_too_is_answered() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("done: adrdom/decisions/example.md"),
        "file should be done once its only question is answered: {}",
        stdout(&out)
    );

    // A plan change on the already-done file: one more heading, added and
    // approved exactly like any other plan change.
    let approved = std::fs::read_to_string(&record).unwrap();
    let mut extended = approved.clone();
    extended.push_str(
        "\n### 3.2 Does the second thing also pass?\n\
depends: pass\n\n\
Problem: a second thing to decide.\n\n\
Standard: clear yes or no.\n\n\
- A) Yes.\n  Why: reasons.\n  Costs: costs.\n\
- B) No.\n  Why: other reasons.\n  Costs: other costs.\n",
    );
    std::fs::write(&record, &extended).unwrap();
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&out).contains("done: adrdom/decisions/example.md"),
        "adding an unanswered heading must undo done: {}",
        stdout(&out)
    );

    // Ask and answer the new question: done again, with no close/reopen
    // command ever invoked.
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));
    let content2 = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content2)
        .into_iter()
        .find(|i| i != &id)
        .expect("a second id should now exist");

    let new_id_arg = id_at_version(&dir, &new_id);
    let out = adr_owner(&dir, &["answer", &new_id_arg, "A"], "");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    write_test_comment(&dir, &new_id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("done: adrdom/decisions/example.md"),
        "file should be done again once the added question is answered: {}",
        stdout(&out)
    );
}

// ---------------------------------------------------------------------
// terms (section 5): missing/unapproved-with-and-without-records, an
// answered/asked section's terms line (missing uuid, retired uuid, absent
// line), no re-lookup of the underlying text, the unused-terms note,
// staged reading, and circular definitions.
// ---------------------------------------------------------------------

// A shape-broken terms.md is also never approved (its hash can never match
// a real `terms` ledger row), so `check` fails it (per "Does the tool stop
// while the terms file waits for approval?", aa475aa3 A) and reports the
// shape error `approve` would refuse instead of naming the owner's move.
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
    assert!(
        text.contains("terms: adrdom/terms.md: line 1: missing h1 \"# Terms\""),
        "{}",
        text
    );
    assert!(!text.contains("owner runs: adr approve adrdom/terms.md"), "{}", text);
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

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// "Does the tool stop while the terms file waits for approval?" (A): an
// unapproved terms.md fails outright even when nothing current pairs any
// of its terms -- there is no "harmless draft" case any more, unlike the
// pre-aa475aa3 version of this test, which expected a passing "waiting"
// note here.
#[test]
fn check_fails_outright_when_terms_is_unapproved_even_if_nothing_pairs_it() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/decisions/example.md",
        "# Example\n\n### 1.1 Some question?\ndepends: decision\n",
    );
    std::fs::write(
        dir.join("adrdom/terms.md"),
        "# Terms\n\n## decision\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A choice.\naliases: \n",
    )
    .unwrap();
    // No `terms` ledger row: unapproved.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("terms: adrdom/terms.md not approved"),
        "{}",
        stdout(&out)
    );
    assert!(
        stdout(&out).contains("owner runs: adr approve adrdom/terms.md"),
        "{}",
        stdout(&out)
    );
}

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// "Does the tool stop while the terms file waits for approval?" (A): once
// `adrdom/terms.md` is hand-edited on top of its latest approval, both
// `check` and `ask` stop the same way regardless of whether anything
// current reaches the edited term -- there is no separate "approved copy
// is missing" case any more, since check no longer trusts anything but the
// ledger's own hash.
#[test]
fn check_and_ask_fail_when_terms_is_edited_after_its_latest_approval() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);

    // Hand-edit the definition after that answer, with no new `terms`
    // ledger row: the file on disk now differs from what the ledger says
    // was approved.
    let edited_content = "# Terms\n\n## pass\nid: 11111111-1111-1111-1111-111111111111\ndefinition: Whether something clears review.\naliases: \n";
    std::fs::write(dir.join("adrdom/terms.md"), edited_content).unwrap();

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(!check_out.status.success());
    assert!(
        stdout(&check_out).contains("terms: adrdom/terms.md not approved"),
        "{}",
        stdout(&check_out)
    );
    assert!(
        stdout(&check_out).contains("owner runs: adr approve adrdom/terms.md"),
        "{}",
        stdout(&check_out)
    );

    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!ask_out.status.success());
    let err = stderr(&ask_out);
    assert!(err.contains("terms not approved"), "{}", err);
    assert!(err.contains("adr approve adrdom/terms.md"), "{}", err);
}

// adr: 6108b8b2-911e-443b-abe0-4c6f8b407384
#[test]
fn check_fails_an_asked_section_whose_terms_line_is_malformed() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let term_id = "55555555-5555-5555-5555-555555555555";
    let terms_content = format!(
        "# Terms\n\n## decision\nid: {}\ndefinition: A choice answered by the owner.\naliases: \n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), &terms_content).unwrap();
    approve_terms_via_ledger(&dir, &terms_content);

    let question_id = "66666666-6666-6666-6666-666666666666";
    // No `=uuid` after the word at all: `terms::parse_terms_line` fails to
    // parse this as a `terms:` line, so `validate_pairs` returns
    // `Err(Malformed)` rather than a per-uuid `Problem`.
    let record_content = format!(
        "# Example\n\n### 1.1 Some question?\n\nid: {}\n\nterms: foo\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n",
        question_id
    );
    write_record(&dir, "adrdom/decisions/example.md", &record_content);
    append_ledger_row(&dir, "asked", question_id, "deadbeef", "1.1 Some question?");

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("adrdom/decisions/example.md: 1.1 Some question?: malformed terms line"),
        "{}",
        text
    );
    assert!(
        text.contains("agent re-asks so the terms line is written correctly"),
        "{}",
        text
    );
}

// adr: 6108b8b2-911e-443b-abe0-4c6f8b407384
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
            "adrdom/decisions/example.md: 1.1 Some question?: term {} (\"decision\") not found",
            missing_uuid
        )),
        "{}",
        text
    );
}

// adr: 6108b8b2-911e-443b-abe0-4c6f8b407384
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
            "adrdom/decisions/example.md: 1.1 Some question?: uses the retired term \"decision\""
        ),
        "{}",
        text
    );
    assert!(
        text.contains("agent re-asks so the terms line drops the retired term"),
        "an asked-only question bound to a retired term is still re-asked: {}",
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
    assert!(
        text.contains("agent re-asks so the terms line is written"),
        "an asked-only question with no terms line is still re-asked: {}",
        text
    );
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// The answered twin of `check_fails_an_asked_section_with_no_terms_line`:
// once `question_id` is answered (both an `asked` and an `answered` row at
// the same hash, mirroring what `adr answer` itself writes), the section
// can never be re-asked (per "Does a decision change after it is
// answered?"), so losing its terms line -- only possible here through an
// edit made after answering -- is reported with the restore-or-supersede
// next move, never "re-asks".
#[test]
fn check_fails_an_answered_section_with_no_terms_line_by_restoring_or_superseding() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_content = "# Terms\n\n## decision\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A choice answered by the owner.\naliases: \n";
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();
    approve_terms_via_ledger(&dir, terms_content);

    let question_id = "66666666-6666-6666-6666-666666666666";
    let record_content = format!(
        "# Example\n\n### 1.1 Some question?\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: y.\n\n**Answer:** A\n",
        question_id
    );
    write_record(&dir, "adrdom/decisions/example.md", &record_content);
    append_ledger_row_at(&dir, "asked", question_id, "deadbeef", "1.1 Some question?", "2024-01-01T00:00:00Z");
    append_ledger_row_at(&dir, "answered", question_id, "deadbeef", "A", "2024-01-01T00:00:01Z");

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("adrdom/decisions/example.md: 1.1 Some question?: no terms line"),
        "{}",
        text
    );
    assert!(
        text.contains(&format!(
            "agent restores the terms line from git (e.g. git checkout -- adrdom/decisions/example.md), or writes a question that supersedes {}",
            question_id
        )),
        "an answered section's next move must be restore-or-supersede: {}",
        text
    );
    assert!(
        !text.contains("re-asks"),
        "an answered section can never be re-asked: {}",
        text
    );
}

// adr: 6108b8b2-911e-443b-abe0-4c6f8b407384
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

// Per "Does the tool stop while the terms file waits for approval?"
// (aa475aa3 A), a terms.md that differs from its latest approval is always
// a failure now, and BAD_TERMS's shape error is what this test reads as
// "the bad copy was seen" -- GOOD_TERMS is approved for real up front so
// the two sides genuinely differ (approved vs not).
#[test]
fn check_staged_reads_terms_from_the_index() {
    // A bad staged copy is caught even though the working tree looks
    // approved: `check --staged` must read the index, not the working file.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        let terms_path = write_record(&dir, "adrdom/terms.md", GOOD_TERMS);
        let approve_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
        assert!(approve_out.status.success(), "{}", stderr(&approve_out));
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "--no-verify", "-q", "-m", "approve terms"]);

        std::fs::write(&terms_path, BAD_TERMS).unwrap();
        run_git(&dir, &["add", "adrdom/terms.md"]);
        // The working tree copy is restored to the approved text, but that
        // fix was never staged.
        std::fs::write(&terms_path, GOOD_TERMS).unwrap();

        let out = adr(&dir, &["check", "--staged"], &[], "");
        assert!(!out.status.success(), "{}", stdout(&out));
        assert!(
            stdout(&out).contains("terms: adrdom/terms.md: line 1: missing h1 \"# Terms\""),
            "{}",
            stdout(&out)
        );
    }

    // A good, approved copy already committed -- not part of this commit's
    // staged diff at all -- is still read from the index: a dirty,
    // unstaged working-tree edit must not fail `check --staged` for a
    // commit that does not include it.
    {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        let terms_path = write_record(&dir, "adrdom/terms.md", GOOD_TERMS);
        let approve_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
        assert!(approve_out.status.success(), "{}", stderr(&approve_out));
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "--no-verify", "-q", "-m", "approve terms"]);

        // Dirty, unstaged edit: not part of any staged diff.
        std::fs::write(&terms_path, BAD_TERMS).unwrap();

        let out = adr(&dir, &["check", "--staged"], &[], "");
        assert!(
            !stdout(&out).contains("terms: adrdom/terms.md not approved"),
            "{}",
            stdout(&out)
        );
    }
}

// The exploit "Does check at a commit read the staged files?" closes: an
// agent stages an edit to an answered option's text, then restores the
// working tree copy so a plain read of the file on disk shows nothing
// wrong. `check --staged` must still catch it, because it reads the text
// that is actually about to enter the commit (the index), not whatever
// sits on disk.
//
// adr: 34e81f57-9b3c-4dc4-a2ba-ca227523177d
#[test]
fn check_staged_refuses_a_reworded_answered_option_restored_on_disk() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "--no-verify", "-q", "-m", "seed answered question"]);

    let original = std::fs::read_to_string(&record).unwrap();
    let reworded = original.replace("Yes it passes.", "Yes, this now passes for sure.");
    std::fs::write(&record, &reworded).unwrap();
    run_git(&dir, &["add", "adrdom/decisions/example.md"]);
    // The working tree copy is put back to the answered text -- only the
    // index still holds the reword.
    std::fs::write(&record, &original).unwrap();

    let out = adr(&dir, &["check", "--staged"], &[], "");
    let text = stdout(&out);
    assert!(!out.status.success(), "{}", text);
    assert!(
        text.contains("text changed since being answered"),
        "a staged reword must be caught even with the working tree restored: {}",
        text
    );
}

// The mirror case: the reword already sits at HEAD (nothing new is staged
// for this commit), and someone applies the real fix only to the working
// tree, forgetting to stage it. `check --staged` reads the index/HEAD
// content, so the unstaged fix must not count.
#[test]
fn check_staged_ignores_an_unstaged_working_tree_fix() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);

    let original = std::fs::read_to_string(&record).unwrap();
    let reworded = original.replace("Yes it passes.", "Yes, this now passes for sure.");
    std::fs::write(&record, &reworded).unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "--no-verify", "-q", "-m", "reword (bad, committed for the fixture)"]);

    // The fix, applied only to the working tree: never staged, so the
    // index (and HEAD) still hold the reworded text.
    std::fs::write(&record, &original).unwrap();

    let out = adr(&dir, &["check", "--staged"], &[], "");
    let text = stdout(&out);
    assert!(!out.status.success(), "{}", text);
    assert!(
        text.contains("text changed since being answered"),
        "an unstaged working-tree fix must not satisfy a staged check: {}",
        text
    );
}

// Outside a commit, the flag is what flips the source: the identical
// stage-bad/restore-good setup from
// `check_staged_refuses_a_reworded_answered_option_restored_on_disk`
// passes under plain `adr check`, which reads the working tree.
#[test]
fn check_without_staged_reads_the_working_tree() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);

    let original = std::fs::read_to_string(&record).unwrap();
    let reworded = original.replace("Yes it passes.", "Yes, this now passes for sure.");
    std::fs::write(&record, &reworded).unwrap();
    run_git(&dir, &["add", "adrdom/decisions/example.md"]);
    std::fs::write(&record, &original).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        out.status.success(),
        "plain check must read the working tree, not the index: {}",
        stdout(&out)
    );
}

// A staged deletion of a topic file is read as the file being gone at a
// commit, even though it is still sitting on disk: the id it answered can
// no longer be found in any topic file the index holds.
#[test]
fn check_staged_treats_a_staged_deleted_topic_file_as_gone() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "--no-verify", "-q", "-m", "seed answered question"]);

    // Staged deletion, left on disk: `git rm --cached` removes the path
    // from the index only.
    run_git(&dir, &["rm", "--cached", "-q", "adrdom/decisions/example.md"]);
    assert!(
        record.exists(),
        "the working tree copy must still be on disk -- only the index drops it"
    );

    let out = adr(&dir, &["check", "--staged"], &[], "");
    let text = stdout(&out);
    assert!(!out.status.success(), "{}", text);
    assert!(
        text.contains(&format!("asked row id {} (\"3.1 Is this pass?\") not found in any topic file", id)),
        "a staged deletion must be read as the file being gone, at a commit: {}",
        text
    );
}

// A brand new test file's tie counts once it is staged, even though its
// working-tree copy is edited back to drop the tie afterward (the index
// content is what is about to enter the commit); the same tie sitting
// only in the working tree, outside a commit, is what plain `adr check`
// reads instead.
// adr: 4319a653-6901-446c-94ab-6420ef472134
#[test]
fn check_staged_reads_a_newly_staged_test_files_tie_from_the_index() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (_record, id) = answer_one_question(&dir);
    // The answered question itself staged too, so it is a current
    // decision in the index this `--staged` run reads, not just on disk.
    run_git(&dir, &["add", "-A"]);

    std::fs::write(dir.join("tested.txt"), format!("adr: {}\n", id)).unwrap();
    run_git(&dir, &["add", "tested.txt"]);
    // Working tree copy edited back to drop the tie: the staged content
    // still has it.
    std::fs::write(dir.join("tested.txt"), "no tie here\n").unwrap();

    let out = adr(&dir, &["check", "--staged"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(stdout(&out).contains("tested: 1"), "{}", stdout(&out));

    // Outside a commit, the working tree is read instead: the tie is gone
    // there, so plain `adr check` notes it untested.
    let out2 = adr(&dir, &["check"], &[], "");
    assert!(out2.status.success(), "{}", stdout(&out2));
    assert!(
        stdout(&out2).contains(&format!("no tracked file holds \"adr: {}\"", id)),
        "{}",
        stdout(&out2)
    );
}

// adr: fcc1c1ec-d8e8-428e-9731-9817cdeb8e76
//
// Per "Does the tool stop while the terms file waits for approval?"
// (aa475aa3 A): `check` only ever reaches the cycle check once the file is
// approved (a real `adr approve` would itself refuse to write a cycle, so
// this is the defense-in-depth case -- a hand-written ledger row, standing
// in for one written before cycle detection existed). Faking the approval
// via `approve_terms_via_ledger`, matching this exact cyclic content, is
// what still lets `check` reach it.
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
    approve_terms_via_ledger(&dir, terms_content);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("terms: adrdom/terms.md: circular definition: alpha -> beta -> alpha"),
        "{}",
        text
    );
}

// A terms file with no `id:` line at all is never approved, whatever the
// ledger says (per "Does the tool stop while the terms file waits for
// approval?", aa475aa3 A): `check` fails it even when a ledger row happens
// to match its exact (id-less) hash, and reports the cycle `approve` would
// refuse instead of naming the owner's move.
#[test]
fn check_fails_an_id_less_terms_file_with_its_cycle_even_with_a_matching_ledger_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms_content = "# Terms\n\n\
## alpha\n\
definition: See beta.\n\
aliases: \n\
\n\
## beta\n\
definition: See alpha.\n\
aliases: \n";
    approve_terms_via_ledger(&dir, terms_content);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("terms: adrdom/terms.md: circular definition: alpha -> beta -> alpha"),
        "{}",
        text
    );
    assert!(!text.contains("owner runs: adr approve adrdom/terms.md"), "{}", text);
}

// adr: 751ef30c-6a2f-4936-bf9b-b9d487146e53
// "Does check fail a test that names a decision that is not current?" (751ef30c A): yes -- a tie to a
// decision whose latest ledger state is no longer `answered` is stale, and
// the agent's move is to remove the tag or point it at the current
// decision. A later decision superseding the first is one way a tie goes
// stale: `old_id` was genuinely current (and tested) when the tie was
// written, then a second, real decision named `supersedes: <old_id>` and
// took it out of force.
#[test]
fn check_fails_a_test_tied_to_a_decision_that_was_later_superseded() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let first = "# Example\n\n### 1.1 What is the unit of authority called?\ndepends: authority\n\nProblem: names leaked into decisions.\n\nStandard: plain words.\n\n- A) The authoritative sentence is a decision.\n  Why: plainest word.\n  Costs: two nouns.\n- B) The whole unit is a block.\n  Why: matches predecessors.\n  Costs: names a region.\n";
    let second = "\n### 1.2 What is the second unit of authority called?\ndepends: authority\n\nProblem: naming the second unit.\n\nStandard: plain words.\n\n- A) The second thing is a widget.\n  Why: reasons.\n  Costs: costs.\n- B) The second thing is a gadget.\n  Why: other reasons.\n  Costs: other costs.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", first);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &old_id, "A").status.success());
    write_test_comment(&dir, &old_id);

    // While `old_id` is still current, the tie is genuine and check passes.
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));

    // A second question, naming `supersedes: <old_id>`, takes the old
    // decision out of force once it is itself answered.
    let mut content = std::fs::read_to_string(&record).unwrap();
    let second_with_supersedes = second.replacen(
        "### 1.2 What is the second unit of authority called?\n",
        &format!(
            "### 1.2 What is the second unit of authority called?\n\nsupersedes: {}\n",
            old_id
        ),
        1,
    );
    content.push_str(&second_with_supersedes);
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id)
        .expect("new id assigned");
    assert!(adr_answer(&dir, &new_id, "A").status.success());

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "check should fail: {}", stdout(&out));
    assert!(
        line_with(&stdout(&out), &old_id, ": tied to a test and not a current decision"),
        "{}",
        stdout(&out)
    );
}

// adr: 751ef30c-6a2f-4936-bf9b-b9d487146e53
// The other way a tie goes stale (751ef30c A): the question it names was
// answered with a refusal, never a lettered decision, so it never becomes
// current at all -- a tie to it is stale from the moment it is written.
#[test]
fn check_fails_a_test_tied_to_a_refused_decision() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = "# Example\n\n### 1.1 What color is the widget?\ndepends: widget\n\nProblem: the widget's color is undecided.\n\nStandard: plain words.\n\n- A) The widget is red.\n  Why: reasons.\n  Costs: costs.\n- B) The widget is blue.\n  Why: other reasons.\n  Costs: other costs.\n";
    let record = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let asked = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&asked)[0].clone();
    assert!(adr_answer(&dir, &id, "refuse").status.success());
    write_test_comment(&dir, &id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "check should fail: {}", stdout(&out));
    assert!(
        line_with(&stdout(&out), &id, ": tied to a test and not a current decision"),
        "{}",
        stdout(&out)
    );
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// A topic file the tool cannot read as text is judged by nothing. check
// fails on it and names the move; a pass would let the file into a commit
// with its questions unread.
#[test]
fn check_fails_a_topic_file_it_cannot_read() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    std::fs::create_dir_all(dir.join("adrdom/decisions")).unwrap();
    std::fs::write(dir.join("adrdom/decisions/broken.md"), [0x23u8, 0x20, 0xff, 0xfe, 0x0a]).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains(
            "parse: adrdom/decisions/broken.md: cannot be read: it holds no text the tool can read; \
agent fixes the file so it can be read as text (e.g. its encoding)"
        ),
        "{}",
        stdout(&out)
    );
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
// adr: 154e4b9a-c811-4a84-a522-598914eb39f1
//
// The commit hooks run check with --quiet. A commit that passes prints
// nothing; one that fails prints what failed and no note. check run by hand
// prints its notes as before.
#[test]
fn check_quiet_prints_what_failed_and_nothing_else() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_empty_terms(&dir);
    run_git(&dir, &["add", "-A"]);

    let pre_commit = std::fs::read_to_string(dir.join(".githooks/pre-commit")).unwrap();
    let commit_msg = std::fs::read_to_string(dir.join(".githooks/commit-msg")).unwrap();
    assert!(pre_commit.contains("adr check --staged --quiet"), "{}", pre_commit);
    assert!(commit_msg.contains("adr check --message \"$1\" --quiet"), "{}", commit_msg);

    let by_hand = adr(&dir, &["check", "--staged"], &[], "");
    assert!(by_hand.status.success(), "{}", stdout(&by_hand));
    assert!(stdout(&by_hand).contains("tested: 0"), "{}", stdout(&by_hand));

    let quiet = adr(&dir, &["check", "--staged", "--quiet"], &[], "");
    assert!(quiet.status.success(), "{}", stdout(&quiet));
    assert_eq!(stdout(&quiet), "");

    std::fs::write(dir.join("adrdom/decisions/broken.md"), [0x23u8, 0x20, 0xff, 0xfe, 0x0a]).unwrap();
    let quiet = adr(&dir, &["check", "--quiet"], &[], "");
    assert!(!quiet.status.success());
    let lines: Vec<String> = stdout(&quiet).lines().map(|l| l.to_string()).collect();
    assert_eq!(lines.len(), 1, "{:?}", lines);
    assert!(lines[0].starts_with("parse: adrdom/decisions/broken.md: cannot be read"), "{:?}", lines);
    assert!(!stdout(&quiet).contains("tested:"), "{}", stdout(&quiet));

    // By hand, without the flag, the same failing repo still prints its notes.
    let by_hand = adr(&dir, &["check"], &[], "");
    assert!(!by_hand.status.success());
    assert!(stdout(&by_hand).contains("tested: 0"), "{}", stdout(&by_hand));
    assert!(stdout(&by_hand).lines().count() > 1, "{}", stdout(&by_hand));
}

const TWO_QUESTIONS: &str = r#"# Example

### Is this pass?
depends: pass

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.

### Is that pass?
depends: pass

Problem: whether that passes.

Standard: clear yes or no.

- A) Yes that passes.
  Why: reasons.
  Costs: costs.
- B) No that does not.
  Why: other reasons.
  Costs: other costs.
"#;

/// Two questions in one tier, both asked, none answered; returns their ids.
fn ask_two_in_one_tier(dir: &std::path::Path) -> (String, String) {
    let terms_content =
        "# Terms\n\n## pass\nid: 11111111-1111-1111-1111-111111111111\ndefinition: Whether something is accepted.\naliases: \n";
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();
    approve_terms_via_ledger(dir, terms_content);
    let record = write_record(dir, "adrdom/decisions/example.md", TWO_QUESTIONS);
    approve_record(dir, "adrdom/decisions/example.md");
    let out = adr(dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    (ids[0].clone(), ids[1].clone())
}

// adr: 3460d220-e523-448b-aeff-baa8051b7d06
// adr: 154e4b9a-c811-4a84-a522-598914eb39f1
//
// An untested current decision is a note whether or not its tier is open:
// with one of two answered, one note; with both answered and no tests, two
// notes; check exits 0 each time.
#[test]
fn check_an_untested_decision_is_a_note_whether_or_not_its_tier_is_open() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (first, second) = ask_two_in_one_tier(&dir);
    assert!(adr_answer(&dir, &first, "A").status.success());

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(out.status.success(), "{}", text);
    assert!(
        text.contains(&format!(
            "untested: adrdom/decisions/example.md: \"Is this pass?\"; no tracked file holds \"adr: {}\"",
            first
        )),
        "{}",
        text
    );
    assert_eq!(text.lines().filter(|l| l.starts_with("untested:")).count(), 1, "{}", text);
    let quiet = adr(&dir, &["check", "--quiet"], &[], "");
    assert!(quiet.status.success(), "{}", stdout(&quiet));
    assert_eq!(stdout(&quiet), "");
    run_git(&dir, &["add", "-A"]);
    let staged = adr(&dir, &["check", "--staged", "--quiet"], &[], "");
    assert!(staged.status.success(), "{}", stdout(&staged));
    assert_eq!(stdout(&staged), "");

    assert!(adr_answer(&dir, &second, "A").status.success());
    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(out.status.success(), "{}", text);
    assert!(text.contains(&format!("no tracked file holds \"adr: {}\"", first)), "{}", text);
    assert!(text.contains(&format!("no tracked file holds \"adr: {}\"", second)), "{}", text);
    assert_eq!(text.lines().filter(|l| l.starts_with("untested:")).count(), 2, "{}", text);
    assert!(!text.contains("waiting:"), "{}", text);

    write_test_comment(&dir, &first);
    write_test_comment(&dir, &second);
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(!stdout(&out).contains("untested:"), "{}", stdout(&out));
}

// adr: 3460d220-e523-448b-aeff-baa8051b7d06
//
// A refused sibling changes nothing for the untested decision: it is still a
// note, and check exits 0.
#[test]
fn check_an_untested_decision_is_a_note_beside_a_refused_sibling() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (first, second) = ask_two_in_one_tier(&dir);
    assert!(adr_answer(&dir, &first, "A").status.success());
    let out = adr_answer(&dir, &second, "refuse");
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(out.status.success(), "{}", text);
    assert!(text.contains(&format!("no tracked file holds \"adr: {}\"", first)), "{}", text);
    assert!(!text.contains("waiting:"), "{}", text);
}
