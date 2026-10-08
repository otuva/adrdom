// Decision 1c27f2d6-ab62-4c5a-91b2-1488722f3c48: "at a merge commit check
// fails unless every ledger row of each of the two branches is in the
// merged ledger." Real git branches and merges, inside the harness's own
// throwaway temp repo -- never this worktree's repository. The helper
// shapes here (conflict markers, resolution, merge-then-commit) mirror
// `merge_lab_ban_test.rs`/`merge_lab_questions_test.rs` (read, not edited).

mod common;

use adr::record;
use common::*;
use std::path::{Path, PathBuf};
use std::process::Output;

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

/// A repo with `adr install`ed and the hook wiring immediately switched off
/// (`core.hooksPath` unset), so a plain `git commit` here is never gated by
/// the pre-commit hook -- these tests drive `adr check`/`adr check
/// --staged` directly instead, at exactly the point in the merge each case
/// wants to look at.
fn new_repo() -> PathBuf {
    let dir = init_repo();
    git_ok(&dir, &["config", "merge.conflictStyle", "merge"]);
    assert!(adr(&dir, &["install"], &[], "").status.success());
    run_git(&dir, &["config", "--unset", "core.hooksPath"]);
    dir
}

fn finish_merge_commit(dir: &Path) {
    let out = run_git(dir, &["commit", "--no-edit"]);
    assert!(out.status.success(), "merge commit failed: {}", stderr(&out));
}

/// Distinctive text from `msg::ledger_merge_drops_rows`, searched for in
/// `check`'s output rather than duplicated field-for-field (the crate has
/// no lib target for tests to call the real formatter directly).
const MERGE_DROPS_ROWS_TEXT: &str = "merge drops a ledger row of a merged branch";

/// Distinctive text from `msg::ledger_merge_parents_unreadable`.
const PARENTS_UNREADABLE_TEXT: &str = "this merge commit's parents cannot be read";

/// Builds a base repo with an empty, tracked `adrdom/state/ledger.tsv`,
/// then branches "a" and "b" off it, each appending one distinct row to
/// the ledger and committing. Returns `(dir, base_branch_name)` with the
/// working tree left on branch "a".
fn setup_two_branches_each_appending_a_row(dir: &Path) {
    std::fs::create_dir_all(dir.join("adrdom/state")).unwrap();
    std::fs::write(dir.join("adrdom/state/ledger.tsv"), "").unwrap();
    commit_all(dir, "base: empty ledger");
    let base = current_branch(dir);

    checkout_new_branch(dir, "a");
    append_ledger_row(
        dir,
        "context",
        "-",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "branch a row",
    );
    commit_all(dir, "a: append row");

    checkout(dir, &base);
    checkout_new_branch(dir, "b");
    append_ledger_row(
        dir,
        "context",
        "-",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "branch b row",
    );
    commit_all(dir, "b: append row");

    checkout(dir, "a");
}

// adr: 1c27f2d6-ab62-4c5a-91b2-1488722f3c48
//
// A merge resolved by keeping both sides' ledger rows: `adr check
// --staged` passes while the merge is still pending, and `adr check`
// passes again once the merge commit is made.
#[test]
fn a_merge_keeping_both_branches_rows_passes_staged_and_committed() {
    let dir = new_repo();
    setup_two_branches_each_appending_a_row(&dir);

    // Read each branch's own row before the merge disturbs the working
    // tree, then force the exact bytes a "keep both" resolution leaves --
    // this environment's `adrdom/.gitattributes` (`merge=union`, committed
    // as part of the shared base before "a" and "b" diverge) already makes
    // this merge auto-resolve with no conflict on the ledger, so the test
    // does not rely on git reporting one.
    let ours_row = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap()
        .trim()
        .to_string();
    let theirs_row = stdout(&git_ok(&dir, &["show", "b:adrdom/state/ledger.tsv"]))
        .trim()
        .to_string();

    run_git(&dir, &["merge", "--no-ff", "--no-commit", "b"]);
    force_ledger_lines(&dir, &[&ours_row, &theirs_row]);

    let staged = adr(&dir, &["check", "--staged"], &[], "");
    assert!(
        staged.status.success(),
        "check --staged should pass while the merge is pending, both branches' rows kept: {}",
        stdout(&staged)
    );

    finish_merge_commit(&dir);

    let committed = adr(&dir, &["check"], &[], "");
    assert!(
        committed.status.success(),
        "check should pass after the merge commit, both branches' rows kept: {}",
        stdout(&committed)
    );
}

