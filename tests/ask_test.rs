mod common;

use common::*;

#[test]
fn example_record_round_trips_and_ask_assigns_ids() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    assert_eq!(
        ids.len(),
        2,
        "expected an id for the question and the principle: {}",
        content
    );
    assert_ne!(ids[0], ids[1]);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_rows: Vec<&str> = ledger.lines().filter(|l| l.contains("\tasked\t")).collect();
    assert_eq!(asked_rows.len(), 2, "ledger: {}", ledger);
    for id in &ids {
        assert!(asked_rows.iter().any(|r| r.contains(id.as_str())));
    }

    // Running ask again with nothing changed should not add more rows.
    let out2 = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out2.status.success());
    let ledger2 = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger, ledger2,
        "re-running ask with no changes should be a no-op"
    );
}

#[test]
fn ask_reasks_on_edit() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_before = ledger_before
        .lines()
        .filter(|l| l.contains("\tasked\t"))
        .count();

    // Edit the question's Problem text (inside the hashed range, before any Answer).
    let content = std::fs::read_to_string(&record).unwrap();
    let edited = content.replace(
        "names leaked into decisions.",
        "names leaked into decisions, badly.",
    );
    std::fs::write(&record, edited).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));
    assert!(
        stdout(&out).contains("re-asked"),
        "expected a re-ask in output: {}",
        stdout(&out)
    );

    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_after = ledger_after
        .lines()
        .filter(|l| l.contains("\tasked\t"))
        .count();
    assert_eq!(
        asked_after,
        asked_before + 1,
        "expected exactly one new asked row"
    );
}

#[test]
fn ask_refuses_to_edit_an_answered_section() {
    let dir = init_repo();
    assert!(adr(&dir, &["init"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    let question_id = &ids[0];

    let lock_out = adr_owner(&dir, &["lock", question_id, "A"], "");
    assert!(
        lock_out.status.success(),
        "lock failed: {}",
        stderr(&lock_out)
    );

    // Now edit the answered question's Problem text.
    let content = std::fs::read_to_string(&record).unwrap();
    let edited = content.replace("names leaked into decisions.", "something else entirely.");
    std::fs::write(&record, edited.clone()).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        !out.status.success(),
        "ask should have refused: {}",
        stdout(&out)
    );
    assert_eq!(out.status.code(), Some(1));

    // Nothing should have changed: the file is exactly what we wrote.
    let after = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        after, edited,
        "ask must not modify the file when it refuses"
    );
}
