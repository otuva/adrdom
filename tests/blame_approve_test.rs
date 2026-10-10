mod common;

use adr::blame;
use adr::record;
use common::*;

fn setup() -> std::path::PathBuf {
    blame_setup()
}

// --- a valid file: both row values, one approve, the id line, one row ------
//
// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
// adr: e879fe72-ec07-48cd-9902-2b158aaae726
//
// "Its path and blob identifier, then one row per line range saying decision
// or detail, no text copied" (d2851ea1 A) and "approve takes a blame file,
// checks it, and writes one ledger event over its hash" (e879fe72 A): a file
// of decision and detail rows approves in one pass. Approve rewrites no row
// (the padded last row stays as written) and adds the `id:` line as line 3,
// and exactly one `blame` row lands in the ledger over the hash of the file
// as written.
#[test]
fn approve_takes_decision_and_detail_rows_writes_the_id_line_and_one_row() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "line 1\nline 2\nline 3\nline 4\n");
    let rows = "1-2\tdecision\n3-3\tdetail\n4-4\t  detail  \n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let before = std::fs::read_to_string(&path).unwrap();

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "approve failed: {}", stderr(&out));
    assert_eq!(stdout(&out).trim_end(), "approved: adrdom/blame/x.md", "{}", stdout(&out));

    // The one change to the file is the `id:` line, line 3; every other line
    // is byte for byte as it was written.
    let saved = std::fs::read_to_string(&path).unwrap();
    let mut lines: Vec<&str> = saved.split_inclusive('\n').collect();
    let id_line = lines.remove(2);
    let id = id_line
        .strip_prefix("id: ")
        .unwrap_or_else(|| panic!("line 3 is no id line:\n{}", saved))
        .trim_end();
    assert!(record::is_uuid(id), "{}", saved);
    assert_eq!(lines.concat(), before, "every other line as it was written");

    assert_eq!(blame_row_count(&dir), 1);
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let blame_line = ledger.lines().find(|l| l.contains("\tblame\t")).unwrap();
    let cols: Vec<&str> = blame_line.split('\t').collect();
    assert_eq!(cols.len(), 6, "{}", blame_line);
    assert_eq!(cols[2], id, "the row carries the identifier on the file's id line");
    assert_eq!(cols[3], blame::file_hash(&record::split_lines(&saved)));
    assert_eq!(cols[4], "adrdom/blame/x.md");
}

// adr: e879fe72-ec07-48cd-9902-2b158aaae726
//
// "a blame file that differs from its latest approval is a draft":
// re-approving the file with no change writes no second row and no second
// `id:` line, and leaves the file as the first approve wrote it.
#[test]
fn approving_again_with_no_change_writes_no_new_row() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "line 1\nline 2\n");
    let rows = "1-1\tdecision\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);

    let out1 = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out1.status.success(), "{}", stderr(&out1));
    assert_eq!(stdout(&out1).trim_end(), "approved: adrdom/blame/x.md");
    assert_eq!(blame_row_count(&dir), 1);
    let first = std::fs::read_to_string(&path).unwrap();

    // No row written, so nothing printed (per "What does a command that
    // writes print?", 069a6b4a A).
    let out2 = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out2.status.success(), "{}", stderr(&out2));
    assert_eq!(stdout(&out2), "", "{}", stdout(&out2));
    assert_eq!(blame_row_count(&dir), 1, "no second row should be written");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), first, "the file is written only when the id was minted");
}

