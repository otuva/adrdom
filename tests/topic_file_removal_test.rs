mod common;

use common::*;
use std::path::{Path, PathBuf};

// adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
// A whole topic file leaves by a plan change the owner approves, once it
// holds no current decision and no open question.

const ONE: &str = "# Example\n\nContext prose.\n\n### 1.1 Only question?\ndepends: decision\n\nProblem: the decision at hand.\n\nStandard: s.\n\n- A) Answer A.\n  Why: reasons.\n  Costs: costs.\n- B) Answer B.\n  Why: other reasons.\n  Costs: other costs.\n";

const TWO: &str = "# Example\n\nContext prose.\n\n### 1.1 First question?\ndepends: decision\n\nProblem: the decision at hand.\n\nStandard: s.\n\n- A) Answer A.\n  Why: reasons.\n  Costs: costs.\n- B) Answer B.\n  Why: other reasons.\n  Costs: other costs.\n\n### 1.2 Second question?\ndepends: decision\n\nProblem: another decision.\n\nStandard: s.\n\n- A) Answer A.\n  Why: reasons.\n  Costs: costs.\n- B) Answer B.\n  Why: other reasons.\n  Costs: other costs.\n";

const SECOND: &str = "\n### 1.2 Added question?\ndepends: decision\n\nProblem: another decision.\n\nStandard: s.\n\n- A) Answer A.\n  Why: reasons.\n  Costs: costs.\n- B) Answer B.\n  Why: other reasons.\n  Costs: other costs.\n";

const REL: &str = "adrdom/decisions/example.md";

fn ledger(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap()
}

fn commit(dir: &Path) {
    run_git(dir, &["add", "-A"]);
    let out = run_git(dir, &["commit", "-q", "--no-verify", "-m", "state"]);
    assert!(out.status.success(), "commit: {}", stderr(&out));
}

fn setup(content: &str, ask: bool) -> (PathBuf, PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, REL, content);
    approve_record(&dir, REL);
    if !ask {
        // Approve asks every question of the file; the state "approved and
        // never asked" is the ledger without its `asked` rows.
        let ledger_path = dir.join("adrdom/state/ledger.tsv");
        let kept: String = std::fs::read_to_string(&ledger_path)
            .unwrap()
            .lines()
            .filter(|l| !l.contains("\tasked\t"))
            .map(|l| format!("{}\n", l))
            .collect();
        std::fs::write(&ledger_path, kept).unwrap();
    }
    (dir, record)
}

/// The owner's approve of the removed file comes only after the agent's
/// move, never on its own.
fn assert_owner_follows_agent(text: &str) {
    let owner = format!("owner runs: adr approve {}", REL);
    let at = text
        .find(&owner)
        .unwrap_or_else(|| panic!("no owner command: {}", text));
    let agent = text
        .find("agent restores")
        .unwrap_or_else(|| panic!("no agent move: {}", text));
    assert!(
        agent < at,
        "owner's approve before the agent's move: {}",
        text
    );
}

fn approve_removal(dir: &Path) -> std::process::Output {
    adr_owner(dir, &["approve", dir.join(REL).to_str().unwrap()], "")
}

#[test]
fn a_removed_file_holding_only_a_refused_question_is_approved_away() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, true);
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "refuse").status.success());
    commit(&dir);
    std::fs::remove_file(&record).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(
        text.contains(&format!("adr approve {}", REL)),
        "check must name the command: {}",
        text
    );

    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(out.status.success(), "approve: {}", stderr(&out));
    let after = ledger(&dir);
    let new: Vec<&str> = after.lines().skip(before.lines().count()).collect();
    assert!(
        new.iter()
            .any(|l| l.contains("\twithdrawn\t") && l.contains(id.as_str())),
        "{:?}",
        new
    );
    assert!(
        new.iter()
            .any(|l| l.contains("\tapproved\t") && l.contains(REL)),
        "{:?}",
        new
    );
    assert!(
        new.iter()
            .any(|l| l.contains("\tcontext\t") && l.contains(REL)),
        "{:?}",
        new
    );

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        out.status.success(),
        "check: {}{}",
        stdout(&out),
        stderr(&out)
    );
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(!text.contains(id.as_str()), "{}", text);
    assert!(!text.contains("awaiting approval; owner runs"), "{}", text);

    // A second approve changes nothing.
    assert!(approve_removal(&dir).status.success());
    assert_eq!(ledger(&dir), after);

    // A file created again at the path is an ordinary draft.
    write_record(&dir, REL, ONE);
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert!(
        !stdout(&out).contains("awaiting approval"),
        "a draft's status is no line of check's: {}",
        stdout(&out)
    );
}

