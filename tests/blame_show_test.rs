mod common;

use common::*;

// adr: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
//
// "show prints, after the topic sections, one section per blame file: the
// origin path, approved or awaiting approval, the row count per
// destination kind ..., every dropped row with its range and reason ...,
// and every decision row with its range, the heading text and the
// question's state" plus "under each decision ... a line `from: <origin
// path> (<n> rows)`".
#[test]
fn show_prints_the_blame_file_counts_dropped_rows_and_the_from_line() {
    let (dir, id1, _id2) = blame_setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs/other.md"), "some other origin\n").unwrap();
    let blob = write_origin(
        &dir,
        "docs/x.md",
        "line 1\nline 2\nline 3\nline 4\nline 5\n",
    );
    let rows = "1-1\tdecision: Does this pass?\n\
2-2\tterm: pass\n\
3-3\tdropped: dead\n\
4-4\tdropped: detail\n\
5-5\tdropped: copy of docs/other.md\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["show"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);

    // The blame section itself.
    assert!(text.contains("blame: adrdom/blame/x.md"), "{}", text);
    assert!(text.contains("docs/x.md"), "{}", text);
    assert!(text.contains("approved"), "{}", text);
    assert!(text.contains("decision 1"), "{}", text);
    assert!(text.contains("term 1"), "{}", text);
    assert!(text.contains("dropped: dead 1"), "{}", text);
    assert!(text.contains("dropped: detail 1"), "{}", text);
    assert!(text.contains("dropped: copy 1"), "{}", text);
    // Dropped rows, with reason.
    assert!(text.contains("dropped: dead"), "{}", text);
    assert!(text.contains("dropped: detail"), "{}", text);
    assert!(text.contains("dropped: copy of docs/other.md"), "{}", text);
    // The decision row: heading text and question state.
    assert!(text.contains("Does this pass?"), "{}", text);
    assert!(text.contains("answered"), "{}", text);

    // Under the topic decision itself.
    assert!(
        text.contains(&format!("from: docs/x.md (1 rows)")),
        "{}",
        text
    );
    let _ = id1;
}

// adr: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
//
// A blame file still a draft shows "awaiting approval" rather than
// "approved".
#[test]
fn show_prints_awaiting_approval_for_a_draft_blame_file() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    write_blame_file(
        &dir,
        "adrdom/blame/x.md",
        "docs/x.md",
        &blob,
        "1-1\tdropped: dead\n2-2\tdropped: detail\n",
    );
    // Never approved.

    let out = adr(&dir, &["show"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("awaiting approval"), "{}", text);
}
