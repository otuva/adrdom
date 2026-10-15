mod common;

use adr::blame;
use adr::record;
use common::*;

// --- 6f6e7f4a A: check compares a blame file's blob with its origin -------

// adr: 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70
//
// "a mismatch fails until the agent rewrites the rows and the owner approves
// again": editing the origin after the blame file approved against its old
// blob fails `check`, naming the origin and the move.
#[test]
fn check_fails_when_the_origin_was_edited_after_approval() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdetail\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    // The origin's content changes; its blob no longer matches the row
    // `approve` already checked in.
    std::fs::write(dir.join("docs/x.md"), "one\ntwo\nedited\n").unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("adrdom/blame/x.md: blob no longer matches docs/x.md"), "{}", text);
    assert!(text.contains("owner runs: adr approve adrdom/blame/x.md"), "{}", text);
}

// adr: 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70
//
// "a mismatch fails until the agent rewrites the rows and the owner approves
// again": once the blame file's `blob:` line is updated to the origin's
// current hash and the owner re-approves, `check` passes again -- no blame
// failure, and no awaiting-approval note either (the file is approved, not a
// draft).
#[test]
fn check_passes_after_the_blob_line_is_updated_and_reapproved() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdetail\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    // Edited in place, same line count, so the existing rows still cover
    // the whole origin once the blob line is updated below.
    std::fs::write(dir.join("docs/x.md"), "one\ntwo-edited\n").unwrap();
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(stdout(&out).contains("no longer matches"), "{}", stdout(&out));

    // The agent updates the blob line to the edited content's current hash,
    // in place, so the file keeps the id line the first approve wrote.
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

// adr: 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70
//
// A blame file awaiting approval is a draft: the same mismatch is a note
// (exit 0, the line printed), and approve refuses the file on that same line.
#[test]
fn a_blob_mismatch_on_a_draft_is_a_note() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdetail\n2-2\tdetail\n");
    // Never approved. The origin is edited after the blob line was written,
    // with the same line count, so the blob is the only finding.
    std::fs::write(dir.join("docs/x.md"), "one\ntwo-edited\n").unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("adrdom/blame/x.md: blob no longer matches docs/x.md"),
        "{}",
        stdout(&out)
    );

    let approve = adr_owner(&dir, &["approve", "adrdom/blame/x.md"], "");
    assert!(!approve.status.success(), "approve takes a blob that differs from its origin");
    let refused = refusal_lines(&approve, "approve: ");
    assert!(!refused.is_empty(), "{}", stderr(&approve));
    for line in &refused {
        assert!(stdout(&out).contains(line.as_str()), "approve refuses with {:?}, check prints:\n{}", line, stdout(&out));
    }
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70
//
// `check --staged`, the command the pre-commit hook runs, reads a tracked
// origin as a file: an approved blame file whose origin is staged with the
// content its blob names passes, with no path failure.
#[test]
fn check_staged_passes_on_an_approved_blame_file_whose_origin_is_tracked() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdetail\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    run_git(&dir, &["add", "-A"]);

    let out = adr(&dir, &["check", "--staged"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("blame:"), "{}", text);
    assert!(!text.contains("is not a regular file"), "{}", text);
}

// adr: 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70
//
// `check --staged` reads the origin from the index: an edit to the origin
// staged there fails with the mismatch even with the old content restored
// on disk; plain `check` reads the working-tree copy and reports none.
#[test]
fn check_staged_reads_the_origin_from_the_index_not_the_working_tree() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdetail\n2-2\tdetail\n";
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

