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
    let rows = "1-1\tdecision: Is this pass?\n\
2-2\tterm: pass\n\
3-3\tprose: example\n\
4-4\tdropped: dead\n\
5-5\tdropped: detail\n\
6-6\tdropped: copy of Is this pass?\n\
7-7\tdropped: copy of docs/other.md\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    assert_eq!(stdout(&out).trim_end(), "approved: adrdom/blame/x.md", "{}", stdout(&out));

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
    assert_eq!(stdout(&out1).trim_end(), "approved: adrdom/blame/x.md");
    assert_eq!(blame_row_count(&dir), 1);

    // No row written, so nothing printed (per "What does a command that
    // writes print?", 069a6b4a A).
    let out2 = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert_eq!(stdout(&out2), "", "{}", stdout(&out2));
    assert_eq!(blame_row_count(&dir), 1, "no second row should be written");
}

// A row put back to the heading after the approval is written to its
// identifier again; the approval stands, and no ledger row is written, so
// the file it rewrote is the one line approve prints (per "What does a
// command that writes print?", 069a6b4a A).
#[test]
fn approving_again_rewrites_the_rows_and_writes_no_row() {
    let (dir, id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/x.md", "line 1\n");
    let rows = "1-1\tdecision: Is this pass?\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let as_written = std::fs::read_to_string(&path).unwrap();

    let out1 = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out1.status.success(), "{}", stderr(&out1));

    std::fs::write(&path, &as_written).unwrap();
    let out2 = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert_eq!(stdout(&out2), "written: adrdom/blame/x.md\n", "{}", stdout(&out2));
    assert_eq!(blame_row_count(&dir), 1, "no second row should be written");
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains(&format!("decision: {}", id1)), "{}", saved);
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
// `dropped: copy of <heading>` resolves the same way a `decision:` row
// does (per the owner's fix: "copy of a question", any state -- not only
// a current one): an asked but unanswered heading still resolves.
#[test]
fn a_copy_target_in_any_state_resolves() {
    let (dir, _id1, id2) = setup();
    let blob = write_origin(&dir, "docs/noncurrent.md", "one\n");
    let rows = "1-1\tdropped: copy of Is this also pass?\n";
    let path = write_blame_file(&dir, "adrdom/blame/noncurrent.md", "docs/noncurrent.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains(&format!("dropped: copy of {}", id2)), "{}", saved);
    assert_eq!(blame_row_count(&dir), 1);
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
    assert!(stderr(&out).contains("blob no longer matches docs/drift.md"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
    // One judgement: check names the same finding with the same move.
    let refused = stderr(&out).lines().map(|l| l.trim_start_matches("approve: ").to_string()).collect::<Vec<_>>();
    let checked = stdout(&adr(&dir, &["check"], &[], ""));
    for r in &refused {
        assert!(checked.contains(r.as_str()), "check lacks {:?}:\n{}", r, checked);
    }
}

// --- origin term: a path must be a regular file outside adrdom ----------
//
// adr: 34f0202a-0be4-4049-94de-d3886f4f8ffc
//
// The "origin" term: "a prose file outside adrdom whose text the records
// take in". A directory is not a file.
#[test]
fn a_directory_origin_refuses_with_the_shape_wording() {
    let (dir, _id1, _id2) = setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    let path = write_blame_file(&dir, "adrdom/blame/dir.md", "docs", "0".repeat(40).as_str(), "1-1\tdropped: dead\n");
    let before = std::fs::read_to_string(&path).unwrap();

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("is not a regular file outside adrdom"),
        "{}",
        stderr(&out)
    );
    assert_eq!(blame_row_count(&dir), 0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
}

// adr: 34f0202a-0be4-4049-94de-d3886f4f8ffc
//
// A path under `adrdom/` is never outside it.
#[test]
fn an_origin_under_adrdom_refuses_with_the_shape_wording() {
    let (dir, _id1, _id2) = setup();
    let path = write_blame_file(
        &dir,
        "adrdom/blame/self.md",
        "adrdom/terms.md",
        "0".repeat(40).as_str(),
        "1-1\tdropped: dead\n",
    );

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("is not a regular file outside adrdom"),
        "{}",
        stderr(&out)
    );
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 38216d39-b064-4927-ab60-951a3c89e388
//
// A `dropped: copy of <path>` target is held to the same rule: never
// under `adrdom/`.
#[test]
fn a_copy_of_path_under_adrdom_refuses_with_the_shape_wording() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/copy-under-adrdom.md", "one\n");
    let rows = "1-1\tdropped: copy of adrdom/blame/notes.md\n";
    let path = write_blame_file(&dir, "adrdom/blame/copy-under-adrdom.md", "docs/copy-under-adrdom.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("is not a regular file outside adrdom"),
        "{}",
        stderr(&out)
    );
    assert_eq!(blame_row_count(&dir), 0);
}

// --- an origin with no trailing newline on its last line ------------------
//
// adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
//
// "the ranges reach every line of the origin": a file's last line with no
// trailing newline is still one more line to cover, the same way `wc -l`
// would miss it but `content.lines().count()` does not.
#[test]
fn an_origin_with_no_trailing_newline_on_its_last_line_covers_and_gaps_correctly() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/notail.md", "one\ntwo\nthree");
    let rows = "1-1\tdropped: dead\n2-2\tdropped: dead\n3-3\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/notail.md", "docs/notail.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 1);
}

// adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
//
// The same origin, with rows covering only its first two lines: the third
// line (the one with no trailing newline) is still counted, and still a
// gap, naming line 3 specifically.
#[test]
fn an_origin_with_no_trailing_newline_leaves_its_last_line_a_gap_when_uncovered() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/notail2.md", "one\ntwo\nthree");
    let rows = "1-2\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/notail2.md", "docs/notail2.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("line(s) 3 of the origin are covered by no row"),
        "{}",
        stderr(&out)
    );
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
//
// One `approve` run lists every refusal it finds, not just the first: a
// coverage gap and a row with an unknown destination together, and neither
// writes anything.
#[test]
fn approve_lists_a_coverage_gap_and_an_unknown_destination_together() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/both.md", "a\nb\nc\nd\n");
    // Lines 1-2: an unrecognized destination. Line 3: covered by no row.
    // Line 4: a good row.
    let rows = "1-2\tsomething else entirely\n4-4\tdropped: dead\n";
    let path = write_blame_file(&dir, "adrdom/blame/both.md", "docs/both.md", &blob, rows);
    let before = std::fs::read_to_string(&path).unwrap();

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("covered by no row"), "{}", err);
    assert!(err.contains("blame row names no known destination"), "{}", err);
    assert_eq!(blame_row_count(&dir), 0);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        before,
        "nothing written on refusal"
    );
}

