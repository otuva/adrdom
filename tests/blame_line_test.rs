// The `blame:` line under a question's heading: the approved blame file and
// the row ranges the question was born from. The agent writes the name of the
// blame file; approve turns it into the blame file's identifier; show turns
// it back. The line is judged by the one judgement every command reads, for a
// question whose text can still change.
mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/gate.md";
const HEADING: &str = "Does the widget gate open?";

/// The origin of the blame file `x`: three lines.
const ORIGIN: &str = "one\ntwo\nthree\n";
/// Its rows: lines 1-2 a decision, line 3 a detail.
const ROWS: &str = "1-2\tdecision\n3-3\tdetail\n";

/// A repository with the term `widget` approved and the approved blame file
/// `x` (`ROWS` over `ORIGIN`); returns (repository, path of the blame file).
fn gate_repo() -> (PathBuf, PathBuf) {
    let dir = widget_repo();
    let x = approved_blame_file(&dir, "x", "docs/x.md", ORIGIN, ROWS);
    (dir, x)
}

/// The topic file of the one question `HEADING`, with `lines` (each ending in
/// a newline) directly under its depends line.
fn topic(lines: &str) -> String {
    gate_topic(HEADING, lines)
}

/// A finding of the topic file: its path, the question's heading, then `rest`.
fn found(rest: &str) -> String {
    format!("{}: \"{}\": {}", REL, HEADING, rest)
}

fn ledger_text(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default()
}

/// Edits a file in place: `from` becomes `to`. An approved blame file edited
/// this way keeps its `id:` line.
fn edit_file(path: &Path, from: &str, to: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(from), "{:?} is not in:\n{}", from, text);
    std::fs::write(path, text.replacen(from, to, 1)).unwrap();
}

/// The one judgement of a topic file awaiting approval: approve refuses with
/// every line of `lines` and writes nothing, and check prints the same lines
/// as notes and exits 0.
fn assert_refused_and_noted(dir: &Path, rec: &Path, lines: &[String]) {
    plan_record(dir, REL);
    let topic_before = std::fs::read_to_string(rec).unwrap();
    let ledger_before = ledger_text(dir);

    let out = approve_unplanned(dir, rec);
    assert!(!out.status.success(), "approve should refuse: {}", stdout(&out));
    let refused = refusal_lines(&out, "approve: ");
    for line in lines {
        assert!(refused.contains(line), "approve lacks {:?}:\n{}", line, stderr(&out));
    }
    assert_eq!(std::fs::read_to_string(rec).unwrap(), topic_before, "nothing written on refusal");
    assert_eq!(ledger_text(dir), ledger_before, "no ledger row on refusal");

    // The topic file is a draft: check prints the same lines and passes.
    let checked = adr(dir, &["check"], &[], "");
    assert!(checked.status.success(), "a draft's finding is a note: {}", stdout(&checked));
    for line in lines {
        assert!(stdout(&checked).lines().any(|l| l == line), "check lacks {:?}:\n{}", line, stdout(&checked));
    }
}