// adr: e879fe72-ec07-48cd-9902-2b158aaae726
//
// "writes one ledger event over its hash": a file edited after its approval
// is a draft; the next approve writes a second row over the new hash and
// keeps the identifier the first approve assigned.
#[test]
fn approving_an_edited_file_keeps_its_id_line_and_writes_a_second_row() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "line 1\nline 2\n");
    let rows = "1-1\tdecision\n2-2\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, rows);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let first = std::fs::read_to_string(&path).unwrap();
    let ids = extract_ids(&first);
    assert_eq!(ids.len(), 1, "{}", first);

    // Edited in place, so the file keeps the id line the first approve wrote.
    std::fs::write(&path, first.replacen("2-2\tdetail", "2-2\tdecision", 1)).unwrap();
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out).trim_end(), "approved: adrdom/blame/x.md", "{}", stdout(&out));

    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("2-2\tdecision"), "{}", saved);
    assert_eq!(extract_ids(&saved), ids, "the identifier is assigned once: {}", saved);
    assert_eq!(blame_row_count(&dir), 2);
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
    let dir = setup();
    let blob = write_origin(
        &dir,
        "docs/gap.md",
        "one\ntwo\nthree\nfour\nfive\n",
    );
    // Line 3 is covered by no row.
    let rows = "1-2\tdetail\n4-5\tdetail\n";
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
    let dir = setup();
    let blob = write_origin(&dir, "docs/beyond.md", "one\ntwo\nthree\n");
    let rows = "1-10\tdetail\n";
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
    let dir = setup();
    let blob = write_origin(&dir, "docs/overlap.md", "a\nb\nc\nd\n");
    let rows = "1-3\tdetail\n2-4\tdecision\n";
    let path = write_blame_file(&dir, "adrdom/blame/overlap.md", "docs/overlap.md", &blob, rows);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 1);
}

// --- row values: each old destination form is a shape fault ----------------
//
// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// "one row per line range saying decision or detail": a row that names where
// its range went (a question, a term, a topic file, a marked drop) or says
// anything else is no row of the file. Each form refuses with the shape
// fault of its line, the move that clears it, and nothing written.
#[test]
fn each_old_destination_is_a_shape_fault() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let forms = [
        "decision: Is this pass?",
        "term: pass",
        "prose: example",
        "dropped: dead",
        "dropped: detail",
        "dropped: copy of Is this pass?",
        "dropped: copy of docs/other.md",
        "something else entirely",
    ];
    for form in forms {
        let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, &format!("1-1\t{}\n", form));
        let before = std::fs::read_to_string(&path).unwrap();

        let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
        assert!(!out.status.success(), "{:?} was approved", form);
        let line = format!(
            "adrdom/blame/x.md: line 4: row value \"{}\" is neither decision nor detail; a blame row says decision or detail; agent writes decision or detail after the tab",
            form
        );
        assert!(refusal_lines(&out, "approve: ").contains(&line), "{:?}:\n{}", form, stderr(&out));
        assert_eq!(blame_row_count(&dir), 0, "{:?}", form);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before, "{:?}: nothing written on refusal", form);
    }
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// The shape of the file is the origin line, the blob line, a blank line and
// then the rows: each fault names its line and the move that clears it, all
// of a file's faults at once, and nothing is written.
#[test]
fn each_shape_fault_names_its_line_and_its_move() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let id = "0e0e0e0e-1111-4222-8333-444455556666";
    let cases = [
        (
            format!("not an origin line\nblob: {}\n\n1-1\tdetail\n", blob),
            "adrdom/blame/x.md: line 1: missing \"origin: <path>\"",
        ),
        (
            "origin: docs/x.md\nblob: not-a-blob\n\n1-1\tdetail\n".to_string(),
            "adrdom/blame/x.md: line 2: missing \"blob: <40 or 64 hex>\"",
        ),
        (
            format!("origin: docs/x.md\nblob: {}\n1-1\tdetail\n", blob),
            "adrdom/blame/x.md: line 3: expected a blank line after the header",
        ),
        (
            format!("origin: docs/x.md\nblob: {}\nid: {}\n1-1\tdetail\n", blob, id),
            "adrdom/blame/x.md: line 4: expected a blank line after the header",
        ),
        (
            format!("origin: docs/x.md\nblob: {}\n\n1-1 detail\n", blob),
            "adrdom/blame/x.md: line 4: not a row (\"<start>-<end>\", a tab, then decision or detail)",
        ),
    ];
    for (content, found) in cases {
        let path = write_record(&dir, "adrdom/blame/x.md", &content);

        let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
        assert!(!out.status.success(), "approved:\n{}", content);
        let refused = refusal_lines(&out, "approve: ");
        let line = refused
            .iter()
            .find(|l| l.starts_with(found))
            .unwrap_or_else(|| panic!("no line opens with {:?}:\n{}", found, stderr(&out)));
        assert!(line.ends_with("; agent rewrites the blame file"), "{}", line);
        assert_eq!(blame_row_count(&dir), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content, "nothing written on refusal");
    }
}

