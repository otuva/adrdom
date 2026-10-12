//! Cases a review of the message work found: each walks to the line the
//! tool prints and, where the line names a move, makes it.

mod common;

use common::*;
use std::path::{Path, PathBuf};

fn question(heading: &str, depends: &str, extra: &str) -> String {
    format!(
        "### {}\ndepends: {}\n{}\nProblem: the widget breaks.\n\nStandard: plain words.\n\n\
- A) Yes the widget holds.\n  Why: reasons.\n  Costs: costs.\n\
- B) No the widget does not hold.\n  Why: other reasons.\n  Costs: other costs.\n",
        heading, depends, extra
    )
}

fn setup() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    dir
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

const A: &str = "adrdom/decisions/a.md";
const B: &str = "adrdom/decisions/b.md";
const WITH_COMMA: &str = "Is the widget hold, in the rain?";

// A supersedes line takes the heading of the decision, and a heading may
// hold a comma: the line is not cut at it. Approve turns the heading into
// the identifier, before any body exists.
//
// adr: f19e7914-ad97-4f6b-87dd-9301e53cd068
#[test]
fn a_supersedes_line_takes_a_heading_that_holds_a_comma() {
    let dir = setup();
    let a = write_record(&dir, A, &format!("# A\n\n{}", question(WITH_COMMA, "widget", "")));
    approve_record(&dir, A);
    assert!(adr(&dir, &["ask", a.to_str().unwrap()], &[], "").status.success());
    let alpha = extract_ids(&std::fs::read_to_string(&a).unwrap())[0].clone();
    let out = adr_answer(&dir, &alpha, "A");
    assert!(out.status.success(), "{}", stderr(&out));

    let b = write_record(
        &dir,
        B,
        &format!(
            "# B\n\n{}",
            question(
                "Is the widget gamma hold?",
                "widget",
                &format!("supersedes: {}\n", WITH_COMMA)
            )
        ),
    );
    approve_record(&dir, B);
    let approved = std::fs::read_to_string(&b).unwrap();
    assert!(approved.contains(&format!("supersedes: {}\n", alpha)), "{}", approved);
    let out = adr(&dir, &["ask", b.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let asked = std::fs::read_to_string(&b).unwrap();
    assert!(asked.contains(&format!("supersedes: {}\n", alpha)), "{}", asked);
}

// One hash on two lines of the ban list is one word: it is removed once,
// with one unbanned row and one line.
#[test]
fn unban_removes_a_word_once_when_the_ban_list_holds_it_twice() {
    let dir = setup();
    assert!(adr(&dir, &["ban"], &[], "zebra\n").status.success());
    let list = dir.join("adrdom/state/banned.sha256");
    let content = std::fs::read_to_string(&list).unwrap();
    std::fs::write(&list, format!("{}{}", content, content)).unwrap();

    let out = adr_owner(&dir, &["unban"], "zebra\nquagga\n");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "unbanned: a word\n", "{}", stdout(&out));
    assert_eq!(std::fs::read_to_string(&list).unwrap(), "");
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// Outside a repository the move names what clears it, once: changing into a
// git repository, then the command as it was given.
#[test]
fn outside_a_repository_the_owners_line_is_the_whole_command() {
    let outside = OutsideDir::new("outside");
    let out = adr_owner(outside.path(), &["approve", "adrdom/decisions/x.md"], "");

    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        "approve: not inside a git repository; owner changes into a git repository and runs: adr approve adrdom/decisions/x.md"
    );
}

// A hook that is not what install writes is left; uninstall prints only
// what it removed or edited (per "What does a command that writes print?",
// 069a6b4a A), so the hook it left is named nowhere in its output.
#[test]
fn uninstall_leaves_a_hook_it_did_not_write_and_prints_only_what_it_removed() {
    let dir = setup();
    let hook = dir.join(".githooks/pre-commit");
    let written = std::fs::read_to_string(&hook).unwrap();
    std::fs::write(&hook, format!("{}# a line of the owner's\n", written)).unwrap();

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains(".githooks/pre-commit"), "{}", text);
    assert!(text.lines().any(|l| l == "removed: .githooks/commit-msg"), "{}", text);
    for line in text.lines() {
        assert!(line.starts_with("removed: ") || line.starts_with("written: "), "{}", text);
    }
    assert!(hook.exists());
    assert!(!dir.join(".githooks/commit-msg").exists());
}

// What was removed before a removal failed is printed by the run that
// removed it.
#[cfg(unix)]
#[test]
fn uninstall_prints_what_it_removed_before_it_failed() {
    let dir = setup();
    let hooks = dir.join(".githooks");
    set_mode(&hooks, 0o555);

    let out = adr_owner(&dir, &["uninstall"], "");
    set_mode(&hooks, 0o755);

    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(stderr(&out).starts_with("uninstall: failed to remove "), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("AGENTS.md"), "{}", text);
    for line in text.lines() {
        assert!(line.starts_with("removed: ") || line.starts_with("written: "), "{}", text);
    }
}
