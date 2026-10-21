mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/example.md";

fn repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    dir
}

fn ledger(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default()
}

fn both(out: &std::process::Output) -> String {
    format!("{}{}", stdout(out), stderr(out))
}

fn approve(dir: &Path) -> std::process::Output {
    plan_record(dir, REL);
    adr_owner(dir, &["approve", dir.join(REL).to_str().unwrap()], "")
}

// adr: 3762e14b-6dfe-45ee-b407-d2718e817348
#[test]
fn approve_asks_every_question_of_the_file_in_the_run_that_accepts_it() {
    let dir = repo();
    let record = write_record(&dir, REL, EXAMPLE_RECORD);
    let out = approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let printed = stdout(&out);
    assert!(printed.contains("approved: "), "{}", printed);
    assert!(
        printed.contains("asked: \"1.1 What is the unit of authority called?\""),
        "{}",
        printed
    );
    assert_eq!(asked_row_count(&dir), 1, "{}", ledger(&dir));

    // The terms line sits under the id line, written by approve.
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content).remove(0);
    let lines: Vec<&str> = content.lines().collect();
    let at = lines.iter().position(|l| l.starts_with("id: ")).unwrap();
    assert!(lines[at + 1].starts_with("terms: "), "{}", content);

    // The next move is the owner's answer; nothing is left to ask.
    let show = stdout(&adr(&dir, &["show", REL], &[], ""));
    assert!(show.contains("[asked @"), "{}", show);
    let moves = file_moves(&dir, REL);
    assert!(
        moves
            .iter()
            .any(|m| m.contains("owner runs: adr answer") && m.contains(&id)),
        "{:?}",
        moves
    );
    assert!(
        !moves.iter().any(|m| m.contains("adr approve")),
        "{:?}",
        moves
    );

    // Approve again with nothing changed writes and prints nothing.
    let before = ledger(&dir);
    let again = approve(&dir);
    assert!(again.status.success(), "{}", stderr(&again));
    assert_eq!(stdout(&again), "");
    assert_eq!(ledger(&dir), before);
    assert_eq!(std::fs::read_to_string(&record).unwrap(), content);
}

// adr: 3762e14b-6dfe-45ee-b407-d2718e817348
#[test]
fn approve_that_refuses_asks_nothing_and_writes_nothing() {
    let dir = repo();
    let long = (1..=25)
        .map(|i| format!("w{}", i))
        .collect::<Vec<_>>()
        .join(" ");
    let text = EXAMPLE_RECORD.replace(
        "The authoritative sentence is a decision, and the question that produced it is a question.",
        &format!("{}.", long),
    );
    let record = write_record(&dir, REL, &text);
    let before = ledger(&dir);
    let out = approve(&dir);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("the option has 25 words"),
        "{}",
        stderr(&out)
    );
    assert_eq!(ledger(&dir), before);
    assert_eq!(asked_row_count(&dir), 0);
    let content = std::fs::read_to_string(&record).unwrap();
    assert!(!content.contains("terms: "), "{}", content);
    assert!(!content.contains("id: "), "{}", content);
}

// adr: 3762e14b-6dfe-45ee-b407-d2718e817348
#[test]
fn approve_of_an_edited_question_asks_it_again_under_its_new_text() {
    let dir = repo();
    let record = write_record(&dir, REL, EXAMPLE_RECORD);
    assert!(approve(&dir).status.success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content).remove(0);
    let first = current_version(&dir, &id);

    std::fs::write(
        &record,
        content.replace(
            "names leaked into decisions.",
            "names leaked into decisions, badly.",
        ),
    )
    .unwrap();
    let show = stdout(&adr(&dir, &["show", REL], &[], ""));
    assert!(show.contains("[changed since asked]"), "{}", show);

    let out = approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("asked: \""), "{}", stdout(&out));
    assert_eq!(asked_row_count(&dir), 2);
    assert_ne!(current_version(&dir, &id), first);
    assert!(adr_answer(&dir, &id, "A").status.success());
}

// adr: 3762e14b-6dfe-45ee-b407-d2718e817348
#[test]
fn approve_of_the_terms_file_asks_nothing() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, REL, EXAMPLE_RECORD);
    approve_terms_with(&dir, &["decision", "example"]);
    assert_eq!(asked_row_count(&dir), 0, "{}", ledger(&dir));
}

// adr: 3762e14b-6dfe-45ee-b407-d2718e817348
#[test]
fn no_ask_command_remains() {
    let dir = repo();
    write_record(&dir, REL, EXAMPLE_RECORD);
    let out = adr(&dir, &["ask", REL], &[], "");
    assert!(!out.status.success());
    assert_eq!(asked_row_count(&dir), 0);
    let help = stdout(&adr(&dir, &["--help"], &[], ""));
    assert!(
        !help.lines().any(|l| l.trim_start().starts_with("ask ")),
        "{}",
        help
    );
}

// adr: 3762e14b-6dfe-45ee-b407-d2718e817348
#[test]
fn approve_asks_a_question_in_conflict_again_with_or_without_its_answer_line() {
    for keep_line in [true, false] {
        let dir = repo();
        let record = write_record(&dir, REL, EXAMPLE_RECORD);
        assert!(approve(&dir).status.success());
        let id = extract_ids(&std::fs::read_to_string(&record).unwrap()).remove(0);
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-q", "-m", "asked"]);
        run_git(&dir, &["checkout", "-q", "-b", "other"]);
        assert!(adr_answer(&dir, &id, "B").status.success());
        run_git(&dir, &["commit", "-q", "-am", "B"]);
        run_git(&dir, &["checkout", "-q", "-"]);
        assert!(adr_answer(&dir, &id, "A").status.success());
        run_git(&dir, &["commit", "-q", "-am", "A"]);
        if !run_git(&dir, &["merge", "-q", "--no-edit", "other"])
            .status
            .success()
        {
            run_git(&dir, &["checkout", "--ours", REL]);
        }
        if !keep_line {
            let text = std::fs::read_to_string(&record).unwrap();
            std::fs::write(&record, text.replace("\n\n**Answer:** A", "")).unwrap();
        }
        let check = both(&adr(&dir, &["check"], &[], ""));
        assert!(check.contains("two standing answers"), "{}", check);
        assert!(check.contains("owner runs: adr approve"), "{}", check);

        let out = adr_owner(&dir, &["approve", dir.join(REL).to_str().unwrap()], "");
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(stdout(&out).contains("asked: \""), "{}", stdout(&out));
        assert!(!std::fs::read_to_string(&record)
            .unwrap()
            .contains("**Answer:**"));
        assert!(adr_answer(&dir, &id, "B").status.success());
        let check = both(&adr(&dir, &["check"], &[], ""));
        assert!(!check.contains("two standing answers"), "{}", check);
    }
}
