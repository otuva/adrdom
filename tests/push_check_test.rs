// Decision P (19e1c023-a678-4842-a414-bb0e45c41601): "does check look at
// the ledger before a push?" -- install writes a pre-push hook that runs
// `adr check --push`, and check fails the push unless every ledger row of
// the remote branch is in the ledger being pushed.
//
// Every test here uses two local temp repositories: the one under test
// (where `adr install` runs and `git push` is actually run from -- a
// pre-push hook is client-side, so it only ever runs on the pushing side)
// and a second, bare, that stands in for "the remote", reached only by its
// own file path, never a network. `run_git`'s PATH-prepending (so a hook's
// bare `adr` finds this test's own build) is exactly what a push needs
// here too, so every git call below goes through it, never a raw
// `Command::new("git")`.

mod common;

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

/// A fresh bare repository at its own temp path, standing in for "the
/// remote" -- reached by `push`/`clone`/`git show` as a plain file path,
/// never a real network.
fn init_bare_remote() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "adr-test-bare-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let out = run_git(&dir, &["init", "-q", "--bare"]);
    assert!(out.status.success(), "bare init failed: {}", stderr(&out));
    dir
}

fn current_branch(dir: &Path) -> String {
    stdout(&git_ok(dir, &["rev-parse", "--abbrev-ref", "HEAD"])).trim().to_string()
}

fn head_sha(dir: &Path) -> String {
    stdout(&git_ok(dir, &["rev-parse", "HEAD"])).trim().to_string()
}

fn commit_all(dir: &Path, msg: &str) {
    run_git(dir, &["add", "-A"]);
    let out = run_git(dir, &["commit", "-q", "-m", msg]);
    assert!(out.status.success(), "commit '{}' failed: {}", msg, stderr(&out));
}

/// A repo with `adr install`ed, its own state committed, and a bare
/// "remote" wired up as `origin`, its default branch already pushed once
/// (so the remote holds the same base every case below diverges from).
/// Returns `(dir, remote_dir, branch_name)`.
fn new_repo_with_remote() -> (PathBuf, PathBuf, String) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    commit_all(&dir, "install");
    let branch = current_branch(&dir);

    let remote = init_bare_remote();
    git_ok(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
    let push = git_ok(&dir, &["push", "-u", "origin", &branch]);
    assert!(push.status.success(), "initial push failed: {}", stderr(&push));

    (dir, remote, branch)
}

const DROPS_ROWS_TEXT: &str = "would drop ledger rows the remote's";
const REMOTE_UNREADABLE_TEXT: &str = "remote commit cannot be read here";

// --- a) hook existence, executable bit, check, install restores it --------

// adr: 19e1c023-a678-4842-a414-bb0e45c41601
#[test]
fn a_install_writes_an_executable_pre_push_hook_check_holds_and_install_restores_it() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let hook_path = dir.join(".githooks/pre-push");
    assert!(hook_path.is_file(), "pre-push hook must exist after install");
    assert!(
        adr::repo::hook_is_executable(&hook_path),
        "pre-push hook must be executable after install"
    );

    let clean = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&clean).contains(".githooks/pre-push"),
        "check must report no install failure for a freshly installed pre-push hook: {}",
        stdout(&clean)
    );

    // Removed outright: check refuses, naming the hook missing.
    std::fs::remove_file(&hook_path).unwrap();
    let removed = adr(&dir, &["check"], &[], "");
    assert!(!removed.status.success());
    assert!(
        stdout(&removed).contains(".githooks/pre-push is missing"),
        "{}",
        stdout(&removed)
    );
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert!(hook_path.is_file(), "install must restore the removed hook");
    assert!(adr(&dir, &["check"], &[], "").status.success());

    // Edited: check refuses, naming the hook stale.
    std::fs::write(&hook_path, "#!/bin/sh\necho edited\n").unwrap();
    let edited = adr(&dir, &["check"], &[], "");
    assert!(!edited.status.success());
    assert!(
        stdout(&edited).contains(".githooks/pre-push differs from what install writes"),
        "{}",
        stdout(&edited)
    );
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert_eq!(
        std::fs::read_to_string(&hook_path).unwrap(),
        "#!/bin/sh\nadr check --push\nexit $?\n",
        "install must rewrite an edited pre-push hook back to exactly its content"
    );
    assert!(adr(&dir, &["check"], &[], "").status.success());
}

// --- b) a normal push of new commits that only add ledger rows passes -----

// adr: 19e1c023-a678-4842-a414-bb0e45c41601
#[test]
fn b_a_push_that_only_adds_ledger_rows_passes() {
    let (dir, _remote, branch) = new_repo_with_remote();

    let ban = adr(&dir, &["ban"], &[], "pushcheckwordalpha\n");
    assert!(ban.status.success(), "ban failed: {}", stderr(&ban));
    commit_all(&dir, "ban a word");

    let push = run_git(&dir, &["push", "origin", &branch]);
    assert!(
        push.status.success(),
        "a push that only adds ledger rows must pass the pre-push hook: stdout={} stderr={}",
        stdout(&push),
        stderr(&push)
    );
}

// --- c) a force push that drops a ledger row the remote holds is refused --