// --- the identifier: the tool assigns it, one file holds it ---------------
//
// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// "traceability runs from the derived artifact to its source by a stable
// identifier": the identifier of a blame file is one the tool assigns, so
// an `id:` line holding anything else refuses, and so does an `id:` line
// below the header.
#[test]
fn a_hand_written_id_line_that_is_no_identifier_refuses() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let content = format!(
        "origin: docs/x.md\nblob: {}\nid: not-an-identifier\n\n1-1\tdecision\n2-2\tdetail\n",
        blob
    );
    let path = write_record(&dir, "adrdom/blame/x.md", &content);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    let line = "adrdom/blame/x.md: line 3: id line holds \"not-an-identifier\", which is no identifier the tool assigns; the tool assigns a blame file's identifier; agent removes the id line, line 3";
    assert!(refusal_lines(&out, "approve: ").contains(&line.to_string()), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), content, "nothing written on refusal");

    // One judgement: check prints the same line, as a note on a draft.
    let checked = adr(&dir, &["check"], &[], "");
    assert!(checked.status.success(), "{}", stdout(&checked));
    assert!(stdout(&checked).lines().any(|l| l == line), "{}", stdout(&checked));
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// The same rule for an `id:` line below the header, among the rows.
#[test]
fn an_id_line_below_the_header_refuses() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let content = format!(
        "origin: docs/x.md\nblob: {}\n\n1-1\tdecision\nid: 0e0e0e0e-1111-4222-8333-444455556666\n2-2\tdetail\n",
        blob
    );
    let path = write_record(&dir, "adrdom/blame/x.md", &content);

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    let line = "adrdom/blame/x.md: line 5: id line below the header; the tool assigns a blame file's identifier; agent removes the id line, line 5";
    assert!(refusal_lines(&out, "approve: ").contains(&line.to_string()), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), content, "nothing written on refusal");
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// "one identifier names one blame file": a copy of an approved blame file
// carries the original's `id:` line, and approve refuses the copy on it,
// naming the file that holds it. The original, approved first, stays clean
// in check.
#[test]
fn a_copied_blame_file_is_refused_on_the_id_it_shares() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let original = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdecision\n2-2\tdetail\n");
    let out = adr_owner(&dir, &["approve", original.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));

    // The agent copies the approved file, `id:` line and all.
    let copy = dir.join("adrdom/blame/y.md");
    std::fs::copy(&original, &copy).unwrap();
    let before = std::fs::read_to_string(&copy).unwrap();

    let out = adr_owner(&dir, &["approve", copy.to_str().unwrap()], "");
    assert!(!out.status.success());
    let line = "adrdom/blame/y.md: its id line holds the identifier of adrdom/blame/x.md; one identifier names one blame file; agent removes the id line from adrdom/blame/y.md";
    assert!(refusal_lines(&out, "approve: ").contains(&line.to_string()), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 1, "only the original has an approval");
    assert_eq!(std::fs::read_to_string(&copy).unwrap(), before, "nothing written on refusal");

    // The copy is a draft, so check prints its line as a note; no line of
    // check's is about the original.
    let checked = adr(&dir, &["check"], &[], "");
    assert!(checked.status.success(), "{}", stdout(&checked));
    assert!(stdout(&checked).lines().any(|l| l == line), "{}", stdout(&checked));
    assert!(
        !stdout(&checked).lines().any(|l| l.starts_with("adrdom/blame/x.md")),
        "{}",
        stdout(&checked)
    );
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// "one identifier names one blame file": a file approved once never loses its
// identifier to a copy that was never approved, whatever order their paths
// sort in. The original is edited (its status is changed) and its copy sorts
// before it: check names the shared id on the copy only, and show offers the
// copy no approval.
#[test]
fn an_edited_original_keeps_its_id_over_a_copy_never_approved() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let original = write_blame_file(&dir, "adrdom/blame/b.md", "docs/x.md", &blob, "1-1\tdecision\n2-2\tdetail\n");
    let out = adr_owner(&dir, &["approve", original.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    std::fs::copy(&original, dir.join("adrdom/blame/a.md")).unwrap();

    // Edited in place, so the original keeps its id line and is changed.
    let text = std::fs::read_to_string(&original).unwrap();
    std::fs::write(&original, text.replacen("2-2\tdetail", "2-2\tdecision", 1)).unwrap();

    let line = "adrdom/blame/a.md: its id line holds the identifier of adrdom/blame/b.md; one identifier names one blame file; agent removes the id line from adrdom/blame/a.md";
    let checked = adr(&dir, &["check"], &[], "");
    assert!(stdout(&checked).lines().any(|l| l == line), "{}", stdout(&checked));
    assert!(
        !stdout(&checked).lines().any(|l| l.starts_with("adrdom/blame/b.md")),
        "{}",
        stdout(&checked)
    );
    let moves = file_moves(&dir, "adrdom/blame/a.md");
    assert_eq!(moves, vec![line.to_string()], "no approve move for the copy");
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// "the tool assigns a blame file's identifier": a blame file approved once
// and written again without its id line is changed, and approve does not mint
// it a new identifier, which would leave every blame line that holds the old
// one naming no blame file. It refuses on the finding check notes and show
// prints, and writes nothing.
#[test]
fn a_changed_blame_file_that_lost_its_id_line_refuses() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdecision\n2-2\tdetail\n");
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let approved = std::fs::read_to_string(&path).unwrap();

    // An agent writes the file again: same rows, the id line gone.
    let id_line = approved.lines().nth(2).unwrap();
    assert!(id_line.starts_with("id: "), "{}", approved);
    std::fs::write(&path, approved.replacen(&format!("{}\n", id_line), "", 1)).unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    let rows_before = blame_row_count(&dir);

    let line = format!(
        "adrdom/blame/x.md: the id line of its approval is gone; the tool assigns a blame file's identifier; agent writes back the id line \"id: {}\"",
        id_line.strip_prefix("id: ").unwrap()
    );
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve minted an identifier: {}", stdout(&out));
    assert_eq!(refusal_lines(&out, "approve: "), vec![line.clone()], "{}", stderr(&out));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), written, "nothing written on refusal");
    assert_eq!(blame_row_count(&dir), rows_before);

    let checked = adr(&dir, &["check"], &[], "");
    assert!(stdout(&checked).lines().any(|l| l == line), "{}", stdout(&checked));
    assert_eq!(file_moves(&dir, "adrdom/blame/x.md"), vec![line.clone()]);

    // The agent restores the line from git; the file is as approved again.
    std::fs::write(&path, &approved).unwrap();
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "", "{}", stdout(&out));
    assert_eq!(blame_row_count(&dir), rows_before);
}

