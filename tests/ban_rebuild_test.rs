// Decision f422c5db-13d0-4ff7-9e43-b68ab44a7cb9: "Does the tool rewrite the
// ban list from the ledger?" Answer: yes, `adr ban --rebuild` writes
// `adrdom/state/banned.sha256` as exactly the hashes the ledger currently
// says are banned, sorted, and writes no ledger row.
//
// The real-branch/real-merge helper shapes here (checkout, commit, resolve
// a conflict, finish the merge commit) mirror ban_sort_test.rs's own (c)
// case and tests/install_merge_rule_test.rs (both read, not edited).

mod common;

use adr::record;
use common::*;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

fn banned_path(dir: &Path) -> PathBuf {
    dir.join("adrdom/state/banned.sha256")
}

fn ledger_path(dir: &Path) -> PathBuf {
    dir.join("adrdom/state/ledger.tsv")
}

fn banned_lines(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(banned_path(dir))
        .unwrap_or_default()
        .lines()
        .map(|s| s.to_string())
        .collect()
}

fn assert_sorted(lines: &[String]) {
    let mut sorted = lines.to_vec();
    sorted.sort();
    assert_eq!(lines, sorted.as_slice(), "banned.sha256 is not sorted by hash: {:?}", lines);
}

fn checkout_new_branch(dir: &Path, name: &str) {
    let out = run_git(dir, &["checkout", "-b", name]);
    assert!(out.status.success(), "checkout -b {} failed: {}", name, stderr(&out));
}

fn checkout(dir: &Path, name: &str) {
    let out = run_git(dir, &["checkout", name]);
    assert!(out.status.success(), "checkout {} failed: {}", name, stderr(&out));
}

fn current_branch(dir: &Path) -> String {
    stdout(&run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])).trim().to_string()
}

fn commit_all(dir: &Path, msg: &str) -> Output {
    run_git(dir, &["add", "-A"]);
    run_git(dir, &["commit", "-q", "-m", msg])
}

