//! A command that could not do a part of its work says so and fails: it
//! never skips the part and reports the rest as a pass.

mod common;

use common::*;
use std::path::Path;

/// Bytes no reader takes as text.
const NOT_TEXT: &[u8] = &[0x23, 0x20, 0xff, 0xfe, 0x0a];

fn installed_repo() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    dir
}

fn write_unreadable_topic_file(dir: &Path) {
    std::fs::write(dir.join("adrdom/decisions/bad.md"), NOT_TEXT).unwrap();
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn show_names_a_topic_file_it_cannot_read() {
    let dir = installed_repo();
    write_unreadable_topic_file(&dir);

    let out = adr(&dir, &["show"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("# adrdom/decisions/bad.md (cannot be read: "),
        "{}",
        stdout(&out)
    );
}

#[test]
fn show_refuses_a_topic_file_argument_it_cannot_read() {
    let dir = installed_repo();
    write_unreadable_topic_file(&dir);

    let out = adr(&dir, &["show", "adrdom/decisions/bad.md"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stderr(&out).starts_with("show: cannot read adrdom/decisions/bad.md: "),
        "{}",
        stderr(&out)
    );
}

#[test]
fn answer_names_the_topic_files_it_could_not_read() {
    let dir = installed_repo();
    write_unreadable_topic_file(&dir);

    let out = adr_owner(&dir, &["answer", "no-such-id@00000000", "A"], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stderr(&out).starts_with(
            "answer: id no-such-id not found in the topic files that could be read; cannot read adrdom/decisions/bad.md; "
        ),
        "{}",
        stderr(&out)
    );
}

#[test]
fn check_fails_on_a_commit_message_file_it_cannot_read() {
    let dir = installed_repo();

    let out = adr(&dir, &["check", "--message", "no-such-message-file"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("banned: cannot read the commit message file no-such-message-file: "),
        "{}",
        stdout(&out)
    );
}

#[test]
fn ban_with_a_term_says_the_terms_file_is_missing() {
    let dir = installed_repo();
    let terms = dir.join("adrdom/terms.md");
    if terms.exists() {
        std::fs::remove_file(&terms).unwrap();
    }

    let out = adr(&dir, &["ban", "--term", "widget"], &[], "word\n");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stderr(&out).starts_with("ban: cannot look up \"widget\": adrdom/terms.md is missing or cannot be read; "),
        "{}",
        stderr(&out)
    );
}

#[test]
fn unban_prints_one_line_for_the_word_it_unbanned() {
    let dir = installed_repo();
    assert!(adr(&dir, &["ban"], &[], "zebra\n").status.success());

    let out = adr_owner(&dir, &["unban"], "zebra\nquagga\n");
    assert!(out.status.success(), "{}", stderr(&out));
    // One line for the one unbanned row; a word that was not banned writes
    // no row and prints nothing (per "What does a command that writes
    // print?", 069a6b4a A).
    assert_eq!(stdout(&out), "unbanned: a word\n", "{}", stdout(&out));

    let out = adr_owner(&dir, &["unban"], "quagga\n");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "", "{}", stdout(&out));
}

#[cfg(unix)]
#[test]
fn install_fails_when_the_ledger_cannot_be_written() {
    let dir = init_repo();
    let state = dir.join("adrdom/state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(dir.join("adrdom/decisions")).unwrap();
    set_mode(&state, 0o555);

    let out = adr(&dir, &["install"], &[], "");
    set_mode(&state, 0o755);

    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(stderr(&out).starts_with("install: failed to write "), "{}", stderr(&out));
    assert!(stderr(&out).contains("ledger.tsv"), "{}", stderr(&out));
}

#[cfg(unix)]
#[test]
fn install_fails_when_git_does_not_take_the_hooks_path() {
    let dir = init_repo();
    let git_dir = dir.join(".git");
    set_mode(&git_dir, 0o555);

    let out = adr(&dir, &["install"], &[], "");
    set_mode(&git_dir, 0o755);

    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stderr(&out).starts_with("install: git config failed to set core.hooksPath; "),
        "{}",
        stderr(&out)
    );
}

#[cfg(unix)]
#[test]
fn uninstall_fails_on_a_hook_it_cannot_remove() {
    let dir = installed_repo();
    let hooks = dir.join(".githooks");
    set_mode(&hooks, 0o555);

    let out = adr_owner(&dir, &["uninstall"], "");
    set_mode(&hooks, 0o755);

    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(stderr(&out).starts_with("uninstall: failed to remove "), "{}", stderr(&out));
    assert!(stderr(&out).contains("pre-commit"), "{}", stderr(&out));
}
