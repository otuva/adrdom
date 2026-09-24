// Wave 2, answer (spec §5): `adr answer <id> refuse`.
mod common;

use common::*;

const Q1: &str = r#"# Example

### 1.1 What color is the widget?
depends: widget

Problem: the widget's color is undecided.

Standard: plain words.

- A) The widget is red.
  Why: reasons.
  Costs: costs.
- B) The widget is blue.
  Why: other reasons.
  Costs: other costs.
"#;

const Q2: &str = r#"
### 1.2 What color is the second widget?
depends: widget

Problem: the second widget's color is undecided.

Standard: plain words.

- A) The second widget is red.
  Why: reasons.
  Costs: costs.
- B) The second widget is blue.
  Why: other reasons.
  Costs: other costs.
"#;

#[test]
fn answer_refuse_writes_the_line_and_the_refused_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    let hash_before = {
        // The asked row's hash: the section is unchanged since ask, so the
        // `refused` row must carry the same hash.
        let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
        ledger
            .lines()
            .find(|l| l.contains("\tasked\t") && l.contains(id.as_str()))
            .unwrap()
            .split('\t')
            .nth(3)
            .unwrap()
            .to_string()
    };

    let out = adr_owner(&dir, &["answer", &id, "refuse"], "");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));
    assert_eq!(
        stdout(&out).trim_end(),
        "answer: refused \"1.1 What color is the widget?\"; agent re-plans, owner runs: adr approve adrdom/decisions/example.md"
    );

    let after = std::fs::read_to_string(&record).unwrap();
    assert!(after.contains("**Answer:** refused"), "{}", after);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let row = ledger
        .lines()
        .find(|l| l.contains("\trefused\t"))
        .expect(&format!("no refused row: {}", ledger));
    let fields: Vec<&str> = row.split('\t').collect();
    assert_eq!(fields[1], "refused");
    assert_eq!(fields[2], id);
    assert_eq!(fields[3], hash_before, "refused row hash: {}", row);
    assert_eq!(fields[4], "1.1 What color is the widget?");
}

#[test]
fn answer_letter_after_refused_refuses() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["answer", &id, "refuse"], "").status.success());

    let out = adr_owner(&dir, &["answer", &id, "A"], "");
    assert!(!out.status.success(), "answer should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        "answer: \"1.1 What color is the widget?\" is already refused; owner runs: adr answer a different, still-open id"
    );

    // Nothing changed: still exactly one Answer line, unmodified.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after.matches("**Answer:**").count(), 1, "{}", after);
    assert!(after.contains("**Answer:** refused"), "{}", after);
}

#[test]
fn answer_refuse_twice_gives_the_already_refused_message() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["answer", &id, "refuse"], "").status.success());

    let out = adr_owner(&dir, &["answer", &id, "refuse"], "");
    assert!(!out.status.success(), "second refuse should have refused: {}", stdout(&out));
    assert_eq!(
        stderr(&out).trim_end(),
        "answer: \"1.1 What color is the widget?\" is already refused; owner runs: adr answer a different, still-open id"
    );

    // Nothing changed: still exactly one Answer line, unmodified, and no
    // second `refused` row.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(after.matches("**Answer:**").count(), 1, "{}", after);
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger.lines().filter(|l| l.contains("\trefused\t")).count(),
        1,
        "{}",
        ledger
    );
}

#[test]
fn answer_refuse_ignores_supersedes_with_a_printed_note_and_writes_no_superseded_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["answer", &old_id, "A"], "").status.success());

    // Q2 carries `supersedes:` naming the now-answered Q1, but will be
    // refused rather than answered.
    let mut content = std::fs::read_to_string(&record).unwrap();
    let with_supersedes = Q2.replacen(
        "### 1.2 What color is the second widget?\n",
        &format!(
            "### 1.2 What color is the second widget?\n\nsupersedes: {}\n",
            old_id
        ),
        1,
    );
    content.push_str(&with_supersedes);
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content)
        .into_iter()
        .find(|id| id != &old_id)
        .expect("second id assigned");

    let out = adr_owner(&dir, &["answer", &new_id, "refuse"], "");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));
    let printed = stdout(&out);
    assert!(
        printed.contains("supersedes ignored on a refused question"),
        "{}",
        printed
    );

    // Q1 must still be current: no `superseded` row for it.
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tsuperseded\t")),
        "refuse must not have superseded anything: {}",
        ledger
    );
    assert!(
        ledger
            .lines()
            .any(|l| l.contains("\trefused\t") && l.contains(new_id.as_str())),
        "missing refused row: {}",
        ledger
    );
}
