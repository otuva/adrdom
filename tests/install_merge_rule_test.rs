// Decision 7 (db56a285-946f-48b1-b073-bb1e90192185): install writes a git
// attributes file under adrdom that keeps the ledger rows of both branches
// at a merge; the ban list gets no such rule.
// Decision 8 (76c6adc6-c018-4df4-b49a-ddcd0a03a48c): check holds install to
// that same file.
//
// The merge-lab harness shapes here (running real branches/merges inside a
// throwaway temp repo, resolving conflicts by keeping both sides) mirror
// merge_lab_ban_test.rs/merge_lab_questions_test.rs and ban_sort_test.rs
// (read, not edited).

mod common;

use common::*;
use std::path::Path;
use std::process::Output;

const GITATTRIBUTES_PATH: &str = "adrdom/.gitattributes";
const GITATTRIBUTES_CONTENT: &str = "state/ledger.tsv merge=union\nstate/reader.tsv merge=union\n";

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

fn conflicted_files(dir: &Path) -> Vec<String> {
    stdout(&run_git(dir, &["diff", "--name-only", "--diff-filter=U"]))
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn banned_lines(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("adrdom/state/banned.sha256"))
        .unwrap_or_default()
        .lines()
        .map(|s| s.to_string())
        .collect()
}

/// Strips conflict markers, keeping the "ours" block then the "theirs"
/// block for every conflicted hunk (the same de-marking policy
/// merge_lab_ban_test.rs's `resolve_text_ours_then_theirs` and
/// ban_sort_test.rs's `resolve_ours_then_theirs` use), then sorts the
/// result: `banned.sha256` is always a whole-file sort by hash (per "Does
/// the tool write the ban list in one fixed order?"), so a hand resolution
/// of a real conflict on it has to restore that order itself, the way a
/// real `adr ban`/`adr unban` call would.
fn resolve_banned_conflict_keep_both_sorted(dir: &Path) {
    let path = dir.join("adrdom/state/banned.sha256");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    if !content.contains("<<<<<<<") {
        return;
    }
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
    let mut lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    lines.sort();
    lines.dedup();
    let sorted = lines.join("\n") + if lines.is_empty() { "" } else { "\n" };
    std::fs::write(&path, sorted).unwrap();
    run_git(dir, &["add", "adrdom/state/banned.sha256"]);
}

// --- Decision 7: install writes the merge rule ---------------------------

// adr: db56a285-946f-48b1-b073-bb1e90192185
#[test]
fn install_writes_the_gitattributes_file_with_exactly_the_ledger_rule() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let content = std::fs::read_to_string(dir.join(GITATTRIBUTES_PATH)).unwrap();
    assert_eq!(
        content, GITATTRIBUTES_CONTENT,
        "adrdom/.gitattributes must hold exactly the ledger merge rule"
    );
    assert!(
        !content.contains("banned.sha256"),
        "adrdom/.gitattributes must not name the ban list: {}",
        content
    );
}

