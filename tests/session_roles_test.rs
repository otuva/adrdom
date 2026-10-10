//! `plan` and `ask` on a topic file make the session a planner; nothing
//! else marks, and nothing stops when there is no session identifier.
mod common;
use common::*;
use std::path::Path;

const TOPIC: &str = "# Gadgets\n\nWhat gadgets do.\n\n### Does a gadget use a widget?\n";
const ROLES: &str = "adrdom/state/roles.tsv";

fn roles(dir: &Path) -> Option<String> {
    std::fs::read_to_string(dir.join(ROLES)).ok()
}

fn topic(dir: &Path) -> String {
    write_record(dir, "adrdom/decisions/gadgets.md", TOPIC).display().to_string()
}

/// No command prints a line of the role file: the write is no ledger event.
fn assert_no_planner_line(out: &std::process::Output) {
    let all = format!("{}{}", stdout(out), stderr(out));
    assert!(!all.lines().any(|l| l.starts_with("planner:")), "{}", all);
}

/// A commit of a file outside adrdom, as the session `who`.
fn commit_outside(dir: &Path, file: &str, who: Option<&str>) -> std::process::Output {
    std::fs::write(dir.join(file), "fn f() {}\n").unwrap();
    run_git_as(dir, &["add", "-A"], who);
    run_git_as(dir, &["commit", "-q", "-m", "feat: code"], who)
}

#[test]
fn plan_marks_the_session_once_and_prints_nothing_of_it() {
    // adr: e53b805c-9272-4b89-985d-61426fb5fa48
    let dir = widget_repo();
    let t = topic(&dir);
    let first = adr_as(&dir, &["plan", &t], &[], Some("s1"), "");
    assert!(first.status.success(), "{}", stderr(&first));
    assert_eq!(roles(&dir).as_deref(), Some("s1\tplanner\n"));
    assert_no_planner_line(&first);
    let second = adr_as(&dir, &["plan", &t], &[], Some("s1"), "");
    assert!(second.status.success());
    assert_no_planner_line(&second);
    assert_eq!(roles(&dir).as_deref(), Some("s1\tplanner\n"));
}

#[test]
fn ask_marks_even_when_it_refuses_and_other_commands_never_mark() {
    // adr: 6c406624-b874-4697-9a44-91941019b7d1
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let t = topic(&dir);
    // The terms file is not approved: ask refuses, after the mark.
    let ask = adr_as(&dir, &["ask", &t], &[], Some("s1"), "");
    assert!(!ask.status.success());
    assert_eq!(roles(&dir).as_deref(), Some("s1\tplanner\n"));
    assert_no_planner_line(&ask);

    let other = widget_repo();
    let t = topic(&other);
    let terms = other.join("adrdom/terms.md").display().to_string();
    let blame = other.join("adrdom/blame/x.md").display().to_string();
    for args in [
        vec!["plan", terms.as_str()],
        vec!["plan", blame.as_str()],
        vec!["ask", terms.as_str()],
        vec!["show"],
        vec!["show", t.as_str()],
        vec!["check"],
        vec!["read", t.as_str()],
    ] {
        adr_as(&other, &args, &[], Some("s9"), "");
        assert!(roles(&other).is_none(), "{:?} marked the session", args);
    }
}

#[test]
fn a_missing_file_or_a_directory_marks_nothing() {
    // adr: 6c406624-b874-4697-9a44-91941019b7d1
    // A repository never installed: nothing creates the roles file.
    let dir = init_repo();
    std::fs::create_dir_all(dir.join("adrdom/decisions/dir.md")).unwrap();
    for rel in ["adrdom/decisions/missing.md", "adrdom/decisions/dir.md"] {
        let path = dir.join(rel).display().to_string();
        for cmd in ["plan", "ask"] {
            let out = adr_as(&dir, &[cmd, &path], &[], Some("s1"), "");
            assert!(!out.status.success(), "{} {}", cmd, rel);
            assert!(roles(&dir).is_none(), "{} {} marked the session", cmd, rel);
            assert!(!dir.join(ROLES).exists(), "{} {} created the roles file", cmd, rel);
        }
    }
}

