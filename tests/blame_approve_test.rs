mod common;

use adr::blame;
use adr::record;
use common::*;

fn setup() -> (std::path::PathBuf, String, String) {
    blame_setup()
}

// --- a valid file: every destination kind, one approve, one row ----------
//
// adr: 34f0202a-0be4-4049-94de-d3886f4f8ffc
// adr: e879fe72-ec07-48cd-9902-2b158aaae726
//
// "the origin's path and git blob identifier, then one row per line range
// ... naming what the range went to" (34f0202a A) and "approve takes a
// blame file, checks it, and writes one ledger event over its hash"
// (e879fe72 A): every destination kind in the closed set resolves in one
// pass, the file is rewritten with ids where the set names one, and
// exactly one `blame` row lands in the ledger over that rewritten
// content's hash.
#[test]
fn approve_resolves_every_destination_kind_and_writes_one_row() {
    let (dir, id1, _id2) = setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs/other.md"), "some other origin\n").unwrap();

    let blob = write_origin(
        &dir,
        "docs/x.md",
        "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\n",
    );
    let rows = "1-1\tdecision: Does this pass?\n\
2-2\tterm: pass\n\
3-3\tprose: example\n\
4-4\tdropped: dead\n\
5-5\tdropped: detail\n\
6-6\tdropped: copy of Does this pass?\n\
7-7\tdropped: copy of docs/other.md\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    assert!(stdout(&out).contains("blame approved"), "{}", stdout(&out));

    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains(&format!("decision: {}", id1)), "{}", saved);
    assert!(
        saved.contains("term: 11111111-1111-1111-1111-111111111111"),
        "{}",
        saved
    );
    assert!(saved.contains("prose: example"), "{}", saved);
    assert!(saved.contains("dropped: dead"), "{}", saved);
    assert!(saved.contains("dropped: detail"), "{}", saved);
    assert!(saved.contains(&format!("dropped: copy of {}", id1)), "{}", saved);
    assert!(saved.contains("dropped: copy of docs/other.md"), "{}", saved);

    assert_eq!(blame_row_count(&dir), 1);
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let blame_line = ledger.lines().find(|l| l.contains("\tblame\t")).unwrap();
    let cols: Vec<&str> = blame_line.split('\t').collect();
    assert_eq!(cols[4], "adrdom/blame/x.md");
    let expected_hash = blame::file_hash(&record::split_lines(&saved));
    assert_eq!(cols[3], expected_hash);
}

// adr: e879fe72-ec07-48cd-9902-2b158aaae726
//
// "a blame file that differs from its latest approval is a draft":
// re-approving the file `approve` already rewrote to its canonical form
// changes nothing, so it writes no second row.
#[test]
fn approving_again_with_no_change_writes_no_new_row() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/x.md", "line 1\nline 2\n");
    let rows = "1-1\tdropped: dead\n2-2\tdropped: detail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);

    let out1 = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out1.status.success(), "{}", stderr(&out1));
    assert!(stdout(&out1).contains("blame approved"));
    assert_eq!(blame_row_count(&dir), 1);

    let out2 = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert!(stdout(&out2).contains("blame unchanged"), "{}", stdout(&out2));
    assert_eq!(blame_row_count(&dir), 1, "no second row should be written");
}

// --- coverage: gap, beyond, and overlap -----------------------------------
//
// adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
//
// "the ranges reach every line of the origin with no gap ... and approve
// stops a file with a gap": a run of lines named by no row refuses, and
// writes nothing.
#[test]
fn a_gap_in_coverage_refuses_and_writes_nothing() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(
        &dir,
        "docs/gap.md",
        "one\ntwo\nthree\nfour\nfive\n",
    );
    // Line 3 is covered by no row.
    let rows = "1-2\tdropped: dead\n4-5\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/gap.md", "docs/gap.md", &blob, rows);
    let before = std::fs::read_to_string(&path).unwrap();

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("covered by no row"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before, "nothing written on refusal");
}

// adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
//
// A row naming lines past the origin's own length refuses the same way.
#[test]
fn a_range_beyond_the_origin_refuses() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/beyond.md", "one\ntwo\nthree\n");
    let rows = "1-10\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/beyond.md", "docs/beyond.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("beyond"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