// adr: db56a285-946f-48b1-b073-bb1e90192185
// Two branches each ban a different nonsense word (a real `adr ban` call
// each, so each appends its own `banned` ledger row); the base commit
// already carries `adrdom/.gitattributes`. Merging one branch into the
// other must report no conflict on adrdom/state/ledger.tsv, and the merged
// ledger must hold both branches' rows.
#[test]
fn two_branches_each_banning_a_word_merge_the_ledger_with_no_conflict() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // The pre-commit/commit-msg hooks are wired by `install`, but nothing
    // in this test bypasses them: every commit below is either a plain
    // `adr ban` (which only writes files) or a `git commit` on a tree that
    // still passes `adr check --staged` (a real, run-of-the-mill ban).
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "base: fresh install"]);
    let base_branch = current_branch(&dir);

    git_ok(&dir, &["checkout", "-b", "branch-a"]);
    let a_ban = adr(&dir, &["ban"], &[], "mergerulewordalpha\n");
    assert!(a_ban.status.success(), "branch a ban failed: {}", stderr(&a_ban));
    run_git(&dir, &["add", "-A"]);
    let a_commit = run_git(&dir, &["commit", "-q", "-m", "a: ban a word"]);
    assert!(a_commit.status.success(), "branch a commit failed: {}", stderr(&a_commit));

    git_ok(&dir, &["checkout", &base_branch]);
    git_ok(&dir, &["checkout", "-b", "branch-b"]);
    let b_ban = adr(&dir, &["ban"], &[], "mergerulewordbeta\n");
    assert!(b_ban.status.success(), "branch b ban failed: {}", stderr(&b_ban));
    run_git(&dir, &["add", "-A"]);
    let b_commit = run_git(&dir, &["commit", "-q", "-m", "b: ban a different word"]);
    assert!(b_commit.status.success(), "branch b commit failed: {}", stderr(&b_commit));

    git_ok(&dir, &["checkout", "branch-a"]);
    let merge_out = run_git(&dir, &["merge", "--no-ff", "--no-edit", "branch-b"]);
    let conflicted = conflicted_files(&dir);

    assert!(
        !conflicted.iter().any(|f| f == "adrdom/state/ledger.tsv"),
        "adrdom/state/ledger.tsv must merge with no conflict: conflicted={:?} merge stderr={}",
        conflicted,
        stderr(&merge_out)
    );

    if !merge_out.status.success() {
        // Only banned.sha256 (no merge rule of its own) is allowed to
        // conflict here; resolve it and finish the merge commit.
        assert_eq!(
            conflicted,
            vec!["adrdom/state/banned.sha256".to_string()],
            "only banned.sha256 may conflict; unexpected conflicted files: {:?}",
            conflicted
        );
        resolve_banned_conflict_keep_both_sorted(&dir);
        let commit_out = run_git(&dir, &["commit", "--no-edit"]);
        assert!(
            commit_out.status.success(),
            "merge commit after resolving banned.sha256 failed: {}",
            stderr(&commit_out)
        );
    }

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let banned_rows: Vec<&str> = ledger.lines().filter(|l| l.contains("\tbanned\t")).collect();
    assert_eq!(
        banned_rows.len(),
        2,
        "merged ledger must hold both branches' banned rows: {:?}",
        ledger
    );

    let words = banned_lines(&dir);
    assert_eq!(words.len(), 2, "merged banned.sha256 must hold both words: {:?}", words);
}

// adr: db56a285-946f-48b1-b073-bb1e90192185
#[test]
fn a_second_install_leaves_the_gitattributes_file_unchanged() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let first = std::fs::read(dir.join(GITATTRIBUTES_PATH)).unwrap();
    let out2 = adr(&dir, &["install"], &[], "");
    assert!(out2.status.success(), "second install failed: {}", stderr(&out2));
    let second = std::fs::read(dir.join(GITATTRIBUTES_PATH)).unwrap();

    assert_eq!(
        first, second,
        "a second install must leave an already-current adrdom/.gitattributes byte-identical"
    );
    assert!(
        !stdout(&out2).contains("adrdom/.gitattributes"),
        "a second install writes no line for a file it did not write: {}",
        stdout(&out2)
    );
}

// --- Decision 8: check holds install to the merge rule --------------------

// adr: 76c6adc6-c018-4df4-b49a-ddcd0a03a48c
#[test]
fn check_has_no_install_failure_for_the_gitattributes_file_after_install() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let out = adr(&dir, &["check"], &[], "");
    let output = stdout(&out);
    assert!(
        !output.contains("adrdom/.gitattributes"),
        "check must report no install failure for a freshly installed adrdom/.gitattributes: {}",
        output
    );
}

// adr: 76c6adc6-c018-4df4-b49a-ddcd0a03a48c
#[test]
fn check_fails_when_the_gitattributes_file_is_missing() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    std::fs::remove_file(dir.join(GITATTRIBUTES_PATH)).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("install: adrdom/.gitattributes is missing; agent runs: adr install"),
        "unexpected check output: {}",
        stdout(&out)
    );

    // Clears after running install again.
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert!(adr(&dir, &["check"], &[], "").status.success());
}

// adr: 76c6adc6-c018-4df4-b49a-ddcd0a03a48c
#[test]
fn check_fails_when_the_gitattributes_file_holds_an_extra_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    std::fs::write(
        dir.join(GITATTRIBUTES_PATH),
        format!("{}state/other.tsv merge=union\n", GITATTRIBUTES_CONTENT),
    )
    .unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains(
            "install: adrdom/.gitattributes differs from what install writes; agent runs: adr install"
        ),
        "unexpected check output: {}",
        stdout(&out)
    );

    // Clears after running install again.
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert!(adr(&dir, &["check"], &[], "").status.success());
    assert_eq!(
        std::fs::read_to_string(dir.join(GITATTRIBUTES_PATH)).unwrap(),
        GITATTRIBUTES_CONTENT,
        "install must rewrite a stale adrdom/.gitattributes back to exactly its content"
    );
}
