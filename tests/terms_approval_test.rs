// "Does check fail a terms file that differs from its latest approval?"
// (03b2853b A, supersedes "Does the owner approve the terms file as a
// whole?"): an unapproved/changed adrdom/terms.md is only ever a note --
// `check` reports it as waiting and `ask` stops -- except when the
// definition of a term some current decision pairs, directly or through
// another term's own definition, actually moved since the latest
// approval, which still fails `check` by name.
mod common;

use common::*;

const PASS_RECORD: &str = r#"# Example

### 1.1 Does this pass?
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

/// Appended to `PASS_RECORD` only once 1.1 is already answered (mirrors
/// `approve_test.rs`'s `GROUP_TWO_ADDITION`): a second, later-tier question
/// that depends on 1.1 having any answer, so it becomes askable exactly
/// when 1.1 is current -- and not a moment before, since `ask` refuses the
/// whole file outright while a later tier's candidate sits next to an
/// earlier tier still open.
const PASS_RECORD_TIER2_ADDITION: &str = r#"
### 1.2 Does this also pass?
depends: 1.1 Does this pass?=*, pass

Problem: whether the second one passes too.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.
"#;

const WIDGET_RECORD: &str = r#"# Example

### 1.1 Does the widget work?
depends: widget

Problem: whether the widget functions.

Standard: clear yes or no.

- A) Yes it works.
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

/// Real `adr approve adrdom/terms.md`, then a real git commit of the
/// result (`--no-verify`: the repo's own pre-commit hook runs `adr check
/// --staged`, which has nothing to do with what these tests are about) so
/// `check`'s recovery of the terms file's latest approved text -- which,
/// unlike the whole-file hash, the ledger never stores directly -- has a
/// commit in history to find. Panics if the real approve refuses, since
/// every fixture here is meant to be one it accepts.
fn approve_and_commit_terms(dir: &std::path::Path, content: &str) -> std::path::PathBuf {
    let path = write_record(dir, "adrdom/terms.md", content);
    let out = adr_owner(dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "approve terms failed: {}", stderr(&out));
    run_git(dir, &["add", "adrdom/terms.md"]);
    run_git(dir, &["commit", "--no-verify", "-q", "-m", "approve terms"]);
    path
}

// adr: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
//
// A brand-new, unapproved term that nothing current pairs is a draft, not
// a risk: `check` passes and only notes the wait, the same way an
// unapproved plan edit is noted rather than failed. `ask` keeps refusing a
// fresh question outright while terms.md waits (unchanged: 03b2853b A
// keeps that gate; this test just confirms it still holds once the
// failure itself has become conditional).
#[test]
fn unapproved_unused_new_term_is_only_a_waiting_note() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = approve_and_commit_terms(&dir, TERMS_PASS_V1);

    let record = write_record(&dir, "adrdom/decisions/example.md", PASS_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    write_test_comment(&dir, &id);

    // A later-tier question, added now that 1.1 is current -- approved
    // (minting its id) while terms.md still matches its latest approval,
    // but deliberately not asked yet.
    let mut with_tier2 = std::fs::read_to_string(&record).unwrap();
    with_tier2.push_str(PASS_RECORD_TIER2_ADDITION);
    std::fs::write(&record, &with_tier2).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    // A new, unapproved term with no pairing to anything current at all.
    // Given an id here (unlike a real agent edit) only so the file stays
    // shape-valid: a missing id is its own, unrelated "approve"/"ask" gate
    // (`require_terms`), not the "differs from its latest approval"
    // question this test is about.
    let terms_v2 = format!(
        "{}\n## spare\nid: 44444444-4444-4444-4444-444444444444\ndefinition: An unused fixture term.\naliases: \n",
        TERMS_PASS_V1
    );
    std::fs::write(&terms_path, &terms_v2).unwrap();

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(check_out.status.success(), "check should pass: {}", stdout(&check_out));
    assert!(
        stdout(&check_out).contains("terms: awaiting approval"),
        "{}",
        stdout(&check_out)
    );

    // 1.2 (tier 2, depending on 1.1) is open now that 1.1 is answered, but
    // still can't be asked while terms.md waits -- already-answered work
    // is unaffected, new work is not.
    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(!ask_out.status.success());
    assert!(stderr(&ask_out).contains("terms not approved"), "{}", stderr(&ask_out));
}

