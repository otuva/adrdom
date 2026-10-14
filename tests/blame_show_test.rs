mod common;

use common::*;
use std::path::Path;

fn commit_all(dir: &Path, message: &str) {
    run_git(dir, &["add", "-A"]);
    let out = run_git(dir, &["commit", "-q", "-m", message]);
    assert!(
        out.status.success(),
        "commit failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// What `adr show` prints for an approved blame file (per "Does show print what
// a file holds past its latest approval?" and "What does a blame file
// hold of its origin?"): its status, its origin and every row as the file
// carries it, `decision` or `detail`, with no status tag on a row, no `id:`
// line, no count per destination and no line under a decision naming the
// origins that fed it.
#[test]
fn show_prints_a_blame_file_with_its_status_origin_and_rows() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "line 1\nline 2\n");
    let rows = "1-1\tdecision\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["show", "adrdom/blame/x.md"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert_eq!(
        text,
        "[approved] adrdom/blame/x.md\n  origin: docs/x.md\n  1-1\tdecision\n  2-2\tdetail\n"
    );
    // No status tag on a row line.
    for row in text.lines().filter(|l| l.starts_with("  1-1") || l.starts_with("  2-2")) {
        assert!(!row.contains('['), "{}", row);
    }

    let all = stdout(&adr(&dir, &["show", "--all"], &[], ""));
    assert!(all.contains(&text), "{}", all);
    assert!(!all.contains("from: "), "{}", all);
    assert!(!all.contains("decision 1"), "{}", all);
}

// A blame file never approved is new, every line of it marked; one approved
// and then edited is changed, the rows it lost and gained marked as written.
#[test]
fn show_marks_a_draft_blame_file_against_its_latest_approval() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let path = write_blame_file(
        &dir,
        "adrdom/blame/x.md",
        "docs/x.md",
        &blob,
        "1-1\tdecision\n2-2\tdetail\n",
    );
    let text = stdout(&adr(&dir, &["show", "adrdom/blame/x.md"], &[], ""));
    assert!(text.starts_with("[new] adrdom/blame/x.md\n"), "{}", text);
    assert!(text.contains("  + 1-1\tdecision\n  + 2-2\tdetail\n"), "{}", text);
    assert!(text.ends_with("owner runs: adr approve adrdom/blame/x.md\n"), "{}", text);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    commit_all(&dir, "approve the blame file");
    let content = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, content.replace("2-2\tdetail", "2-2\tdecision")).unwrap();
    let text = stdout(&adr(&dir, &["show", "adrdom/blame/x.md"], &[], ""));
    assert!(text.starts_with("[changed] adrdom/blame/x.md\n"), "{}", text);
    assert!(text.contains("    1-1\tdecision\n  - 2-2\tdetail\n  + 2-2\tdecision\n"), "{}", text);
}

