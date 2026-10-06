mod common;

use common::*;
use std::path::{Path, PathBuf};

// adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
// A whole topic file leaves by a plan change the owner approves, once it
// holds no current decision and no open question.

const ONE: &str = "# Example\n\nContext prose.\n\n### 1.1 Only question?\ndepends: decision\n\nProblem: the decision at hand.\n\nStandard: s.\n\n- A) Answer A.\n  Why: reasons.\n  Costs: costs.\n- B) Answer B.\n  Why: other reasons.\n  Costs: other costs.\n";

const TWO: &str = "# Example\n\nContext prose.\n\n### 1.1 First question?\ndepends: decision\n\nProblem: the decision at hand.\n\nStandard: s.\n\n- A) Answer A.\n  Why: reasons.\n  Costs: costs.\n- B) Answer B.\n  Why: other reasons.\n  Costs: other costs.\n\n### 1.2 Second question?\ndepends: decision\n\nProblem: another decision.\n\nStandard: s.\n\n- A) Answer A.\n  Why: reasons.\n  Costs: costs.\n- B) Answer B.\n  Why: other reasons.\n  Costs: other costs.\n";

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
    if ask {
        assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    }
    (dir, record)
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
    assert!(text.contains(&format!("adr approve {}", REL)), "check must name the command: {}", text);

    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(out.status.success(), "approve: {}", stderr(&out));
    let after = ledger(&dir);
    let new: Vec<&str> = after.lines().skip(before.lines().count()).collect();
    assert!(new.iter().any(|l| l.contains("\twithdrawn\t") && l.contains(id.as_str())), "{:?}", new);
    assert!(new.iter().any(|l| l.contains("\tapproved\t") && l.contains(REL)), "{:?}", new);
    assert!(new.iter().any(|l| l.contains("\tcontext\t") && l.contains(REL)), "{:?}", new);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check: {}{}", stdout(&out), stderr(&out));
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
    assert!(stdout(&out).contains(REL), "draft is awaiting approval: {}", stdout(&out));
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
    assert!(adr(&dir, &["ask", rb.to_str().unwrap()], &[], "").status.success());
    let new_id = extract_ids(&std::fs::read_to_string(&rb).unwrap())[0].clone();
    assert!(adr_answer(&dir, &new_id, "A").status.success());
    write_test_comment(&dir, &new_id);
    commit(&dir);
    std::fs::remove_file(&record).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains(&format!("adr approve {}", REL)), "{}", stdout(&out));

    let out = approve_removal(&dir);
    assert!(out.status.success(), "approve: {}", stderr(&out));
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert!(!stdout(&out).contains(&format!("adr approve {}", REL)), "{}", stdout(&out));
}

#[test]
fn approve_refuses_the_removal_of_a_file_holding_a_current_decision() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, true);
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    commit(&dir);
    std::fs::remove_file(&record).unwrap();
    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains(&id) && err.contains("Only question?"), "{}", err);
    assert_eq!(ledger(&dir), before);
}

#[test]
fn approve_refuses_the_removal_of_a_file_holding_an_asked_question() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, record) = setup(ONE, true);
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    commit(&dir);
    std::fs::remove_file(&record).unwrap();

    // check names the file for the asked id, not the bare "not found".
    let out = adr(&dir, &["check"], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(text.contains(&id) && text.contains(&format!("adr approve {}", REL)), "{}", text);
    assert!(!text.contains("ledger id"), "{}", text);

    let before = ledger(&dir);
    let out = approve_removal(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains(&id) && err.contains("asked and not answered"), "{}", err);
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
    assert!(err.contains("First question?") && err.contains("Second question?"), "{}", err);
    assert!(err.contains(&ids[0]) && err.contains(&ids[1]), "{}", err);
    assert_eq!(ledger(&dir), before);
}

#[test]
fn approve_refuses_a_path_never_approved_and_absent() {
    // adr: 79da6500-2246-4a80-baf1-0eaf7cad49ee
    let (dir, _record) = setup(ONE, false);
    let before = ledger(&dir);
    let out = adr_owner(&dir, &["approve", dir.join("adrdom/decisions/nothing.md").to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("not a topic file"), "{}", stderr(&out));
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
    assert!(stdout(&out).contains(&format!("adr approve {}", REL)), "{}", stdout(&out));
    let out = adr(&dir, &["plan", dir.join(REL).to_str().unwrap()], &[], "");
    assert!(out.status.code().is_some(), "plan must exit normally");
}
