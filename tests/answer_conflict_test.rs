// Decision e3200081-9995-4bbe-983f-2f430dc341e7: "Does check fail a
// question answered with two different options?" Answer: "Yes: check fails
// a question whose answers in the ledger name two different options,
// answer takes a new answer for it, and that answer stands." Real git
// branches and a real merge in a temp repo, the same harness shape
// `tests/ledger_merge_test.rs` uses (never this worktree's own repository).

mod common;

use common::*;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

const REL: &str = "adrdom/decisions/example.md";

/// Distinctive text from `msg::answer_conflict`.
const CONFLICT_TEXT: &str = "two standing answers,";

fn git_ok(dir: &Path, args: &[&str]) -> Output {
    let out = run_git(dir, args);
    assert!(
        out.status.success(),
        "git {:?} failed: stdout={} stderr={}",
        args,
        stdout(&out),
        stderr(&out)
    );
    out
}

fn current_branch(dir: &Path) -> String {
    stdout(&git_ok(dir, &["rev-parse", "--abbrev-ref", "HEAD"])).trim().to_string()
}

fn checkout_new_branch(dir: &Path, name: &str) {
    git_ok(dir, &["checkout", "-b", name]);
}

fn checkout(dir: &Path, name: &str) {
    git_ok(dir, &["checkout", name]);
}

fn commit_all(dir: &Path, msg: &str) {
    run_git(dir, &["add", "-A"]);
    let out = run_git(dir, &["commit", "-q", "-m", msg]);
    assert!(out.status.success(), "commit '{}' failed: {}", msg, stderr(&out));
}

fn finish_merge_commit(dir: &Path) {
    let out = run_git(dir, &["commit", "--no-edit"]);
    assert!(out.status.success(), "merge commit failed: {}", stderr(&out));
}

/// A repo with `adr install`ed and its hook wiring immediately switched off
/// (same as `ledger_merge_test.rs`'s `new_repo`), so a plain `git commit`
/// here is never gated by the pre-commit hook: these tests drive `adr
/// check`/`adr check --staged` directly instead.
fn new_repo() -> PathBuf {
    let dir = init_repo();
    git_ok(&dir, &["config", "merge.conflictStyle", "merge"]);
    assert!(adr(&dir, &["install"], &[], "").status.success());
    run_git(&dir, &["config", "--unset", "core.hooksPath"]);
    dir
}

/// A repo with terms and `EXAMPLE_RECORD` approved (plan/context, a minted
/// id), the question already asked once on the shared base -- committed, so
/// both branches diverge from a point that already has the `asked` row.
/// Returns `(dir, id, record path)`.
fn base_asked() -> (PathBuf, String, PathBuf) {
    let dir = new_repo();
    approve_terms_with(&dir, &["decision", "example"]);
    let record = write_record(&dir, REL, EXAMPLE_RECORD);
    approve_record(&dir, REL);
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    assert_eq!(ids.len(), 1);
    let id = ids[0].clone();
    commit_all(&dir, "base: asked the question");
    (dir, id, record)
}

/// Same as `base_asked`, but the question is only approved (id minted),
/// never asked on the base -- c2's shape, where each branch calls `adr ask`
/// itself.
fn base_unasked() -> (PathBuf, PathBuf) {
    let dir = new_repo();
    approve_terms_with(&dir, &["decision", "example"]);
    let record = write_record(&dir, REL, EXAMPLE_RECORD);
    approve_record(&dir, REL);
    commit_all(&dir, "base: approved, not yet asked");
    (dir, record)
}

/// Merges `source` into whichever branch is currently checked out
/// (`target`), forcing the topic file to exactly the content it held on
/// `target` just before the merge -- the "keep one letter" resolution the
/// decision's own framing describes, forced by bytes (mirroring
/// `ledger_merge_test.rs`'s `force_ledger_lines`) rather than trusted to
/// git's own conflict markers, since which side actually gets flagged as
/// conflicting can depend on merge-recursive's own heuristics. The ledger
/// itself is never forced: it is expected to merge cleanly by the `merge =
/// union` rule `adr install` wrote and this fixture committed on the base,
/// which is exactly what these tests exist to exercise.
fn merge_keep_target(dir: &Path, source: &str, record_rel: &str) {
    let target_content = std::fs::read_to_string(dir.join(record_rel)).unwrap();
    run_git(dir, &["merge", "--no-ff", "--no-commit", source]);
    std::fs::write(dir.join(record_rel), &target_content).unwrap();
    run_git(dir, &["add", record_rel]);
}

fn sleep_a_beat() {
    // More than two seconds apart, so two rows written on different
    // branches never land in the same wall-clock second (`ledger::write_time`
    // floors to whole seconds).
    std::thread::sleep(Duration::from_secs(3));
}