// adr: 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70
//
// The index holds plain paths only, so an origin written `./docs/x.md` or
// `docs//x.md` is looked up, read and hashed by its plain path in every
// source: `check --staged` and plain `check` agree, and the messages keep
// naming the origin as written.
#[test]
fn check_staged_compares_an_origin_written_with_a_leading_dot_slash() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdetail\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "./docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    run_git(&dir, &["add", "-A"]);

    std::fs::write(dir.join("docs/x.md"), "one\ntwo-edited\n").unwrap();
    run_git(&dir, &["add", "docs/x.md"]);

    let line = "adrdom/blame/x.md: blob no longer matches ./docs/x.md";
    let plain_out = adr(&dir, &["check"], &[], "");
    assert!(!plain_out.status.success());
    assert!(stdout(&plain_out).contains(line), "{}", stdout(&plain_out));
    let staged_out = adr(&dir, &["check", "--staged"], &[], "");
    assert!(!staged_out.status.success(), "{}", stdout(&staged_out));
    assert!(stdout(&staged_out).contains(line), "{}", stdout(&staged_out));

    // A draft whose origin is written with a doubled slash finds its staged
    // origin: no `is missing` line.
    let blob = write_origin(&dir, "docs/y.md", "one\n");
    write_blame_file(&dir, "adrdom/blame/y.md", "docs//y.md", &blob, "1-1\tdetail\n");
    run_git(&dir, &["add", "-A"]);
    let staged_out = adr(&dir, &["check", "--staged"], &[], "");
    assert!(!stdout(&staged_out).contains("is missing"), "{}", stdout(&staged_out));
}

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
    let rows = "1-1\tdetail\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let good = std::fs::read_to_string(&path).unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "seed blame file"]);

    // Stage a shape error: a row with no tab.
    let bad = good.replacen("1-1\tdetail\n", "1-1 detail\n", 1);
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

// --- 0c3d498a A: an origin leaves once its blame file is approved ----------

// adr: 0c3d498a-c540-47f3-8384-050fc044c109
//
// "the approved blame file is the threshold; the owner deletes the origin,
// git keeps the text, check stays silent": with the origin gone, an approved
// blame file is no finding of check's, show prints no line for it, and
// approve has nothing to write.
#[test]
fn check_is_silent_on_a_gone_origin_once_its_blame_file_is_approved() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdecision\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    std::fs::remove_file(dir.join("docs/x.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(!stdout(&out).contains("adrdom/blame/x.md"), "{}", stdout(&out));
    assert!(!stdout(&out).contains("docs/x.md"), "{}", stdout(&out));

    // Show says nothing of it either: no line in the listing, and no finding
    // or move under the file when it is named.
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert!(!shown.contains("blame"), "{}", shown);
    assert_eq!(file_moves(&dir, "adrdom/blame/x.md"), Vec::<String>::new());

    // Approve has nothing to write: no line, no row.
    let rows_before = blame_row_count(&dir);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "", "{}", stdout(&out));
    assert_eq!(blame_row_count(&dir), rows_before);
}

// adr: 0c3d498a-c540-47f3-8384-050fc044c109
//
// The pre-commit hook runs `check --staged`: the commit that deletes the
// origin of an approved blame file passes it, and the commit goes through.
#[test]
fn check_staged_passes_the_commit_that_deletes_an_approved_origin() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdecision\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    run_git(&dir, &["add", "-A"]);
    let out = run_git(&dir, &["commit", "-q", "-m", "approve the blame file"]);
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));

    let out = run_git(&dir, &["rm", "-q", "docs/x.md"]);
    assert!(out.status.success(), "{}", stderr(&out));

    let out = adr(&dir, &["check", "--staged"], &[], "");
    assert!(out.status.success(), "check --staged should pass: {}", stdout(&out));
    assert!(!stdout(&out).contains("adrdom/blame/x.md"), "{}", stdout(&out));

    let out = run_git(&dir, &["commit", "-q", "-m", "delete the origin"]);
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
}

