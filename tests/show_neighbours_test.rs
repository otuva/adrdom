// What `adr show --neighbours` and the page print under each open question:
// one line for each neighbour the reader recorded, with its word at each step
// and the agent's line, a view computed from the records and so printed under
// its flag (per "Does a command print under a flag what it computes from the
// records?"). The embedding model and the reader are the fakes; show and the
// page read the records only.

mod common;

use adr::reader;
use adr::record;
use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/topic.md";
const BASE: &str = "adrdom/decisions/base.md";
const BASE_Q: &str = "Does the base hold?";
const H1: &str = "Does the first widget glow?";
const H2: &str = "Does the second widget glow?";
const NO_TOOLS: [(&str, &str); 2] = [("ADR_NO_MODEL", "1"), ("ADR_NO_READER", "1")];

fn body(n: &str) -> String {
    format!(
        "\nProblem: problem {n}.\n\nStandard: standard {n}.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        n = n
    )
}

fn section(heading: &str, body: &str) -> String {
    format!("\n### {}\ndepends: widget\n{}", heading, body)
}

/// An installed repository with two terms and one current decision.
fn repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget", "paint"]);
    let base = write_record(&dir, BASE, &format!("# Base\n{}", section(BASE_Q, &body("base widget"))));
    approve_record(&dir, BASE);
    assert!(adr(&dir, &["ask", base.to_str().unwrap()], &[], "").status.success());
    let id = extract_ids(&std::fs::read_to_string(&base).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    dir
}

fn write_topic(dir: &Path) {
    let text = format!("# Topic\n{}{}", section(H1, &body("one")), section(H2, &body("two")));
    std::fs::write(dir.join(REL), text).unwrap();
}

fn show(dir: &Path, envs: &[(&str, &str)]) -> String {
    let out = adr(dir, &["show", dir.join(REL).to_str().unwrap(), "--neighbours"], envs, "");
    assert!(out.status.success(), "{}", stderr(&out));
    stdout(&out)
}

/// The lines of `shown` under the question `heading`, up to the next
/// status line or move.
fn under(shown: &str, heading: &str) -> Vec<String> {
    shown
        .lines()
        .skip_while(|l| !l.ends_with(&format!("] {}", heading)))
        .skip(1)
        .take_while(|l| l.starts_with("  "))
        .map(|l| l.trim().to_string())
        .collect()
}

fn base_id(dir: &Path) -> String {
    extract_ids(&std::fs::read_to_string(dir.join(BASE)).unwrap())[0].clone()
}

/// The reader's word for the base decision at both steps of `heading`,
/// written as newer rows than the ones the fake reader made.
fn reader_says(dir: &Path, heading: &str, heading_word: &str, body_word: &str) {
    let rf = record::load(&dir.join(REL)).unwrap();
    let sec = rf.sections.iter().find(|s| s.heading_text == heading).unwrap();
    let id = base_id(dir);
    let mut new = Vec::new();
    for (hash, word) in [(record::heading_hash(sec), heading_word), (record::body_hash(&rf, sec), body_word)] {
        let mut row = reader::read_file(dir)
            .rows
            .into_iter()
            .find(|r| r.text_hash == hash && r.neighbour == id)
            .expect("the fake reader's row");
        row.word = word.to_string();
        row.reason = "The reader's reason.".to_string();
        new.push(row);
    }
    reader::append(dir, &new).unwrap();
}