// adr: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
//
// "pass" is paired directly by 1.1, a current (answered) decision. Its
// definition changes with no new approve: unlike the unused-term case
// above, this is exactly the edit 03b2853b A carves out as a failure, not
// a note.
#[test]
fn changed_definition_of_a_directly_paired_term_fails_check() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = approve_and_commit_terms(&dir, TERMS_PASS_V1);

    let record = write_record(&dir, "adrdom/decisions/example.md", PASS_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    write_test_comment(&dir, &id);

    let terms_v2 = "# Terms\n\n\
## pass\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: Whether something clears review.\n\
aliases: \n";
    std::fs::write(&terms_path, terms_v2).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "check should fail: {}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("terms:"), "{}", text);
    assert!(text.contains("pass"), "{}", text);
    assert!(text.contains("1.1 Does this pass?"), "{}", text);
    assert!(text.contains("owner runs: adr approve adrdom/terms.md"), "{}", text);
}

// adr: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
//
// "gadget" is never named by any decision directly -- only "widget" is,
// via its heading -- but "widget"'s own definition uses the word "gadget",
// so approving TERMS_WIDGET_V1 gives "widget" a terms line pairing
// "gadget"'s id. "gadget"'s definition then changes, reachable from the
// current widget decision only through that second hop, and 03b2853b A's
// "directly, or through another term's definition" still fails check.
const TERMS_WIDGET_V1: &str = "# Terms\n\n\
## gadget\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A small mechanical device.\n\
aliases: \n\
\n\
## widget\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A gadget used to answer a question.\n\
aliases: \n";

#[test]
fn changed_definition_of_a_transitively_reached_term_fails_check() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = approve_and_commit_terms(&dir, TERMS_WIDGET_V1);

    let record = write_record(&dir, "adrdom/decisions/example.md", WIDGET_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    write_test_comment(&dir, &id);

    // Only "gadget"'s definition moves; "widget"'s stays exactly as
    // approved, terms line included -- that line is only ever written by
    // `approve`, and this edit is meant to look like an agent's, which
    // never touches it.
    let terms_v2 = "# Terms\n\n\
## gadget\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A small mechanical device with moving parts.\n\
aliases: \n\
\n\
## widget\n\
id: 33333333-3333-3333-3333-333333333333\n\
terms: gadget=22222222-2222-2222-2222-222222222222\n\
definition: A gadget used to answer a question.\n\
aliases: \n";
    std::fs::write(&terms_path, terms_v2).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "check should fail: {}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("gadget"), "{}", text);
    assert!(!text.contains("definition of widget"), "{}", text);
    assert!(text.contains("1.1 Does the widget work?"), "{}", text);
}

// adr: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
//
// The owner running `adr approve adrdom/terms.md` over the changed
// definition is what clears the wait: `check` fails right up until that
// approve, and passes cleanly right after.
#[test]
fn approving_the_changed_definition_clears_the_wait() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = approve_and_commit_terms(&dir, TERMS_PASS_V1);

    let record = write_record(&dir, "adrdom/decisions/example.md", PASS_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    write_test_comment(&dir, &id);

    let terms_v2 = "# Terms\n\n\
## pass\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: Whether something clears review.\n\
aliases: \n";
    std::fs::write(&terms_path, terms_v2).unwrap();

    let before = adr(&dir, &["check"], &[], "");
    assert!(
        !before.status.success(),
        "sanity: check should still be failing before approve: {}",
        stdout(&before)
    );

    let approve_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(approve_out.status.success(), "{}", stderr(&approve_out));

    let after = adr(&dir, &["check"], &[], "");
    assert!(after.status.success(), "check should pass once approved: {}", stdout(&after));
}