// adr: 1c27f2d6-ab62-4c5a-91b2-1488722f3c48
//
// Git runs no pre-commit hook for a merge it commits itself, only
// commit-msg: at a merge the commit-msg hook's `check --message` judges the
// staged files too, so a merge that keeps only its own ledger rows
// (`-s ours`) is refused there, through the installed hooks.
#[test]
fn a_merge_dropping_a_branchs_row_is_refused_by_the_message_hook() {
    let dir = new_repo();
    setup_two_branches_each_appending_a_row(&dir);
    let ours_row = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap().trim().to_string();

    run_git(&dir, &["merge", "--no-ff", "--no-commit", "b"]);
    force_ledger_lines(&dir, &[&ours_row]);
    let msg = dir.join(".git/MERGE_MSG");
    let hook = adr(&dir, &["check", "--message", msg.to_str().unwrap()], &[], "");
    assert!(!hook.status.success(), "the message hook refuses the merge: {}", stdout(&hook));
    assert!(stdout(&hook).contains(MERGE_DROPS_ROWS_TEXT), "{}", stdout(&hook));
    run_git(&dir, &["merge", "--abort"]);

    // Through git itself, with the hooks install wrote.
    git_ok(&dir, &["config", "core.hooksPath", ".githooks"]);
    let merged = run_git(&dir, &["merge", "-s", "ours", "--no-ff", "-m", "merge b", "b"]);
    assert!(!merged.status.success(), "git merge -s ours lands: {}", stdout(&merged));
    let text = format!("{}{}", stdout(&merged), stderr(&merged));
    assert!(text.contains(MERGE_DROPS_ROWS_TEXT), "{}", text);
}

// adr: 1c27f2d6-ab62-4c5a-91b2-1488722f3c48
//
// The same merge, resolved by dropping the merged-in branch's row
// instead: `adr check --staged` fails with the new message while the
// merge is still pending.
#[test]
fn b_merge_dropping_the_other_branchs_row_fails_staged() {
    let dir = new_repo();
    setup_two_branches_each_appending_a_row(&dir);

    // Read HEAD's own row before the merge disturbs the working tree, then
    // force the ledger to exactly that one row -- the bytes a "keep ours"
    // resolution leaves, dropping "b"'s row entirely -- regardless of
    // whether this environment's `merge=union` attribute auto-resolved the
    // merge cleanly or git reported a real conflict.
    let ours_row = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap()
        .trim()
        .to_string();

    run_git(&dir, &["merge", "--no-ff", "--no-commit", "b"]);
    force_ledger_lines(&dir, &[&ours_row]);

    let staged = adr(&dir, &["check", "--staged"], &[], "");
    assert!(
        !staged.status.success(),
        "check --staged should refuse a merge that drops a branch's rows"
    );
    let text = stdout(&staged);
    assert!(text.contains(MERGE_DROPS_ROWS_TEXT), "{}", text);
    assert!(text.contains("no ledger row is lost in a merge; owner adds the missing rows back from that branch"), "{}", text);
}

// adr: 1c27f2d6-ab62-4c5a-91b2-1488722f3c48
//
// A merge commit already made (no hooks installed, so nothing gated the
// commit) that dropped the other branch's row: plain `adr check`, with no
// merge pending any more, still catches it via HEAD's second parent.
#[test]
fn c_committed_merge_that_dropped_a_branchs_row_fails_plain_check() {
    let dir = new_repo();
    setup_two_branches_each_appending_a_row(&dir);

    // Same forced "keep ours" resolution as the staged case above, carried
    // through to a real merge commit.
    let ours_row = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap()
        .trim()
        .to_string();

    run_git(&dir, &["merge", "--no-ff", "--no-commit", "b"]);
    force_ledger_lines(&dir, &[&ours_row]);
    finish_merge_commit(&dir);

    // Confirm this really is a merge commit before trusting the refusal.
    assert!(run_git(&dir, &["rev-parse", "-q", "--verify", "HEAD^2"]).status.success());

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        !out.status.success(),
        "plain check should refuse a committed merge that dropped a branch's rows"
    );
    let text = stdout(&out);
    assert!(text.contains(MERGE_DROPS_ROWS_TEXT), "{}", text);
    assert!(text.contains("no ledger row is lost in a merge; owner adds the missing rows back from that branch"), "{}", text);
}