/// c1, both merge directions, walked end to end: base asks once; branch
/// `first` answers `letter_first`, branch `second` answers `letter_second`;
/// merging `merge_source` into `merge_target` conflicts (`adr check`
/// fails, staged and committed); `adr ask` re-asks it (c6: open, no
/// conflict any more); the owner answers again and `adr check` passes.
fn c1_flow(first: &str, second: &str, merge_target: &str, merge_source: &str) {
    let (dir, id, record) = base_asked();
    let base = current_branch(&dir);

    checkout_new_branch(&dir, first);
    let out = adr_answer(&dir, &id, "A");
    assert!(out.status.success(), "{} answer failed: {}", first, stderr(&out));
    commit_all(&dir, &format!("{}: answer A", first));

    checkout(&dir, &base);
    sleep_a_beat();
    checkout_new_branch(&dir, second);
    let out = adr_answer(&dir, &id, "B");
    assert!(out.status.success(), "{} answer failed: {}", second, stderr(&out));
    commit_all(&dir, &format!("{}: answer B", second));

    checkout(&dir, merge_target);
    merge_keep_target(&dir, merge_source, REL);

    let staged = adr(&dir, &["check", "--staged"], &[], "");
    assert!(
        !staged.status.success(),
        "check --staged should fail on the merged conflict: {}",
        stdout(&staged)
    );
    assert!(stdout(&staged).contains(CONFLICT_TEXT), "{}", stdout(&staged));
    assert!(stdout(&staged).contains(&id), "{}", stdout(&staged));

    finish_merge_commit(&dir);
    let committed = adr(&dir, &["check"], &[], "");
    assert!(
        !committed.status.success(),
        "check should still fail after the merge commit: {}",
        stdout(&committed)
    );
    assert!(stdout(&committed).contains(CONFLICT_TEXT), "{}", stdout(&committed));

    // c6: `adr ask` re-asks the conflicted question -- open, no conflict.
    let reask = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(reask.status.success(), "conflict re-ask failed: {}", stderr(&reask));
    assert!(
        stdout(&reask).contains("re-asked"),
        "expected a re-ask in the output: {}",
        stdout(&reask)
    );

    let after_reask = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&after_reask).contains(CONFLICT_TEXT),
        "c6: no conflict once the question is open again: {}",
        stdout(&after_reask)
    );

    // Full path: the owner answers again, A stands, check passes.
    let answer = adr_answer(&dir, &id, "A");
    assert!(answer.status.success(), "final answer failed: {}", stderr(&answer));
    write_test_comment(&dir, &id);

    let final_check = adr(&dir, &["check"], &[], "");
    assert!(
        final_check.status.success(),
        "check should pass once the conflict is resolved: {}",
        stdout(&final_check)
    );
}

// adr: e3200081-9995-4bbe-983f-2f430dc341e7
#[test]
fn c1_conflict_then_resolved_merging_b_into_a() {
    c1_flow("a", "b", "a", "b");
}

// adr: e3200081-9995-4bbe-983f-2f430dc341e7
//
// The reverse merge direction: same two branches, `a` merged into `b`
// instead of `b` into `a`.
#[test]
fn c1_conflict_then_resolved_merging_a_into_b() {
    c1_flow("a", "b", "b", "a");
}

// adr: e3200081-9995-4bbe-983f-2f430dc341e7
//
// c2: the base never asks the question at all; each branch calls `adr ask`
// itself, then answers differently. The later branch's own `asked` row has
// no `answered` row before it (only the other branch's, off to the side in
// ledger order), so it is not an "asked again" row: every `answered` row of
// the question stands, and they disagree.
#[test]
fn c2_independently_asked_on_each_branch_conflicts() {
    let (dir, record) = base_unasked();
    let base = current_branch(&dir);

    checkout_new_branch(&dir, "a");
    let ask_a = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(ask_a.status.success(), "a: ask failed: {}", stderr(&ask_a));
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    assert_eq!(ids.len(), 1);
    let id = ids[0].clone();
    let ans_a = adr_answer(&dir, &id, "A");
    assert!(ans_a.status.success(), "a: answer failed: {}", stderr(&ans_a));
    commit_all(&dir, "a: ask and answer A");

    checkout(&dir, &base);
    sleep_a_beat();
    checkout_new_branch(&dir, "b");
    let ask_b = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(ask_b.status.success(), "b: ask failed: {}", stderr(&ask_b));
    let ans_b = adr_answer(&dir, &id, "B");
    assert!(ans_b.status.success(), "b: answer failed: {}", stderr(&ans_b));
    commit_all(&dir, "b: ask and answer B");

    checkout(&dir, "a");
    merge_keep_target(&dir, "b", REL);
    finish_merge_commit(&dir);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "check should fail on the conflict: {}", stdout(&out));
    assert!(stdout(&out).contains(CONFLICT_TEXT), "{}", stdout(&out));
    assert!(stdout(&out).contains(&id), "{}", stdout(&out));
}

