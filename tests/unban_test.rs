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
fn unban_rewrites_two_column_lines() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());

    let term_id = "33333333-3333-3333-3333-333333333333";
    let terms_content = format!(
        "# Terms\n\n## widget\nid: {}\ndefinition: A small thing.\naliases: \n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();

    assert!(adr(&dir, &["ban", "--term", "widget"], &[], "forbiddenword\n")
        .status
        .success());
    assert!(adr(&dir, &["ban"], &[], "plainword\n").status.success());

    let before = std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert_eq!(before.lines().count(), 2, "{}", before);
    assert!(
        before.lines().any(|l| l.ends_with(&format!(" {}", term_id))),
        "expected a two-column line before unban: {}",
        before
    );
    assert!(
        before.lines().any(|l| !l.contains(' ')),
        "expected a single-column line before unban: {}",
        before
    );

    let out = adr_owner(&dir, &["unban"], "forbiddenword\n");
    assert!(out.status.success(), "unban failed: {}", stderr(&out));

    let after = std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert_eq!(after.lines().count(), 1, "{}", after);
    assert!(
        !after.contains(term_id),
        "removed line's uuid must not remain: {}",
        after
    );
    assert!(
        !after.trim().contains(' '),
        "remaining line should carry no uuid column: {}",
        after
    );
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