// --- the terms file does not hold a blame file ------------------------------
//
// No row names a term, so approving a blame file does not wait for the terms
// file: with an entry of it awaiting approval the blame file approves, and
// the terms file is left as the agent wrote it.
#[test]
fn a_blame_file_approves_while_the_terms_file_awaits_approval() {
    let dir = setup();
    let terms = dir.join("adrdom/terms.md");
    let mut text = std::fs::read_to_string(&terms).unwrap();
    text.push_str("\n## gizmo\ndefinition: A fixture term for tests.\naliases: \n");
    std::fs::write(&terms, &text).unwrap();
    let blob = write_origin(&dir, "docs/old.md", "one\ntwo\n");
    let path = write_blame_file(&dir, "adrdom/blame/old.md", "docs/old.md", &blob, "1-2\tdetail\n");

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out).trim_end(), "approved: adrdom/blame/old.md", "{}", stdout(&out));
    assert_eq!(blame_row_count(&dir), 1);
    assert_eq!(std::fs::read_to_string(&terms).unwrap(), text, "no identifier written for the new entry");
}

// The blob line is a content hash, not a promise that the object is in the
// store: an origin that was never staged or committed (the first blame file
// of a fresh repository) approves from the file at its path.
#[test]
fn an_unstaged_origin_approves_from_its_path() {
    let dir = setup();
    let rel = "docs/fresh.md";
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join(rel), "one\ntwo\n").unwrap();
    // Hash only, nothing written to the object store.
    let out = run_git(&dir, &["hash-object", rel]);
    let blob = stdout(&out).trim().to_string();
    let path = write_blame_file(&dir, "adrdom/blame/fresh.md", rel, &blob, "1-2\tdetail\n");

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 1);
}