fn conflicted_files(dir: &Path) -> Vec<String> {
    stdout(&run_git(dir, &["diff", "--name-only", "--diff-filter=U"]))
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Strips conflict markers, keeping the "ours" block then the "theirs"
/// block for every conflicted hunk -- the same de-marking policy
/// ban_sort_test.rs's `resolve_ours_then_theirs` and
/// install_merge_rule_test.rs's `resolve_banned_conflict_keep_both_sorted`
/// use, copied here (integration tests each compile as their own crate)
/// for the one file this shape can still show up on even after `adr ban
/// --rebuild` fixes banned.sha256: `adrdom/state/ledger.tsv`, which carries
/// `merge=union` and so should auto-resolve on its own, but is de-marked
/// defensively the same way the sibling files already do.
fn resolve_ours_then_theirs(content: &str) -> String {
    let mut out = String::new();
    let mut in_ours = false;
    let mut in_theirs = false;
    let mut ours_buf: Vec<&str> = Vec::new();
    let mut theirs_buf: Vec<&str> = Vec::new();
    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_ours = true;
            in_theirs = false;
            ours_buf.clear();
            theirs_buf.clear();
            continue;
        }
        if line.starts_with("=======") && in_ours {
            in_ours = false;
            in_theirs = true;
            continue;
        }
        if line.starts_with(">>>>>>>") && in_theirs {
            in_theirs = false;
            for l in &ours_buf {
                out.push_str(l);
                out.push('\n');
            }
            for l in &theirs_buf {
                out.push_str(l);
                out.push('\n');
            }
            continue;
        }
        if in_ours {
            ours_buf.push(line);
            continue;
        }
        if in_theirs {
            theirs_buf.push(line);
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Finishes a merge left pending by `git merge --no-ff --no-commit
/// <other>`: rewrites `banned.sha256` from the ledger alone (`adr ban
/// --rebuild`, ignoring whatever the merge left there, conflict markers or
/// not) and stages it, resolves `adrdom/state/ledger.tsv` the same
/// "ours-then-theirs" way if it somehow still carries conflict markers
/// (its own `merge=union` attribute should already have kept that from
/// happening), then completes the merge commit.
fn finish_merge_with_rebuild(dir: &Path) {
    let rebuild_out = adr(dir, &["ban", "--rebuild"], &[], "");
    assert!(rebuild_out.status.success(), "ban --rebuild failed: {}", stderr(&rebuild_out));
    run_git(dir, &["add", "adrdom/state/banned.sha256"]);

    let ledger_content = std::fs::read_to_string(ledger_path(dir)).unwrap_or_default();
    if ledger_content.contains("<<<<<<<") {
        let resolved = resolve_ours_then_theirs(&ledger_content);
        std::fs::write(ledger_path(dir), resolved).unwrap();
        run_git(dir, &["add", "adrdom/state/ledger.tsv"]);
    }

    let commit_out = run_git(dir, &["commit", "--no-edit"]);
    assert!(commit_out.status.success(), "merge commit failed: {}", stderr(&commit_out));
}

// (a): a garbage file, conflict markers included, is fully replaced.
#[test]
fn a_rebuild_recovers_from_a_garbage_file_with_conflict_markers() {
    // adr: f422c5db-13d0-4ff7-9e43-b68ab44a7cb9
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let ban_out = adr(&dir, &["ban"], &[], "rebuildwordalpha\nrebuildwordbeta\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));
    let unban_out = adr_owner(&dir, &["unban"], "rebuildwordbeta\n");
    assert!(unban_out.status.success(), "unban failed: {}", stderr(&unban_out));

    let expected_hash = record::sha256_hex("rebuildwordalpha");
    let ledger_before = std::fs::read_to_string(ledger_path(&dir)).unwrap();

    std::fs::write(
        banned_path(&dir),
        "<<<<<<< HEAD\nnotarealhash\n=======\nalsonotreal\n>>>>>>> branch\n",
    )
    .unwrap();

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(
        !check_out.status.success(),
        "check should fail on a garbage ban list: {}",
        stdout(&check_out)
    );

    let rebuild_out = adr(&dir, &["ban", "--rebuild"], &[], "");
    assert!(rebuild_out.status.success(), "ban --rebuild failed: {}", stderr(&rebuild_out));

    let lines = banned_lines(&dir);
    assert_eq!(lines, vec![expected_hash], "{:?}", lines);

    let check_after = adr(&dir, &["check"], &[], "");
    let text = stdout(&check_after);
    assert!(!text.contains("banned:"), "unexpected banned failure after rebuild: {}", text);

    let ledger_after = std::fs::read_to_string(ledger_path(&dir)).unwrap();
    assert_eq!(ledger_before, ledger_after, "rebuild must write no ledger row");
}

// (b): a term uuid carried by a ban survives a rebuild.
#[test]
fn b_rebuild_keeps_a_bans_term_identifier() {
    // adr: f422c5db-13d0-4ff7-9e43-b68ab44a7cb9
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let term_id = "55555555-5555-5555-5555-555555555555";
    let terms_content = format!(
        "# Terms\n\n## widget\nid: {}\ndefinition: A small thing.\naliases: \n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();

    let ban_out = adr(&dir, &["ban", "--term", "widget"], &[], "rebuildtermword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));

    let rebuild_out = adr(&dir, &["ban", "--rebuild"], &[], "");
    assert!(rebuild_out.status.success(), "ban --rebuild failed: {}", stderr(&rebuild_out));

    let lines = banned_lines(&dir);
    assert_eq!(lines.len(), 1, "{:?}", lines);
    assert!(
        lines[0].ends_with(&format!(" {}", term_id)),
        "expected the term uuid to survive a rebuild: {:?}",
        lines
    );
}

// (c): two branches each ban a different word; banned.sha256 starts empty
// (the shortest possible list, right after a fresh install), so the merge
// really conflicts on it; `adr ban --rebuild` clears the conflict and the
// merge commit and `check` both pass.
#[test]
fn c_a_real_merge_conflict_on_banned_sha256_is_cleared_by_rebuild() {
    // adr: f422c5db-13d0-4ff7-9e43-b68ab44a7cb9
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let install_commit = commit_all(&dir, "install");
    assert!(install_commit.status.success(), "install commit failed: {}", stderr(&install_commit));
    let base_branch = current_branch(&dir);

    let word1_hash = record::sha256_hex("mergerebuildworda");
    let word2_hash = record::sha256_hex("mergerebuildwordb");

    checkout_new_branch(&dir, "a");
    let a_ban = adr(&dir, &["ban"], &[], "mergerebuildworda\n");
    assert!(a_ban.status.success(), "a: ban failed: {}", stderr(&a_ban));
    let a_commit = commit_all(&dir, "a: ban word1");
    assert!(a_commit.status.success(), "a commit failed: {}", stderr(&a_commit));

    checkout(&dir, &base_branch);
    checkout_new_branch(&dir, "b");
    let b_ban = adr(&dir, &["ban"], &[], "mergerebuildwordb\n");
    assert!(b_ban.status.success(), "b: ban failed: {}", stderr(&b_ban));
    let b_commit = commit_all(&dir, "b: ban word2");
    assert!(b_commit.status.success(), "b commit failed: {}", stderr(&b_commit));

    checkout(&dir, "a");
    run_git(&dir, &["merge", "--no-ff", "--no-commit", "b"]);
    let conflicted = conflicted_files(&dir);
    assert!(
        conflicted.iter().any(|f| f == "adrdom/state/banned.sha256"),
        "expected banned.sha256 to conflict on a short list: conflicted={:?}",
        conflicted
    );

    finish_merge_with_rebuild(&dir);

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(check_out.status.success(), "check failed after merge: {}", stdout(&check_out));

    let lines = banned_lines(&dir);
    assert_eq!(lines.len(), 2, "{:?}", lines);
    assert!(lines.iter().any(|l| l.starts_with(&word1_hash)), "{:?}", lines);
    assert!(lines.iter().any(|l| l.starts_with(&word2_hash)), "{:?}", lines);
    assert_sorted(&lines);
}

/// A base repo with `mergeunbanword1` already banned in the base commit,
/// branch "a" unbanning it (an owner command, through `adr_owner`, per
/// tests/unban_test.rs) and branch "b" banning a second, unrelated word --
/// branch "b"'s command run more than two seconds after branch "a"'s, so
/// their real wall-clock ledger times differ rather than land in the same
/// second. Returns `(dir, base_branch, word2_hash)`, left on branch "b".
fn unban_then_ban_setup(word2: &str) -> (PathBuf, String, String) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let base_ban = adr(&dir, &["ban"], &[], "mergeunbanword1\n");
    assert!(base_ban.status.success(), "base: ban failed: {}", stderr(&base_ban));
    let base_commit = commit_all(&dir, "base: ban word1");
    assert!(base_commit.status.success(), "base commit failed: {}", stderr(&base_commit));
    let base_branch = current_branch(&dir);

    checkout_new_branch(&dir, "a");
    let a_unban = adr_owner(&dir, &["unban"], "mergeunbanword1\n");
    assert!(a_unban.status.success(), "a: unban failed: {}", stderr(&a_unban));
    let a_commit = commit_all(&dir, "a: unban word1");
    assert!(a_commit.status.success(), "a commit failed: {}", stderr(&a_commit));

    // More than two seconds between the two branches' commands, so the
    // ledger rows they each write carry real, distinct wall-clock times.
    std::thread::sleep(Duration::from_secs(3));

    checkout(&dir, &base_branch);
    checkout_new_branch(&dir, "b");
    let b_ban = adr(&dir, &["ban"], &[], &format!("{}\n", word2));
    assert!(b_ban.status.success(), "b: ban failed: {}", stderr(&b_ban));
    let b_commit = commit_all(&dir, "b: ban word2");
    assert!(b_commit.status.success(), "b commit failed: {}", stderr(&b_commit));

    let word2_hash = record::sha256_hex(word2);
    (dir, base_branch, word2_hash)
}

/// One direction of case (d): checks out `head_branch`, merges
/// `other_branch` in, finishes with `adr ban --rebuild`, and asserts the
/// rebuilt file holds only `word2_hash` and `check` passes.
fn assert_merge_leaves_only_word2(dir: &Path, head_branch: &str, other_branch: &str, word2_hash: &str) {
    checkout(dir, head_branch);
    run_git(dir, &["merge", "--no-ff", "--no-commit", other_branch]);
    finish_merge_with_rebuild(dir);

    let check_out = adr(dir, &["check"], &[], "");
    assert!(
        check_out.status.success(),
        "check failed after merging {} into {}: {}",
        other_branch,
        head_branch,
        stdout(&check_out)
    );

    let lines = banned_lines(dir);
    assert_eq!(
        lines,
        vec![word2_hash.to_string()],
        "expected only word2's hash after merging {} into {}: {:?}",
        other_branch,
        head_branch,
        lines
    );
}

// (d): the reverse shape from (c) -- one branch unbans an already-banned
// word, the other bans an unrelated one -- checked in both merge
// directions, each in its own fresh repo.
#[test]
fn d_an_unban_and_a_ban_merge_correctly_in_either_direction() {
    // adr: f422c5db-13d0-4ff7-9e43-b68ab44a7cb9
    let (dir_ab, base_ab, word2_hash_ab) = unban_then_ban_setup("mergeunbanword2ab");
    assert_merge_leaves_only_word2(&dir_ab, "a", "b", &word2_hash_ab);
    // `base_ab` is unused past setup beyond documenting where "a"/"b" forked.
    let _ = base_ab;

    let (dir_ba, base_ba, word2_hash_ba) = unban_then_ban_setup("mergeunbanword2ba");
    assert_merge_leaves_only_word2(&dir_ba, "b", "a", &word2_hash_ba);
    let _ = base_ba;
}

// (e): a missing file is written outright.
#[test]
fn e_rebuild_writes_the_file_when_it_is_missing() {
    // adr: f422c5db-13d0-4ff7-9e43-b68ab44a7cb9
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let ban_out = adr(&dir, &["ban"], &[], "rebuildmissingword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));
    let expected_hash = record::sha256_hex("rebuildmissingword");

    std::fs::remove_file(banned_path(&dir)).unwrap();
    assert!(!banned_path(&dir).exists());

    let rebuild_out = adr(&dir, &["ban", "--rebuild"], &[], "");
    assert!(rebuild_out.status.success(), "ban --rebuild failed: {}", stderr(&rebuild_out));

    let lines = banned_lines(&dir);
    assert_eq!(lines, vec![expected_hash], "{:?}", lines);
}