// --- approve turns the name into the identifier ----------------------------

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
// adr: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
//
// "approve turns the name into an identifier" (672201dd A) and "the agent
// writes the name, approve turns it into the identifier, and show and the page
// turn it back into the name" (7cad9922 A): the line the agent wrote with the
// name of the blame file holds its identifier after approve, directly above
// the question's `id:` line, and show prints the name again.
#[test]
fn approve_turns_the_name_on_a_blame_line_into_the_identifier() {
    let (dir, x) = gate_repo();
    let id = id_line_of(&x);
    let rec = write_record(&dir, REL, &topic("blame: x 1-2\n"));
    approve_record(&dir, REL);

    let saved = std::fs::read_to_string(&rec).unwrap();
    let lines: Vec<&str> = saved.lines().collect();
    let at = lines
        .iter()
        .position(|l| *l == format!("blame: {} 1-2", id))
        .unwrap_or_else(|| panic!("no blame line holds the identifier {}:\n{}", id, saved));
    assert!(!saved.contains("blame: x"), "the name is gone:\n{}", saved);
    assert!(lines[at + 1].starts_with("id: "), "the id line comes directly under the blame line:\n{}", saved);

    let shown = stdout(&adr(&dir, &["show", REL], &[], ""));
    assert!(shown.lines().any(|l| l == "  blame: x 1-2"), "{}", shown);
    assert!(!shown.contains(&id), "show prints the name, not the identifier:\n{}", shown);

    // The line in its identifier form names an approved blame file and the
    // range of a row: check has nothing to say of it.
    let checked = adr(&dir, &["check"], &[], "");
    assert!(checked.status.success(), "{}", stdout(&checked));
    assert!(!stdout(&checked).contains("blame line"), "{}", stdout(&checked));
    assert!(!stdout(&checked).contains("blame entry"), "{}", stdout(&checked));
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// "A blame line naming the approved blame file and the row ranges the
// question was born from": a row of either value may be named, a `detail`
// row as well as a `decision` row.
#[test]
fn a_blame_line_may_name_a_detail_row() {
    let (dir, x) = gate_repo();
    let id = id_line_of(&x);
    let rec = write_record(&dir, REL, &topic("blame: x 3-3\n"));
    approve_record(&dir, REL);

    let saved = std::fs::read_to_string(&rec).unwrap();
    assert!(saved.lines().any(|l| l == format!("blame: {} 3-3", id)), "{}", saved);
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// "naming the approved blame file and the row ranges": one line holds the
// blame files a question came from, each with its ranges, in any directory
// under adrdom/blame. Approve turns both names into identifiers and show
// turns them back.
#[test]
fn a_blame_line_names_rows_of_two_blame_files() {
    let (dir, x) = gate_repo();
    let other = approved_blame_file(&dir, "sub/other", "docs/other.md", "a\nb\n", "1-1\tdecision\n2-2\tdetail\n");
    let (id_x, id_other) = (id_line_of(&x), id_line_of(&other));
    assert_ne!(id_x, id_other);
    let rec = write_record(&dir, REL, &topic("blame: x 1-2 3-3, sub/other 1-1\n"));
    approve_record(&dir, REL);

    let saved = std::fs::read_to_string(&rec).unwrap();
    let want = format!("blame: {} 1-2 3-3, {} 1-1", id_x, id_other);
    assert!(saved.lines().any(|l| l == want), "no line {:?}:\n{}", want, saved);

    let shown = stdout(&adr(&dir, &["show", REL], &[], ""));
    assert!(shown.lines().any(|l| l == "  blame: x 1-2 3-3, sub/other 1-1"), "{}", shown);
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// A line the agent wrote with the identifier already names the blame file all
// the same: "the name of a blame file ... or, once approve has turned it, its
// identifier". Approve takes it and leaves it as written.
#[test]
fn a_blame_line_in_identifier_form_approves_as_written() {
    let (dir, x) = gate_repo();
    let id = id_line_of(&x);
    let rec = write_record(&dir, REL, &topic(&format!("blame: {} 1-2\n", id)));
    approve_record(&dir, REL);

    let saved = std::fs::read_to_string(&rec).unwrap();
    assert!(saved.lines().any(|l| l == format!("blame: {} 1-2", id)), "{}", saved);
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// A repeated entry or range on the line is tolerated: no rule refuses it,
// and the line still names the blame file by its identifier.
#[test]
fn a_repeated_entry_or_range_on_a_blame_line_is_tolerated() {
    let (dir, x) = gate_repo();
    let id = id_line_of(&x);
    let rec = write_record(&dir, REL, &topic("blame: x 1-2 1-2, x 1-2\n"));
    approve_record(&dir, REL);

    let saved = std::fs::read_to_string(&rec).unwrap();
    assert!(saved.contains(&format!("blame: {} 1-2", id)), "{}", saved);
    assert!(!saved.contains("blame: x"), "{}", saved);
}

// --- what approve refuses ---------------------------------------------------

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// "A blame line naming the approved blame file": a name or an identifier that
// is no blame file refuses on the line, and check prints the same line as a
// note.
#[test]
fn approve_refuses_a_blame_line_naming_no_blame_file() {
    let (dir, _x) = gate_repo();
    for entry in ["nosuch", "0e0e0e0e-1111-4222-8333-444455556666"] {
        let rec = write_record(&dir, REL, &topic(&format!("blame: {} 1-2\n", entry)));
        let line = found(&format!(
            "the blame entry \"{}\" names no blame file; a blame line names an approved blame file; agent writes the path of a blame file under adrdom/blame without .md, or removes the entry",
            entry
        ));
        assert_refused_and_noted(&dir, &rec, &[line]);
    }
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// "A blame line naming the approved blame file": a blame file never approved
// is new, and one approved and then edited is changed; neither is the file the
// line names yet, and the finding says which it is and that the line waits for
// the approval.
#[test]
fn approve_refuses_a_blame_line_naming_a_blame_file_awaiting_approval() {
    // Never approved: new.
    let (dir, _x) = gate_repo();
    blame_file(&dir, "y", "docs/y.md", "one\ntwo\n", "1-1\tdecision\n2-2\tdetail\n");
    let rec = write_record(&dir, REL, &topic("blame: y 1-1\n"));
    let line = found(
        "the blame line names adrdom/blame/y.md, which is new; a blame line names an approved blame file; not yet, after it is approved",
    );
    assert_refused_and_noted(&dir, &rec, &[line]);

    // Approved, then edited in place: changed.
    let (dir, x) = gate_repo();
    edit_file(&x, "3-3\tdetail", "3-3\tdecision");
    let rec = write_record(&dir, REL, &topic("blame: x 1-2\n"));
    let line = found(
        "the blame line names adrdom/blame/x.md, which is changed; a blame line names an approved blame file; not yet, after it is approved",
    );
    assert_refused_and_noted(&dir, &rec, &[line]);
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// "the row ranges the question was born from": a range that is the range of
// no row of the blame file refuses on that range alone; a range of a row of
// the file passes beside it.
#[test]
fn approve_refuses_a_blame_line_naming_a_range_of_no_row() {
    let (dir, _x) = gate_repo();
    let missing = |range: &str| {
        found(&format!(
            "the blame line names {}, the range of no row of adrdom/blame/x.md; a blame line names row ranges of its blame file; agent writes the range of a row of that blame file, or removes the range",
            range
        ))
    };

    // Lines 1-3 are covered by rows, but by two rows and no one row.
    let rec = write_record(&dir, REL, &topic("blame: x 1-3\n"));
    assert_refused_and_noted(&dir, &rec, &[missing("1-3")]);

    // One range of a row and one of none: the finding names the second.
    let rec = write_record(&dir, REL, &topic("blame: x 1-2 2-3\n"));
    assert_refused_and_noted(&dir, &rec, &[missing("2-3")]);
    let out = approve_unplanned(&dir, &rec);
    assert!(!stderr(&out).contains("names 1-2,"), "{}", stderr(&out));
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// "naming the approved blame file and the row ranges": an entry that names a
// blame file and no range refuses.
#[test]
fn approve_refuses_a_blame_entry_with_no_range() {
    let (dir, _x) = gate_repo();
    let rec = write_record(&dir, REL, &topic("blame: x\n"));
    let line = found(
        "the blame entry \"x\" names no row range; a blame line names row ranges of its blame file; agent writes the name of a blame file and the ranges of its rows",
    );
    assert_refused_and_noted(&dir, &rec, &[line]);
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// The first token of an entry is always its name, so a lone range-shaped
// token is a name with no range and refuses as such.
#[test]
fn approve_refuses_a_lone_range_shaped_token_as_a_name_with_no_range() {
    let (dir, _x) = gate_repo();
    let rec = write_record(&dir, REL, &topic("blame: 1-2\n"));
    let line = found(
        "the blame entry \"1-2\" names no row range; a blame line names row ranges of its blame file; agent writes the name of a blame file and the ranges of its rows",
    );
    assert_refused_and_noted(&dir, &rec, &[line]);
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// A blame file may be called like a range: the first token of the entry is
// its name, the rest its ranges, and approve turns the name into the
// identifier.
#[test]
fn a_blame_file_named_like_a_range_is_named_by_the_first_token() {
    let dir = widget_repo();
    let r = approved_blame_file(&dir, "10-14", "docs/r.md", "a\nb\n", "1-2\tdecision\n");
    let id = id_line_of(&r);
    let rec = write_record(&dir, REL, &topic("blame: 10-14 1-2\n"));
    approve_record(&dir, REL);
    let saved = std::fs::read_to_string(&rec).unwrap();
    assert!(saved.lines().any(|l| l == format!("blame: {} 1-2", id)), "{}", saved);
}

// --- nothing forces the link ------------------------------------------------

// adr: ee27fd4c-89b7-411f-a3e4-2535c49b6d1b
//
// "no finding, no refusal": a question that carries no blame line is
// complete. It approves with no blame line written for it, and ask and check
// have nothing to say of the decision rows of the blame file no question
// names.
#[test]
fn a_question_with_no_blame_line_approves() {
    let (dir, _x) = gate_repo();
    let rec = write_record(&dir, REL, &topic(""));
    approve_record(&dir, REL);

    let saved = std::fs::read_to_string(&rec).unwrap();
    assert!(!saved.contains("blame:"), "{}", saved);

    let out = adr(&dir, &["ask", rec.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask should not stop for a missing blame line: {}", stderr(&out));
    let checked = adr(&dir, &["check"], &[], "");
    assert!(checked.status.success(), "{}", stdout(&checked));
    assert!(!stdout(&checked).contains("blame"), "{}", stdout(&checked));
}

// --- what the line is part of -----------------------------------------------

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// "a line under the heading": a heading with a depends line and a blame line
// and nothing else has no body yet; the blame line is no stray line of a body.
#[test]
fn a_heading_with_a_depends_and_a_blame_line_has_no_body() {
    let (dir, _x) = gate_repo();
    write_record(&dir, REL, &format!("# Gate\n\n### {}\ndepends: widget\nblame: x 1-2\n", HEADING));

    let shown = stdout(&adr(&dir, &["show", REL], &[], ""));
    assert!(shown.contains(&format!("[no body] {}\n", HEADING)), "{}", shown);
    assert!(shown.lines().any(|l| l == "  + blame: x 1-2"), "{}", shown);
    assert!(!shown.contains("stray line"), "{}", shown);
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// "A blame line naming the approved blame file and the row ranges the
// question was born from": the line is part of the question's approved text,
// as the depends line is. Added to an approved question that is not asked, it
// is a change beyond the approval, and show marks that line and no other.
#[test]
fn a_blame_line_is_part_of_the_approved_question() {
    let (dir, _x) = gate_repo();
    let rec = write_record(&dir, REL, &topic(""));
    approve_record(&dir, REL);
    run_git(&dir, &["add", "-A"]);
    assert!(run_git(&dir, &["commit", "-q", "-m", "approve the gate"]).status.success());

    let text = std::fs::read_to_string(&rec).unwrap();
    let depends = text.lines().find(|l| l.starts_with("depends: ")).unwrap().to_string();
    std::fs::write(&rec, text.replacen(&format!("{}\n", depends), &format!("{}\nblame: x 1-2\n", depends), 1)).unwrap();

    let shown = stdout(&adr(&dir, &["show", REL], &[], ""));
    assert!(shown.starts_with("[changed] Gate (adrdom/decisions/gate.md)\n"), "{}", shown);
    assert!(shown.lines().any(|l| l == "  + blame: x 1-2"), "{}", shown);
    // A marked line opens with its mark; an unmarked line of the question
    // (the option lines among them) is indented past it.
    let marked: Vec<&str> = shown
        .lines()
        .filter(|l| ["+ ", "- ", "  + ", "  - "].iter().any(|p| l.starts_with(p)))
        .collect();
    assert_eq!(marked, vec!["  + blame: x 1-2"], "{}", shown);
}

// --- a question whose text cannot change is not judged ------------------------

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
//
// "the approved blame file never changes": a question asked and standing as
// asked has text that cannot change, so its blame line is not judged again.
// The blame file edited after the ask, so that it is changed and no longer
// holds the range the line names, gives check nothing to print of the topic
// file.
#[test]
fn an_asked_question_is_not_judged_again_when_its_blame_file_changes() {
    let (dir, x) = gate_repo();
    let rec = write_record(&dir, REL, &topic("blame: x 1-2\n"));
    approve_record(&dir, REL);
    let asked = adr(&dir, &["ask", rec.to_str().unwrap()], &[], "");
    assert!(asked.status.success(), "{}", stderr(&asked));

    edit_file(&x, "1-2\tdecision\n3-3\tdetail\n", "1-1\tdecision\n2-3\tdetail\n");

    let checked = adr(&dir, &["check"], &[], "");
    assert!(checked.status.success(), "{}", stdout(&checked));
    assert!(!stdout(&checked).contains(REL), "{}", stdout(&checked));
    assert!(!stdout(&checked).contains("blame line"), "{}", stdout(&checked));
}

// adr: 672201dd-8f22-4dc3-8b21-a73eb7606e83
// adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
//
// "what approve refuses, check reports with the same move" (fe14991d A): on
// an approved topic file whose question is not asked, a blame file edited
// since is changed, and the line naming it fails check; ask refuses the
// question on the same line and writes no row.
#[test]
fn ask_refuses_what_check_fails_on_a_blame_line() {
    let (dir, x) = gate_repo();
    let rec = write_record(&dir, REL, &topic("blame: x 1-2\n"));
    approve_record(&dir, REL);
    edit_file(&x, "3-3\tdetail", "3-3\tdecision");
    let line = found(
        "the blame line names adrdom/blame/x.md, which is changed; a blame line names an approved blame file; not yet, after it is approved",
    );

    let checked = adr(&dir, &["check"], &[], "");
    assert!(!checked.status.success(), "check should fail: {}", stdout(&checked));
    assert!(stdout(&checked).lines().any(|l| l == line), "check lacks {:?}:\n{}", line, stdout(&checked));

    let asked = adr(&dir, &["ask", rec.to_str().unwrap()], &[], "");
    assert!(!asked.status.success(), "ask should refuse: {}", stdout(&asked));
    assert!(refusal_lines(&asked, "ask: ").contains(&line), "ask lacks {:?}:\n{}", line, stderr(&asked));
    assert_eq!(asked_row_count(&dir), 0, "no question asked");
}