/// A fresh, otherwise-empty directory to clone into: its own path, never a
/// git repo of its own, so `git clone`'s destination argument can sit
/// beside it without colliding with any other test's temp directory.
fn fresh_clone_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "adr-test-shallow-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

// adr: 1c27f2d6-ab62-4c5a-91b2-1488722f3c48
//
// A `--depth 1` clone of a repo whose HEAD is a merge commit: `HEAD^2`
// doesn't resolve there (a shallow clone hides a parent it never fetched
// from `rev-parse`), so `check` reads HEAD's own commit object instead,
// which still names both parents regardless of which of them the clone
// actually holds. Confirmed by experiment before writing this test: in
// such a clone, `git cat-file commit HEAD` still prints both `parent`
// lines, while `git cat-file -e <sha>^{commit}` fails on each of them, and
// `git fetch --deepen=1` (cloned over a `file://` URL, so depth is
// honoured) brings both parents in and makes `HEAD^2` resolve again.
#[test]
fn d_a_shallow_clone_of_a_merge_commit_fails_until_deepened() {
    let dir = new_repo();
    setup_two_branches_each_appending_a_row(&dir);

    // Same "keep both" resolution as the passing case above, carried
    // through to a real merge commit, so the full-history repo itself
    // passes check before it is ever cloned.
    let ours_row = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap()
        .trim()
        .to_string();
    let theirs_row = stdout(&git_ok(&dir, &["show", "b:adrdom/state/ledger.tsv"]))
        .trim()
        .to_string();

    run_git(&dir, &["merge", "--no-ff", "--no-commit", "b"]);
    force_ledger_lines(&dir, &[&ours_row, &theirs_row]);
    finish_merge_commit(&dir);

    let full = adr(&dir, &["check"], &[], "");
    assert!(
        full.status.success(),
        "the full-history repo should pass before any clone: {}",
        stdout(&full)
    );

    let clone_dir = fresh_clone_dir();
    let url = format!("file://{}", dir.display());
    let clone_out = run_git(
        &std::env::temp_dir(),
        &["clone", "--depth", "1", &url, clone_dir.to_str().unwrap()],
    );
    assert!(clone_out.status.success(), "shallow clone failed: {}", stderr(&clone_out));

    // The premise this case depends on: a shallow clone really does hide
    // the merge commit's second parent from `rev-parse`.
    assert!(
        !run_git(&clone_dir, &["rev-parse", "-q", "--verify", "HEAD^2"]).status.success(),
        "HEAD^2 unexpectedly resolved in the shallow clone"
    );

    let shallow = adr(&clone_dir, &["check"], &[], "");
    assert!(
        !shallow.status.success(),
        "check should refuse a shallow clone of a merge commit whose parents it cannot read"
    );
    let text = stdout(&shallow);
    assert!(text.contains("check: "), "{}", text);
    assert!(text.contains(PARENTS_UNREADABLE_TEXT), "{}", text);
    assert!(text.contains("the clone is too shallow; agent runs: git fetch --deepen=1"), "{}", text);

    let deepen = run_git(&clone_dir, &["fetch", "--deepen=1"]);
    assert!(deepen.status.success(), "git fetch --deepen=1 failed: {}", stderr(&deepen));

    let after_deepen = adr(&clone_dir, &["check"], &[], "");
    assert!(
        after_deepen.status.success(),
        "check should pass once the merge commit's parents are fetched: {}",
        stdout(&after_deepen)
    );
}

// --- c10c3a49-e993-4c9f-a46b-4a1ba3646dcf: "Does check fail a ledger --------
// whose rows changed place?" A: no, only a missing or changed row fails; a
// row's place in the file is free. The three tests below force the exact
// ledger bytes a `--no-commit` merge leaves pending (`force_ledger_lines`),
// rather than trust git's own choice of order for two independent
// appends: this environment's `git merge` already auto-resolves that case
// without a real text conflict, so relying on one (the way the
// 1c27f2d6 tests above do) would not exercise the row order these tests
// mean to pin down.

