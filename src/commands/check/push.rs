// Decision P (19e1c023-a678-4842-a414-bb0e45c41601): "does check look at
// the ledger before a push?" -- install writes a pre-push hook that runs
// `adr check --push`, and this judges the one thing that hook is for: no
// ref being pushed may drop a ledger row the remote already has. Nothing
// else `check` reports is judged here (the plain report's HEAD/merge/asked
// etc. categories already ran, and will run again, at the commit that made
// this push's local sha; a push is not itself a commit to judge those
// against).

use super::{stopped, Failure};
use crate::msg::{self, Actor};
use crate::out::{self, Line};
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
            out::eprint(&[stopped(msg::not_a_repo("check", &["--push"], Actor::Agent))]);
            return 1;
        }
    };

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        out::eprint(&[stopped(msg::stdin_read_failed(msg::after_fix_agent_runs(crate::moves::Cmd::fixed("git push"))))]);
        return 1;
    }

    let mut failures: Vec<Failure> = Vec::new();
    for line in input.lines() {
        failures.extend(check_one_ref(&root, line));
    }

    let failed = !failures.is_empty();
    let lines: Vec<Line> = failures.into_iter().map(|f| Line::Finding(f.finding())).collect();
    out::print(&lines);
    if !failed {
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
fn check_one_ref(root: &Path, line: &str) -> Vec<Failure> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 4 {
        return Vec::new();
    }
    let (local_ref, local_sha, remote_ref, remote_sha) = (parts[0], parts[1], parts[2], parts[3]);

    if repo::is_zero_sha(local_sha) {
        // The ref is being deleted on the remote: nothing of this push's
        // own to compare.
        return Vec::new();
    }
    if repo::is_zero_sha(remote_sha) {
        // The ref is new on the remote: nothing it already holds to drop.
        return Vec::new();
    }

    if !super::ledger::parent_object_present(root, remote_sha) {
        return vec![Failure {
            check: "ledger",
            message: msg::ledger_push_remote_unreadable(remote_ref),
        }];
    }

    // The ledger and the reader file are held the same way ("Does check hold
    // the reader's file as it holds the ledger?", 26c5352f-3f82-4376-9dc2-8ccc0074d080).
    let mut out = Vec::new();
    let files: [(&str, &'static str, String); 2] = [
        (repo::LEDGER_PATH, "ledger", msg::ledger_push_drops_rows(local_ref, remote_ref)),
        (repo::READER_PATH, "reader", msg::reader_push_drops_rows(local_ref, remote_ref)),
    ];
    for (path, check, message) in files {
        let Some(remote_content) = repo::git(root, &["show", &format!("{}:{}", remote_sha, path)]) else {
            // The remote commit holds no such file: nothing to compare.
            continue;
        };
        let local_content = repo::git(root, &["show", &format!("{}:{}", local_sha, path)]).unwrap_or_default();
        let present: std::collections::HashSet<&str> = local_content.lines().collect();
        if super::ledger::lines_missing(&remote_content, &present) {
            out.push(Failure { check, message });
        }
    }
    out
}
