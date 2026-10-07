mod common;

use common::*;

// --- 5fddeeb5 A: check holds a blame file to its origin's blob -----------

// adr: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6
//
// "check compares the origin at its path with the blob identifier in the
// blame file, and a mismatch fails until the agent updates the rows and
// the owner approves again": editing the origin after the blame file
// approved against its old blob fails `check`, naming the origin and the
// move.
#[test]
fn check_fails_when_the_origin_was_edited_after_approval() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdropped: dead\n2-2\tdropped: detail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    // The origin's content changes; its blob no longer matches the row
    // `approve` already checked in.
    std::fs::write(dir.join("docs/x.md"), "one\ntwo\nedited\n").unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("blame:"), "{}", text);
    assert!(text.contains("no longer matches"), "{}", text);
    assert!(text.contains("docs/x.md"), "{}", text);
}

// adr: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6
//
// "a mismatch fails until the agent updates the rows and the owner
// approves again": once the blame file's `blob:` line is updated to the
// origin's current hash and the owner re-approves, `check` passes again --
// no blame failure, and no awaiting-approval note either (the file is
// approved, not a draft).
#[test]
fn check_passes_after_the_blob_line_is_updated_and_reapproved() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdropped: dead\n2-2\tdropped: detail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    // Edited in place, same line count, so the existing rows still cover
    // the whole origin once the blob line is updated below.
    std::fs::write(dir.join("docs/x.md"), "one\ntwo-edited\n").unwrap();
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(stdout(&out).contains("no longer matches"), "{}", stdout(&out));

    // The agent updates the blob line to the edited content's current hash.
    let new_blob_out = run_git(&dir, &["hash-object", "docs/x.md"]);
    let new_blob = stdout(&new_blob_out).trim().to_string();
    let content = std::fs::read_to_string(&path).unwrap();
    let updated = content.replacen(&format!("blob: {}", blob), &format!("blob: {}", new_blob), 1);
    std::fs::write(&path, &updated).unwrap();

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(!stdout(&out).contains("no longer matches"), "{}", stdout(&out));
    assert!(!stdout(&out).contains("awaiting approval"), "{}", stdout(&out));
}

// adr: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6
//
// `check --staged`, the command the pre-commit hook runs, reads a tracked
// origin as a file: an approved blame file whose origin is staged with the
// content its blob names passes, with no path failure.
#[test]
fn check_staged_passes_on_an_approved_blame_file_whose_origin_is_tracked() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdropped: dead\n2-2\tdropped: detail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    run_git(&dir, &["add", "-A"]);

    let out = adr(&dir, &["check", "--staged"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("blame:"), "{}", text);
    assert!(!text.contains("is not a regular file"), "{}", text);
}

// adr: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6
//
// `check --staged` reads the origin from the index: an edit to the origin
// staged there fails with the mismatch even with the old content restored
// on disk; plain `check` reads the working-tree copy and reports none.
#[test]
fn check_staged_reads_the_origin_from_the_index_not_the_working_tree() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdropped: dead\n2-2\tdropped: detail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    run_git(&dir, &["add", "-A"]);

    std::fs::write(dir.join("docs/x.md"), "one\ntwo-edited\n").unwrap();
    run_git(&dir, &["add", "docs/x.md"]);
    // Restore the old content on disk; only the index still holds the edit.
    std::fs::write(dir.join("docs/x.md"), "one\ntwo\n").unwrap();

    let staged_out = adr(&dir, &["check", "--staged"], &[], "");
    assert!(!staged_out.status.success());
    assert!(
        stdout(&staged_out).contains("no longer matches"),
        "{}",
        stdout(&staged_out)
    );

    let plain_out = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&plain_out).contains("no longer matches"),
        "{}",
        stdout(&plain_out)
    );
}

// adr: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6
//
// `check --staged` also reads the blame file itself from the index: a
// shape error staged there is caught even with a good copy restored on
// disk; plain `check` reads the good working-tree copy and reports no
// shape error.
#[test]
fn check_staged_reads_the_blame_file_from_the_index_not_the_working_tree() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdropped: dead\n2-2\tdropped: detail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let good = std::fs::read_to_string(&path).unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "seed blame file"]);

    // Stage a shape error: a row with no tab.
    let bad = good.replacen("1-1\tdropped: dead\n", "1-1 dropped: dead\n", 1);
    std::fs::write(&path, &bad).unwrap();
    run_git(&dir, &["add", "adrdom/blame/x.md"]);
    // Restore the good file on disk; only the index still holds the bad shape.
    std::fs::write(&path, &good).unwrap();

    // A shape fault in a blame file awaiting approval is a note
    // (fe14991d, 05141f49): printed, failing nothing.
    let staged_out = adr(&dir, &["check", "--staged"], &[], "");
    assert!(staged_out.status.success(), "{}", stdout(&staged_out));
    assert!(stdout(&staged_out).contains("not a row"), "{}", stdout(&staged_out));

    let plain_out = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&plain_out).contains("not a row"),
        "{}",
        stdout(&plain_out)
    );
}