// Two blame files: `show --all` prints each in the file form, and `show`
// with one of them prints that one alone.
#[test]
fn show_prints_each_blame_file_and_one_alone_when_it_is_named() {
    let dir = blame_setup();

    let blob_a = write_origin(&dir, "docs/a.md", "one\n");
    let path_a = write_blame_file(&dir, "adrdom/blame/a.md", "docs/a.md", &blob_a, "1-1\tdecision\n");
    let out = adr_owner(&dir, &["approve", path_a.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let blob_b = write_origin(&dir, "docs/b.md", "one\ntwo\n");
    let rows_b = "1-1\tdecision\n2-2\tdetail\n";
    let path_b = write_blame_file(&dir, "adrdom/blame/b.md", "docs/b.md", &blob_b, rows_b);
    let out = adr_owner(&dir, &["approve", path_b.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let text = stdout(&adr(&dir, &["show", "--all"], &[], ""));
    assert!(text.contains("[approved] adrdom/blame/a.md\n"), "{}", text);
    assert!(text.contains("[approved] adrdom/blame/b.md\n"), "{}", text);

    let one = stdout(&adr(&dir, &["show", "adrdom/blame/a.md"], &[], ""));
    assert!(!one.contains("adrdom/blame/b.md"), "{}", one);
    // A topic file named alone prints no blame file.
    let topic = stdout(&adr(&dir, &["show", "adrdom/decisions/example.md"], &[], ""));
    assert!(!topic.contains("blame"), "{}", topic);
}

// A blame file with a shape error is still printed: its status, the lines it
// carries, and the agent's move on each shape error.
#[test]
fn a_blame_file_with_a_bad_header_and_one_with_a_bad_row_are_both_still_printed() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    // File A: line 1 is not an "origin: ..." line at all.
    write_record(
        &dir,
        "adrdom/blame/a.md",
        "not an origin line\nblob: 0000000000000000000000000000000000000000\n\n1-1\tdetail\n",
    );

    // File B: a valid header, one malformed row (no tab) alongside a good one.
    write_record(
        &dir,
        "adrdom/blame/b.md",
        "origin: docs/y.md\nblob: 0000000000000000000000000000000000000000\n\n1-1 detail\n2-2\tdetail\n",
    );

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);

    assert!(text.contains("[new] adrdom/blame/a.md\n"), "{}", text);
    assert!(text.contains("[new] adrdom/blame/b.md\n"), "{}", text);
    assert!(
        text.lines().any(|l| l.starts_with("adrdom/blame/a.md: line 1: missing \"origin: <path>\"")),
        "{}",
        text
    );
    assert!(
        text.lines().any(|l| l.starts_with(
            "adrdom/blame/b.md: line 4: not a row (\"<start>-<end>\", a tab, then decision or detail)"
        )),
        "{}",
        text
    );
    // File B's good row still prints alongside its own shape error.
    assert!(text.contains("  + 2-2\tdetail"), "{}", text);
}

// The moves of every blame file join the listing with no file named: a blame
// file awaiting approval names the owner's command, and once the owner
// approves it the line is gone. A blame file with nothing to do prints
// nothing.
#[test]
fn short_show_names_the_approval_a_blame_file_waits_for() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let path = write_blame_file(
        &dir,
        "adrdom/blame/x.md",
        "docs/x.md",
        &blob,
        "1-1\tdetail\n2-2\tdetail\n",
    );

    let out = adr(&dir, &["show"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("[new] adrdom/blame/x.md\n")
            && text.trim_end().ends_with("owner runs: adr approve adrdom/blame/x.md"),
        "{}",
        text
    );

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&adr(&dir, &["show"], &[], ""));
    assert!(!text.contains("blame"), "{}", text);

    // The origin changes: the agent's line names the blob that no longer matches.
    write_origin(&dir, "docs/x.md", "one\ntwo\nthree\n");
    let text = stdout(&adr(&dir, &["show"], &[], ""));
    assert!(
        text.contains("adrdom/blame/x.md: blob no longer matches docs/x.md"),
        "{}",
        text
    );
}

// adr: a469ecb9-a4f1-4a2a-89c5-7378cc1138b4
// adr: cdc905a2-5b58-4c38-beff-4a8bd3b6f42e
//
// "the count of decision rows with and without a question is printed for the
// owner" (ee27fd4c A), "a computed view under a flag" (cdc905a2 A): with
// `--questions` the listing of a blame file ends in one line counting its
// decision rows a question names and the ones none names, the same by the name
// of the blame file in the question and by its identifier; without the flag
// nothing about questions is printed, and the flag changes nothing for a topic
// file or the terms file.
#[test]
fn show_counts_decision_rows_with_and_without_a_question_under_its_flag() {
    let dir = widget_repo();
    let rows = "1-1\tdecision\n2-2\tdecision\n3-3\tdecision\n4-4\tdetail\n";
    approved_blame_file(&dir, "x", "docs/x.md", "one\ntwo\nthree\nfour\n", rows);
    let topic = "adrdom/decisions/gate.md";
    write_record(&dir, topic, &gate_topic("Does the widget gate open?", "blame: x 1-1\n"));

    let listing = "[approved] adrdom/blame/x.md\n  origin: docs/x.md\n  1-1\tdecision\n  2-2\tdecision\n  3-3\tdecision\n  4-4\tdetail\n";
    let count = "  decision rows: 1 with a question, 2 with none\n";

    // The topic file is a draft: its line holds the name of the blame file.
    let flagged = adr(&dir, &["show", "--questions", "adrdom/blame/x.md"], &[], "");
    assert!(flagged.status.success(), "{}", stderr(&flagged));
    assert_eq!(stdout(&flagged), format!("{}{}", listing, count));

    // Without the flag: the listing alone, no line about questions.
    let plain = adr(&dir, &["show", "adrdom/blame/x.md"], &[], "");
    assert!(plain.status.success(), "{}", stderr(&plain));
    assert_eq!(stdout(&plain), listing);
    assert!(!stdout(&plain).contains("decision rows"), "{}", stdout(&plain));

    // `--all` takes the flag too, and without it prints no count.
    let all = stdout(&adr(&dir, &["show", "--all", "--questions"], &[], ""));
    assert_eq!(all.matches("decision rows:").count(), 1, "{}", all);
    assert!(all.contains(&format!("{}{}", listing, count)), "{}", all);
    let all = stdout(&adr(&dir, &["show", "--all"], &[], ""));
    assert!(!all.contains("decision rows"), "{}", all);

    // Approved: the line holds the identifier, and the count is the same.
    approve_record(&dir, topic);
    let flagged = adr(&dir, &["show", "--questions", "adrdom/blame/x.md"], &[], "");
    assert_eq!(stdout(&flagged), format!("{}{}", listing, count));

    // A topic file and the terms file print what they print without the flag.
    for file in [topic, "adrdom/terms.md"] {
        let without = adr(&dir, &["show", file], &[], "");
        let with = adr(&dir, &["show", "--questions", file], &[], "");
        assert!(with.status.success(), "{}: {}", file, stderr(&with));
        assert_eq!(stdout(&with), stdout(&without), "{}", file);
        assert!(!stdout(&with).contains("decision rows"), "{}: {}", file, stdout(&with));
    }
}

// adr: 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40
//
// "show turns an identifier back into the name": a blame file whose id line
// holds the identifier of a question does not rename it. The question keeps
// its heading in the depends line of the question standing on it.
#[test]
fn a_blame_id_never_replaces_the_name_of_a_question() {
    let dir = widget_repo();
    let rel = "adrdom/decisions/gate.md";
    let text = format!(
        "# Gate\n\nThe gate.\n\n{}\n{}",
        gate_question("Is alpha ready?", "the widget breaks."),
        gate_question("Is beta ready?", "the widget breaks.").replacen(
            "depends: widget\n",
            "depends: widget, Is alpha ready?=*\n",
            1
        )
    );
    write_record(&dir, rel, &text);
    approve_record(&dir, rel);
    let ids = extract_ids(&std::fs::read_to_string(dir.join(rel)).unwrap());
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let content = format!("origin: docs/x.md\nblob: {}\nid: {}\n\n1-1\tdetail\n", blob, ids[0]);
    write_record(&dir, "adrdom/blame/x.md", &content);

    let shown = stdout(&adr(&dir, &["show", rel], &[], ""));
    assert!(shown.contains("  depends: widget, Is alpha ready?=*\n"), "{}", shown);
    assert!(!shown.contains("depends: widget, x"), "{}", shown);
}

// adr: b0acc95b-3dd5-4158-ac5c-65a2159972a9
//
// "what the next move on the file reads, and the rest under a flag": show of
// a draft blame file prints its rows, which approve reads, and the move; the
// count of decision rows a question names comes only under `--questions`.
#[test]
fn show_of_a_draft_blame_file_prints_its_rows_and_the_count_only_under_questions() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdecision\n2-2\tdetail\n");

    let plain = stdout(&adr(&dir, &["show", "adrdom/blame/x.md"], &[], ""));
    assert!(plain.starts_with("[new] adrdom/blame/x.md\n"), "{}", plain);
    assert!(plain.contains("  + 1-1\tdecision\n  + 2-2\tdetail\n"), "{}", plain);
    assert!(plain.ends_with("owner runs: adr approve adrdom/blame/x.md\n"), "{}", plain);
    assert!(!plain.contains("decision rows"), "{}", plain);

    let flagged = stdout(&adr(&dir, &["show", "--questions", "adrdom/blame/x.md"], &[], ""));
    assert!(flagged.contains("  + 1-1\tdecision\n  + 2-2\tdetail\n"), "{}", flagged);
    assert!(flagged.contains("  decision rows: 0 with a question, 1 with none\n"), "{}", flagged);
}