#[test]
fn a_removed_file_holding_only_superseded_sections_is_approved_away() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, true);
    let old_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &old_id, "A").status.success());
    let b = format!(
        "# B\n\n### 2.1 Replacement in another file?\ndepends: decision\n\nsupersedes: {}\n\nProblem: the decision needs revisiting.\n\nStandard: s2.\n\n- A) New answer.\n  Why: reasons.\n  Costs: costs.\n- B) Other new answer.\n  Why: other reasons.\n  Costs: other costs.\n",
        old_id
    );
    let rb = write_record(&dir, "adrdom/decisions/b.md", &b);
    approve_record(&dir, "adrdom/decisions/b.md");
    let new_id = extract_ids(&std::fs::read_to_string(&rb).unwrap())[0].clone();
    assert!(adr_answer(&dir, &new_id, "A").status.success());
    write_test_comment(&dir, &new_id);
    commit(&dir);
    std::fs::remove_file(&record).unwrap();

    // Nothing holds the removal: it is a status, no line of check's
    // (bd83aa22); the file stands removed until the owner approves.
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert!(!stdout(&out).contains(REL), "{}", stdout(&out));
    let shown = adr(&dir, &["show"], &[], "");
    assert!(
        stdout(&shown).contains(&format!("[removed] {}", REL)),
        "{}",
        stdout(&shown)
    );

    let out = approve_removal(&dir);
    assert!(out.status.success(), "approve: {}", stderr(&out));
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert!(
        !stdout(&out).contains(&format!("adr approve {}", REL)),
        "{}",
        stdout(&out)
    );
}

#[test]
fn approve_refuses_the_removal_of_a_file_holding_a_current_decision() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, true);
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    commit(&dir);
    std::fs::remove_file(&record).unwrap();

    // check and show name the agent's move, never the owner's refused approve.
    let out = adr(&dir, &["check"], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(
        text.contains("agent restores") && text.contains(&format!("supersedes {}", id)),
        "{}",
        text
    );
    assert_owner_follows_agent(&text);
    let out = adr(&dir, &["show"], &[], "");
    let text = stdout(&out);
    assert!(
        text.contains("agent restores") && text.contains(&format!("supersedes {}", id)),
        "{}",
        text
    );
    assert_owner_follows_agent(&text);

    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains(&id) && err.contains("Only question?"),
        "{}",
        err
    );
    assert!(err.contains("agent restores"), "{}", err);
    assert!(
        err.contains(&format!("owner runs: adr approve {}", REL)),
        "{}",
        err
    );
    assert_eq!(ledger(&dir), before);
}

#[test]
fn approve_refuses_the_removal_of_a_file_holding_an_asked_question() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, true);
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    commit(&dir);
    std::fs::remove_file(&record).unwrap();

    // check names the file for the asked id, not the bare "not found", and
    // the agent's move: the owner's approve would be refused.
    let out = adr(&dir, &["check"], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(
        text.contains(&id) && text.contains(&format!("agent restores {}", REL)),
        "{}",
        text
    );
    assert_owner_follows_agent(&text);
    assert!(!text.contains("ledger id"), "{}", text);
    let out = adr(&dir, &["show"], &[], "");
    assert_owner_follows_agent(&stdout(&out));

    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains(&id) && err.contains("asked and not answered"),
        "{}",
        err
    );
    assert_eq!(ledger(&dir), before);
}

#[test]
fn approve_refuses_the_removal_of_a_file_holding_an_approved_unasked_heading() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(TWO, false);
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    commit(&dir);
    std::fs::remove_file(&record).unwrap();
    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("First question?") && err.contains("Second question?"),
        "{}",
        err
    );
    assert!(err.contains(&ids[0]) && err.contains(&ids[1]), "{}", err);
    assert_eq!(ledger(&dir), before);
}

#[test]
fn approve_refuses_a_path_never_approved_and_absent() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, _record) = setup(ONE, false);
    let before = ledger(&dir);
    let out = adr_owner(
        &dir,
        &[
            "approve",
            dir.join("adrdom/decisions/nothing.md").to_str().unwrap(),
        ],
        "",
    );
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("nothing to approve"),
        "{}",
        stderr(&out)
    );
    assert_eq!(ledger(&dir), before);
}