// --- a copy target naming this blame file's own origin ---------------------
//
// adr: 38216d39-b064-4927-ab60-951a3c89e388
//
// "a copy is of a question or of another origin": a `dropped: copy of`
// row naming this blame file's own origin refuses.
#[test]
fn a_copy_of_this_files_own_origin_refuses() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let rows = "1-1\tdropped: copy of docs/x.md\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("is this blame file's own origin"),
        "{}",
        stderr(&out)
    );
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: 38216d39-b064-4927-ab60-951a3c89e388
//
// The same refusal when the copy target is spelled with a leading "./".
#[test]
fn a_copy_of_this_files_own_origin_with_a_leading_dot_slash_refuses() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let rows = "1-1\tdropped: copy of ./docs/x.md\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("is this blame file's own origin"),
        "{}",
        stderr(&out)
    );
    assert_eq!(blame_row_count(&dir), 0);
}

// --- a draft row with extra whitespace after the prefix ---------------------
//
// adr: 38216d39-b064-4927-ab60-951a3c89e388
//
// A row written with extra spaces after the "dropped:" prefix approves and
// is rewritten to its canonical, single-spaced form.
#[test]
fn a_dropped_row_with_extra_spaces_after_the_prefix_approves_and_is_rewritten_canonically() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/spaced.md", "one\n");
    let rows = "1-1\tdropped:  dead\n"; // two spaces after "dropped:"
    let path = write_blame_file(&dir, "adrdom/blame/spaced.md", "docs/spaced.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("1-1\tdropped: dead\n"), "{}", saved);
    assert!(!saved.contains("dropped:  dead"), "{}", saved);
    assert_eq!(blame_row_count(&dir), 1);
}

// --- an origin line spelled with a leading "./" under adrdom ---------------
//
// adr: 34f0202a-0be4-4049-94de-d3886f4f8ffc
//
// The "origin" term: "a prose file outside adrdom". A path under adrdom
// spelled with a leading "./" is still under adrdom.
#[test]
fn an_origin_spelled_with_a_leading_dot_slash_under_adrdom_refuses() {
    let (dir, _id1, _id2) = setup();
    let path = write_blame_file(
        &dir,
        "adrdom/blame/self2.md",
        "./adrdom/terms.md",
        "0".repeat(40).as_str(),
        "1-1\tdropped: dead\n",
    );

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("is not a regular file outside adrdom"),
        "{}",
        stderr(&out)
    );
    assert_eq!(blame_row_count(&dir), 0);
}

// --- the owner-command gate applies to a blame file too ---------------------
//
// adr: e879fe72-ec07-48cd-9902-2b158aaae726
//
// "approve takes a blame file": the same owner-command gate every `adr
// approve` call is held to applies here too. Without the owner bypass,
// `approve` on a blame file refuses as the owner's command, exit code 2,
// and writes nothing.
#[test]
fn approve_without_the_owner_bypass_refuses_as_an_owner_command() {
    let (dir, _id1, _id2) = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdropped: dead\n2-2\tdropped: detail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let before = std::fs::read_to_string(&path).unwrap();

    let out = adr(&dir, &["approve", path.to_str().unwrap()], &[], "");
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("this is the owner's command"),
        "{}",
        stderr(&out)
    );
    assert_eq!(blame_row_count(&dir), 0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
}

// --- fail-open: a present origin that cannot be read never falls back to
// the object store ----------------------------------------------------------
//
// adr: 34f0202a-0be4-4049-94de-d3886f4f8ffc
//
// A present origin that fails to read as text (non-UTF-8, here) refuses
// outright: it must never be treated the same as a gone origin, which
// would fall back to `git cat-file` and silently pass on stale content.
#[test]
fn a_present_non_utf8_origin_refuses_and_writes_nothing() {
    let (dir, _id1, _id2) = setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs/binary.md"), [0x66, 0x6f, 0xff, 0xfe, 0x0a]).unwrap();
    let path = write_blame_file(
        &dir,
        "adrdom/blame/binary.md",
        "docs/binary.md",
        "0".repeat(40).as_str(),
        "1-1\tdropped: dead\n",
    );
    let before = std::fs::read_to_string(&path).unwrap();

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("cannot be read as text"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
}
