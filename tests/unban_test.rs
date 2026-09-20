mod common;

use common::*;

#[test]
fn unban_removes_hash_and_writes_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());

    let ban_out = adr(&dir, &["ban"], &[], "forbiddenword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));

    let banned_before =
        std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert!(!banned_before.trim().is_empty());

    let out = adr_owner(&dir, &["unban"], "forbiddenword\n");
    assert!(out.status.success(), "unban failed: {}", stderr(&out));
    assert!(stdout(&out).contains("unban: removed"));

    let banned_after =
        std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert!(
        banned_after.trim().is_empty(),
        "expected hash removed: {}",
        banned_after
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger.lines().any(|l| l.contains("\tunbanned\t")),
        "ledger missing unbanned row: {}",
        ledger
    );
}

#[test]
fn unban_reports_nothing_to_remove() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());

    let out = adr_owner(&dir, &["unban"], "neverbanned\n");
    assert!(out.status.success(), "unban failed: {}", stderr(&out));
    assert!(stdout(&out).contains("unban: nothing to remove"));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.lines().any(|l| l.contains("\tunbanned\t")));
}

#[test]
fn unban_refuses_when_claudecode_is_set() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());

    let ban_out = adr(&dir, &["ban"], &[], "forbiddenword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));

    let out = adr(
        &dir,
        &["unban"],
        &[("CLAUDECODE", "1")],
        "forbiddenword\n",
    );
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));

    let banned_after =
        std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert!(
        !banned_after.trim().is_empty(),
        "unban must not have modified the banned list without owner authorization: {}",
        banned_after
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.lines().any(|l| l.contains("\tunbanned\t")));
}
