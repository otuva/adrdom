// approve refuses retiring a term while any question that is answered,
// asked, or a heading of an approved plan names it, and names those
// questions. A question only refused is not live.
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

const TERMS_DECISION_RETIRED: &str = "# Terms\n\n\
## decision\n\
id: 33333333-3333-3333-3333-333333333333\n\
definition: A choice answered by the owner.\n\
aliases: \n\
retired: yes\n\
\n\
## verdict\n\
id: 44444444-4444-4444-4444-444444444444\n\
definition: A ruling reached after deliberation.\n\
aliases: \n";

const BODY: &str = "\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";

const H_DECISION: &str = "What is the decision?";
const H_VERDICT: &str = "What is the verdict?";

fn sec(heading: &str, deps: &str, body: bool) -> String {
    format!("\n### {}\ndepends: {}\n{}", heading, deps, if body { BODY } else { "" })
}

fn terms_rows(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap_or_default()
        .lines()
        .filter(|l| l.contains("\tterms\t"))
        .count()
}

fn id_of(path: &Path, heading: &str) -> String {
    let c = std::fs::read_to_string(path).unwrap();
    let mut lines = c.lines().skip_while(|l| *l != format!("### {}", heading));
    assert!(lines.next().is_some(), "no heading {} in {}", heading, c);
    for l in lines {
        if let Some(id) = l.strip_prefix("id: ") {
            return id.trim().to_string();
        }
    }
    panic!("no id under {}: {}", heading, c);
}

/// Repo with the two-entry terms approved and `record_body` written to
/// example.md and approved.
fn setup(record_body: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", record_body);
    let terms = write_record(&dir, "adrdom/terms.md", TERMS_TWO_ENTRIES);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "").status.success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "").status.success());
    (dir, record, terms)
}

fn retire(dir: &Path, terms: &Path) -> std::process::Output {
    std::fs::write(terms, TERMS_DECISION_RETIRED).unwrap();
    adr_owner(dir, &["approve", terms.to_str().unwrap()], "")
}

fn assert_retire_refused(dir: &Path, terms: &Path, rows_before: usize, heading: &str) {
    let out = retire(dir, terms);
    assert!(!out.status.success(), "retire should have refused: {}{}", stdout(&out), stderr(&out));
    let err = stderr(&out);
    assert!(err.contains(heading), "stderr should name {:?}: {}", heading, err);
    assert!(
        err.contains("restores adrdom/terms.md from git"),
        "stderr should name the restore: {}",
        err
    );
    assert_eq!(terms_rows(dir), rows_before, "a refused retire writes no terms row");
}

// adr: db58d4b8-9e64-44b0-99a2-acb7f05d5b4e
#[test]
fn retire_refused_while_an_asked_question_names_the_term() {
    let (dir, record, terms) =
        setup(&format!("# Example\n{}", sec(H_DECISION, "decision", true)));
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let rows = terms_rows(&dir);
    assert_retire_refused(&dir, &terms, rows, H_DECISION);
}

// adr: db58d4b8-9e64-44b0-99a2-acb7f05d5b4e
#[test]
fn retire_refused_while_a_bodyless_plan_heading_names_the_term() {
    let (dir, _record, terms) =
        setup(&format!("# Example\n{}", sec(H_DECISION, "verdict", false)));
    let rows = terms_rows(&dir);
    assert_retire_refused(&dir, &terms, rows, H_DECISION);
}

// adr: db58d4b8-9e64-44b0-99a2-acb7f05d5b4e
#[test]
fn retire_succeeds_once_the_asked_question_is_struck_from_the_plan() {
    let (dir, record, terms) = setup(&format!(
        "# Example\n{}{}",
        sec(H_DECISION, "decision", true),
        sec(H_VERDICT, "verdict", true)
    ));
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let rows = terms_rows(&dir);
    // Refused first, while the asked question stands in the plan.
    assert_retire_refused(&dir, &terms, rows, H_DECISION);

    // The refused retire left an unapproved terms.md on disk; approving a
    // topic file is refused while terms.md waits for approval, so the
    // retire edit is put back first.
    std::fs::write(&terms, TERMS_TWO_ENTRIES).unwrap();

    // Strike the question: remove its section, owner re-approves the topic
    // file (which withdraws it).
    let c = std::fs::read_to_string(&record).unwrap();
    let start = c.find(&format!("### {}", H_DECISION)).unwrap();
    let end = c.find(&format!("### {}", H_VERDICT)).unwrap();
    std::fs::write(&record, format!("{}{}", &c[..start], &c[end..])).unwrap();
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "striking approve failed: {}", stderr(&out));

    let out = retire(&dir, &terms);
    assert!(out.status.success(), "retire should pass once struck: {}", stderr(&out));
    assert_eq!(terms_rows(&dir), rows + 1);
}

// adr: db58d4b8-9e64-44b0-99a2-acb7f05d5b4e
#[test]
fn retire_succeeds_when_only_a_refused_question_names_the_term() {
    let (dir, record, terms) = setup(&format!(
        "# Example\n{}{}",
        sec(H_DECISION, "decision", true),
        sec(H_VERDICT, "verdict", true)
    ));
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let out = adr_answer(&dir, &id_of(&record, H_DECISION), "refuse");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));
    let rows = terms_rows(&dir);
    let out = retire(&dir, &terms);
    assert!(out.status.success(), "a refused question is not live: {}", stderr(&out));
    assert_eq!(terms_rows(&dir), rows + 1);
}