/// A fresh, depth-1 clone of `src` (a repo built with `init_repo()`), the
/// same shape a CI checkout of `.github/workflows/check.yml` produces:
/// every file tracked at `HEAD`, but no history behind it. Panics if the
/// clone itself fails.
fn shallow_clone(src: &std::path::Path) -> std::path::PathBuf {
    let dest = std::env::temp_dir().join(format!(
        "adr-test-clone-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let out = run_git(
        &std::env::temp_dir(),
        &[
            "clone",
            "--depth",
            "1",
            &format!("file://{}", src.display()),
            dest.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "shallow clone failed: {}", stderr(&out));
    dest
}

// adr: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
//
// `approve adrdom/terms.md` writes `adrdom/state/terms.approved.md`
// alongside its ledger row, byte for byte what it just wrote to
// adrdom/terms.md itself: `check`'s only source, from here on, for the
// file's latest approved text.
#[test]
fn approve_writes_the_snapshot() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = write_record(&dir, "adrdom/terms.md", TERMS_PASS_V1);
    let out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let snapshot_path = dir.join("adrdom/state/terms.approved.md");
    assert!(snapshot_path.exists(), "snapshot should exist after approve");
    let snapshot = std::fs::read_to_string(&snapshot_path).unwrap();
    let terms_md = std::fs::read_to_string(&terms_path).unwrap();
    assert_eq!(
        snapshot, terms_md,
        "snapshot should be exactly what approve wrote to adrdom/terms.md"
    );
}

// adr: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
//
// The CI case this mechanism exists for: a workflow that checks out with
// `fetch-depth: 1` carries no history behind `HEAD` at all, so walking
// `git log` to recover the terms file's latest approved text (the old
// mechanism) always came up empty there -- failing safe on every commit,
// even the plain "add an unused term" draft 03b2853b A means to allow. The
// snapshot is an ordinary tracked file at `HEAD`, so a shallow clone still
// carries it.
#[test]
fn unapproved_draft_that_only_adds_a_term_passes_check_in_a_shallow_clone() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "--no-verify", "-q", "-m", "install"]);

    let terms_path = write_record(&dir, "adrdom/terms.md", TERMS_PASS_V1);
    let out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "--no-verify", "-q", "-m", "approve terms"]);

    let record = write_record(&dir, "adrdom/decisions/example.md", PASS_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    write_test_comment(&dir, &id);
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "--no-verify", "-q", "-m", "decision"]);

    // A new, unapproved term with no pairing to anything current at all,
    // committed exactly as an agent's commit of a draft would look.
    let terms_v2 = format!(
        "{}\n## spare\nid: 44444444-4444-4444-4444-444444444444\ndefinition: An unused fixture term.\naliases: \n",
        TERMS_PASS_V1
    );
    std::fs::write(&terms_path, &terms_v2).unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "--no-verify", "-q", "-m", "draft term"]);

    let clone_dir = shallow_clone(&dir);
    let check_out = adr(&clone_dir, &["check"], &[], "");
    assert!(
        check_out.status.success(),
        "check should pass in a shallow clone: {}",
        stdout(&check_out)
    );
    assert!(
        stdout(&check_out).contains("terms: awaiting approval"),
        "{}",
        stdout(&check_out)
    );
}

// adr: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
//
// The snapshot lives under adrdom/state/, which the brief tells an agent
// never to edit: `check` still catches a hand-edited one, since it is the
// one thing standing between a real approval and a forged "this was
// already approved at this text" claim.
#[test]
fn edited_snapshot_fails_check() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = write_record(&dir, "adrdom/terms.md", TERMS_PASS_V1);
    let out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let snapshot_path = dir.join("adrdom/state/terms.approved.md");
    let mut snapshot = std::fs::read_to_string(&snapshot_path).unwrap();
    snapshot.push_str(
        "\n## tampered\nid: 99999999-9999-9999-9999-999999999999\ndefinition: Not real.\naliases: \n",
    );
    std::fs::write(&snapshot_path, &snapshot).unwrap();

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(!check_out.status.success(), "{}", stdout(&check_out));
    let text = stdout(&check_out);
    assert!(text.contains("does not match the ledger"), "{}", text);
    assert!(
        text.contains("owner runs: adr approve adrdom/terms.md"),
        "{}",
        text
    );
}

