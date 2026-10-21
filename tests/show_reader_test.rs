// What `adr show` prints of a reader's word on a question (per "What does
// show print of a reader's word on a question?"): under a question not yet
// answered, a line for each neighbour the reader marked inconsistent, same or
// closes with a letter that no supersedes line names; every word, with its
// reason, under --reader. A view: no move stands on a line. The embedding model and the
// reader are the fakes; the rows are then set to the words the test wants.

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

/// A repository with two terms, one current decision, and the topic file
/// planned, its reader rows set: the base decision `closes B`, the other
/// question `same`, one term `inconsistent`, the other term `consistent`.
fn repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget", "paint"]);
    let base = write_record(
        &dir,
        BASE,
        &format!("# Base\n{}", section(BASE_Q, &body("base widget"))),
    );
    approve_record(&dir, BASE);
    let id = extract_ids(&std::fs::read_to_string(&base).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    let text = format!(
        "# Topic\n{}{}",
        section(H1, &body("one")),
        section(H2, &body("two"))
    );
    std::fs::write(dir.join(REL), text).unwrap();
    assert!(
        adr(&dir, &["plan", dir.join(REL).to_str().unwrap()], &[], "")
            .status
            .success()
    );
    plan_record(&dir, REL);
    reader_says(&dir, &id);
    approve_record(&dir, REL);
    dir
}

/// Newer rows at the body of H1 than the fake reader's, with the words the
/// tests read.
fn reader_says(dir: &Path, base_id: &str) {
    let rf = record::load(&dir.join(REL)).unwrap();
    let sec = rf.sections.iter().find(|s| s.heading_text == H1).unwrap();
    let hash = record::body_hash(&rf, sec);
    let mut new = Vec::new();
    let mut terms: Vec<String> = Vec::new();
    let mut rows: Vec<reader::Row> = reader::read_file(dir)
        .rows
        .into_iter()
        .filter(|r| r.text_hash == hash)
        .collect();
    rows.sort_by(|a, b| a.neighbour.cmp(&b.neighbour));
    for mut row in rows {
        let other = row.neighbour != base_id && !row.neighbour.starts_with("text:");
        if other {
            terms.push(row.neighbour.clone());
        }
        row.word = if row.neighbour == base_id {
            "closes B"
        } else if row.neighbour.starts_with("text:") {
            "same"
        } else if terms.len() == 1 {
            "inconsistent"
        } else {
            "consistent"
        }
        .to_string();
        row.reason = format!("Reason for {}.", row.word);
        new.push(row);
    }
    assert!(new.len() >= 3, "the fake reader's rows: {:?}", new);
    reader::append(dir, &new).unwrap();
}

fn show(dir: &Path, args: &[&str]) -> String {
    let mut all = vec!["show", REL];
    all.extend_from_slice(args);
    let out = adr(dir, &all, &NO_TOOLS, "");
    assert!(out.status.success(), "{}", stderr(&out));
    stdout(&out)
}

/// The lines under the question `heading`, up to the next status line or
/// move.
fn under(shown: &str, heading: &str) -> Vec<String> {
    shown
        .lines()
        .skip_while(|l| !l.ends_with(&format!("] {}", heading)))
        .skip(1)
        .take_while(|l| l.starts_with("  "))
        .map(str::to_string)
        .collect()
}

// adr: 6e72f34e-d484-4c63-8332-d284857176fc
//
// Plain show: [inconsistent], [same] and [closes B] under a question not yet
// answered, after its reference lines, nothing for the other words.
#[test]
fn plain_show_prints_inconsistent_same_and_closes_under_a_question_not_answered() {
    let dir = repo();
    let shown = show(&dir, &[]);
    let lines = under(&shown, H1);
    assert!(
        lines.contains(&format!("  [closes B] {}", BASE_Q)),
        "{}",
        shown
    );
    assert!(lines.contains(&format!("  [same] {}", H2)), "{}", shown);
    let marks: Vec<&String> = lines.iter().filter(|l| l.starts_with("  [")).collect();
    assert_eq!(marks.len(), 3, "{}", shown);
    assert_eq!(
        marks
            .iter()
            .filter(|l| l.starts_with("  [inconsistent] "))
            .count(),
        1,
        "{}",
        shown
    );
    assert!(
        lines.iter().position(|l| l == "  depends: widget").unwrap()
            < lines.iter().position(|l| l.starts_with("  [")).unwrap()
    );
    for word in ["[consistent]", "[depends]", "[unrelated]", "Reason for"] {
        assert!(!shown.contains(word), "{}: {}", word, shown);
    }
    // The other question carries no row of its own.
    assert!(
        under(&shown, H2).iter().all(|l| !l.starts_with("  [")),
        "{}",
        shown
    );
}