#[test]
fn show_and_plan_do_not_crash_on_a_removed_file() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, true);
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "refuse").status.success());
    commit(&dir);
    std::fs::remove_file(&record).unwrap();
    let out = adr(&dir, &["show"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains(&format!("adr approve {}", REL)),
        "{}",
        stdout(&out)
    );
    let out = adr(&dir, &["plan", dir.join(REL).to_str().unwrap()], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert_ne!(
        out.status.code(),
        Some(101),
        "plan must not panic: {}",
        text
    );
    // A named refusal, an exit code of 1: the file is not there to plan.
    assert_eq!(out.status.code(), Some(1), "{}", text);
    assert!(text.contains(&format!("plan: cannot read {}:", REL)) && text.contains("agent rewrites the topic file given to adr plan to one under adrdom/decisions that exists"), "{}", text);
}

#[test]
fn a_question_added_asked_and_uncommitted_holds_the_removal() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, true);
    let first = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &first, "refuse").status.success());
    commit(&dir);

    // Q2 is added, approved and asked, and never committed.
    let both = format!("{}{}", std::fs::read_to_string(&record).unwrap(), SECOND);
    write_record(&dir, REL, &both);
    approve_record(&dir, REL);
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    let second = ids.iter().find(|i| **i != first).unwrap().clone();
    std::fs::remove_file(&record).unwrap();

    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(!out.status.success(), "{}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(&second) && err.contains("Added question?"),
        "{}",
        err
    );
    assert_eq!(ledger(&dir), before);

    // check and show agree: the agent's move, not the owner's.
    let out = adr(&dir, &["check"], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert_owner_follows_agent(&text);
    let out = adr(&dir, &["show"], &[], "");
    assert_owner_follows_agent(&stdout(&out));
}

#[test]
fn approving_an_unrelated_file_leaves_the_ids_of_a_removed_file_alone() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, true);
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();

    // A live file with two asked questions, the second then struck from its
    // edited plan: an asked id in no file and tied to no removed file.
    let live_text = TWO
        .replace("# Example", "# Live")
        .replace("1.1 First", "4.1 Live first")
        .replace("1.2 Second", "4.2 Live second");
    let live = write_record(&dir, "adrdom/decisions/live.md", &live_text);
    approve_record(&dir, "adrdom/decisions/live.md");
    let live_now = std::fs::read_to_string(&live).unwrap();
    let struck = extract_ids(&live_now)[1].clone();
    std::fs::write(&live, &live_now[..live_now.find("### 4.2").unwrap()]).unwrap();

    commit(&dir);
    std::fs::remove_file(&record).unwrap();

    let other = "# Other\n\n### 3.1 Unrelated decision?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) Answer A.\n  Why: reasons.\n  Costs: costs.\n- B) Answer B.\n  Why: other reasons.\n  Costs: other costs.\n";
    write_record(&dir, "adrdom/decisions/other.md", other);
    approve_record(&dir, "adrdom/decisions/other.md");
    assert!(
        !ledger(&dir)
            .lines()
            .any(|l| l.contains("\twithdrawn\t") && l.contains(id.as_str())),
        "the removed file's asked id must not be withdrawn by another file's approval"
    );
    // Positive control: the same approve withdraws the struck live id.
    assert!(
        ledger(&dir)
            .lines()
            .any(|l| l.contains("\twithdrawn\t") && l.contains(struck.as_str())),
        "an asked id in no file and tied to no removed file is withdrawn: {}",
        ledger(&dir)
    );

    // The removal is still refused for the open question.
    let out = approve_removal(&dir);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("asked and not answered"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn deleting_a_draft_never_approved_needs_no_owner_act() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, REL, ONE);
    commit(&dir);
    std::fs::remove_file(&record).unwrap();

    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stderr(&out).contains("never approved") && stderr(&out).contains("nothing to approve"),
        "{}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("agent runs: git commit"),
        "{}",
        stderr(&out)
    );
    assert_eq!(ledger(&dir), before);

    let out = adr(&dir, &["check"], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(
        !text.contains(REL),
        "check says nothing of a removed draft: {}",
        text
    );
    let out = adr(&dir, &["show"], &[], "");
    assert!(!stdout(&out).contains(REL), "{}", stdout(&out));
}

