mod common;

use common::*;

const REL: &str = "adrdom/decisions/example.md";

const Q1: &str = "### 1.1 First question?\ndepends: decision\n\nProblem: the decision is unclear.\n\nStandard: s1.\n\n- A) Answer one.\n  Why: reasons.\n  Costs: costs.\n- B) Answer two.\n  Why: other reasons.\n  Costs: other costs.\n";
const Q2: &str = "### 1.2 Second question?\ndepends: decision\n\nProblem: another decision.\n\nStandard: s2.\n\n- A) Answer one.\n  Why: reasons.\n  Costs: costs.\n- B) Answer two.\n  Why: other reasons.\n  Costs: other costs.\n";

fn file_of(questions: &[&str]) -> String {
    format!("# Example\n\nContext.\n\n{}", questions.join("\n"))
}

fn setup(body: &str) -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    write_record(&dir, REL, body);
    dir
}

fn approve(dir: &std::path::Path) -> std::process::Output {
    adr_owner(dir, &["approve", dir.join(REL).to_str().unwrap()], "")
}

fn ledger_text(dir: &std::path::Path) -> String {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap()
}

fn ledger_rows(dir: &std::path::Path, event: &str) -> Vec<Vec<String>> {
    ledger_text(dir)
        .lines()
        .map(|l| l.split('\t').map(|s| s.to_string()).collect::<Vec<_>>())
        .filter(|f| f[1] == event)
        .collect()
}

fn check_out(dir: &std::path::Path) -> std::process::Output {
    adr(dir, &["check"], &[], "")
}

// adr: 61e73795-63de-4669-b09f-48be96919325
//
// "approve hashes each question on its own": one `approved` row per
// question (its id, its own hash, the file), and the file's row holds the
// hash derived from the sorted question hashes. Reordering the headings
// moves neither; an edited body awaits approval.
#[test]
fn approve_writes_a_row_per_question_and_a_file_hash_over_the_sorted_parts() {
    let dir = setup(&file_of(&[Q1, Q2]));
    let out = approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));

    let rows = ledger_rows(&dir, "approved");
    let per_question: Vec<&Vec<String>> = rows.iter().filter(|r| r[2] != "-").collect();
    let file_rows: Vec<&Vec<String>> = rows.iter().filter(|r| r[2] == "-").collect();
    assert_eq!(per_question.len(), 2, "{:?}", rows);
    assert_eq!(file_rows.len(), 1, "{:?}", rows);

    let rf = adr::record::load(&dir.join(REL)).unwrap();
    let hashes = adr::record::question_hashes(&rf);
    for (sec, h) in rf.sections.iter().zip(&hashes) {
        let id = sec.id.clone().expect("approve minted an id");
        let row = per_question.iter().find(|r| r[2] == id).expect("a row for each id");
        assert_eq!(&row[3], h);
        assert_eq!(row[4], REL);
    }
    assert_eq!(file_rows[0][3], adr::record::derive_file_hash(&hashes));

    // Approving again writes nothing.
    let before = ledger_text(&dir);
    assert!(approve(&dir).status.success());
    assert_eq!(before, ledger_text(&dir));

    // Reordered headings: still approved.
    let content = std::fs::read_to_string(dir.join(REL)).unwrap();
    let second_at = content.find("### 1.2 ").unwrap();
    let first_at = content.find("### 1.1 ").unwrap();
    let swapped = format!(
        "{}{}{}",
        &content[..first_at],
        &content[second_at..],
        &content[first_at..second_at]
    );
    std::fs::write(dir.join(REL), swapped).unwrap();
    let out = check_out(&dir);
    assert!(!stdout(&out).contains("awaiting approval"), "{}", stdout(&out));

    // An edited body awaits approval.
    let edited = std::fs::read_to_string(dir.join(REL))
        .unwrap()
        .replace("another decision.", "another, changed decision.");
    std::fs::write(dir.join(REL), edited).unwrap();
    assert!(stdout(&check_out(&dir)).contains("awaiting approval"));
}