// adr: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
//
// Items 1+2 of the bootstrap-gaps review: a repo that approved
// adrdom/terms.md before this snapshot mechanism existed carries a real
// `terms` ledger row but never got adrdom/state/terms.approved.md written
// (approve_terms_via_ledger recreates exactly that shape -- a hand-written
// ledger row with no real `adr approve` run to also snapshot the file).
// `adr install` now backfills the snapshot from the file itself whenever it
// still matches its latest approval, so a later edit that touches nothing
// reachable compares against real "before" text instead of failing safe on
// a missing copy.
#[test]
fn install_backfills_the_snapshot_for_an_already_approved_repo_and_a_blank_line_edit_then_only_waits() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = write_record(&dir, "adrdom/terms.md", TERMS_PASS_V1);
    approve_terms_via_ledger(&dir, TERMS_PASS_V1);
    assert!(
        !dir.join("adrdom/state/terms.approved.md").exists(),
        "sanity: no snapshot should exist yet"
    );

    let record = write_record(&dir, "adrdom/decisions/example.md", PASS_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    write_test_comment(&dir, &id);

    let install_out = adr(&dir, &["install"], &[], "");
    assert!(install_out.status.success(), "{}", stderr(&install_out));

    let snapshot_path = dir.join("adrdom/state/terms.approved.md");
    assert!(
        snapshot_path.exists(),
        "install should have backfilled the snapshot: {}",
        stdout(&install_out)
    );
    assert!(
        stdout(&install_out).contains("terms.approved.md"),
        "install should report the backfill: {}",
        stdout(&install_out)
    );
    let snapshot = std::fs::read_to_string(&snapshot_path).unwrap();
    let terms_md = std::fs::read_to_string(&terms_path).unwrap();
    assert_eq!(
        snapshot, terms_md,
        "backfilled snapshot should be byte-identical to adrdom/terms.md"
    );

    // A blank-line edit, touching nothing reachable, now has a real
    // snapshot to compare against and passes with only the waiting note --
    // not the "approved copy is missing" failure this used to earn.
    let edited = format!("{}\n", TERMS_PASS_V1);
    std::fs::write(&terms_path, &edited).unwrap();

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(
        check_out.status.success(),
        "check should pass: {}",
        stdout(&check_out)
    );
    assert!(
        stdout(&check_out).contains("terms: awaiting approval"),
        "{}",
        stdout(&check_out)
    );
    assert!(
        !stdout(&check_out).contains("approved copy is missing"),
        "{}",
        stdout(&check_out)
    );
}

// adr: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
//
// `do_approve_terms` used to write the snapshot before appending the
// ledger row: a failed append left the two mismatched (a snapshot claiming
// an approval the ledger never recorded). The snapshot is now written only
// after the append succeeds, so a failed append leaves no snapshot at all.
#[test]
fn approve_terms_does_not_write_the_snapshot_when_the_ledger_append_fails() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = write_record(&dir, "adrdom/terms.md", TERMS_PASS_V1);

    // Replace the ledger file with a directory: `ledger::append`'s
    // `OpenOptions::open` fails on it, while `ledger::read` (used earlier
    // in `do_approve_terms`) fails safe to an empty ledger instead of
    // panicking.
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    std::fs::remove_file(&ledger_path).unwrap();
    std::fs::create_dir(&ledger_path).unwrap();

    let out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(
        !out.status.success(),
        "approve should fail when the ledger can't be written: {}",
        stdout(&out)
    );

    assert!(
        !dir.join("adrdom/state/terms.approved.md").exists(),
        "the snapshot must not be written when the ledger append failed"
    );
}

// adr: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
//
// A term some current decision reaches that is present in the current
// draft but entirely missing from the approved snapshot (no "before" text
// to compare an "after" against) used to be folded into "definition
// changed" via `affected_by_edit`'s `unwrap_or(true)`; it now earns its own,
// accurate message.
#[test]
fn term_missing_from_the_approved_copy_gets_its_own_message() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let terms_path = approve_and_commit_terms(&dir, TERMS_PASS_V1);

    let record = write_record(&dir, "adrdom/decisions/example.md", PASS_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    write_test_comment(&dir, &id);

    // Hand-edit the answered decision's own terms line to also pair a
    // brand-new term -- real `ask` never touches an already-answered
    // section's terms line, so this stands in for whatever got that
    // pairing onto it; the check being proved has to hold regardless of
    // how the pairing got there.
    let after_answer = std::fs::read_to_string(&record).unwrap();
    let edited_record = after_answer.replace(
        "terms: pass=11111111-1111-1111-1111-111111111111",
        "terms: pass=11111111-1111-1111-1111-111111111111, extra=55555555-5555-5555-5555-555555555555",
    );
    assert_ne!(
        edited_record, after_answer,
        "the terms line should have been found and edited"
    );
    std::fs::write(&record, edited_record).unwrap();

    // "extra" exists in the current, unapproved draft but never made it
    // into the approved snapshot at all.
    let terms_v2 = format!(
        "{}\n## extra\nid: 55555555-5555-5555-5555-555555555555\ndefinition: A term the snapshot never saw.\naliases: \n",
        TERMS_PASS_V1
    );
    std::fs::write(&terms_path, &terms_v2).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "check should fail: {}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("extra is not in the approved copy"), "{}", text);
    assert!(text.contains("1.1 Does this pass?"), "{}", text);
    assert!(!text.contains("definition of"), "{}", text);
    assert!(
        text.contains("owner runs: adr approve adrdom/terms.md"),
        "{}",
        text
    );
}