// adr: 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70
//
// "a mismatch fails until the agent rewrites the rows and the owner approves
// again": a blob line that differs from the origin at its path refuses, and
// check names the same finding with the same move.
#[test]
fn a_blob_that_differs_from_the_origin_at_its_path_refuses() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/drift.md", "one\ntwo\n");
    std::fs::write(dir.join("docs/drift.md"), "one\ntwo\nthree\n").unwrap();
    let path = write_blame_file(&dir, "adrdom/blame/drift.md", "docs/drift.md", &blob, "1-2\tdetail\n");

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
// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// The "origin" term: "a prose file outside adrdom whose text the records
// take in", and a blame file holds "its path" (d2851ea1 A). A directory is
// not a file.
#[test]
fn a_directory_origin_refuses_with_the_shape_wording() {
    let dir = setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    let path = write_blame_file(&dir, "adrdom/blame/dir.md", "docs", "0".repeat(40).as_str(), "1-1\tdetail\n");
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

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// A symbolic link is not a regular file: it would approve from the working
// tree and read as the link's target text from the index. The path of a link
// to a file with the blob the rows name refuses all the same.
#[cfg(unix)]
#[test]
fn a_symlink_origin_refuses() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/real.md", "one\ntwo\n");
    std::os::unix::fs::symlink("real.md", dir.join("docs/link.md")).unwrap();
    let path = write_blame_file(&dir, "adrdom/blame/link.md", "docs/link.md", &blob, "1-2\tdetail\n");
    let before = std::fs::read_to_string(&path).unwrap();

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    let line = "adrdom/blame/link.md: \"docs/link.md\" is not a regular file outside adrdom; an origin is a regular file outside adrdom; agent rewrites the origin line";
    assert_eq!(refusal_lines(&out, "approve: "), vec![line.to_string()], "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// A path under `adrdom/` is never outside it.
#[test]
fn an_origin_under_adrdom_refuses_with_the_shape_wording() {
    let dir = setup();
    let path = write_blame_file(
        &dir,
        "adrdom/blame/self.md",
        "adrdom/terms.md",
        "0".repeat(40).as_str(),
        "1-1\tdetail\n",
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

// --- an origin with no trailing newline on its last line ------------------
//
// adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
//
// "the ranges reach every line of the origin": a file's last line with no
// trailing newline is still one more line to cover, the same way `wc -l`
// would miss it but `content.lines().count()` does not.
#[test]
fn an_origin_with_no_trailing_newline_on_its_last_line_covers_and_gaps_correctly() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/notail.md", "one\ntwo\nthree");
    let rows = "1-1\tdetail\n2-2\tdetail\n3-3\tdetail\n";
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
    let dir = setup();
    let blob = write_origin(&dir, "docs/notail2.md", "one\ntwo\nthree");
    let rows = "1-2\tdetail\n";
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
// coverage gap and a row with a bad value together, and neither writes
// anything. A row with a bad value still counts for the lines of its range,
// so the gap is line 3 alone.
#[test]
fn approve_lists_a_coverage_gap_and_a_bad_row_value_together() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/both.md", "a\nb\nc\nd\n");
    // Lines 1-2: a row whose value is neither word. Line 3: covered by no
    // row. Line 4: a good row.
    let rows = "1-2\tsomething else entirely\n4-4\tdetail\n";
    let path = write_blame_file(&dir, "adrdom/blame/both.md", "docs/both.md", &blob, rows);
    let before = std::fs::read_to_string(&path).unwrap();

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("line(s) 3 of the origin are covered by no row"), "{}", err);
    assert!(!err.contains("line(s) 1-2"), "{}", err);
    assert!(err.contains("row value \"something else entirely\" is neither decision nor detail"), "{}", err);
    assert_eq!(blame_row_count(&dir), 0);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        before,
        "nothing written on refusal"
    );
}

// --- an origin line spelled with a leading "./" under adrdom ---------------
//
// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// The "origin" term: "a prose file outside adrdom". A path under adrdom
// spelled with a leading "./" is still under adrdom.
#[test]
fn an_origin_spelled_with_a_leading_dot_slash_under_adrdom_refuses() {
    let dir = setup();
    let path = write_blame_file(
        &dir,
        "adrdom/blame/self2.md",
        "./adrdom/terms.md",
        "0".repeat(40).as_str(),
        "1-1\tdetail\n",
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
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let rows = "1-1\tdetail\n2-2\tdetail\n";
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
// A present origin that fails to read as text (non-UTF-8, here) refuses
// outright: it must never be treated the same as a gone origin, which
// would fall back to `git cat-file` and silently pass on stale content.
#[test]
fn a_present_non_utf8_origin_refuses_and_writes_nothing() {
    let dir = setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs/binary.md"), [0x66, 0x6f, 0xff, 0xfe, 0x0a]).unwrap();
    let path = write_blame_file(
        &dir,
        "adrdom/blame/binary.md",
        "docs/binary.md",
        "0".repeat(40).as_str(),
        "1-1\tdetail\n",
    );
    let before = std::fs::read_to_string(&path).unwrap();

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("cannot be read as text"), "{}", stderr(&out));
    assert_eq!(blame_row_count(&dir), 0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
}

/// Approve refuses `origin` as written with the not-a-regular-file finding
/// and writes nothing.
fn assert_origin_path_refused(dir: &std::path::Path, name: &str, origin: &str, blob: &str) {
    let path = write_blame_file(dir, &format!("adrdom/blame/{}.md", name), origin, blob, "1-1\tdetail\n");
    let before = std::fs::read_to_string(&path).unwrap();
    let out = adr_owner(dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success(), "{}: {}", origin, stdout(&out));
    let line = format!(
        "adrdom/blame/{}.md: \"{}\" is not a regular file outside adrdom; an origin is a regular file outside adrdom; agent rewrites the origin line",
        name, origin
    );
    assert_eq!(refusal_lines(&out, "approve: "), vec![line], "{}", stderr(&out));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// An origin is a file inside the repository: an absolute path, a path that
// climbs above the root and a path ending in a slash are no origin.
#[test]
fn an_origin_path_that_leaves_the_repository_refuses() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let readme = write_origin(&dir, "README.md", "r\n");
    assert_origin_path_refused(&dir, "up", "../README.md", &readme);
    assert_origin_path_refused(&dir, "abs", "/README.md", &readme);
    assert_origin_path_refused(&dir, "slash", "docs/x.md/", &blob);
    assert_origin_path_refused(&dir, "deep", "docs/../../README.md", &readme);
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// A `..` that stays inside the repository is only another spelling.
#[test]
fn an_origin_path_with_a_dotdot_inside_the_repository_approves() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let path = write_blame_file(&dir, "adrdom/blame/in.md", "docs/../docs/x.md", &blob, "1-1\tdetail\n");
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// Any component of the path that is a symbolic link, not only the last one,
// makes the origin no regular file: `docs/ad` pointing into adrdom must not
// carry an origin out of the tree it is refused in.
#[cfg(unix)]
#[test]
fn an_origin_below_a_symlinked_directory_refuses() {
    let dir = setup();
    let blob = write_origin(&dir, "real/x.md", "one\n");
    std::os::unix::fs::symlink("real", dir.join("docs_link")).unwrap();
    assert_origin_path_refused(&dir, "via", "docs_link/x.md", &blob);
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// Approve adds the id line and keeps every other byte: a blame file with
// CRLF line endings gets an id line ended the same way.
#[test]
fn approve_keeps_the_line_endings_of_a_crlf_blame_file() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let path = dir.join("adrdom/blame/crlf.md");
    std::fs::create_dir_all(dir.join("adrdom/blame")).unwrap();
    std::fs::write(&path, format!("origin: docs/x.md\r\nblob: {}\r\n\r\n1-1\tdetail\r\n", blob)).unwrap();
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = std::fs::read_to_string(&path).unwrap();
    let id = id_line_of(&path);
    assert_eq!(
        text,
        format!("origin: docs/x.md\r\nblob: {}\r\nid: {}\r\n\r\n1-1\tdetail\r\n", blob, id)
    );
    let checked = adr(&dir, &["check"], &[], "");
    assert!(!stdout(&checked).contains("adrdom/blame/crlf.md"), "{}", stdout(&checked));
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// A blame file with LF endings keeps them.
#[test]
fn approve_keeps_the_line_endings_of_an_lf_blame_file() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let path = write_blame_file(&dir, "adrdom/blame/lf.md", "docs/x.md", &blob, "1-1\tdetail\n");
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains('\r'), "{:?}", text);
    assert_eq!(text, format!("origin: docs/x.md\nblob: {}\nid: {}\n\n1-1\tdetail\n", blob, id_line_of(&path)));
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// Two identifiers that differ only in letter case are one identifier: the
// second file is refused on the first one's.
#[test]
fn identifiers_differing_only_in_case_are_one_identifier() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let mk = |name: &str, id: &str| {
        write_record(
            &dir,
            &format!("adrdom/blame/{}.md", name),
            &format!("origin: docs/x.md\nblob: {}\nid: {}\n\n1-1\tdetail\n", blob, id),
        )
    };
    let u = write_blame_file(&dir, "adrdom/blame/u.md", "docs/x.md", &blob, "1-1\tdetail\n");
    let out = adr_owner(&dir, &["approve", u.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let lower = id_line_of(&u);
    let v = mk("v", &lower.to_uppercase());
    let out = adr_owner(&dir, &["approve", v.to_str().unwrap()], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    let line = "adrdom/blame/v.md: its id line holds the identifier of adrdom/blame/u.md; one identifier names one blame file; agent removes the id line from adrdom/blame/v.md";
    assert_eq!(refusal_lines(&out, "approve: "), vec![line.to_string()], "{}", stderr(&out));
}

// adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
//
// The move names "line 3" for a gap of one line and "lines 3-4" for a longer one.
#[test]
fn a_coverage_gap_move_says_line_for_one_line_and_lines_for_more() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/gap.md", "one\ntwo\nthree\nfour\nfive\nsix\n");
    let path = write_blame_file(&dir, "adrdom/blame/gap.md", "docs/gap.md", &blob, "1-2\tdetail\n4-4\tdetail\n6-6\tdetail\n");
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("agent adds a row for line 3"), "{}", stderr(&out));
    assert!(stderr(&out).contains("agent adds a row for line 5"), "{}", stderr(&out));
    let blob = write_origin(&dir, "docs/gap2.md", "one\ntwo\nthree\nfour\nfive\n");
    let path = write_blame_file(&dir, "adrdom/blame/gap2.md", "docs/gap2.md", &blob, "1-1\tdetail\n5-5\tdetail\n");
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(stderr(&out).contains("agent adds a row for lines 2-4"), "{}", stderr(&out));
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// "an identifier line approve writes": the `blame` row carries the
// identifier of the file's id line in its identifier column, and a second
// approval of a changed file carries the same one.
#[test]
fn the_blame_row_carries_the_identifier_of_the_id_line() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdecision\n2-2\tdetail\n");
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let id = id_line_of(&path);
    let rows_of = || -> Vec<String> {
        std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
            .unwrap()
            .lines()
            .filter(|l| l.contains("\tblame\t"))
            .map(|l| l.split('\t').nth(2).unwrap().to_string())
            .collect()
    };
    assert_eq!(rows_of(), vec![id.clone()]);

    let text = std::fs::read_to_string(&path).unwrap().replace("2-2\tdetail", "2-2\tdecision");
    std::fs::write(&path, text).unwrap();
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(rows_of(), vec![id.clone(), id]);
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// An id line no `blame` row holds was not written by the tool: on a file
// never approved it is a finding check notes, and approve refuses it and
// writes nothing.
#[test]
fn a_hand_written_id_on_a_never_approved_file_is_a_finding_and_approve_refuses() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    let path = write_record(
        &dir,
        "adrdom/blame/x.md",
        &format!("origin: docs/x.md\nblob: {}\nid: 12345678-1234-4234-8234-123456789abc\n\n1-1\tdetail\n", blob),
    );
    let before = std::fs::read_to_string(&path).unwrap();
    let line = "adrdom/blame/x.md: its id line holds an identifier the tool did not write; the tool assigns a blame file's identifier; agent removes the id line";

    let checked = adr(&dir, &["check"], &[], "");
    assert!(checked.status.success(), "a note on a draft: {}", stdout(&checked));
    assert!(stdout(&checked).lines().any(|l| l == line), "{}", stdout(&checked));

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert_eq!(refusal_lines(&out, "approve: "), vec![line.to_string()], "{}", stderr(&out));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    assert_eq!(blame_row_count(&dir), 0);
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// A `blame` row an older build wrote holds `-` for the identifier: the file
// is approved with no identifier assigned yet, so check asks for no id line
// to be written back and the next approve mints one as for a first approval.
#[test]
fn a_blame_row_with_no_identifier_counts_as_approved_with_none_assigned() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdecision\n2-2\tdetail\n");
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let old_id = id_line_of(&path);

    // The ledger as an older build left it: `-` where the identifier stands.
    let ledger = dir.join("adrdom/state/ledger.tsv");
    let text = std::fs::read_to_string(&ledger).unwrap().replace(&old_id, "-");
    std::fs::write(&ledger, text).unwrap();
    // The file as that build left it: no id line, then an edit.
    let approved = std::fs::read_to_string(&path).unwrap();
    let edited = approved
        .replacen(&format!("id: {}\n", old_id), "", 1)
        .replace("2-2\tdetail", "2-2\tdecision");
    std::fs::write(&path, edited).unwrap();

    let checked = adr(&dir, &["check"], &[], "");
    assert!(!stdout(&checked).contains("id line"), "{}", stdout(&checked));

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let new_id = id_line_of(&path);
    assert!(record::is_uuid(&new_id), "{}", new_id);
    let checked = adr(&dir, &["check"], &[], "");
    assert!(!stdout(&checked).contains("blame/x.md"), "{}", stdout(&checked));
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
//
// An approved file whose id line was overwritten with a non-identifier is told
// to write back the id line of its approval, not to remove the line.
#[test]
fn an_approved_file_with_a_faulty_id_line_is_told_to_write_back_its_id() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "one\ntwo\n");
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdecision\n2-2\tdetail\n");
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let approved = std::fs::read_to_string(&path).unwrap();
    let id_line = approved.lines().nth(2).unwrap().to_string();
    std::fs::write(&path, approved.replacen(&id_line, "id: oops", 1)).unwrap();

    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    let expect = format!("agent writes back the id line \"{}\"", id_line);
    assert!(
        refusal_lines(&out, "approve: ").iter().any(|l| l.contains("which is no identifier") && l.ends_with(&expect)),
        "{}",
        stderr(&out)
    );
}

// adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
//
// A row partly beyond the origin names the beyond finding only; the lines it
// reaches inside the origin count as covered.
#[test]
fn a_row_partly_beyond_the_origin_leaves_no_coverage_gap() {
    let dir = setup();
    let blob = write_origin(&dir, "docs/x.md", "1\n2\n3\n4\n5\n");
    let path = write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-10\tdecision\n");
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    let lines = refusal_lines(&out, "approve: ");
    assert!(lines.iter().any(|l| l.contains("beyond")), "{}", stderr(&out));
    assert!(!lines.iter().any(|l| l.contains("gap") || l.contains("not covered") || l.contains("uncovered")), "{}", stderr(&out));
    assert_eq!(lines.len(), 1, "{}", stderr(&out));
}
