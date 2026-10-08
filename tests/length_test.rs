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

fn is_body_line(l: &str) -> bool {
    let t = l.trim_start();
    t.starts_with("- A) ") || t.starts_with("- B) ") || t.starts_with("Why:") || t.starts_with("Costs:")
}

fn shortened(content: &str) -> String {
    content
        .lines()
        .map(|l| {
            let t = l.trim_start();
            let ind = &l[..l.len() - t.len()];
            if t.starts_with("- A) ") || t.starts_with("- B) ") {
                format!("{}{}Yes.", ind, &t[..5])
            } else if t.starts_with("Why:") {
                format!("{}Why: w.", ind)
            } else if t.starts_with("Costs:") {
                format!("{}Costs: c.", ind)
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Approve takes the short version (approve refuses the long one), then the
/// body lines are put back, so ask meets them.
fn setup(content: &str) -> (PathBuf, PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision"]);
    let rec = write_record(&dir, "adrdom/decisions/example.md", &shortened(content));
    approve_record(&dir, "adrdom/decisions/example.md");
    let mut orig = content.lines().filter(|l| is_body_line(l));
    let text = std::fs::read_to_string(&rec).unwrap();
    let back: Vec<String> = text
        .lines()
        .map(|l| if is_body_line(l) { orig.next().unwrap().to_string() } else { l.to_string() })
        .collect();
    // Approve cannot take these lines, so the approval rows are written by
    // hand: ask is the second stop for text that reaches it.
    std::fs::write(&rec, back.join("\n") + "\n").unwrap();
    let rf = adr::record::load(&rec).unwrap();
    let rel = "adrdom/decisions/example.md";
    let later = "2099-01-01T00:00:00Z";
    append_ledger_row_at(&dir, "approved", "-", &adr::record::plan_hash(&rf), rel, later);
    append_ledger_row_at(&dir, "context", "-", &adr::record::context_hash(&rf), rel, later);
    (dir, rec)
}

fn unapproved(content: &str) -> (PathBuf, PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision"]);
    let rec = write_record(&dir, "adrdom/decisions/example.md", content);
    (dir, rec)
}

fn both(o: &std::process::Output) -> String {
    format!("{}{}", stdout(o), stderr(o))
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
        assert!(err.contains("agent rewrites the "), "{}", err);
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
    assert!(stderr(&out).contains("an option under a Does heading holds one word"), "{}", stderr(&out));

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

#[test]
fn the_word_limit_holds_for_the_why_line() {
    // adr: 0487a1cf-eb61-4bf8-bd16-eed899d22d2c
    assert!(!refused_for(&question("What does the decision hold?", "Short.", &words(20), "c."), "Why line"));
    assert!(refused_for(&question("What does the decision hold?", "Short.", &words(21), "c."), "Why line"));
}

#[test]
fn the_word_limit_holds_for_the_costs_line() {
    // adr: c96518bf-470d-45c0-8b86-eaf8f54cb490
    assert!(!refused_for(&question("What does the decision hold?", "Short.", "w.", &words(20)), "Costs line"));
    assert!(refused_for(&question("What does the decision hold?", "Short.", "w.", &words(21)), "Costs line"));
}

#[test]
fn the_word_limit_does_not_read_an_answered_question() {
    // adr: 55a8abb2-0f6d-430b-99f9-a378f3f154e5
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
    // The answer comes after the ask, as answer writes it.
    append_ledger_row_at(&dir, "asked", &id, &hash, "-", "2024-01-01T00:00:00Z");
    append_ledger_row_at(&dir, "answered", &id, &hash, "A", "2024-01-01T00:00:01Z");
    let with_answer = format!("{}\n\n**Answer:** A\n{}", &text[..end].trim_end(), &text[end..]);
    std::fs::write(&rec, with_answer).unwrap();

    let out = ask(&dir, &rec);
    assert!(!stderr(&out).contains("the option has"), "{}", stderr(&out));
    assert!(out.status.success(), "{}", stderr(&out));
    // check never reads the answered question either.
    let chk = both(&adr(&dir, &["check"], &[], ""));
    assert!(!chk.contains("the option has 30"), "{}", chk);
}

// adr: d808917b-0717-45ad-ab83-9fa274b9a9d4
#[test]
fn check_approve_and_plan_find_a_too_long_option_before_approval() {
    let long = question("What does the decision hold?", &words(25), "w.", "c.");
    let (dir, rec) = unapproved(&long);
    let out = both(&adr(&dir, &["check"], &[], ""));
    assert!(out.contains("the option has 25 words"), "{}", out);
    let out = adr_owner(&dir, &["approve", rec.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("the option has 25 words"), "{}", stderr(&out));
    assert!(stderr(&out).contains("agent rewrites the "), "{}", stderr(&out));
    let out = adr(&dir, &["plan", rec.to_str().unwrap()], &[], "");
    let all = format!("{}{}", stdout(&out), stderr(&out));
    assert!(all.contains("the option has 25 words"), "{}", all);
    assert!(!all.contains("owner runs: adr approve"), "{}", all);
}

// adr: b3cb190d-d6f1-4930-bdcd-7827e31ac62a
#[test]
fn check_and_approve_find_a_many_word_option_under_a_does_heading() {
    let (dir, rec) = unapproved(&question("Does the decision hold?", "Yes: because.", "w.", "c."));
    let out = both(&adr(&dir, &["check"], &[], ""));
    assert!(out.contains("an option under a Does heading holds one word"), "{}", out);
    let out = adr_owner(&dir, &["approve", rec.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("agent rewrites the option"), "{}", stderr(&out));
}

// adr: d808917b-0717-45ad-ab83-9fa274b9a9d4
#[test]
fn plan_reports_a_too_long_option_while_another_heading_waits_for_its_body() {
    let long = question("What does the decision hold?", &words(25), "w.", "c.");
    let content = format!("{}\n### What does the decision name?\ndepends: decision\n", long);
    let (dir, rec) = unapproved(&content);
    let out = adr(&dir, &["plan", rec.to_str().unwrap()], &[], "");
    let all = both(&out);
    // Lines first: the neighbours of each heading come with no line yet.
    assert!(all.contains("the option has 25 words"), "{}", all);
}
