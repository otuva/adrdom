// A planner's commit holds files under adrdom only: check fails it when it
// stages a path outside, through the real git hooks.

mod common;

use common::*;
use std::path::{Path, PathBuf};

const PLANNER: &str = "planner-session-1";
const OTHER: &str = "plain-session-2";

fn planner_repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/old.rs"), "fn old() {}\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    let out = run_git(&dir, &["commit", "-q", "-m", "chore: install"]);
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    std::fs::write(dir.join("adrdom/state/roles.tsv"), format!("{}\tplanner\n", PLANNER)).unwrap();
    dir
}

fn commit(dir: &Path, session: Option<&str>) -> std::process::Output {
    run_git_as(dir, &["commit", "-q", "-m", "docs: change"], session)
}

fn text(out: &std::process::Output) -> String {
    format!("{}{}", stdout(out), stderr(out))
}

fn assert_refused_naming(out: &std::process::Output, path: &str) {
    assert!(!out.status.success(), "{}", text(out));
    let t = text(out);
    let line = t.lines().find(|l| l.contains(path)).unwrap_or_else(|| panic!("no line names {}: {}", path, t));
    assert!(line.contains("outside adrdom"), "{}", line);
    assert!(line.split("; ").last().unwrap().starts_with("builder "), "{}", line);
}

#[test]
fn a_planner_commit_of_a_path_outside_adrdom_is_refused() {
    // adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
    let dir = planner_repo();
    std::fs::write(dir.join("src/x.rs"), "fn x() {}\n").unwrap();
    run_git(&dir, &["add", "src/x.rs"]);
    assert_refused_naming(&commit(&dir, Some(PLANNER)), "src/x.rs");
}

#[test]
fn a_planner_commit_of_adrdom_paths_only_passes() {
    // adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
    let dir = planner_repo();
    std::fs::write(dir.join("adrdom/notes.md"), "notes\n").unwrap();
    run_git(&dir, &["add", "adrdom/notes.md"]);
    let out = commit(&dir, Some(PLANNER));
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn a_planner_mixed_commit_is_refused() {
    // adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
    let dir = planner_repo();
    std::fs::write(dir.join("adrdom/notes.md"), "notes\n").unwrap();
    std::fs::write(dir.join("src/x.rs"), "fn x() {}\n").unwrap();
    run_git(&dir, &["add", "adrdom/notes.md", "src/x.rs"]);
    let out = commit(&dir, Some(PLANNER));
    assert_refused_naming(&out, "src/x.rs");
    assert!(!text(&out).lines().any(|l| l.contains("outside adrdom") && l.contains("notes.md")));
}

#[test]
fn a_planner_deleting_a_tracked_file_outside_adrdom_is_refused() {
    // adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
    let dir = planner_repo();
    run_git(&dir, &["rm", "-q", "src/old.rs"]);
    assert_refused_naming(&commit(&dir, Some(PLANNER)), "src/old.rs");
}

#[test]
fn a_planner_moving_an_outside_file_into_adrdom_is_refused() {
    // adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
    let dir = planner_repo();
    run_git(&dir, &["mv", "src/old.rs", "adrdom/old.rs"]);
    assert_refused_naming(&commit(&dir, Some(PLANNER)), "src/old.rs");
}

#[test]
fn a_plain_session_or_none_commits_outside_adrdom() {
    // adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
    let dir = planner_repo();
    std::fs::write(dir.join("src/x.rs"), "fn x() {}\n").unwrap();
    run_git(&dir, &["add", "src/x.rs"]);
    let out = commit(&dir, Some(OTHER));
    assert!(out.status.success(), "{}", text(&out));
    std::fs::write(dir.join("src/y.rs"), "fn y() {}\n").unwrap();
    run_git(&dir, &["add", "src/y.rs"]);
    let out = commit(&dir, None);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn a_planner_merge_that_brings_an_outside_path_is_refused() {
    // adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
    let dir = planner_repo();
    let base = stdout(&run_git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"])).trim().to_string();
    run_git(&dir, &["checkout", "-q", "-b", "side"]);
    std::fs::write(dir.join("src/y.rs"), "fn y() {}\n").unwrap();
    run_git(&dir, &["add", "src/y.rs"]);
    assert!(commit(&dir, Some(OTHER)).status.success());
    run_git(&dir, &["checkout", "-q", &base]);
    std::fs::write(dir.join("adrdom/notes.md"), "notes\n").unwrap();
    run_git(&dir, &["add", "adrdom/notes.md"]);
    assert!(commit(&dir, Some(PLANNER)).status.success());
    let out = run_git_as(&dir, &["merge", "--no-ff", "-m", "docs: merge side", "side"], Some(PLANNER));
    assert_refused_naming(&out, "src/y.rs");
}

#[test]
fn plain_check_in_a_planner_session_has_no_planner_failure() {
    // adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
    let dir = planner_repo();
    std::fs::write(dir.join("src/x.rs"), "fn x() {}\n").unwrap();
    run_git(&dir, &["add", "src/x.rs"]);
    let out = adr_as(&dir, &["check"], &[], Some(PLANNER), "");
    assert!(!text(&out).contains("outside adrdom"), "{}", text(&out));
}

#[test]
fn an_unreadable_roles_file_refuses_a_commit_when_a_session_has_an_identifier() {
    // adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
    let dir = planner_repo();
    let roles = dir.join("adrdom/state/roles.tsv");
    std::fs::remove_file(&roles).unwrap();
    std::fs::create_dir(&roles).unwrap();
    std::fs::write(dir.join("src/x.rs"), "fn x() {}\n").unwrap();
    run_git(&dir, &["add", "src/x.rs"]);
    let out = commit(&dir, Some(PLANNER));
    assert!(!out.status.success(), "{}", text(&out));
    let t = text(&out);
    assert!(t.lines().any(|l| l.contains("adrdom/state/roles.tsv") && l.contains("cannot be read")), "{}", t);
    // With no identifier the file is never consulted.
    let out = commit(&dir, None);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn a_planner_line_beside_invalid_utf8_still_counts() {
    // adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
    let dir = planner_repo();
    let mut bytes = vec![0xff, 0xfe, b'\n'];
    bytes.extend_from_slice(format!("{}\tplanner\n", PLANNER).as_bytes());
    std::fs::write(dir.join("adrdom/state/roles.tsv"), bytes).unwrap();
    std::fs::write(dir.join("src/x.rs"), "fn x() {}\n").unwrap();
    run_git(&dir, &["add", "src/x.rs"]);
    assert_refused_naming(&commit(&dir, Some(PLANNER)), "src/x.rs");
}