// adr: 61e73795-63de-4669-b09f-48be96919325
//
// A struck heading awaits approval; once a `dropped` row carries its
// approved hash, the rest of the file stands approved.
#[test]
fn a_struck_heading_awaits_approval_unless_its_hash_is_dropped() {
    let dir = setup(&file_of(&[Q1, Q2]));
    assert!(approve(&dir).status.success());
    let rows = ledger_rows(&dir, "approved");
    let id2 = adr::record::load(&dir.join(REL)).unwrap().sections[1].id.clone().unwrap();
    let row2 = rows.iter().find(|r| r[2] == id2).unwrap().clone();

    let content = std::fs::read_to_string(dir.join(REL)).unwrap();
    let cut = content.find("### 1.2 ").unwrap();
    std::fs::write(dir.join(REL), &content[..cut]).unwrap();
    assert!(stdout(&check_out(&dir)).contains("awaiting approval"));

    append_ledger_row_at(&dir, "dropped", &id2, &row2[3], "dropped for the test", "2999-01-01T00:00:00Z");
    let out = check_out(&dir);
    assert!(!stdout(&out).contains("awaiting approval"), "{}", stdout(&out));
}

// adr: f5eeaacf-1c9f-478b-8a97-2df1aaf39458
//
// "Does the agent write the bodies before approve?": approve stops a
// question not yet asked that has only a heading, names the move, and
// writes nothing; with the body written it approves.
#[test]
fn approve_refuses_a_heading_with_no_body_and_writes_nothing() {
    let dir = setup(&file_of(&[Q1, "### 1.2 Second question?\ndepends: decision\n"]));
    let before = ledger_text(&dir);
    let out = approve(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("1.2 Second question?") && err.contains("no body"), "{}", err);
    assert!(err.contains("agent writes") && err.contains("owner runs: adr approve"), "{}", err);
    assert_eq!(before, ledger_text(&dir));

    write_record(&dir, REL, &file_of(&[Q1, Q2]));
    assert!(approve(&dir).status.success());
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
//
// "Does a depends entry on a question in its file carry an answer set?":
// approve stops an entry on a question of the same file with no set, and
// accepts it once the set is written.
#[test]
fn approve_refuses_a_depends_on_a_question_in_its_file_without_a_set() {
    let without = Q2.replace("depends: decision", "depends: 1.1 First question?");
    let dir = setup(&file_of(&[Q1, &without]));
    let out = approve(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("without an answer set"), "{}", err);
    assert!(
        err.contains("agent writes =A, =A|B or =*") && err.contains("owner runs: adr approve"),
        "{}",
        err
    );
    assert!(ledger_rows(&dir, "approved").is_empty());

    let with_set = Q2.replace("depends: decision", "depends: 1.1 First question?=A|B");
    write_record(&dir, REL, &file_of(&[Q1, &with_set]));
    let out = approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = std::fs::read_to_string(dir.join(REL)).unwrap();
    assert!(text.contains("=A|B"), "{}", text);
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
//
// A depends entry on an answered question carries no set: approve refuses
// a set on it and names the move, and accepts the entry without one.
#[test]
fn approve_refuses_a_set_on_an_entry_naming_an_answered_question() {
    let dir = setup(&file_of(&[Q1]));
    assert!(approve(&dir).status.success());
    assert!(adr(&dir, &["ask", dir.join(REL).to_str().unwrap()], &[], "").status.success());
    let id1 = adr::record::load(&dir.join(REL)).unwrap().sections[0].id.clone().unwrap();
    assert!(adr_answer(&dir, &id1, "A").status.success());

    let text = std::fs::read_to_string(dir.join(REL)).unwrap();
    let q2 = Q2.replace("depends: decision", "depends: 1.1 First question?=A");
    std::fs::write(dir.join(REL), format!("{}\n{}", text.trim_end(), q2)).unwrap();
    let out = approve(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("answered, under an answer set"), "{}", err);
    assert!(err.contains("agent removes the answer set") && err.contains("owner runs: adr approve"), "{}", err);

    let fixed = std::fs::read_to_string(dir.join(REL)).unwrap().replace("?=A", "?");
    std::fs::write(dir.join(REL), fixed).unwrap();
    let out = approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
}

// adr: 912f4f3b-1213-435a-af0d-7617017fcff4
//
// "Does check fail a topic file awaiting approval?" (No): a file with its
// bodies, committed before approval, is counted and never failed.
#[test]
fn check_counts_a_file_awaiting_approval_and_passes() {
    let dir = setup(&file_of(&[Q1, Q2]));
    let out = check_out(&dir);
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("1 file(s) awaiting approval") && text.contains(REL), "{}", text);

    // After approve the count is gone; after an edit it is back and check
    // still passes.
    assert!(approve(&dir).status.success());
    assert!(!stdout(&check_out(&dir)).contains("awaiting approval"));
    let edited = std::fs::read_to_string(dir.join(REL)).unwrap().replace("s2.", "s2, reworded.");
    std::fs::write(dir.join(REL), edited).unwrap();
    let out = check_out(&dir);
    assert!(stdout(&out).contains("1 file(s) awaiting approval"), "{}", stdout(&out));
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
//
// A set approved before its target was answered stays: the question keeps
// it through a re-approve after the answer, and after its body is edited.
#[test]
fn a_set_approved_before_the_answer_survives_a_reapprove_after_it() {
    let q2 = Q2.replace("depends: decision", "depends: 1.1 First question?=A");
    let dir = setup(&file_of(&[Q1, &q2]));
    assert!(approve(&dir).status.success());
    assert!(adr(&dir, &["ask", dir.join(REL).to_str().unwrap()], &[], "").status.success());
    let id1 = adr::record::load(&dir.join(REL)).unwrap().sections[0].id.clone().unwrap();
    assert!(adr_answer(&dir, &id1, "A").status.success());

    let out = approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let edited = std::fs::read_to_string(dir.join(REL)).unwrap().replace("s2.", "s2, reworded.");
    std::fs::write(dir.join(REL), edited).unwrap();
    let out = approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(std::fs::read_to_string(dir.join(REL)).unwrap().contains("?=A") || std::fs::read_to_string(dir.join(REL)).unwrap().contains("=A"));
    assert!(adr(&dir, &["ask", dir.join(REL).to_str().unwrap()], &[], "").status.success());
    let out = check_out(&dir);
    assert!(!stdout(&out).contains("answers:"), "{}{}", stdout(&out), stderr(&out));
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
//
// A set changed after the target was answered is refused, even on a question
// that was approved before the answer: only the set frozen when the target
// was asked stays.
#[test]
fn a_set_changed_after_the_answer_on_an_approved_question_is_refused() {
    let q2 = Q2.replace("depends: decision", "depends: 1.1 First question?=A");
    let dir = setup(&file_of(&[Q1, &q2]));
    assert!(approve(&dir).status.success());
    assert!(adr(&dir, &["ask", dir.join(REL).to_str().unwrap()], &[], "").status.success());
    let id1 = adr::record::load(&dir.join(REL)).unwrap().sections[0].id.clone().unwrap();
    assert!(adr_answer(&dir, &id1, "A").status.success());

    let text = std::fs::read_to_string(dir.join(REL)).unwrap();
    let (head, q2) = text.split_at(text.find("### 1.2 ").unwrap());
    let at = q2.find("depends: ").unwrap();
    let end = at + q2[at..].find('\n').unwrap();
    let q2 = format!("{}depends: 1.1 First question?=A|B{}", &q2[..at], &q2[end..]);
    std::fs::write(dir.join(REL), format!("{}{}", head, q2)).unwrap();
    let out = approve(&dir);
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(stderr(&out).contains("answered, under an answer set"), "{}", stderr(&out));
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
//
// A set first approved after its target was asked, before it was answered,
// stays: approve writes the entry's `assumes` row, so the re-approve after
// the answer and a body edit accepts it, and check passes.
#[test]
fn a_set_first_approved_after_the_ask_stays_after_the_answer() {
    let dir = setup(&file_of(&[Q1]));
    assert!(approve(&dir).status.success());
    assert!(adr(&dir, &["ask", dir.join(REL).to_str().unwrap()], &[], "").status.success());
    let id1 = adr::record::load(&dir.join(REL)).unwrap().sections[0].id.clone().unwrap();

    let q2 = Q2.replace("depends: decision", "depends: 1.1 First question?=A");
    let text = std::fs::read_to_string(dir.join(REL)).unwrap();
    std::fs::write(dir.join(REL), format!("{}\n{}", text.trim_end(), q2)).unwrap();
    let out = approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(ledger_rows(&dir, "assumes").len(), 1);

    assert!(adr_answer(&dir, &id1, "A").status.success());
    let edited = std::fs::read_to_string(dir.join(REL)).unwrap().replace("s2.", "s2, reworded.");
    std::fs::write(dir.join(REL), edited).unwrap();
    let out = approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(ledger_rows(&dir, "assumes").len(), 1);
    let out = check_out(&dir);
    assert!(!stdout(&out).contains("answers:"), "{}{}", stdout(&out), stderr(&out));
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
//
// An answer set names the letters of another question: a term has no
// options, so approve refuses a set on an entry naming a term.
#[test]
fn approve_refuses_a_set_on_an_entry_naming_a_term() {
    let with_set = Q2.replace("depends: decision", "depends: 99999999-9999-9999-9999-999999999999=A");
    let dir = setup(&file_of(&[Q1, &with_set]));
    let out = approve(&dir);
    assert!(!out.status.success(), "{}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("a term has no options"), "{}", err);
    assert!(err.contains("agent removes the answer set"), "{}", err);
    assert!(ledger_rows(&dir, "approved").is_empty());
}

// adr: 61e73795-63de-4669-b09f-48be96919325
//
// "Does approve hash each question on its own?": check names the questions
// that differ from the last approval, a changed one and a struck one, and
// still passes.
#[test]
fn check_names_the_changed_and_the_struck_questions() {
    let dir = setup(&file_of(&[Q1, Q2]));
    assert!(approve(&dir).status.success());
    let edited = std::fs::read_to_string(dir.join(REL)).unwrap().replace("s2.", "s2, reworded.");
    std::fs::write(dir.join(REL), edited).unwrap();
    let out = check_out(&dir);
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("new or changed: \"1.2 Second question?\""), "{}", text);
    assert!(!text.contains("struck"), "{}", text);

    let id2 = adr::record::load(&dir.join(REL)).unwrap().sections[1].id.clone().unwrap();
    let kept = std::fs::read_to_string(dir.join(REL)).unwrap();
    let cut = kept.find("### 1.2 ").unwrap();
    std::fs::write(dir.join(REL), kept[..cut].trim_end().to_string() + "\n").unwrap();
    let out = check_out(&dir);
    let text = stdout(&out);
    assert!(text.contains(&format!("struck: {}", id2)), "{}", text);
}

// adr: 946263aa-519a-47aa-b743-5cdcf52f5d35
//
// "Does a depends entry on a question in its file carry an answer set?": a
// letter the same-file target has no option for is found before approval:
// check reports it on a file not yet approved, and approve refuses it.
#[test]
fn a_set_letter_the_target_lacks_is_reported_by_check_and_refused_by_approve() {
    let bad = Q2.replace("depends: decision", "depends: 1.1 First question?=A|D");
    let dir = setup(&file_of(&[Q1, &bad]));
    let out = adr(&dir, &["check"], &[], "");
    let all = format!("{}{}", stdout(&out), stderr(&out));
    assert!(all.contains("names option D which \"1.1 First question?\" does not have"), "{}", all);

    let out = approve(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("names option D which"), "{}", err);
    assert!(ledger_rows(&dir, "approved").is_empty());

    let good = Q2.replace("depends: decision", "depends: 1.1 First question?=A|B");
    write_record(&dir, REL, &file_of(&[Q1, &good]));
    assert!(approve(&dir).status.success());
}