//
// "a range may appear in more than one row": overlapping rows that still
// cover the whole origin pass.
#[test]
fn overlapping_rows_that_fully_cover_the_origin_pass() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/overlap.md", "a\nb\nc\nd\n");
    let rows = "1-3\tdropped: dead\n2-4\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/overlap.md", "docs/overlap.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 1);
}

// --- destination refusals --------------------------------------------------
//
// adr: 38216d39-b064-4927-ab60-951a3c89e388
//
// "a controlled vocabulary is a closed set of values, each with a defined
// referent, and anything else is rejected": an unknown heading refuses.
#[test]
fn an_unknown_heading_refuses() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/unknown-heading.md", "one\n");
    let rows = "1-1\tdecision: Does this heading not exist anywhere?\n";
    let path = write_blame_file(&dir, "adrdom/blame/unknown-heading.md", "docs/unknown-heading.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("no question in any topic file"),
        "{}",
        stderr(&out)
    );
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 38216d39-b064-4927-ab60-951a3c89e388
//
// A retired term refuses.
#[test]
fn a_retired_term_refuses() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/retired-term.md", "one\n");
    let rows = "1-1\tterm: gone\n";
    let path = write_blame_file(&dir, "adrdom/blame/retired-term.md", "docs/retired-term.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("retired"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 38216d39-b064-4927-ab60-951a3c89e388
//
// `dropped: copy of <heading>` must name a CURRENT decision: an asked but
// unanswered heading refuses.
#[test]
fn a_copy_target_that_is_not_a_current_decision_refuses() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/noncurrent.md", "one\n");
    let rows = "1-1\tdropped: copy of Does this also pass?\n";
    let path = write_blame_file(&dir, "adrdom/blame/noncurrent.md", "docs/noncurrent.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("not a current decision"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 38216d39-b064-4927-ab60-951a3c89e388
//
// `prose: <stem>` names no topic file that exists.
#[test]
fn a_missing_prose_stem_refuses() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/missing-stem.md", "one\n");
    let rows = "1-1\tprose: no-such-topic\n";
    let path = write_blame_file(&dir, "adrdom/blame/missing-stem.md", "docs/missing-stem.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("does not exist"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 38216d39-b064-4927-ab60-951a3c89e388
//
// Free text naming none of the closed set refuses with the fixed wording.
#[test]
fn free_text_refuses_with_the_no_known_destination_wording() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/freetext.md", "one\n");
    let rows = "1-1\tsomething else entirely\n";
    let path = write_blame_file(&dir, "adrdom/blame/freetext.md", "docs/freetext.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("blame row names no known destination"),
        "{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains("agent rewrites the row"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 38216d39-b064-4927-ab60-951a3c89e388
//
// `dropped: copy of <path>` naming a path that is neither in the working
// tree nor already mapped by another blame file refuses.
#[test]
fn a_copy_of_a_missing_path_refuses() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/copy-missing.md", "one\n");
    let rows = "1-1\tdropped: copy of docs/does-not-exist.md\n";
    let path = write_blame_file(&dir, "adrdom/blame/copy-missing.md", "docs/copy-missing.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("no file in the working tree"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 34f0202a-0be4-4049-94de-d3886f4f8ffc
// The blob line is a content hash, not a promise that the object is in the
// store: an origin that was never staged or committed (the first blame file
// of a fresh repository) approves from the file at its path.
#[test]
fn an_unstaged_origin_approves_from_its_path() {
    let (dir, _id1, _id2) = setup();
    let rel = "docs/fresh.md";
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join(rel), "one\ntwo\n").unwrap();
    // Hash only, nothing written to the object store.
    let out = run_git(&dir, &["hash-object", rel]);
    let blob = stdout(&out).trim().to_string();
    let path = write_blame_file(&dir, "adrdom/blame/fresh.md", rel, &blob, "1-2\tdropped: dead\n");

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 1);
}

// adr: 34f0202a-0be4-4049-94de-d3886f4f8ffc
#[test]
fn a_blob_that_differs_from_the_origin_at_its_path_refuses() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/drift.md", "one\ntwo\n");
    std::fs::write(dir.join("docs/drift.md"), "one\ntwo\nthree\n").unwrap();
    let path = write_blame_file(&dir, "adrdom/blame/drift.md", "docs/drift.md", &blob, "1-2\tdropped: dead\n");

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("blob does not match the content of docs/drift.md"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
}