#[test]
fn show_prints_a_line_for_each_neighbour_with_both_words_and_the_agents_line() {
    let dir = repo();
    write_topic(&dir);
    // Nothing is recorded for a question not yet planned: one line, with the move.
    let shown = show(&dir, &NO_TOOLS);
    for h in [H1, H2] {
        assert_eq!(
            under(&shown, h).last().unwrap(),
            &format!("neighbours: no reader records yet; agent runs: adr plan {}", REL),
            "{}",
            shown
        );
    }

    // The heading step is on record, and no neighbour is on a line but the
    // term the depends line names.
    assert!(adr(&dir, &["plan", dir.join(REL).to_str().unwrap()], &[], "").status.success());
    let shown = show(&dir, &NO_TOOLS);
    let lines = under(&shown, H1);
    assert!(lines.iter().any(|l| l.starts_with(&format!("decision \"{}\": heading unrelated; no line; ", BASE_Q))), "{}", shown);
    assert!(lines.iter().any(|l| l.starts_with("term \"widget\": heading unrelated; on the depends line; ")), "{}", shown);
    assert!(lines.iter().any(|l| l.starts_with("term \"paint\": heading unrelated; no line; ")), "{}", shown);

    // Both steps on record: one line for a neighbour at each step, each with
    // the reader's word and reason and the agent's line.
    plan_record(&dir, REL);
    let shown = show(&dir, &NO_TOOLS);
    let lines = under(&shown, H1);
    assert!(
        lines.iter().any(|l| l.starts_with(&format!("decision \"{}\": body unrelated; on the unrelated line; ", BASE_Q))),
        "{}",
        shown
    );
    assert!(
        lines.iter().any(|l| l.starts_with(&format!("question \"{}\": body unrelated; on the unrelated line; ", H2))),
        "{}",
        shown
    );
    assert!(!lines.iter().any(|l| l.contains("no reader records")), "{}", shown);
    assert_eq!(lines.iter().filter(|l| l.starts_with("decision ") && l.contains(BASE_Q)).count(), 2, "{}", shown);
    assert!(lines.iter().filter(|l| l.starts_with("decision ")).all(|l| l.ends_with("The fake reader's reason.")), "{}", shown);

    // The reader's own words, and a supersedes line.
    reader_says(&dir, H1, "same", "consistent");
    let text = std::fs::read_to_string(dir.join(REL)).unwrap();
    std::fs::write(
        dir.join(REL),
        text.replacen(
            &format!("### {}\ndepends: widget\n", H1),
            &format!("### {}\ndepends: widget\nsupersedes: {}\n", H1, BASE_Q),
            1,
        ),
    )
    .unwrap();
    let shown = show(&dir, &NO_TOOLS);
    assert!(
        under(&shown, H1)
            .iter()
            .any(|l| l.starts_with(&format!("decision \"{}\": heading same; on the supersedes line; ", BASE_Q))),
        "{}",
        shown
    );
    // `--all` prints the same lines under the same flag.
    let all = stdout(&adr(&dir, &["show", "--all", "--neighbours"], &NO_TOOLS, ""));
    assert_eq!(under(&all, H1), under(&shown, H1));
}

#[test]
fn an_asked_question_with_no_records_prints_no_neighbour_line() {
    let dir = repo();
    write_topic(&dir);
    approve_record(&dir, REL);
    let asked = adr(&dir, &["ask", dir.join(REL).to_str().unwrap()], &[], "");
    assert!(asked.status.success(), "{}", stderr(&asked));
    let shown = show(&dir, &NO_TOOLS);
    assert!(
        under(&shown, H1).iter().any(|l| l.starts_with("decision ") || l.starts_with("term ")),
        "an asked question prints its records: {}",
        shown
    );
    // Planned before the records existed: no rows, and no invented line.
    std::fs::write(dir.join("adrdom/state/reader.tsv"), "").unwrap();
    let shown = show(&dir, &NO_TOOLS);
    assert!(shown.contains(&format!("] {}", H1)), "{}", shown);
    assert!(!shown.contains("neighbour"), "{}", shown);
}

#[test]
fn the_next_lines_follow_the_planning_steps() {
    let dir = repo();
    let p = dir.join(REL);
    // The moves: every line that is neither a status line nor under one.
    let next = |dir: &Path| -> Vec<String> {
        show(dir, &NO_TOOLS)
            .lines()
            .filter(|l| !l.starts_with([' ', '[', '+', '-']))
            .map(str::to_string)
            .collect()
    };
    std::fs::write(&p, format!("# Topic\n{}{}", section(H1, ""), section(H2, ""))).unwrap();
    // Each step's findings carry the step's move, one per question (per
    // "Does a command compute a move of its own?", aac5c313 A).
    let ends = |lines: &[String], mv: &dyn Fn(&str) -> String| {
        assert!(!lines.is_empty());
        for h in [H1, H2] {
            assert!(lines.iter().any(|l| l.ends_with(&format!("; {}", mv(h)))), "{} in {:?}", h, lines);
        }
        assert!(lines.iter().all(|l| [H1, H2].iter().any(|h| l.ends_with(&format!("; {}", mv(h))))), "{:?}", lines);
    };
    ends(&next(&dir), &|_| format!("agent runs: adr plan {}", REL));
    assert!(adr(&dir, &["plan", p.to_str().unwrap()], &[], "").status.success());
    ends(&next(&dir), &|h| format!("agent writes lines for \"{}\"", h));
    plan_record(&dir, REL);
    ends(&next(&dir), &|h| format!("agent writes the body of \"{}\"", h));
    // The bodies written over a file without the lines: the lines come first.
    write_topic(&dir);
    ends(&next(&dir), &|h| format!("agent writes lines for \"{}\"", h));
    plan_record(&dir, REL);
    assert_eq!(next(&dir), vec![format!("owner runs: adr approve {}", REL)]);
}