#[test]
fn the_roles_file_is_untracked_and_the_ledger_untouched() {
    // adr: 1cec44fc-2685-42f8-9742-7facabb52251
    let dir = widget_repo();
    let t = topic(&dir);
    let ledger = std::fs::read(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let out = adr_as(&dir, &["plan", &t], &[], Some("s1"), "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(roles(&dir).is_some());
    let status = run_git(&dir, &["status", "--porcelain", "--untracked-files=all"]);
    let status = stdout(&status);
    assert!(!status.contains("roles.tsv"), "{}", status);
    assert_eq!(std::fs::read(dir.join("adrdom/state/ledger.tsv")).unwrap(), ledger);
}

#[test]
fn every_planner_keeps_its_line_and_others_stay_plain() {
    // adr: 1ee664a1-cac0-4917-9e46-8199a387d235
    let dir = widget_repo();
    let t = topic(&dir);
    assert!(adr_as(&dir, &["plan", &t], &[], Some("s1"), "").status.success());
    let s2 = adr_as(&dir, &["plan", &t], &[], Some("s2"), "");
    assert_no_planner_line(&s2);
    assert_eq!(roles(&dir).as_deref(), Some("s1\tplanner\ns2\tplanner\n"));
    // s3 was never marked: its first plan is the one that marks it.
    // Until then s3 is plain: its commit outside adrdom passes the hooks
    // while the marked s1's is refused.
    assert!(commit_outside(&dir, "plain.rs", Some("s3")).status.success());
    let refused = commit_outside(&dir, "planner.rs", Some("s1"));
    assert!(!refused.status.success());
    assert!(format!("{}{}", stdout(&refused), stderr(&refused)).contains("outside adrdom"));
    run_git(&dir, &["reset", "-q"]);
    let s3 = adr_as(&dir, &["plan", &t], &[], Some("s3"), "");
    assert_no_planner_line(&s3);
    assert_eq!(roles(&dir).as_deref(), Some("s1\tplanner\ns2\tplanner\ns3\tplanner\n"));
}

#[test]
fn a_commit_marks_no_builder() {
    // adr: 8d3337b9-6fb3-4756-80f1-b388fe81d856
    let dir = widget_repo();
    std::fs::write(dir.join("code.rs"), "fn main() {}\n").unwrap();
    run_git_as(&dir, &["add", "-A"], Some("s1"));
    let commit = run_git_as(&dir, &["commit", "-q", "-m", "feat: code"], Some("s1"));
    assert!(commit.status.success(), "{}", stderr(&commit));
    assert!(roles(&dir).is_none());
    let t = topic(&dir);
    let plan = adr_as(&dir, &["plan", &t], &[], Some("s1"), "");
    assert!(plan.status.success(), "{}", stderr(&plan));
    assert_eq!(roles(&dir).as_deref(), Some("s1\tplanner\n"));
}

#[test]
fn no_identifier_writes_nothing_and_stops_nothing() {
    // adr: 8081c82d-e5c6-485b-a302-07d586f1a01b
    let dir = widget_repo();
    let t = topic(&dir);
    let out = adr_as(&dir, &["plan", &t], &[], None, "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(roles(&dir).is_none());
    assert_no_planner_line(&out);
    // After a session with an identifier was marked in the same repo, a
    // run with none is still plain: check passes and so does a commit
    // outside adrdom.
    let marked = adr_as(&dir, &["plan", &t], &[], Some("s1"), "");
    assert!(marked.status.success(), "{}", stderr(&marked));
    assert_eq!(roles(&dir).as_deref(), Some("s1\tplanner\n"));
    let commit = commit_outside(&dir, "plain.rs", None);
    assert!(commit.status.success(), "{}", stderr(&commit));
    let check = adr_as(&dir, &["check"], &[], None, "");
    assert!(check.status.success(), "{}{}", stdout(&check), stderr(&check));
}