// adr: 19e1c023-a678-4842-a414-bb0e45c41601
#[test]
fn c_a_force_push_that_drops_a_remote_ledger_row_is_refused() {
    let (dir, _remote, branch) = new_repo_with_remote();

    let ban = adr(&dir, &["ban"], &[], "pushcheckwordbeta\n");
    assert!(ban.status.success(), "ban failed: {}", stderr(&ban));
    commit_all(&dir, "ban a word");
    let push = git_ok(&dir, &["push", "origin", &branch]);
    assert!(push.status.success(), "{}", stderr(&push));

    // Rewrite local history so the row the remote now holds is gone: reset
    // to the parent of the commit that added it, then make a new, different
    // commit -- a real rewrite, not a fast-forward.
    git_ok(&dir, &["reset", "--hard", "HEAD~1"]);
    std::fs::write(dir.join("unrelated.txt"), "unrelated change\n").unwrap();
    commit_all(&dir, "an unrelated commit, replacing the one that banned a word");

    let forced = run_git(&dir, &["push", "--force", "origin", &branch]);
    assert!(
        !forced.status.success(),
        "a force push that drops a remote ledger row must be refused by the pre-push hook"
    );
    let text = format!("{}{}", stdout(&forced), stderr(&forced));
    assert!(text.contains("ledger:"), "{}", text);
    assert!(text.contains(DROPS_ROWS_TEXT), "{}", text);
}

// --- d) a hand-fed line whose remote sha this repo cannot read fails ------

// adr: 19e1c023-a678-4842-a414-bb0e45c41601
#[test]
fn d_a_remote_sha_not_present_locally_fails_check_push() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    commit_all(&dir, "install");
    let local_sha = head_sha(&dir);

    let bogus_remote_sha = "1111111111111111111111111111111111111d";
    let line = format!(
        "refs/heads/main {} refs/heads/main {}\n",
        local_sha, bogus_remote_sha
    );

    let out = adr(&dir, &["check", "--push"], &[], &line);
    assert!(
        !out.status.success(),
        "a remote sha this repo cannot read must fail check --push"
    );
    let text = stdout(&out);
    assert!(text.contains("ledger:"), "{}", text);
    assert!(text.contains(REMOTE_UNREADABLE_TEXT), "{}", text);
}

// --- f) --push refuses alongside the flags `check --push` ignores ---------
//
// `check --push` short-circuits straight into `push::run`, before `staged`/
// `message`/`since` are ever read (see `commands::check::run`): combined
// with any of them, the other flag was silently ignored rather than
// applied. Refused up front by clap instead, the same way `ban --term`
// conflicts with `ban --rebuild`.

// adr: 19e1c023-a678-4842-a414-bb0e45c41601
#[test]
fn f_push_conflicts_with_staged() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    commit_all(&dir, "install");

    let out = adr(&dir, &["check", "--push", "--staged"], &[], "");
    assert!(!out.status.success(), "--push and --staged together must be refused");
    assert!(
        stderr(&out).contains("cannot be used with"),
        "{}",
        stderr(&out)
    );
}

// adr: 19e1c023-a678-4842-a414-bb0e45c41601
#[test]
fn f_push_conflicts_with_message() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    commit_all(&dir, "install");
    let msg_path = dir.join("msg.txt");
    std::fs::write(&msg_path, "a commit message\n").unwrap();

    let out = adr(
        &dir,
        &["check", "--push", "--message", msg_path.to_str().unwrap()],
        &[],
        "",
    );
    assert!(!out.status.success(), "--push and --message together must be refused");
    assert!(
        stderr(&out).contains("cannot be used with"),
        "{}",
        stderr(&out)
    );
}

// adr: 19e1c023-a678-4842-a414-bb0e45c41601
#[test]
fn f_push_conflicts_with_since() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    commit_all(&dir, "install");
    let since = head_sha(&dir);

    let out = adr(&dir, &["check", "--push", "--since", &since], &[], "");
    assert!(!out.status.success(), "--push and --since together must be refused");
    assert!(
        stderr(&out).contains("cannot be used with"),
        "{}",
        stderr(&out)
    );
}

// --- e) an all-zero remote sha and an all-zero local sha both pass --------

// adr: 19e1c023-a678-4842-a414-bb0e45c41601
#[test]
fn e_an_all_zero_remote_sha_and_an_all_zero_local_sha_both_pass() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    commit_all(&dir, "install");
    let local_sha = head_sha(&dir);
    let zero = "0000000000000000000000000000000000000000";

    // A new branch on the remote: nothing to compare.
    let new_branch_line = format!("refs/heads/main {} refs/heads/main {}\n", local_sha, zero);
    let out_new = adr(&dir, &["check", "--push"], &[], &new_branch_line);
    assert!(
        out_new.status.success(),
        "an all-zero remote sha must pass check --push: {}",
        stdout(&out_new)
    );

    // A deleted branch: nothing of this push's own to compare either.
    let deleted_branch_line = format!("refs/heads/gone {} refs/heads/gone {}\n", zero, local_sha);
    let out_deleted = adr(&dir, &["check", "--push"], &[], &deleted_branch_line);
    assert!(
        out_deleted.status.success(),
        "an all-zero local sha must pass check --push: {}",
        stdout(&out_deleted)
    );
}