// adr: 0c3d498a-c540-47f3-8384-050fc044c109
//
// "an origin missing while its blame file is unapproved is the only
// finding": a blame file never approved, its origin gone, is a note with the
// exact line, check passes, and approve refuses on that same line.
#[test]
fn check_names_an_origin_gone_while_its_blame_file_awaits_approval() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdetail\n2-2\tdetail\n";
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    // Never approved.

    std::fs::remove_file(dir.join("docs/x.md")).unwrap();

    let line = "adrdom/blame/x.md: origin docs/x.md is missing while the blame file is new; agent restores docs/x.md from git";
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(stdout(&out).lines().any(|l| l == line), "{}", stdout(&out));

    let approve = adr_owner(&dir, &["approve", "adrdom/blame/x.md"], "");
    assert!(!approve.status.success(), "approve takes a file whose origin is gone");
    assert!(refusal_lines(&approve, "approve: ").contains(&line.to_string()), "{}", stderr(&approve));
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 0c3d498a-c540-47f3-8384-050fc044c109
//
// A renamed origin counts as missing too: the blame file still names the old
// path, and nothing sits there any more. Here the file was approved and then
// edited, so it is changed, and the line says so.
#[test]
fn a_renamed_origin_counts_as_missing_and_names_the_old_path() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdecision\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    // Edited in place after the approval, so the file is a draft again.
    let saved = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, saved.replacen("2-2\tdetail", "2-2\tdecision", 1)).unwrap();

    std::fs::rename(dir.join("docs/x.md"), dir.join("docs/x-renamed.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    let line = "adrdom/blame/x.md: origin docs/x.md is missing while the blame file is changed; agent restores docs/x.md from git";
    assert!(text.lines().any(|l| l == line), "{}", text);
}

// adr: 0c3d498a-c540-47f3-8384-050fc044c109
//
// A missing origin with a blame file never approved is a note, not a
// failure: check passes and names the origin, the wait for the approval and
// the move.
#[test]
fn check_notes_a_gone_origin_while_the_blame_file_is_unapproved() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdetail\n2-2\tdetail\n";
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    // Never approved.

    std::fs::remove_file(dir.join("docs/x.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("adrdom/blame/x.md: origin docs/x.md is missing"), "{}", text);
    assert!(text.contains("is missing while the blame file is new"), "{}", text);
    assert!(text.contains("agent restores docs/x.md from git"), "{}", text);
}

// --- d2851ea1 A: a row says decision or detail -------------------------

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// "one row per line range saying decision or detail": a file approved with
// rows in the old destination form (a question, a term, a marked drop) is
// not a file of that shape, and check fails on each row of it.
#[test]
fn an_old_destination_row_fails_check_on_an_approved_file() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\nthree\n");
    let rows = "1-1\tdecision: Is this pass?\n2-2\tterm: pass\n3-3\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    // Approve refuses this file, so its approval is written to the ledger by
    // hand, over the hash of the file as it stands.
    let content = std::fs::read_to_string(&path).unwrap();
    let hash = blame::file_hash(&record::split_lines(&content));
    append_ledger_row(&dir, "blame", "-", &hash, "adrdom/blame/x.md");

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "check should fail: {}", stdout(&out));
    let text = stdout(&out);
    for (n, form) in [(4, "decision: Is this pass?"), (5, "term: pass"), (6, "dropped: dead")] {
        let found = format!(
            "adrdom/blame/x.md: line {}: row value \"{}\" is neither decision nor detail",
            n, form
        );
        assert!(text.contains(&found), "{}", text);
    }
}

// --- ee27fd4c A: check reports no decision row that no question names ------

// adr: a469ecb9-a4f1-4a2a-89c5-7378cc1138b4
//
// "No ... no finding, no refusal": an approved blame file with decision rows
// that no question names is no line of check's, and ask and answer on a
// topic file are not refused because of it.
#[test]
fn check_is_silent_on_a_decision_row_no_question_names() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_via_ledger(&dir, BLAME_TERMS);
    let record = write_record(&dir, "adrdom/decisions/example.md", BLAME_TOPIC);
    approve_record(&dir, "adrdom/decisions/example.md");

    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\nthree\n");
    let rows = "1-1\tdecision\n2-3\tdecision\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    // No question anywhere names the rows; ask and answer run all the same.
    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask should not stop for the rows: {}", stderr(&out));
    let content = std::fs::read_to_string(&record).unwrap();
    let id1 = extract_ids(&content)[0].clone();
    let out = adr_answer(&dir, &id1, "A");
    assert!(out.status.success(), "answer should not stop for the rows: {}", stderr(&out));
    write_test_comment(&dir, &id1);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(!stdout(&out).contains("adrdom/blame/x.md"), "{}", stdout(&out));
    assert_eq!(file_moves(&dir, "adrdom/blame/x.md"), Vec::<String>::new());
}

