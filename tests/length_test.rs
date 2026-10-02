// The length rules of a question's options: what `ask` refuses on a
// question not yet answered. Each fixture goes through a real approve, then
// ask.
mod common;

use common::*;
use std::path::PathBuf;

fn words(n: usize) -> String {
    format!("{}.", vec!["word"; n].join(" "))
}

fn question(heading: &str, a: &str, a_why: &str, a_costs: &str) -> String {
    format!(
        "# Example\n\n### {}\ndepends: decision\n\nProblem: a decision is unclear.\n\nStandard: plain words.\n\n- A) {}\n  Why: {}\n  Costs: {}\n- B) No.\n  Why: w.\n  Costs: c.\n",
        heading, a, a_why, a_costs
    )
}

fn setup(content: &str) -> (PathBuf, PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision"]);
    let rec = write_record(&dir, "adrdom/decisions/example.md", content);
    approve_record(&dir, "adrdom/decisions/example.md");
    (dir, rec)
}

fn ask(dir: &PathBuf, rec: &PathBuf) -> std::process::Output {
    adr(dir, &["ask", rec.to_str().unwrap()], &[], "")
}

fn refused_for(content: &str, kind: &str) -> bool {
    let (dir, rec) = setup(content);
    let out = ask(&dir, &rec);
    let err = stderr(&out);
    let refused = !out.status.success() && err.contains(&format!("the {} has", kind));
    if refused {
        assert!(err.contains("agent shortens the line"), "{}", err);
        assert_eq!(asked_row_count(&dir), 0, "a refusal writes no asked row");
    }
    refused
}

// adr: b3cb190d-d6f1-4930-bdcd-7827e31ac62a
#[test]
fn an_option_under_a_does_heading_holds_one_word() {
    let (dir, rec) = setup(&question("Does the decision hold?", "Yes: because.", "w.", "c."));
    let out = ask(&dir, &rec);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("a heading that opens with Does takes one"), "{}", stderr(&out));

    let (dir, rec) = setup(&question("Does the decision hold?", "Yes.", "w.", "c."));
    let out = ask(&dir, &rec);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(asked_row_count(&dir), 1);

    // A heading that does not open with Does takes a sentence.
    let (dir, rec) = setup(&question("What does the decision hold?", "It holds the rule.", "w.", "c."));
    assert!(ask(&dir, &rec).status.success());
    let _ = dir;
}

// adr: d808917b-0717-45ad-ab83-9fa274b9a9d4
#[test]
fn ask_refuses_an_option_over_the_word_limit() {
    assert!(refused_for(&question("What does the decision hold?", &words(25), "w.", "c."), "option"));
}

// adr: 8f324216-80a3-4012-b689-e27b6eea7714
#[test]
fn the_word_limit_of_an_option_is_twenty() {
    assert_eq!(adr::tells::WORD_LIMIT, 20);
    assert!(!refused_for(&question("What does the decision hold?", &words(20), "w.", "c."), "option"));
    assert!(refused_for(&question("What does the decision hold?", &words(21), "w.", "c."), "option"));
}

// adr: 39b76886-98f9-4019-9194-db9ab51aea1e
#[test]
fn the_word_limit_holds_for_the_why_line() {
    assert!(!refused_for(&question("What does the decision hold?", "Short.", &words(20), "c."), "Why line"));
    assert!(refused_for(&question("What does the decision hold?", "Short.", &words(21), "c."), "Why line"));
}

// adr: 32a189a0-86e0-44d1-9392-a50c84570ada
#[test]
fn the_word_limit_holds_for_the_costs_line() {
    assert!(!refused_for(&question("What does the decision hold?", "Short.", "w.", &words(20)), "Costs line"));
    assert!(refused_for(&question("What does the decision hold?", "Short.", "w.", &words(21)), "Costs line"));
}

// adr: 423065fc-6d9f-40c8-a0bf-8e1779e86b38
#[test]
fn the_word_limit_does_not_read_an_answered_question() {
    // The first question was answered with an option over the limit (hand-
    // written ledger rows: ask would have refused it). Asking the second
    // question still passes and never names the first.
    let first = question("What does the decision hold?", &words(30), "w.", "c.");
    let second = "\n### What else does the decision hold?\ndepends: What does the decision hold?=A\n\nProblem: a decision is unclear.\n\nStandard: plain words.\n\n- A) Short.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";
    let (dir, rec) = setup(&format!("{}{}", first, second));
    let text = std::fs::read_to_string(&rec).unwrap();
    let start = text.find("### What does the decision hold?").unwrap();
    let end = text.find("\n### What else").unwrap();
    let section = text[start..end].trim_end().to_string();
    let id = extract_ids(&section)[0].clone();
    let hash = mirror_section_hash(&section);
    append_ledger_row(&dir, "asked", &id, &hash, "-");
    append_ledger_row(&dir, "answered", &id, &hash, "A");
    let with_answer = format!("{}\n\n**Answer:** A\n{}", &text[..end].trim_end(), &text[end..]);
    std::fs::write(&rec, with_answer).unwrap();

    let out = ask(&dir, &rec);
    assert!(!stderr(&out).contains("the option has"), "{}", stderr(&out));
    assert!(out.status.success(), "{}", stderr(&out));
}
