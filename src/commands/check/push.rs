// Decision P (19e1c023-a678-4842-a414-bb0e45c41601): "does check look at
// the ledger before a push?" -- install writes a pre-push hook that runs
// `adr check --push`, and this judges the one thing that hook is for: no
// ref being pushed may drop a ledger row the remote already has. Nothing
// else `check` reports is judged here (the plain report's HEAD/merge/asked
// etc. categories already ran, and will run again, at the commit that made
// this push's local sha; a push is not itself a commit to judge those
// against).

use super::Failure;
use crate::msg::{self, Actor, Next};
use crate::repo;
use std::io::Read;
use std::path::Path;

/// `adr check --push`: reads git's own pre-push hook input from stdin,
/// lines of `<local ref> <local sha> <remote ref> <remote sha>` (one per
/// ref the push touches), and judges each independently. Prints one
/// `ledger: ...` line per refused ref and exits non-zero if any ref was
/// refused, the same shape the plain report's own `failures` print in.
pub fn run() -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("check", Actor::Agent));
            return 1;
        }
    };

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!(
            "check: {}",
            msg::stdin_read_failed(Next::agent("checks stdin and retries"))
        );
        return 1;
    }

    let mut failures: Vec<Failure> = Vec::new();
    for line in input.lines() {
        if let Some(f) = check_one_ref(&root, line) {
            failures.push(f);
        }
    }

    for f in &failures {
        println!("{}: {}", f.check, f.message);
    }
    if failures.is_empty() {
        0
    } else {
        1
    }
}

/// One pre-push line, `<local ref> <local sha> <remote ref> <remote sha>`:
/// `None` when there is nothing to refuse (a malformed line -- git never
/// writes one, so this only guards against a hand-fed line in a test --
/// a deleted branch, a new branch, a remote whose commit holds no ledger
/// file, or a clean comparison). A remote sha this repository cannot read
/// at all refuses on its own message; otherwise the same row-presence test
/// `check_ledger`'s HEAD/merge/`since` comparisons already share
/// (`ledger::ref_ledger_has_rows_missing`'s sibling, `lines_missing`,
/// against content this function reads itself).
fn check_one_ref(root: &Path, line: &str) -> Option<Failure> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 4 {
        return None;
    }
    let (local_ref, local_sha, remote_ref, remote_sha) = (parts[0], parts[1], parts[2], parts[3]);

    if repo::is_zero_sha(local_sha) {
        // The ref is being deleted on the remote: nothing of this push's
        // own to compare.
        return None;
    }
    if repo::is_zero_sha(remote_sha) {
        // The ref is new on the remote: nothing it already holds to drop.
        return None;
    }

    if !super::ledger::parent_object_present(root, remote_sha) {
        return Some(Failure {
            check: "ledger".to_string(),
            message: msg::ledger_push_remote_unreadable(remote_ref),
        });
    }

    let Some(remote_content) = repo::git(root, &["show", &format!("{}:{}", remote_sha, repo::LEDGER_PATH)]) else {
        // The remote commit holds no ledger file at that path: nothing to
        // compare.
        return None;
    };
    let local_content = repo::git(root, &["show", &format!("{}:{}", local_sha, repo::LEDGER_PATH)]).unwrap_or_default();
    let present: std::collections::HashSet<&str> = local_content.lines().collect();

    if super::ledger::lines_missing(&remote_content, &present) {
        Some(Failure {
            check: "ledger".to_string(),
            message: msg::ledger_push_drops_rows(local_ref, remote_ref),
        })
    } else {
        None
    }
}