/// c1's shape, but branch `b` refuses instead of naming a second letter: a
/// refusal is a ruling too (rule 8), so it conflicts with branch `a`'s
/// answer exactly as a second letter would, re-asks the same way, and
/// settles by either a fresh letter or a fresh refusal, chosen by
/// `settle_with_refusal`.
fn c1_flow_answer_then_refusal(settle_with_refusal: bool) {
    let (dir, id, record) = base_asked();
    let base = current_branch(&dir);

    checkout_new_branch(&dir, "a");
    let out = adr_answer(&dir, &id, "A");
    assert!(out.status.success(), "a: answer failed: {}", stderr(&out));
    commit_all(&dir, "a: answer A");

    checkout(&dir, &base);
    sleep_a_beat();
    checkout_new_branch(&dir, "b");
    let out = adr_answer(&dir, &id, "refuse");
    assert!(out.status.success(), "b: refuse failed: {}", stderr(&out));
    commit_all(&dir, "b: refuse");

    checkout(&dir, "a");
    merge_keep_target(&dir, "b", REL);

    let staged = adr(&dir, &["check", "--staged"], &[], "");
    assert!(
        !staged.status.success(),
        "check --staged should fail on the merged conflict: {}",
        stdout(&staged)
    );
    assert!(stdout(&staged).contains(CONFLICT_TEXT), "{}", stdout(&staged));
    assert!(stdout(&staged).contains(&id), "{}", stdout(&staged));

    finish_merge_commit(&dir);
    let committed = adr(&dir, &["check"], &[], "");
    assert!(
        !committed.status.success(),
        "check should still fail after the merge commit: {}",
        stdout(&committed)
    );
    assert!(stdout(&committed).contains(CONFLICT_TEXT), "{}", stdout(&committed));

    // The re-ask clears the conflict (asked-again is now decided from the
    // rulings before the latest ask, an answer and a refusal both counted).
    let reask = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(reask.status.success(), "conflict re-ask failed: {}", stderr(&reask));
    assert!(
        stdout(&reask).contains("re-asked"),
        "expected a re-ask in the output: {}",
        stdout(&reask)
    );

    let after_reask = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&after_reask).contains(CONFLICT_TEXT),
        "no conflict once the question is open again: {}",
        stdout(&after_reask)
    );

    if settle_with_refusal {
        let answer = adr_answer(&dir, &id, "refuse");
        assert!(answer.status.success(), "final refuse failed: {}", stderr(&answer));
        // A refused question is never "current", so it needs no test tie.
    } else {
        let answer = adr_answer(&dir, &id, "A");
        assert!(answer.status.success(), "final answer failed: {}", stderr(&answer));
        write_test_comment(&dir, &id);
    }

    let final_check = adr(&dir, &["check"], &[], "");
    assert!(
        final_check.status.success(),
        "check should pass once the conflict is resolved: {}",
        stdout(&final_check)
    );
}

// adr: e3200081-9995-4bbe-983f-2f430dc341e7
//
// Rule 8: an answer on one branch and a refusal on the other, merged, is a
// conflict exactly like two different letters; settling it again with a
// fresh letter clears it.
#[test]
fn rule_8_answer_then_refusal_conflict_then_resolved_by_answer() {
    c1_flow_answer_then_refusal(false);
}

// adr: e3200081-9995-4bbe-983f-2f430dc341e7
//
// Same conflict, but settled again with a fresh refusal instead of a
// letter.
#[test]
fn rule_8_answer_then_refusal_conflict_then_resolved_by_refusal() {
    c1_flow_answer_then_refusal(true);
}

// adr: e3200081-9995-4bbe-983f-2f430dc341e7
//
// c5: both branches answer the same letter -- not a conflict, `adr check`
// passes straight through the merge.
#[test]
fn c5_same_letter_on_both_branches_is_not_a_conflict() {
    let (dir, id, _record) = base_asked();
    let base = current_branch(&dir);

    checkout_new_branch(&dir, "a");
    let out = adr_answer(&dir, &id, "A");
    assert!(out.status.success(), "a: answer failed: {}", stderr(&out));
    write_test_comment(&dir, &id);
    commit_all(&dir, "a: answer A");

    checkout(&dir, &base);
    sleep_a_beat();
    checkout_new_branch(&dir, "b");
    let out = adr_answer(&dir, &id, "A");
    assert!(out.status.success(), "b: answer failed: {}", stderr(&out));
    commit_all(&dir, "b: answer A");

    checkout(&dir, "a");
    merge_keep_target(&dir, "b", REL);

    let staged = adr(&dir, &["check", "--staged"], &[], "");
    assert!(
        staged.status.success(),
        "check --staged should pass, both branches agree: {}",
        stdout(&staged)
    );

    finish_merge_commit(&dir);
    let committed = adr(&dir, &["check"], &[], "");
    assert!(
        committed.status.success(),
        "check should pass after the merge commit too: {}",
        stdout(&committed)
    );
}
