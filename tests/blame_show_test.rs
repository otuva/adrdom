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

//
// What `adr show` prints for a blame file (per "What does show print with a
// file named?" and "Does show print what a file holds beyond its latest
// approval?"): its status, its origin and every row as it carries it, a row
// naming a question with that question's status; a draft's lines marked
// against its latest approval; no count per destination and no line under a
// decision naming the origins that fed it (a view computed from the records).
#[test]
fn show_prints_a_blame_file_with_its_status_origin_and_rows() {
    let (dir, _id1, _id2) = blame_setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs/other.md"), "some other origin\n").unwrap();
    let blob = write_origin(&dir, "docs/x.md", "line 1\nline 2\nline 3\nline 4\nline 5\n");
    let rows = "1-1\tdecision: Is this pass?\n\
2-2\tterm: pass\n\
3-3\tdropped: dead\n\
4-4\tdropped: detail\n\
5-5\tdropped: copy of docs/other.md\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["show", "adrdom/blame/x.md"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert_eq!(
        text,
        "[approved] adrdom/blame/x.md\n  origin: docs/x.md\n  1-1\tdecision: Is this pass? [answered A]\n  \
2-2\tterm: pass\n  3-3\tdropped: dead\n  4-4\tdropped: detail\n  5-5\tdropped: copy of docs/other.md\n"
    );

    let all = stdout(&adr(&dir, &["show", "--all"], &[], ""));
    assert!(all.contains(&text), "{}", all);
    assert!(!all.contains("from: "), "{}", all);
    assert!(!all.contains("decision 1"), "{}", all);
}

// A blame file never approved is new, every line of it marked; one approved
// and then edited is changed, the rows it lost and gained marked.
#[test]
fn show_marks_a_draft_blame_file_against_its_latest_approval() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let path = write_blame_file(
        &dir,
        "adrdom/blame/x.md",
        "docs/x.md",
        &blob,
        "1-1\tdropped: dead\n2-2\tdropped: detail\n",
    );
    let text = stdout(&adr(&dir, &["show", "adrdom/blame/x.md"], &[], ""));
    assert!(text.starts_with("[new] adrdom/blame/x.md\n"), "{}", text);
    assert!(text.contains("  + 1-1\tdropped: dead\n  + 2-2\tdropped: detail\n"), "{}", text);
    assert!(text.ends_with("owner runs: adr approve adrdom/blame/x.md\n"), "{}", text);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    commit_all(&dir, "approve the blame file");
    let content = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, content.replace("2-2\tdropped: detail", "2-2\tdropped: dead")).unwrap();
    let text = stdout(&adr(&dir, &["show", "adrdom/blame/x.md"], &[], ""));
    assert!(text.starts_with("[changed] adrdom/blame/x.md\n"), "{}", text);
    assert!(text.contains("    1-1\tdropped: dead\n  - 2-2\tdropped: detail\n  + 2-2\tdropped: dead\n"), "{}", text);
}

//
// A `decision:` row naming a question that was approved but never asked
// carries that question's status, not "answered".
#[test]
fn show_tags_a_decision_row_naming_an_unasked_question_with_the_unasked_state() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_via_ledger(&dir, BLAME_TERMS);
    let record = write_record(&dir, "adrdom/decisions/example.md", BLAME_TOPIC);
    approve_record(&dir, "adrdom/decisions/example.md");
    // Never asked.
    let content = std::fs::read_to_string(&record).unwrap();
    let id1 = extract_ids(&content)[0].clone(); // "Is this pass?"

    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let rows = format!("1-1\tdecision: {}\n", id1);
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, &rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("decision: Is this pass? [not asked]"), "{}", text);
}

//
// A `decision:` row naming a question that is asked but not yet answered
// carries the status "asked".
#[test]
fn show_tags_a_decision_row_naming_an_asked_unanswered_question_with_the_asked_state() {
    let (dir, _id1, id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let rows = format!("1-1\tdecision: {}\n", id2);
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, &rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("decision: Is this also pass? [asked]"), "{}", text);
}

// Two blame files: `show --all` prints each in the file form, and `show`
// with one of them prints that one alone.
#[test]
fn show_prints_each_blame_file_and_one_alone_when_it_is_named() {
    let (dir, _id1, _id2) = blame_setup();

    let blob_a = write_origin(&dir, "docs/a.md", "one\n");
    let path_a = write_blame_file(&dir, "adrdom/blame/a.md", "docs/a.md", &blob_a, "1-1\tdecision: Is this pass?\n");
    let out = adr_owner(&dir, &["approve", path_a.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let blob_b = write_origin(&dir, "docs/b.md", "one\ntwo\n");
    let rows_b = "1-1\tdecision: Is this pass?\n2-2\tdecision: Is this pass?\n";
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
        "not an origin line\nblob: 0000000000000000000000000000000000000000\n\n1-1\tdropped: dead\n",
    );

    // File B: a valid header, one malformed row (no tab) alongside a good one.
    write_record(
        &dir,
        "adrdom/blame/b.md",
        "origin: docs/y.md\nblob: 0000000000000000000000000000000000000000\n\n1-1 dropped: dead\n2-2\tdropped: detail\n",
    );

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);

    assert!(text.contains("[new] adrdom/blame/a.md\n"), "{}", text);
    assert!(text.contains("[new] adrdom/blame/b.md\n"), "{}", text);
    assert!(text.lines().any(|l| l.starts_with("adrdom/blame/a.md:")), "{}", text);
    assert!(text.lines().any(|l| l.starts_with("adrdom/blame/b.md:")), "{}", text);
    // File B's good row still prints alongside its own shape error.
    assert!(text.contains("  + 2-2\tdropped: detail"), "{}", text);
}

// A draft row `dropped:  dead` (two spaces after the prefix) prints as the
// file carries it, and no count of rows per destination.
#[test]
fn a_dropped_row_with_extra_spaces_after_the_prefix_prints_as_written() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let rows = "1-1\tdropped:  dead\n"; // two spaces after "dropped:"
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);

    let text = stdout(&adr(&dir, &["show", "adrdom/blame/x.md"], &[], ""));
    assert!(text.contains("  + 1-1\tdropped:  dead\n"), "{}", text);
    assert!(!text.contains("dropped: dead 1"), "{}", text);
}

//
// A `dropped: copy of <heading>` row carries the status of the question it
// names, as a `decision:` row does.
#[test]
fn show_tags_a_copy_of_heading_row_with_the_targets_state() {
    let (dir, id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let rows = "1-1\tdropped: copy of Is this pass?\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains(&format!("dropped: copy of {}", id1)), "{}", saved);

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("dropped: copy of Is this pass? [answered A]"), "{}", text);
}

// The moves of every blame file join the listing with no file named: a blame
// file awaiting approval names the owner's command, and once the owner
// approves it the line is gone. A blame file with nothing to do prints
// nothing.
#[test]
fn short_show_names_the_approval_a_blame_file_waits_for() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let path = write_blame_file(
        &dir,
        "adrdom/blame/x.md",
        "docs/x.md",
        &blob,
        "1-1\tdropped: dead\n2-2\tdropped: detail\n",
    );

    let out = adr(&dir, &["show"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("[new] adrdom/blame/x.md\nowner runs: adr approve adrdom/blame/x.md\n"),
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