// adr: 6e72f34e-d484-4c63-8332-d284857176fc
//
// A supersedes line that names the neighbour leaves its line out, a closes
// line too.
#[test]
fn a_supersedes_line_that_names_the_neighbour_leaves_its_word_out() {
    let dir = repo();
    let text = std::fs::read_to_string(dir.join(REL)).unwrap();
    // The approved file holds identifiers; the supersedes line names the
    // decision by its heading, as an agent writes it.
    let at = text.find("\ndepends:").unwrap() + 1;
    let end = at + text[at..].find('\n').unwrap() + 1;
    let edited = format!("{}supersedes: {}\n{}", &text[..end], BASE_Q, &text[end..]);
    std::fs::write(dir.join(REL), edited).unwrap();
    let shown = show(&dir, &[]);
    let lines = under(&shown, H1);
    assert!(
        lines.contains(&format!("  + supersedes: {}", BASE_Q)),
        "{}",
        shown
    );
    assert!(
        !lines.iter().any(|l| l.starts_with("  [closes")),
        "{}",
        shown
    );
    assert!(
        lines.iter().any(|l| l.starts_with("  [inconsistent] ")),
        "{}",
        shown
    );
    assert!(lines.contains(&format!("  [same] {}", H2)), "{}", shown);
}

// adr: 6e72f34e-d484-4c63-8332-d284857176fc
//
// An answered question prints no word in the plain listing; --reader prints
// every word on it, the reason a line under, and the plain lines once.
#[test]
fn reader_prints_every_word_with_its_reason_and_an_answered_question_none_plain() {
    let dir = repo();
    let plain = show(&dir, &[]);
    let all = show(&dir, &["--reader"]);
    let lines = under(&all, H1);
    let term_words: Vec<&String> = lines
        .iter()
        .filter(|l| l.starts_with("  [inconsistent] ") || l.starts_with("  [consistent] "))
        .collect();
    assert_eq!(term_words.len(), 2, "{}", all);
    for (word, name) in [("closes B", BASE_Q), ("same", H2)] {
        let at = lines
            .iter()
            .position(|l| *l == format!("  [{}] {}", word, name));
        let at = at.unwrap_or_else(|| panic!("{} {}: {}", word, name, all));
        assert_eq!(
            lines[at + 1],
            format!("    Reason for {}.", word),
            "{}",
            all
        );
    }
    for w in &term_words {
        let name = w.rsplit(' ').next().unwrap();
        assert!(["paint", "widget"].contains(&name), "{}", all);
    }
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.starts_with("  [inconsistent]"))
            .count(),
        1,
        "{}",
        all
    );
    assert!(plain.len() < all.len());

    // Answered: nothing in the plain listing, everything under --reader.
    let id = extract_ids(&std::fs::read_to_string(dir.join(REL)).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    let plain = show(&dir, &[]);
    assert!(plain.contains(&format!("[answered A] {}", H1)), "{}", plain);
    assert!(
        !plain.contains("[inconsistent]")
            && !plain.contains("[same]")
            && !plain.contains("[closes"),
        "{}",
        plain
    );
    let all = show(&dir, &["--reader"]);
    assert!(
        all.contains(&format!(
            "  [closes B] {}\n    Reason for closes B.\n",
            BASE_Q
        )),
        "{}",
        all
    );

    // A flag that needs a file.
    assert!(!adr(&dir, &["show", "--reader"], &[], "").status.success());
}
