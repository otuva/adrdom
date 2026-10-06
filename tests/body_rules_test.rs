// The body rules `ask` refuses, found before approval: a loaded phrase in a
// question's body, and a question none of whose words pair to a term. Plan
// and check report them, approve stops them.
mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/example.md";

fn question(heading: &str, problem: &str) -> String {
    format!(
        "# Example\n\n### {}\ndepends: decision\n\nProblem: {}\n\nStandard: plain words.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        heading, problem
    )
}

fn unapproved(content: &str) -> (PathBuf, PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision"]);
    let rec = write_record(&dir, REL, content);
    (dir, rec)
}

fn both(o: &std::process::Output) -> String {
    format!("{}{}", stdout(o), stderr(o))
}

/// What check, approve and plan each print on the file.
fn three(dir: &Path, rec: &Path) -> (String, String, String) {
    let chk = adr(dir, &["check"], &[], "");
    assert!(!chk.status.success(), "{}", both(&chk));
    let apr = adr_owner(dir, &["approve", rec.to_str().unwrap()], "");
    assert!(!apr.status.success(), "{}", both(&apr));
    let pln = both(&adr(dir, &["plan", rec.to_str().unwrap()], &[], ""));
    assert!(!pln.contains("next: owner runs: adr approve"), "{}", pln);
    (both(&chk), stderr(&apr), pln)
}

// adr: 923818a7-05c6-4977-bbaf-53ee8f3773ef
#[test]
fn check_approve_and_plan_find_a_loaded_phrase_in_a_body() {
    let (dir, rec) = unapproved(&question("Does the decision hold?", "the decision currently holds."));
    let (chk, apr, pln) = three(&dir, &rec);
    for out in [&chk, &apr, &pln] {
        assert!(out.contains("\"currently\" states what the code does now"), "{}", out);
    }
    assert!(apr.contains("agent rewrites the line"), "{}", apr);
}

// adr: 711a7636-9f4d-402d-ae62-265208adf0ac
#[test]
fn check_approve_and_plan_find_a_question_that_pairs_no_term() {
    let (dir, rec) = unapproved(&question("Does the lid hold?", "the lid is unclear."));
    let (chk, apr, pln) = three(&dir, &rec);
    for out in [&chk, &apr, &pln] {
        assert!(out.contains("\"Does the lid hold?\" uses no term"), "{}", out);
    }
    assert!(apr.contains("agent adds the term to adrdom/terms.md or rewrites the question"), "{}", apr);
}

// adr: 711a7636-9f4d-402d-ae62-265208adf0ac
#[test]
fn a_heading_with_no_body_is_no_question_yet() {
    let (dir, _) = unapproved("# Example\n\n### Does the lid hold?\ndepends: decision\n");
    let out = both(&adr(&dir, &["check"], &[], ""));
    assert!(!out.contains("uses no term"), "{}", out);
}

// adr: 711a7636-9f4d-402d-ae62-265208adf0ac
#[test]
fn a_question_with_a_term_passes() {
    let (dir, rec) = unapproved(&question("Does the decision hold?", "the decision is unclear."));
    let out = adr_owner(&dir, &["approve", rec.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", both(&out));
}

// adr: 923818a7-05c6-4977-bbaf-53ee8f3773ef
#[test]
fn an_asked_question_is_not_read_for_loaded_phrases() {
    let (dir, rec) = unapproved(&question("Does the decision hold?", "the decision is unclear."));
    approve_record(&dir, REL);
    // The asked text is held by hash and cannot be edited, so a phrase the
    // list grew to cover later must not fail check or approve on it: model it
    // as an asked row for a body that carries a loaded phrase.
    let text = std::fs::read_to_string(&rec).unwrap().replace("is unclear.", "currently holds.");
    std::fs::write(&rec, &text).unwrap();
    let rf = adr::record::load(&rec).unwrap();
    let sec = &rf.sections[0];
    let id = sec.id.clone().expect("approve gave the heading an id");
    let hash = adr::record::section_hash(&rf.lines, sec);
    append_ledger_row(&dir, "asked", &id, &hash, "Does the decision hold?");
    let out = both(&adr(&dir, &["check"], &[], ""));
    assert!(!out.contains("states what the code does now"), "{}", out);
    let out = both(&adr_owner(&dir, &["approve", rec.to_str().unwrap()], ""));
    assert!(!out.contains("states what the code does now"), "{}", out);
}

fn options() -> &'static str {
    "- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n"
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
#[test]
fn check_approve_and_plan_find_a_set_on_a_term() {
    let (dir, _) = unapproved("# Example\n");
    let terms = std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap();
    let id = terms.lines().find_map(|l| l.strip_prefix("id: ")).expect("approved term has an id").trim().to_string();
    let content = format!(
        "# Example\n\n### Does the decision hold?\ndepends: {}=A\n\nProblem: the decision is unclear.\n\nStandard: plain words.\n\n{}",
        id,
        options()
    );
    let rec = write_record(&dir, REL, &content);
    let (chk, apr, pln) = three(&dir, &rec);
    for out in [&chk, &apr, &pln] {
        assert!(out.contains("depends on a term under an answer set, and a term has no options"), "{}", out);
    }
    assert_eq!(apr.matches("under an answer set").count(), 1, "{}", apr);
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
#[test]
fn check_approve_and_plan_find_an_entry_with_no_set_on_an_unanswered_question() {
    let content = format!(
        "# Example\n\n### Does the decision hold?\ndepends: decision\n\nProblem: the decision is unclear.\n\nStandard: plain words.\n\n{o}\n### Does the decision stay?\ndepends: Does the decision hold?\n\nProblem: the decision is unclear.\n\nStandard: plain words.\n\n{o}",
        o = options()
    );
    let (dir, rec) = unapproved(&content);
    let (chk, apr, pln) = three(&dir, &rec);
    for out in [&chk, &apr, &pln] {
        assert!(
            out.contains("\"Does the decision stay?\" depends on \"Does the decision hold?\" without an answer set"),
            "{}",
            out
        );
    }
    assert_eq!(apr.matches("without an answer set").count(), 1, "{}", apr);
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
#[test]
fn check_approve_and_plan_find_a_set_on_an_answered_question() {
    let target = "11111111-2222-3333-4444-555555555555";
    let (dir, rec) = unapproved(&format!(
        "# Example\n\n### Does the decision stand?\ndepends: {}=A\n\nProblem: the decision is unclear.\n\nStandard: plain words.\n\n{}",
        target,
        options()
    ));
    append_ledger_row_at(&dir, "asked", target, "h1", "Does the lid stay?", "2024-01-01T00:00:00Z");
    append_ledger_row_at(&dir, "answered", target, "h1", "A", "2024-01-01T00:00:01Z");
    write_record(
        &dir,
        "adrdom/decisions/other.md",
        &format!(
            "# Other\n\n### Does the lid stay?\ndepends: decision\n\nid: {}\n\nProblem: p of the decision.\n\nStandard: s.\n\n{}\n**Answer:** A\n",
            target,
            options()
        ),
    );
    let (chk, apr, pln) = three(&dir, &rec);
    for out in [&chk, &apr, &pln] {
        assert!(
            out.contains("\"Does the decision stand?\" depends on \"Does the lid stay?\", which is answered, under an answer set"),
            "{}",
            out
        );
    }
    assert_eq!(apr.matches("which is answered, under an answer set").count(), 1, "{}", apr);
}
