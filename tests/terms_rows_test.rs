// approve of the terms file writes one `term` ledger row per term (id,
// hash of the entry's text, name), and says which live questions name a
// term whose definition changed. Information only: nothing is refused.
mod common;

use common::*;
use std::path::{Path, PathBuf};

const TERMS_TWO_ENTRIES: &str = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A choice answered by the owner.\n\
aliases: \n\
\n\
## verdict\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A ruling reached after deliberation.\n\
aliases: \n";

const TERMS_DECISION_EDITED: &str = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A choice the owner picked from options.\n\
aliases: \n\
\n\
## verdict\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A ruling reached after deliberation.\n\
aliases: \n";

const TERMS_VERDICT_RETIRED: &str = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A choice answered by the owner.\n\
aliases: \n\
\n\
## verdict\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A ruling reached after deliberation.\n\
aliases: \n\
retired: yes\n";

const BODY: &str = "\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";

const H_ONE: &str = "What is the decision?";
const H_TWO: &str = "What does the decision hold?";

fn rows_of(dir: &Path, event: &str) -> Vec<String> {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap_or_default()
        .lines()
        .filter(|l| l.split('\t').nth(1) == Some(event))
        .map(|l| l.to_string())
        .collect()
}

fn setup() -> (PathBuf, PathBuf, PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_TWO_ENTRIES);
    (dir, terms, PathBuf::new())
}

fn approve_terms(dir: &Path, terms: &Path) -> std::process::Output {
    adr_owner(dir, &["approve", terms.to_str().unwrap()], "")
}

#[test]
fn first_approve_writes_one_term_row_per_term_and_a_terms_row_per_approve() {
    let (dir, terms, _) = setup();
    let terms_before = rows_of(&dir, "terms").len();
    let out = approve_terms(&dir, &terms);
    assert!(out.status.success(), "{}", stderr(&out));
    // The two entries are approved one at a time: each approve writes one
    // terms row, and each entry gets its term row.
    assert!(rows_of(&dir, "terms").len() > terms_before);
    let rows = rows_of(&dir, "term");
    assert_eq!(rows.len(), 2, "{:?}", rows);
    assert!(rows.iter().any(|r| r.contains("\t33333333-3333-3333-3333-333333333333\t") && r.contains("\tdecision\t")));
    assert!(rows.iter().any(|r| r.contains("\t44444444-4444-4444-4444-444444444444\t") && r.contains("\tverdict\t")));
    assert!(!stdout(&out).contains("\" changed; named by"), "{}", stdout(&out));
}

#[test]
fn edited_definition_writes_one_row_and_names_the_live_questions() {
    let (dir, terms, _) = setup();
    assert!(approve_terms(&dir, &terms).status.success());
    let record = write_record(
        &dir,
        "adrdom/decisions/example.md",
        &format!(
            "# Example\n\n### {}\ndepends: decision\n{}\n### {}\ndepends: decision\n{}",
            H_ONE, BODY, H_TWO, BODY
        ),
    );
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "").status.success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let c = std::fs::read_to_string(&record).unwrap();
    let id = c
        .lines()
        .skip_while(|l| *l != format!("### {}", H_ONE))
        .find_map(|l| l.strip_prefix("id: ").map(|s| s.trim().to_string()))
        .unwrap();
    let out = adr_answer(&dir, &id, "A");
    assert!(out.status.success(), "{}", stderr(&out));

    let before = rows_of(&dir, "term").len();
    let terms_before = rows_of(&dir, "terms").len();
    std::fs::write(&terms, TERMS_DECISION_EDITED).unwrap();
    let out = approve_terms(&dir, &terms);
    assert!(out.status.success(), "{}", stderr(&out));
    // One line per row written, the edited term's row and the terms row,
    // and one for the terms brief its edit rewrote (per "What does a
    // command that writes print?", 069a6b4a A).
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{}", text);
    assert!(lines.contains(&"written: adrdom/state/terms-brief.md"), "{}", text);
    assert!(lines.contains(&"approved: term \"decision\""), "{}", text);
    assert!(lines.contains(&"approved: adrdom/terms.md"), "{}", text);
    assert_eq!(rows_of(&dir, "term").len(), before + 1);
    assert_eq!(rows_of(&dir, "terms").len(), terms_before + 1);
}

#[test]
fn approve_with_nothing_changed_writes_no_term_row() {
    let (dir, terms, _) = setup();
    assert!(approve_terms(&dir, &terms).status.success());
    let before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let out = approve_terms(&dir, &terms);
    assert!(out.status.success(), "{}", stderr(&out));
    // No row written, so nothing printed.
    assert_eq!(stdout(&out), "", "{}", stdout(&out));
    assert_eq!(std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap(), before);
}

#[test]
fn a_passing_retire_writes_a_term_row_with_the_new_hash() {
    let (dir, terms, _) = setup();
    assert!(approve_terms(&dir, &terms).status.success());
    let old: Vec<String> = rows_of(&dir, "term");
    std::fs::write(&terms, TERMS_VERDICT_RETIRED).unwrap();
    let out = approve_terms(&dir, &terms);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).lines().any(|l| l == "approved: term \"verdict\""), "{}", stdout(&out));
    let rows = rows_of(&dir, "term");
    assert_eq!(rows.len(), old.len() + 1);
    let last = rows
        .iter()
        .rfind(|r| r.contains("\t44444444-4444-4444-4444-444444444444\t"))
        .unwrap();
    let prev = old.iter().rfind(|r| r.contains("\t44444444-4444-4444-4444-444444444444\t")).unwrap();
    assert_ne!(last.split('\t').nth(3), prev.split('\t').nth(3));
}