#[test]
fn a_heading_approved_never_asked_and_struck_does_not_hold_the_removal() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, false);
    let text = std::fs::read_to_string(&record).unwrap();
    // Q2 is added and approved, never asked, then struck by a later approve.
    write_record(&dir, REL, &format!("{}{}", text, SECOND));
    approve_record(&dir, REL);
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    let cut = std::fs::read_to_string(&record).unwrap();
    let cut = cut[..cut.find("### 1.2").unwrap()].trim_end().to_string() + "\n";
    std::fs::write(&record, &cut).unwrap();
    approve_record(&dir, REL);
    commit(&dir);
    std::fs::remove_file(&record).unwrap();

    let out = approve_removal(&dir);
    let err = stderr(&out);
    // Q1 still holds the removal; the struck Q2 does not.
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(err.contains(&ids[0]) && !err.contains(&ids[1]), "{}", err);

    // After Q1 is refused, the removal passes.
    std::fs::write(&record, &cut).unwrap();
    assert!(adr_answer(&dir, &ids[0], "refuse").status.success());
    commit(&dir);
    std::fs::remove_file(&record).unwrap();
    let out = approve_removal(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn a_committed_deletion_still_holds_an_approved_unasked_heading() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(TWO, false);
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    commit(&dir);
    std::fs::remove_file(&record).unwrap();
    // The deletion is committed: HEAD has no copy, history does.
    commit(&dir);
    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(!out.status.success(), "{}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("First question?") && err.contains("Second question?"),
        "{}",
        err
    );
    assert!(err.contains(&ids[0]) && err.contains(&ids[1]), "{}", err);
    assert_eq!(ledger(&dir), before);
}

#[test]
fn a_heading_struck_by_a_committed_plan_change_does_not_hold_a_committed_deletion() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(TWO, false);
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    commit(&dir);
    let text = std::fs::read_to_string(&record).unwrap();
    let kept = text[..text.find("### 1.2").unwrap()].trim_end().to_string() + "\n";
    // The entry naming the struck heading goes with it.
    let kept: String = kept
        .lines()
        .filter(|l| !l.starts_with("unrelated:"))
        .map(|l| format!("{}\n", l))
        .collect();
    std::fs::write(&record, &kept).unwrap();
    approve_record(&dir, REL);
    commit(&dir);
    std::fs::remove_file(&record).unwrap();
    // The deletion is committed: HEAD has no copy, only the walk through
    // history finds the version the strike approved.
    commit(&dir);

    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(!out.status.success(), "{}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(&ids[0]) && err.contains("First question?"),
        "{}",
        err
    );
    assert!(
        !err.contains(&ids[1]) && !err.contains("Second question?"),
        "{}",
        err
    );
    assert_eq!(ledger(&dir), before);
}

#[test]
fn check_names_each_id_of_a_committed_deletion_once_with_the_removal_move() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(TWO, true);
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    assert!(adr_answer(&dir, &ids[0], "A").status.success());
    commit(&dir);
    std::fs::remove_file(&record).unwrap();
    commit(&dir);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = format!("{}{}", stdout(&out), stderr(&out));
    for id in &ids {
        // A failure line is about the id it opens with; the move names the others.
        let lines: Vec<&str> = text
            .lines()
            .filter(|l| {
                !l.starts_with("plan:")
                    && l.split_once(": ")
                        .is_some_and(|(_, rest)| rest.starts_with(id.as_str()))
            })
            .collect();
        assert_eq!(
            lines.len(),
            1,
            "{} named {} times: {}",
            id,
            lines.len(),
            text
        );
        assert!(
            lines[0].contains("agent restores")
                && lines[0].contains(&format!("owner runs: adr approve {}", REL)),
            "{}",
            lines[0]
        );
    }
}

#[test]
fn an_uncommitted_approval_of_a_strike_falls_back_to_the_newest_committed_plan() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(TWO, false);
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    commit(&dir);
    let text = std::fs::read_to_string(&record).unwrap();
    let kept = text[..text.find("### 1.2").unwrap()].trim_end().to_string() + "\n";
    let kept: String = kept
        .lines()
        .filter(|l| !l.starts_with("unrelated:"))
        .map(|l| format!("{}\n", l))
        .collect();
    std::fs::write(&record, &kept).unwrap();
    approve_record(&dir, REL);
    // Not committed: no committed version matches the latest approval, so the
    // newest committed version stands in and still names the struck heading.
    std::fs::remove_file(&record).unwrap();
    let out = approve_removal(&dir);
    assert!(!out.status.success(), "{}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains(&ids[0]) && err.contains(&ids[1]), "{}", err);
}

#[test]
fn an_approved_removal_leaves_check_silent_about_the_path_and_a_second_approve_writes_nothing() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, true);
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "refuse").status.success());
    commit(&dir);
    std::fs::remove_file(&record).unwrap();
    assert!(approve_removal(&dir).status.success());

    let out = adr(&dir, &["check"], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(out.status.success(), "check: {}", text);
    assert!(
        !text.contains(REL),
        "check names the removed path: {}",
        text
    );

    let after = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(out.status.success(), "approve: {}", stderr(&out));
    assert_eq!(ledger(&dir), after);
}
