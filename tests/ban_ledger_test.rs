mod common;

use common::*;

// "Does check hold the ban list equal to the ledger?": the file must hold
// exactly the hashes the ledger currently says are banned, no more and no
// fewer. `check_fails_when_banned_file_and_ledger_disagree_both_ways` (in
// ban_test.rs) already covers the "missing from file" and "missing from
// ledger" halves; these two cover the third: a hash the ledger already
// says is `unbanned` but that still stands in the file.
#[test]
fn check_fails_when_a_hash_is_back_in_the_file_after_being_unbanned() {
    // adr: 06b7e003-4d9f-427a-a798-c1af97865411
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let ban_out = adr(&dir, &["ban"], &[], "forbiddenword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));
    let banned_path = dir.join("adrdom/state/banned.sha256");
    let hash_line = std::fs::read_to_string(&banned_path)
        .unwrap()
        .trim()
        .to_string();
    assert!(!hash_line.is_empty(), "expected a banned hash line");

    let unban_out = adr_owner(&dir, &["unban"], "forbiddenword\n");
    assert!(
        unban_out.status.success(),
        "unban failed: {}",
        stderr(&unban_out)
    );
    assert!(
        std::fs::read_to_string(&banned_path)
            .unwrap()
            .trim()
            .is_empty(),
        "expected the file empty right after unban"
    );

    // Put the hash line back into banned.sha256 by hand: the file now
    // disagrees with the ledger's latest (unbanned) row for this hash.
    std::fs::write(&banned_path, format!("{}\n", hash_line)).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("in the ban list, ledger says unbanned"),
        "{}",
        text
    );
}

#[test]
fn check_has_no_banned_failure_after_ban_or_after_ban_then_unban() {
    // adr: 06b7e003-4d9f-427a-a798-c1af97865411
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let ban_out = adr(&dir, &["ban"], &[], "quietword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains("banned:"),
        "unexpected banned failure right after ban: {}",
        text
    );

    let unban_out = adr_owner(&dir, &["unban"], "quietword\n");
    assert!(
        unban_out.status.success(),
        "unban failed: {}",
        stderr(&unban_out)
    );

    let out2 = adr(&dir, &["check"], &[], "");
    let text2 = stdout(&out2);
    assert!(
        !text2.contains("banned:"),
        "unexpected banned failure after ban then unban: {}",
        text2
    );
}