// --- an unapproved blame file is no line of check's ----------------------

//
// A draft blame file with no fault (its origin still present) never fails
// `check`, and its status is no line of check's: its move is in `adr show`
// (bd83aa22).
#[test]
fn check_passes_an_unapproved_blame_file_and_prints_nothing_of_it() {
    // adr: bd83aa22-94fc-49ac-b215-a82990ff8c65
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdetail\n2-2\tdetail\n";
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    // Never approved.

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "check should pass: {}", stdout(&out));
    assert!(!stdout(&out).contains("adrdom/blame/x.md"), "{}", stdout(&out));
}

//
// `ask` and `answer` stop while the terms file awaits approval ("Does the tool
// stop while the terms file waits for approval?"); a blame file awaiting
// approval is not among what stops them, so both run on an ordinary topic
// file with one present.
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
        "1-1\tdetail\n2-2\tdetail\n",
    );
    // Never approved -- a draft the whole time.

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask should ignore blame files: {}", stderr(&out));

    let content = std::fs::read_to_string(&record).unwrap();
    let id1 = extract_ids(&content)[0].clone();
    let out = adr_answer(&dir, &id1, "A");
    assert!(out.status.success(), "answer should ignore blame files: {}", stderr(&out));
}

// A blame file's shape error on a file awaiting approval: approve refuses
// it, and check reports the same line, as a note (05141f49).
#[test]
fn check_notes_a_shape_error_in_a_blame_file_awaiting_approval() {
    // adr: 80b89c24-7d65-43f4-bb03-55e9d85c327d
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // The origin is there: the shape fault is the file's only finding.
    write_origin(&dir, "docs/x.md", "one\n");
    write_blame_file(&dir, "adrdom/blame/bad.md", "docs/x.md", "not-a-blob", "1-1\tdetail\n");

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("adrdom/blame/bad.md: line 2: missing \"blob: <40 or 64 hex>\""),
        "{}",
        stdout(&out)
    );
    let approve = adr_owner(&dir, &["approve", "adrdom/blame/bad.md"], "");
    assert!(!approve.status.success(), "approve takes a blame file with a shape fault");
    for line in stderr(&approve).lines() {
        let finding = line;
        assert!(stdout(&out).contains(finding), "approve refuses with {:?}, check prints:\n{}", finding, stdout(&out));
    }
}

//
// A draft blame file naming a directory as its origin: `check` prints the
// fault of its header (the "origin" term: "a prose file outside adrdom"),
// never the missing-origin message -- a directory is never "missing", it
// is just never a valid origin at all. On a draft it is a note like every
// shape fault (05141f49), on an approved file a failure.
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
        "1-1\tdetail\n",
    );
    // Never approved: a note.

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("is not a regular file outside adrdom"), "{}", text);
    assert!(!text.contains("is missing"), "{}", text);
}

// --- fail-open: check must never silently skip or pass -------------------

//
// A blame file whose own content cannot be read as text is a named failure,
// never silently skipped from the listing.
#[test]
fn check_fails_naming_a_blame_file_that_cannot_be_read() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::create_dir_all(dir.join("adrdom/blame")).unwrap();
    std::fs::write(dir.join("adrdom/blame/binary.md"), [0x66, 0x6f, 0xff, 0xfe, 0x0a]).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("adrdom/blame/binary.md: cannot be read"), "{}", text);
    assert!(text.contains("agent rewrites adrdom/blame/binary.md"), "{}", text);
}

