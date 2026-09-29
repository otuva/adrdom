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

    let out = adr(&dir, &["show", "--all"], &[], "");
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
        text.contains("from: docs/x.md (1 row)"),
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

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("awaiting approval"), "{}", text);
}

// adr: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
//
// A `decision:` row naming a question that was approved but never asked
// prints the "unasked" state, not "answered".
#[test]
fn show_tags_a_decision_row_naming_an_unasked_question_with_the_unasked_state() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_via_ledger(&dir, BLAME_TERMS);
    let record = write_record(&dir, "adrdom/decisions/example.md", BLAME_TOPIC);
    approve_record(&dir, "adrdom/decisions/example.md");
    // Never asked.
    let content = std::fs::read_to_string(&record).unwrap();
    let id1 = extract_ids(&content)[0].clone(); // "Does this pass?"

    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let rows = format!("1-1\tdecision: {}\n", id1);
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, &rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("decision: Does this pass? [not asked]"), "{}", text);
}

// adr: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
//
// A `decision:` row naming a question that is asked but not yet answered
// prints the "open" state.
#[test]
fn show_tags_a_decision_row_naming_an_asked_unanswered_question_with_the_open_state() {
    let (dir, _id1, id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let rows = format!("1-1\tdecision: {}\n", id2);
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, &rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("decision: Does this also pass? [asked]"), "{}", text);
}

// adr: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
//
// Two blame files, two different origins, both holding a `decision:` row
// naming the same answered question: `show` prints both `# blame:`
// sections and, under that decision, one `from:` line per origin with its
// own row count.
#[test]
fn show_prints_a_from_line_per_origin_for_two_blame_files_naming_the_same_decision() {
    let (dir, id1, _id2) = blame_setup();

    let blob_a = write_origin(&dir, "docs/a.md", "one\n");
    let path_a = write_blame_file(
        &dir,
        "adrdom/blame/a.md",
        "docs/a.md",
        &blob_a,
        "1-1\tdecision: Does this pass?\n",
    );
    let out = adr_owner(&dir, &["approve", path_a.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let blob_b = write_origin(&dir, "docs/b.md", "one\ntwo\n");
    let rows_b = "1-1\tdecision: Does this pass?\n2-2\tdecision: Does this pass?\n";
    let path_b = write_blame_file(&dir, "adrdom/blame/b.md", "docs/b.md", &blob_b, rows_b);
    let out = adr_owner(&dir, &["approve", path_b.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);

    assert!(text.contains("# blame: adrdom/blame/a.md"), "{}", text);
    assert!(text.contains("# blame: adrdom/blame/b.md"), "{}", text);
    assert!(text.contains("from: docs/a.md (1 row)"), "{}", text);
    assert!(text.contains("from: docs/b.md (2 rows)"), "{}", text);
    let _ = id1;
}

// adr: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
//
// A blame file with a shape error is still printed (7b05f8df A: "each
// blame file"): a bad header (no "origin: ..." on line 1) prints only its
// `# blame:` line and a `  shape: ` line; a valid header with one
// malformed row (no tab) alongside a good one prints its `# blame:` line,
// a `  shape: ` line, and its good rows.
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

    assert!(text.contains("# blame: adrdom/blame/a.md"), "{}", text);
    assert!(text.contains("# blame: adrdom/blame/b.md"), "{}", text);
    assert!(
        text.lines().any(|l| l.starts_with("  shape: ")),
        "{}",
        text
    );
    // File B's good row still prints alongside its own shape error.
    assert!(text.contains("dropped: detail"), "{}", text);
}

// adr: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
//
// A draft row `dropped:  dead` (two spaces after the prefix) is counted in
// the counts line as `dropped: dead 1` and printed as a dropped row with
// reason `dead`, the same as the canonical single-spaced form.
#[test]
fn a_dropped_row_with_extra_spaces_after_the_prefix_is_counted_and_printed_as_dead() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let rows = "1-1\tdropped:  dead\n"; // two spaces after "dropped:"
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    // Never approved: show renders a draft file's rows as written.

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("dropped: dead 1"), "{}", text);
    assert!(text.contains("1\tdropped: dead"), "{}", text);
}

// adr: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
//
// The owner's follow-up: a `dropped: copy of <heading>` row earns the
// same `[state]` tag a `decision:` row gets, e.g. `[answered A]`.
#[test]
fn show_tags_a_copy_of_heading_row_with_the_targets_state() {
    let (dir, id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let rows = "1-1\tdropped: copy of Does this pass?\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains(&format!("dropped: copy of {}", id1)), "{}", saved);

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("dropped: copy of Does this pass? [answered A]"),
        "{}",
        text
    );
}
