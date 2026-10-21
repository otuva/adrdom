// The build move in a planner's session, and the retie move of a test tie
// that names a decision that is not current.

mod common;

use common::*;
use std::path::{Path, PathBuf};

const FIRST_HEADING: &str = "1.1 What is the unit of authority called?";

fn question(n: usize, heading: &str, supersedes: Option<&str>) -> String {
    let sup = supersedes
        .map(|id| format!("\nsupersedes: {}\n", id))
        .unwrap_or_default();
    format!(
        "\n### {heading}\ndepends: authority\n{sup}\nProblem: naming unit number {n}.\n\nStandard: plain words.\n\n- A) The unit {n} is a widget{n}.\n  Why: reason {n}.\n  Costs: cost {n}.\n- B) The unit {n} is a gadget{n}.\n  Why: other reason {n}.\n  Costs: other cost {n}.\n"
    )
}

/// Adds a question to the record, approves (which asks) and answers it A; returns its id.
fn add_answered(
    dir: &Path,
    record: &Path,
    n: usize,
    heading: &str,
    supersedes: Option<&str>,
) -> String {
    let mut content = std::fs::read_to_string(record).unwrap_or_else(|_| "# Example\n".to_string());
    let before = extract_ids(&content);
    content.push_str(&question(n, heading, supersedes));
    std::fs::write(record, &content).unwrap();
    approve_record(dir, "adrdom/decisions/example.md");
    let id = extract_ids(&std::fs::read_to_string(record).unwrap())
        .into_iter()
        .find(|i| !before.contains(i))
        .expect("new id");
    let out = adr_answer(dir, &id, "A");
    assert!(out.status.success(), "answer: {}", stderr(&out));
    id
}

fn repo_with_first() -> (PathBuf, PathBuf, String) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = dir.join("adrdom/decisions/example.md");
    let id = add_answered(&dir, &record, 1, FIRST_HEADING, None);
    (dir, record, id)
}

fn make_planner(dir: &Path, session: &str) {
    let path = dir.join("adrdom/state/roles.tsv");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("{}\tplanner\n", session)).unwrap();
}

fn build_line(heading: &str, _id: &str, who: &str) -> String {
    format!("{} builds \"{}\"", who, heading)
}

// adr: 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8
#[test]
fn a_build_move_opens_with_builder_in_a_planners_session_and_agent_elsewhere() {
    let (dir, _record, id) = repo_with_first();
    make_planner(&dir, "s1");
    let file = "adrdom/decisions/example.md";
    for args in [vec!["show"], vec!["show", file], vec!["check"]] {
        let planner = stdout(&adr_as(&dir, &args, &[], Some("s1"), ""));
        assert!(
            planner.contains(&build_line(FIRST_HEADING, &id, "builder")),
            "{:?} as s1: {}",
            args,
            planner
        );
        assert!(
            !planner.contains(&build_line(FIRST_HEADING, &id, "agent")),
            "{:?} as s1: {}",
            args,
            planner
        );
        for session in [Some("s2"), None] {
            let plain = stdout(&adr_as(&dir, &args, &[], session, ""));
            assert!(
                plain.contains(&build_line(FIRST_HEADING, &id, "agent")),
                "{:?} as {:?}: {}",
                args,
                session,
                plain
            );
            assert!(
                !plain.contains("builder builds"),
                "{:?} as {:?}: {}",
                args,
                session,
                plain
            );
        }
    }
}

fn check_text(dir: &Path, session: Option<&str>) -> (bool, String) {
    let out = adr_as(dir, &["check"], &[], session, "");
    (out.status.success(), stdout(&out))
}

// adr: 2381d759-f85f-484e-9031-fadbe6ea045a
#[test]
fn a_tie_to_a_superseded_decision_names_the_current_successor_and_fails_nothing() {
    let (dir, record, old) = repo_with_first();
    write_test_comment(&dir, &old);
    let new = add_answered(
        &dir,
        &record,
        2,
        "2.1 What is the second unit of authority called?",
        Some(&old),
    );
    write_test_comment(&dir, &new);
    let (ok, text) = check_text(&dir, None);
    assert!(ok, "{}", text);
    let line = text
        .lines()
        .find(|l| l.contains(&old) && l.contains("tied to a test and not a current decision"))
        .unwrap_or_else(|| panic!("{}", text));
    assert!(
        line.ends_with(&format!("agent rewrites the tie comment to {}", new)),
        "{}",
        line
    );
    assert!(line.contains(FIRST_HEADING), "{}", line);
    // In a planner's session the move opens with builder.
    make_planner(&dir, "s1");
    let (ok, text) = check_text(&dir, Some("s1"));
    assert!(ok, "{}", text);
    assert!(
        text.lines().any(|l| l.contains(&old)
            && l.contains(&format!("builder rewrites the tie comment to {}", new))),
        "{}",
        text
    );
}