//
// A truly missing origin (nothing at all at the path) still takes the
// missing-origin branch, not the unreadable-origin one -- pinning the
// `Source::exists` split. On a draft it is a note.
#[test]
fn check_still_takes_the_missing_branch_for_a_truly_gone_origin() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdecision\n2-2\tdetail\n";
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    // Never approved.

    std::fs::remove_file(dir.join("docs/x.md")).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("is missing"), "{}", text);
    assert!(!text.contains("cannot be read"), "{}", text);
}

//
// A present-but-unreadable origin (non-UTF-8) is its own failure, never
// routed through the missing-origin branch: a present file is not "gone".
#[test]
fn check_fails_a_present_unreadable_origin_as_its_own_failure() {
    let dir = blame_setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs/binary.md"), [0x66, 0x6f, 0xff, 0xfe, 0x0a]).unwrap();
    let rows = "1-1\tdetail\n";
    write_blame_file(&dir, "adrdom/blame/binary.md", "docs/binary.md", "0".repeat(40).as_str(), rows);
    // Never approved.

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("exists but cannot be read"), "{}", text);
    assert!(!text.contains("is missing"), "{}", text);
}

// adr: 80b89c24-7d65-43f4-bb03-55e9d85c327d
//
// A row with a bad value, in check, in show and in approve: the one finding,
// word for word, with its move. A good row, and a `decision` row that no
// question names, give no line.
#[test]
fn check_show_and_approve_print_the_same_line_for_a_bad_row_value() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdecision\n2-2\tdecision: Is nothing here at all?\n";
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);

    let check = adr(&dir, &["check"], &[], "");
    let show = adr(&dir, &["show", "adrdom/blame/x.md"], &[], "");
    assert!(check.status.success(), "{}", stdout(&check));
    assert!(show.status.success(), "{}", stderr(&show));
    let blame_lines = |text: &str| -> Vec<String> {
        let mut v: Vec<String> = text.lines().filter(|l| l.starts_with("adrdom/blame/")).map(str::to_string).collect();
        v.sort();
        v
    };
    let from_check = blame_lines(&stdout(&check));
    let from_show = blame_lines(&stdout(&show));
    let line = "adrdom/blame/x.md: line 5: row value \"decision: Is nothing here at all?\" is neither decision nor detail; agent writes decision or detail after the tab";
    assert_eq!(from_check, vec![line.to_string()], "check:\n{}", stdout(&check));
    assert_eq!(from_check, from_show, "check:\n{}\nshow:\n{}", stdout(&check), stdout(&show));
    // approve refuses the file on that same finding.
    let approve = adr_owner(&dir, &["approve", "adrdom/blame/x.md"], "");
    assert!(!approve.status.success(), "approve takes a row with a bad value");
    let mut from_approve = refusal_lines(&approve, "approve: ");
    from_approve.sort();
    assert_eq!(from_approve, from_check, "approve:\n{}", stderr(&approve));
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// "one identifier names one blame file": of two drafts holding one
// identifier, the first by path keeps it and the other gets the finding.
#[test]
fn of_two_drafts_sharing_an_id_the_first_by_path_keeps_it() {
    let dir = blame_setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let content = format!(
        "origin: docs/x.md\nblob: {}\nid: 0e0e0e0e-1111-4222-8333-444455556666\n\n1-1\tdetail\n",
        blob
    );
    write_record(&dir, "adrdom/blame/a.md", &content);
    write_record(&dir, "adrdom/blame/b.md", &content);

    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    let line = "adrdom/blame/b.md: its id line holds the identifier of adrdom/blame/a.md; agent removes the id line from adrdom/blame/b.md";
    assert!(stdout(&out).lines().any(|l| l == line), "{}", stdout(&out));
    assert!(
        !stdout(&out).lines().any(|l| l.starts_with("adrdom/blame/a.md") && l.contains("holds the identifier of")),
        "{}",
        stdout(&out)
    );
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// A staged symbolic link is no regular file: `check --staged` reads the mode
// of the index entry and agrees with plain `check`.
#[cfg(unix)]
#[test]
fn check_staged_refuses_a_staged_symlink_origin_as_check_does() {
    let dir = blame_setup();
    write_origin(&dir, "docs/x.md", "a\nb\n");
    std::os::unix::fs::symlink("x.md", dir.join("docs/link.md")).unwrap();
    run_git(&dir, &["add", "docs"]);
    let blob = String::from_utf8(run_git(&dir, &["rev-parse", ":docs/link.md"]).stdout).unwrap().trim().to_string();
    write_blame_file(&dir, "adrdom/blame/l.md", "docs/link.md", &blob, "1-1\tdetail\n");
    run_git(&dir, &["add", "adrdom/blame/l.md"]);
    let line = "adrdom/blame/l.md: \"docs/link.md\" is not a regular file outside adrdom; agent rewrites the origin line";
    for args in [&["check", "--staged"][..], &["check"][..]] {
        let out = adr(&dir, args, &[], "");
        assert!(stdout(&out).lines().any(|l| l == line), "{:?}: {}", args, stdout(&out));
    }
}

// adr: 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70
//
// The blob identifier is the one git stores: with line-ending conversion on,
// a CRLF file in the working tree matches the blob of its staged LF copy in
// both sources.
#[test]
fn a_blob_of_a_converted_origin_matches_in_both_sources() {
    let dir = blame_setup();
    run_git(&dir, &["config", "core.autocrlf", "input"]);
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs/x.md"), "a\r\nb\r\n").unwrap();
    run_git(&dir, &["add", "docs/x.md"]);
    let blob = String::from_utf8(run_git(&dir, &["rev-parse", ":docs/x.md"]).stdout).unwrap().trim().to_string();
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-2\tdetail\n");
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    run_git(&dir, &["add", "-A"]);
    for args in [&["check", "--staged"][..], &["check"][..]] {
        let out = adr(&dir, args, &[], "");
        assert!(!stdout(&out).contains("no longer matches"), "{:?}: {}", args, stdout(&out));
        assert!(!stdout(&out).contains("adrdom/blame/x.md"), "{:?}: {}", args, stdout(&out));
    }
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// An id line edited after approval differs from the identifier of the
// latest `blame` row: check fails, and the move writes the approved
// identifier back.
#[test]
fn a_hand_edited_id_makes_an_approved_file_a_draft_check_notes_it_and_approve_refuses() {
    let dir = blame_setup();
    let path = approved_blame_file(&dir, "x", "docs/x.md", "one\n", "1-1\tdetail\n");
    let id = id_line_of(&path);
    let text = std::fs::read_to_string(&path).unwrap();
    let other = "22222222-2222-4222-8222-222222222222";
    std::fs::write(&path, text.replace(&id, other)).unwrap();

    // The edit changes the file's hash, so the file is a draft again: the
    // finding is a note in check, and approve refuses on it.
    let out = adr(&dir, &["check"], &[], "");
    let line = format!(
        "adrdom/blame/x.md: its id line differs from the identifier of its approval; agent writes back the id line \"id: {}\"",
        id
    );
    assert!(stdout(&out).lines().any(|l| l == line), "{}", stdout(&out));
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert_eq!(refusal_lines(&out, "approve: "), vec![line], "{}", stderr(&out));
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// A renamed approved file holds an identifier of a row for another path and
// has no row of its own: neither id fault applies.
#[test]
fn a_renamed_approved_file_is_clean() {
    let dir = blame_setup();
    let path = approved_blame_file(&dir, "x", "docs/x.md", "one\n", "1-1\tdetail\n");
    std::fs::rename(&path, dir.join("adrdom/blame/y.md")).unwrap();
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(!stdout(&out).contains("id line"), "{}", stdout(&out));
}