/// Overwrites the ledger with exactly `lines`, one per line, and stages
/// it: pins the exact row order a test wants `check` to judge after a
/// `--no-commit` merge, regardless of whether git auto-merged the two
/// sides cleanly or left conflict markers of its own -- either way, this
/// replaces the working copy with a known, controlled byte sequence.
fn force_ledger_lines(dir: &Path, lines: &[&str]) {
    let path = dir.join("adrdom/state/ledger.tsv");
    let mut out = String::new();
    for l in lines {
        out.push_str(l);
        out.push('\n');
    }
    std::fs::write(&path, out).unwrap();
    git_ok(dir, &["add", "adrdom/state/ledger.tsv"]);
}

/// Same, for `adrdom/state/banned.sha256`: `check` holds this file to the
/// ledger's own `banned`/`unbanned` history (per "Does check hold the ban
/// list equal to the ledger?"), by hash set, order irrelevant -- so a test
/// that forces `force_ledger_lines` to carry both branches' `banned` rows
/// has to carry both hashes here too, or that check fails on its own.
fn force_banned_hashes(dir: &Path, hashes: &[&str]) {
    let path = dir.join("adrdom/state/banned.sha256");
    let mut out = String::new();
    for h in hashes {
        out.push_str(h);
        out.push('\n');
    }
    std::fs::write(&path, out).unwrap();
    git_ok(dir, &["add", "adrdom/state/banned.sha256"]);
}

// adr: c10c3a49-e993-4c9f-a46b-4a1ba3646dcf
//
// The flow the byte-prefix rule used to refuse: `feat` merges `main` in
// first, its own row kept ahead of main's in the resolved ledger; `main`
// then merges `feat` back, and HEAD (`main`)'s one row now sits second,
// not first, in the ledger `check` judges at that second merge -- exactly
// the shape a byte-prefix test refuses and a presence test does not.
#[test]
fn a_the_flow_the_old_prefix_rule_refused_now_passes() {
    let dir = new_repo();
    // `adr install` only writes files; nothing is committed yet (per "an
    // owner command writes the file and commits nothing" -- the same is
    // true of `install`). Commit it now, on the shared base, so `feat` and
    // `main` both start from a commit that already tracks `adrdom/state/`
    // -- otherwise the branch neither one committed it on loses the
    // directory outright when checked out.
    commit_all(&dir, "install");
    let main_branch = current_branch(&dir);

    // `adr ban` writes the word's hash, never the plain word, so each
    // row is found by that hash (`record::sha256_hex`, the same hash
    // `banned::tokenize`'s one token for a bare lowercase word hashes to),
    // not by searching the file for the word itself.
    let feat_hash = record::sha256_hex("featnonsenseword");
    let main_hash = record::sha256_hex("mainnonsenseword");

    checkout_new_branch(&dir, "feat");
    let feat_ban = adr(&dir, &["ban"], &[], "featnonsenseword\n");
    assert!(feat_ban.status.success(), "feat: ban failed: {}", stderr(&feat_ban));
    commit_all(&dir, "feat: ban featnonsenseword");
    let feat_row = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap()
        .lines()
        .find(|l| l.contains(&feat_hash))
        .expect("feat's own row")
        .to_string();

    checkout(&dir, &main_branch);
    let main_ban = adr(&dir, &["ban"], &[], "mainnonsenseword\n");
    assert!(main_ban.status.success(), "main: ban failed: {}", stderr(&main_ban));
    commit_all(&dir, "main: ban mainnonsenseword");
    let main_row = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap()
        .lines()
        .find(|l| l.contains(&main_hash))
        .expect("main's own row")
        .to_string();

    // feat merges main: `--no-commit` leaves the merge pending
    // (`MERGE_HEAD` set) whether or not the two independent appends
    // actually conflict; the ledger is then pinned by hand to feat's row
    // first, main's row second.
    checkout(&dir, "feat");
    run_git(&dir, &["merge", "--no-ff", "--no-commit", &main_branch]);
    force_ledger_lines(&dir, &[&feat_row, &main_row]);
    force_banned_hashes(&dir, &[&feat_hash, &main_hash]);

    let staged1 = adr(&dir, &["check", "--staged"], &[], "");
    assert!(
        staged1.status.success(),
        "check --staged should pass mid the first merge: {}",
        stdout(&staged1)
    );
    finish_merge_commit(&dir);
    let committed1 = adr(&dir, &["check"], &[], "");
    assert!(
        committed1.status.success(),
        "check should pass after the first merge commit: {}",
        stdout(&committed1)
    );

    // main merges feat back: HEAD is main, whose own row now sits second
    // in the ledger being judged, feat's row still first -- the exact
    // shape the byte-prefix test refused.
    checkout(&dir, &main_branch);
    run_git(&dir, &["merge", "--no-ff", "--no-commit", "feat"]);
    force_ledger_lines(&dir, &[&feat_row, &main_row]);
    force_banned_hashes(&dir, &[&feat_hash, &main_hash]);

    let staged2 = adr(&dir, &["check", "--staged"], &[], "");
    assert!(
        staged2.status.success(),
        "check --staged should pass even though HEAD's own row now sits second, not first: {}",
        stdout(&staged2)
    );
    finish_merge_commit(&dir);
    let committed2 = adr(&dir, &["check"], &[], "");
    assert!(
        committed2.status.success(),
        "check should pass after the second merge commit too: {}",
        stdout(&committed2)
    );
}