// --- 4bb5ef1b A: an origin leaving while its blame file has an open row --

// adr: 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc
//
// "check fails when an origin is missing while its blame file is
// unapproved or names a question that is not answered": a `decision:` row
// naming an unanswered heading, with the origin deleted, fails and names
// the open heading.
#[test]
fn check_fails_when_the_origin_is_gone_with_an_open_decision_row() {
    let (dir, _id1, id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdecision: Is this also pass?\n2-2\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains(&format!("decision: {}", id2)), "{}", saved);

    std::fs::remove_file(dir.join("docs/x.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("blame:"), "{}", text);
    assert!(text.contains("is missing"), "{}", text);
    assert!(text.contains("docs/x.md"), "{}", text);
}

// adr: 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc
//
// A renamed origin counts as missing too: the blame file still names the
// old path, and nothing sits there any more.
#[test]
fn a_renamed_origin_counts_as_missing_and_names_the_old_path() {
    let (dir, _id1, id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdecision: Is this also pass?\n2-2\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains(&format!("decision: {}", id2)), "{}", saved);

    std::fs::rename(dir.join("docs/x.md"), dir.join("docs/x-renamed.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("blame:"), "{}", text);
    assert!(text.contains("is missing"), "{}", text);
    assert!(text.contains("docs/x.md"), "{}", text);
}

// adr: 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc
//
// "the origin may go once every such row is closed": once the row's own
// heading is answered (current) and the blame file re-approved, a missing
// origin passes.
#[test]
fn check_passes_when_the_origin_is_gone_but_every_row_is_closed() {
    let (dir, id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdecision: Is this pass?\n2-2\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let _ = id1; // already answered by `blame_setup`.

    std::fs::remove_file(dir.join("docs/x.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(!stdout(&out).contains("is missing"));
}

// adr: 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc
//
// "the origin may go once every such row is closed": a decision row whose
// target was answered and later superseded is closed, not open again --
// the origin's row-holding claim is not revived by a later supersession.
#[test]
fn check_passes_when_the_origin_is_gone_and_the_decision_row_target_was_superseded() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_via_ledger(&dir, BLAME_TERMS);
    let record = write_record(&dir, "adrdom/decisions/example.md", BLAME_TOPIC);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let old_ids = extract_ids(&content);
    let old_id = old_ids[0].clone(); // "Is this pass?"
    let out = adr_answer(&dir, &old_id, "A");
    assert!(out.status.success(), "{}", stderr(&out));
    // Not tagged: `old_id` is about to be superseded, so it is never a
    // current decision by the end of this test -- tagging it would earn
    // check's own "stale" failure instead.

    // A third question supersedes it.
    let mut content = std::fs::read_to_string(&record).unwrap();
    let third = format!(
        "\n### Is this pass, revised?\ndepends: pass\nsupersedes: {}\n\n\
Problem: revisiting.\n\nStandard: clear yes or no.\n\n\
- A) Yes it still passes.\n  Why: reasons.\n  Costs: costs.\n\
- B) No.\n  Why: other reasons.\n  Costs: other costs.\n",
        old_id
    );
    content.push_str(&third);
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let new_id = extract_ids(&content)
        .into_iter()
        .find(|id| !old_ids.contains(id))
        .expect("new id assigned");
    let out = adr_answer(&dir, &new_id, "A");
    assert!(out.status.success(), "{}", stderr(&out));
    write_test_comment(&dir, &new_id);

    // `old_id`'s latest ledger state is now "superseded".
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = format!("1-1\tdecision: {}\n2-2\tdropped: dead\n", old_id);
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, &rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    std::fs::remove_file(dir.join("docs/x.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(!stdout(&out).contains("is missing"), "{}", stdout(&out));
}

// adr: 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc
//
// A missing origin with no open decision row, but the blame file itself
// still a draft, still fails: the blame file's own approval is one more
// open reason.
#[test]
fn check_fails_when_the_origin_is_gone_and_the_blame_file_is_unapproved() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdropped: dead\n2-2\tdropped: detail\n";
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    // Never approved.

    std::fs::remove_file(dir.join("docs/x.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("blame:"), "{}", text);
    assert!(text.contains("is missing"), "{}", text);
    assert!(text.contains("approval"), "{}", text);
}

// --- an unapproved blame file is no line of check's ----------------------

//
// A draft blame file with no fault (its origin still present) never fails
// `check`, and its status is no line of check's: its move is in `adr show`
// (3fc35d4f).
#[test]
fn check_passes_an_unapproved_blame_file_and_prints_nothing_of_it() {
    // adr: 3fc35d4f-90f3-4425-9dac-951a89631fdf
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdropped: dead\n2-2\tdropped: detail\n";
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    // Never approved.

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(!stdout(&out).contains("adrdom/blame/x.md"), "{}", stdout(&out));
}

//
// "ask, answer ... must not read blame files at all": with an unapproved
// blame file present, `ask` and `answer` on an ordinary topic file still
// succeed.
#[test]
fn ask_and_answer_ignore_an_unapproved_blame_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_via_ledger(&dir, BLAME_TERMS);
    let record = write_record(&dir, "adrdom/decisions/example.md", BLAME_TOPIC);
    approve_record(&dir, "adrdom/decisions/example.md");

    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    write_blame_file(
        &dir,
        "adrdom/blame/x.md",
        "docs/x.md",
        &blob,
        "1-1\tdropped: dead\n2-2\tdropped: detail\n",
    );
    // Never approved -- a draft the whole time.

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask should ignore blame files: {}", stderr(&out));

    let content = std::fs::read_to_string(&record).unwrap();
    let id1 = extract_ids(&content)[0].clone();
    let out = adr_answer(&dir, &id1, "A");
    assert!(out.status.success(), "answer should ignore blame files: {}", stderr(&out));
}

// A blame file's own shape error is always a failure, draft or not.
#[test]
fn check_notes_a_shape_error_in_a_blame_file_awaiting_approval() {
    // adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
    // approve refuses the shape fault; check reports it with the same move,
    // as a note, since the file awaits approval (05141f49).
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_blame_file(&dir, "adrdom/blame/bad.md", "docs/x.md", "not-a-blob", "1-1\tdropped: dead\n");

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(stdout(&out).contains("blame:"), "{}", stdout(&out));
}

// adr: 34f0202a-0be4-4049-94de-d3886f4f8ffc
//
// A draft blame file naming a directory as its origin: `check` prints the
// shape failure (the "origin" term: "a prose file outside adrdom"), never
// the missing-origin message -- a directory is never "missing", it is
// just never a valid origin at all.
#[test]
fn check_prints_the_shape_failure_not_missing_origin_for_a_directory() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    write_blame_file(
        &dir,
        "adrdom/blame/dir.md",
        "docs",
        "0".repeat(40).as_str(),
        "1-1\tdropped: dead\n",
    );
    // Never approved.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("is not a regular file outside adrdom"), "{}", text);
    assert!(!text.contains("is missing"), "{}", text);
}

// --- fail-open: check must never silently skip or pass -------------------

// adr: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6
//
// "check holds a blame file to its origin": a blame file whose own content
// cannot be read as text is a named failure, never silently skipped from
// the listing.
#[test]
fn check_fails_naming_a_blame_file_that_cannot_be_read() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::create_dir_all(dir.join("adrdom/blame")).unwrap();
    std::fs::write(dir.join("adrdom/blame/binary.md"), [0x66, 0x6f, 0xff, 0xfe, 0x0a]).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("blame:"), "{}", text);
    assert!(text.contains("adrdom/blame/binary.md"), "{}", text);
    assert!(text.contains("cannot be read"), "{}", text);
}

// adr: 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc
//
// A truly missing origin (nothing at all at the path) still takes the
// missing-origin branch, not the new unreadable-origin one -- pinning the
// `Source::exists` split this fix introduced.
#[test]
fn check_still_takes_the_missing_branch_for_a_truly_gone_origin() {
    let (dir, _id1, _id2) = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdecision: Is this also pass?\n2-2\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    std::fs::remove_file(dir.join("docs/x.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("is missing"), "{}", text);
    assert!(!text.contains("cannot be read"), "{}", text);
}

// adr: 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc
//
// A present-but-unreadable origin (non-UTF-8) is its own failure, never
// routed through the missing-origin branch: a present file is not "gone".
#[test]
fn check_fails_a_present_unreadable_origin_as_its_own_failure() {
    let (dir, _id1, _id2) = blame_setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs/binary.md"), [0x66, 0x6f, 0xff, 0xfe, 0x0a]).unwrap();
    let rows = "1-1\tdropped: dead\n";
    write_blame_file(&dir, "adrdom/blame/binary.md", "docs/binary.md", "0".repeat(40).as_str(), rows);
    // Never approved.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("exists but cannot be read"), "{}", text);
    assert!(!text.contains("is missing"), "{}", text);
}
