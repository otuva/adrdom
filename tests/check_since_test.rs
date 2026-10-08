// Decision C (a26876ba-5789-4d05-965e-853d00d9edd2): "does check in CI
// compare the ledger with the commit before the push?" -- `adr check
// --since <commit>` runs the whole check plus one extra comparison: every
// row of `<commit>`'s own ledger must be present in the ledger being
// judged.

mod common;

use common::*;
use std::path::Path;

fn commit_all(dir: &Path, msg: &str) {
    run_git(dir, &["add", "-A"]);
    let out = run_git(dir, &["commit", "-q", "-m", msg]);
    assert!(out.status.success(), "commit '{}' failed: {}", msg, stderr(&out));
}

fn head_sha(dir: &Path) -> String {
    stdout(&run_git(dir, &["rev-parse", "HEAD"])).trim().to_string()
}

/// `adr install`ed, with the hook wiring immediately switched off
/// (`core.hooksPath` unset) -- these tests drive `adr check --since`
/// directly, at exactly the ledger bytes each case wants to hand it, some
/// of which a real pre-commit hook would itself refuse to let land; the
/// same shape `tests/ledger_merge_test.rs`'s `new_repo` already uses.
fn new_repo() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    commit_all(&dir, "install");
    run_git(&dir, &["config", "--unset", "core.hooksPath"]);
    dir
}

const SINCE_DROPS_ROWS_TEXT: &str = "ledger drops a row";
const SINCE_UNREADABLE_TEXT: &str = "cannot be read here";

// adr: a26876ba-5789-4d05-965e-853d00d9edd2
#[test]
fn a_passes_when_rows_were_only_added_since_commit() {
    let dir = new_repo();
    let since = head_sha(&dir);

    let ban = adr(&dir, &["ban"], &[], "sincecheckwordalpha\n");
    assert!(ban.status.success(), "ban failed: {}", stderr(&ban));
    commit_all(&dir, "ban a word");

    let out = adr(&dir, &["check", "--since", &since], &[], "");
    assert!(
        out.status.success(),
        "check --since must pass when only rows were added: {}",
        stdout(&out)
    );
}

// adr: a26876ba-5789-4d05-965e-853d00d9edd2
#[test]
fn b_fails_when_a_row_of_commit_is_gone() {
    let dir = new_repo();

    let ban = adr(&dir, &["ban"], &[], "sincecheckwordbeta\n");
    assert!(ban.status.success(), "ban failed: {}", stderr(&ban));
    commit_all(&dir, "ban a word");
    let since = head_sha(&dir);

    // Drop the row the commit at `since` holds, without adding a new one.
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    std::fs::write(&ledger_path, "").unwrap();
    commit_all(&dir, "drop the ledger row");

    let out = adr(&dir, &["check", "--since", &since], &[], "");
    assert!(!out.status.success(), "check --since must fail when a row of <commit> is gone");
    let text = stdout(&out);
    assert!(text.contains(SINCE_DROPS_ROWS_TEXT), "{}", text);
    assert!(text.contains("no ledger row is lost between pushes; owner adds the missing rows back from that commit"), "{}", text);
}

// adr: a26876ba-5789-4d05-965e-853d00d9edd2
#[test]
fn c_fails_when_a_row_of_commit_changed() {
    let dir = new_repo();

    let ban = adr(&dir, &["ban"], &[], "sincecheckwordgamma\n");
    assert!(ban.status.success(), "ban failed: {}", stderr(&ban));
    commit_all(&dir, "ban a word");
    let since = head_sha(&dir);

    // Flip one hex digit inside the row's own hash field (never the fixed
    // "banned" event text, which a stray flip could turn into an unrelated
    // malformed row instead of the one-byte-changed row this case means to
    // pin down).
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let content = std::fs::read_to_string(&ledger_path).unwrap();
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let idx = lines
        .iter()
        .position(|l| l.contains("\tbanned\t"))
        .expect("a banned row must be present");
    let mut fields: Vec<String> = lines[idx].split('\t').map(|s| s.to_string()).collect();
    let mut hash_chars: Vec<char> = fields[3].chars().collect();
    hash_chars[0] = if hash_chars[0] == '0' { '1' } else { '0' };
    fields[3] = hash_chars.into_iter().collect();
    lines[idx] = fields.join("\t");
    let changed = lines.join("\n") + "\n";
    assert_ne!(content, changed, "the byte flip must actually change the file");
    std::fs::write(&ledger_path, &changed).unwrap();
    commit_all(&dir, "change one byte of the ledger row");

    let out = adr(&dir, &["check", "--since", &since], &[], "");
    assert!(!out.status.success(), "check --since must fail when a row of <commit> changed");
    let text = stdout(&out);
    assert!(text.contains(SINCE_DROPS_ROWS_TEXT), "{}", text);
    assert!(text.contains("no ledger row is lost between pushes; owner adds the missing rows back from that commit"), "{}", text);
}

// adr: a26876ba-5789-4d05-965e-853d00d9edd2
#[test]
fn d_an_all_zero_commit_passes() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    commit_all(&dir, "install");

    let zero = "0000000000000000000000000000000000000000";
    let out = adr(&dir, &["check", "--since", zero], &[], "");
    assert!(
        out.status.success(),
        "check --since must pass when <commit> is the all-zero sha: {}",
        stdout(&out)
    );
}

// adr: a26876ba-5789-4d05-965e-853d00d9edd2
#[test]
fn e_an_unknown_commit_fails_with_its_own_message() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    commit_all(&dir, "install");

    let bogus = "2222222222222222222222222222222222222d";
    let out = adr(&dir, &["check", "--since", bogus], &[], "");
    assert!(!out.status.success(), "check --since must fail for a commit this repo doesn't have");
    let text = stdout(&out);
    assert!(text.contains("check: "), "{}", text);
    assert!(text.contains(SINCE_UNREADABLE_TEXT), "{}", text);
    assert!(text.contains("; agent fetches that commit first"), "{}", text);
}