// adr: c10c3a49-e993-4c9f-a46b-4a1ba3646dcf
//
// The reverse order from this file's own 1c27f2d6 "keep both" test: the
// merged-in branch's row kept first, the target branch's own row second.
// Still passes -- a row's place is free either way.
#[test]
fn b_merge_resolved_with_the_merged_branchs_rows_first_still_passes() {
    let dir = new_repo();
    setup_two_branches_each_appending_a_row(&dir);
    // Left on branch "a" by the helper above; its own row and branch
    // "b"'s row, read before the merge disturbs the working tree.
    let ours_row = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap()
        .trim()
        .to_string();
    let theirs_row = stdout(&git_ok(&dir, &["show", "b:adrdom/state/ledger.tsv"]))
        .trim()
        .to_string();

    run_git(&dir, &["merge", "--no-ff", "--no-commit", "b"]);
    force_ledger_lines(&dir, &[&theirs_row, &ours_row]);

    let staged = adr(&dir, &["check", "--staged"], &[], "");
    assert!(
        staged.status.success(),
        "check --staged should pass with the merged branch's row kept first: {}",
        stdout(&staged)
    );
    finish_merge_commit(&dir);
    let committed = adr(&dir, &["check"], &[], "");
    assert!(
        committed.status.success(),
        "check should pass after the commit too: {}",
        stdout(&committed)
    );
}

// adr: c10c3a49-e993-4c9f-a46b-4a1ba3646dcf
//
// The flip side of "the place of a row in the file is free": a HEAD row
// that is genuinely missing, or genuinely changed by one byte, still
// fails -- moving is free, dropping or editing is not.
#[test]
fn c_a_missing_or_changed_head_row_still_fails() {
    let dir = new_repo();
    append_ledger_row(
        &dir,
        "context",
        "-",
        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "some-detail",
    );
    commit_all(&dir, "commit a ledger row");

    let clean = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&clean).contains("past ledger rows stay byte for byte"),
        "unexpected ledger failure before tampering: {}",
        stdout(&clean)
    );

    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let committed = std::fs::read_to_string(&ledger_path).unwrap();

    // Remove the row entirely.
    std::fs::write(&ledger_path, "").unwrap();
    let removed = adr(&dir, &["check"], &[], "");
    assert!(!removed.status.success());
    assert!(stdout(&removed).contains("past ledger rows stay byte for byte"), "{}", stdout(&removed));

    // Restore, then change one byte in the middle of the row.
    let changed = committed.replace("some-detail", "some-detait");
    std::fs::write(&ledger_path, &changed).unwrap();
    let byte_changed = adr(&dir, &["check"], &[], "");
    assert!(!byte_changed.status.success());
    assert!(stdout(&byte_changed).contains("past ledger rows stay byte for byte"), "{}", stdout(&byte_changed));
}