// adr: 2381d759-f85f-484e-9031-fadbe6ea045a
#[test]
fn a_tie_to_a_decision_superseded_twice_names_the_last_current_one() {
    let (dir, record, first) = repo_with_first();
    write_test_comment(&dir, &first);
    let second = add_answered(
        &dir,
        &record,
        2,
        "2.1 What is the second unit of authority called?",
        Some(&first),
    );
    let third = add_answered(
        &dir,
        &record,
        3,
        "3.1 What is the third unit of authority called?",
        Some(&second),
    );
    let (ok, text) = check_text(&dir, None);
    assert!(ok, "{}", text);
    let line = text
        .lines()
        .find(|l| l.contains(&first) && l.contains("tied to a test"))
        .unwrap_or_else(|| panic!("{}", text));
    assert!(
        line.ends_with(&format!("agent rewrites the tie comment to {}", third)),
        "{}",
        line
    );
    assert!(!line.contains(&second), "{}", line);
}

// adr: 2381d759-f85f-484e-9031-fadbe6ea045a
#[test]
fn a_tie_to_a_refused_asked_or_unknown_decision_is_removed_and_fails_nothing() {
    let (dir, record, _id) = repo_with_first();
    // A refused question.
    let mut content = std::fs::read_to_string(&record).unwrap();
    let before = extract_ids(&content);
    content.push_str(&question(
        2,
        "2.1 What is the second unit of authority called?",
        None,
    ));
    content.push_str(&question(
        3,
        "3.1 What is the third unit of authority called?",
        None,
    ));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    let ids: Vec<String> = extract_ids(&std::fs::read_to_string(&record).unwrap())
        .into_iter()
        .filter(|i| !before.contains(i))
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(adr_answer(&dir, &ids[0], "refuse").status.success());
    // ids[1] stays asked, not answered. A third id is unknown.
    let unknown = "99999999-9999-4999-8999-999999999999";
    for id in [ids[0].as_str(), ids[1].as_str(), unknown] {
        write_test_comment(&dir, id);
    }
    let (ok, text) = check_text(&dir, None);
    assert!(ok, "{}", text);
    for id in [ids[0].as_str(), ids[1].as_str(), unknown] {
        let line = text
            .lines()
            .find(|l| l.contains(id) && l.contains("tied to a test and not a current decision"))
            .unwrap_or_else(|| panic!("{}: {}", id, text));
        assert!(line.contains("agent removes the tie comment"), "{}", line);
    }
    make_planner(&dir, "s1");
    let (ok, text) = check_text(&dir, Some("s1"));
    assert!(ok, "{}", text);
    assert!(
        text.lines()
            .any(|l| l.contains(unknown) && l.contains("builder removes the tie comment")),
        "{}",
        text
    );
}

// adr: 2381d759-f85f-484e-9031-fadbe6ea045a
#[test]
fn an_answer_that_supersedes_a_tied_decision_commits_through_the_hooks() {
    let (dir, record, old) = repo_with_first();
    write_test_comment(&dir, &old);
    run_git(&dir, &["add", "-A"]);
    let out = run_git(&dir, &["commit", "-q", "-m", "first decision and its tie"]);
    assert!(
        out.status.success(),
        "first commit: {}{}",
        stdout(&out),
        stderr(&out)
    );
    // The superseding decision is answered; the tie still names the old one.
    let _new = add_answered(
        &dir,
        &record,
        2,
        "2.1 What is the second unit of authority called?",
        Some(&old),
    );
    run_git(&dir, &["add", "-A"]);
    let out = run_git(
        &dir,
        &["commit", "-q", "-m", "answer the superseding decision"],
    );
    assert!(
        out.status.success(),
        "the answer commit must pass: {}{}",
        stdout(&out),
        stderr(&out)
    );
}
