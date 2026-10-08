// What `adr show <file>` prints of a file beyond its latest approval, at the
// edges: a part whose approved text no commit holds, a question put back
// after a plan change struck it, blank lines and entries reordered, a
// removed topic file, and identifiers in prose (per "Does show print what a
// file holds beyond its latest approval?", bceceda8 A, and "What does show
// print with a file named?", 6addc2f4 A).

mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/gate.md";
const ONE: &str = "Is the widget gate open?";
const TWO: &str = "Is the widget gate close?";

fn commit_all(dir: &Path) {
    run_git(dir, &["add", "-A"]);
    assert!(run_git(dir, &["commit", "-q", "-m", "step"]).status.success());
}

fn show(dir: &Path, args: &[&str]) -> String {
    let mut all = vec!["show"];
    all.extend_from_slice(args);
    let out = adr(dir, &all, &[], "");
    stdout(&out) + &stderr(&out)
}

fn id_of(path: &Path, heading: &str) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    let at = text.find(&format!("### {}\n", heading)).unwrap();
    extract_ids(&text[at..])[0].clone()
}

/// The gate with its two questions, approved and committed.
fn approved() -> (PathBuf, PathBuf) {
    let dir = widget_repo();
    let path = write_record(&dir, REL, &format!("# Gate\n\nThe gate.\n\nIt opens.\n\n{}\n{}", gate_question(ONE, "the widget breaks."), gate_question(TWO, "the widget breaks.")));
    approve_record(&dir, REL);
    commit_all(&dir);
    (dir, path)
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// An edit of blank lines alone in the prose changes what approve signs, so
// show marks the line it added: never a changed part with nothing marked.
#[test]
fn a_blank_line_edit_of_the_prose_is_marked() {
    let (dir, path) = approved();
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("The gate.\n\nIt opens.", "The gate.\n\n\nIt opens.", 1)).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains("[changed] prose\n"), "{}", shown);
    assert!(shown.contains("\n  + \n"), "{}", shown);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// The approved text no commit holds (approved, not committed, then edited):
// the change says so above its lines rather than pass them off as new.
#[test]
fn a_change_whose_approved_text_no_commit_holds_says_so() {
    let dir = widget_repo();
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}", gate_question(ONE, "the widget breaks.")));
    approve_record(&dir, REL);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("the widget breaks.", "the widget breaks in use.", 1)).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains("the approved text is in no commit: every line shows as added"), "{}", shown);
    assert!(shown.contains("+ Problem: the widget breaks in use."), "{}", shown);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// A question a plan change struck and the owner approved, then put back:
// beyond the latest approved plan, it reads as new.
#[test]
fn a_question_put_back_after_a_plan_change_struck_it_reads_as_new() {
    let (dir, path) = approved();
    let full = std::fs::read_to_string(&path).unwrap();
    let at = full.find(&format!("### {}\n", TWO)).unwrap();
    // Struck, and no line of the question left names it.
    let kept: String = full[..at].lines().filter(|l| !l.starts_with("unrelated:")).map(|l| format!("{}\n", l)).collect();
    std::fs::write(&path, kept).unwrap();
    approve_record(&dir, REL);
    commit_all(&dir);
    std::fs::write(&path, &full).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains(&format!("+ [not asked] {}\n", TWO)), "{}", shown);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// A struck question whose approved plan no commit holds is named by its
// heading (the one its asked row carries), not by its identifier.
#[test]
fn a_question_struck_from_an_uncommitted_asked_plan_is_named_by_its_heading() {
    let dir = widget_repo();
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}\n{}", gate_question(ONE, "the widget breaks."), gate_question(TWO, "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    let full = std::fs::read_to_string(&path).unwrap();
    let at = full.find(&format!("### {}\n", TWO)).unwrap();
    std::fs::write(&path, &full[..at]).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains(&format!("- [removed] {}\n", TWO)), "{}", shown);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// When the ledger alone cannot tell which questions approved for the file
// the latest plan held, every one gone is named, and show says why: a
// struck question is never silent.
#[test]
fn every_question_gone_is_named_when_the_ledger_cannot_tell_which_were_struck() {
    let dir = widget_repo();
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}", gate_question(ONE, "the widget breaks.")));
    approve_record(&dir, REL);
    // Thirteen questions approved for the file once and gone from it, and a
    // file hash no set of them completes.
    let gone: Vec<String> = (0..13).map(|i| format!("aaaaaaaa-0000-4000-8000-{:012}", i)).collect();
    for id in &gone {
        append_ledger_row_at(&dir, "approved", id, &format!("{:064}", 1), REL, "2999-01-01T00:00:00Z");
    }
    append_ledger_row_at(&dir, "approved", "-", &format!("{:064}", 2), REL, "2999-01-01T00:00:01Z");
    let shown = show(&dir, &[REL]);
    assert!(shown.contains("the approved plan is in no commit: every question approved for this file and gone from it is named"), "{}", shown);
    for id in &gone {
        assert!(shown.contains(&format!("- [removed] {}\n", id)), "{}: {}", id, shown);
    }
    let _ = path;
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// The terms file reordered: the file differs from its approval though no
// entry does, and show prints the whole file's change marked.
#[test]
fn a_terms_file_reordered_prints_its_change() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget", "gadget"]);
    commit_all(&dir);
    let text = std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap();
    let (head, rest) = text.split_once("\n## widget").unwrap();
    let (widget, gadget) = rest.split_once("\n## gadget").unwrap();
    std::fs::write(dir.join("adrdom/terms.md"), format!("{}\n## gadget{}\n## widget{}", head, gadget.trim_end(), widget)).unwrap();
    let shown = show(&dir, &["adrdom/terms.md"]);
    assert!(shown.starts_with("[changed] adrdom/terms.md\n"), "{}", shown);
    assert!(shown.lines().any(|l| l.starts_with("  - ")) && shown.lines().any(|l| l.starts_with("  + ")), "{}", shown);
}

//
// A line the change added that reads like a heading is printed with it;
// only the question's own heading line is left to its status line.
#[test]
fn an_added_line_that_reads_like_a_heading_is_printed() {
    let (dir, path) = approved();
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("It opens.", "It opens.\n\n## Notes\n\nSome more words here.", 1)).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains("  + ## Notes\n"), "{}", shown);
}

//
// An identifier in prose stays as the file holds it, the text approve
// signs; only a reference line turns its identifiers back into words.
#[test]
fn an_identifier_in_prose_is_printed_as_the_file_holds_it() {
    let (dir, path) = approved();
    let one = id_of(&path, ONE);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("It opens.", &format!("It opens, as {} says.", one), 1)).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains(&format!("+ It opens, as {} says.", one)), "{}", shown);
}

// adr: c84a4bd9-fc2d-44f0-b333-172978f61544
//
// A topic file gone from disk, its removal not approved: its current
// decisions are listed under it, as every file's are; its file form prints
// each question's lines and the letter of an answer.
#[test]
fn a_removed_topic_file_lists_its_decisions_and_their_letters() {
    let (dir, path) = approved();
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    assert!(adr_answer(&dir, &id_of(&path, ONE), "A").status.success());
    commit_all(&dir);
    std::fs::remove_file(&path).unwrap();
    let listing = show(&dir, &[]);
    assert!(listing.contains(&format!("[removed] {}\n  {}\n    Yes.\n", REL, ONE)), "{}", listing);
    let form = show(&dir, &[REL]);
    assert!(form.contains(&format!("[answered A] {}\n  depends: widget\n", ONE)), "{}", form);
    assert!(form.contains(&format!("[asked @")), "{}", form);
    assert!(form.contains("  Problem: the widget breaks.\n"), "{}", form);
}

//
// A terms file that exists and cannot be read is named in the listing and
// in its file form, never skipped; an unreadable topic file's file form
// prints the finding show and check print for it.
#[test]
fn an_unreadable_terms_or_topic_file_is_named() {
    let dir = widget_repo();
    std::fs::write(dir.join("adrdom/terms.md"), [0x23, 0x20, 0xff, 0xfe, 0x0a]).unwrap();
    let listing = show(&dir, &[]);
    assert!(listing.contains("terms: adrdom/terms.md: cannot be read: "), "{}", listing);
    let form = show(&dir, &["adrdom/terms.md"]);
    assert!(form.contains("adrdom/terms.md: cannot be read: "), "{}", form);
    assert!(!form.contains("no such file"), "{}", form);

    std::fs::write(dir.join(REL), [0x23, 0x20, 0xff, 0x0a]).unwrap();
    let form = show(&dir, &[REL]);
    assert!(form.contains(&format!("{}: cannot be read: ", REL)), "{}", form);
    assert!(form.contains("agent fixes the file so it can be read as text"), "{}", form);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// A blame file's change names, after each row that names a question, that
// question's status: the owner reading it before approve sees which rows
// name an open question.
#[test]
fn a_blame_draft_names_the_status_of_each_question_a_row_names() {
    let (dir, _, _) = blame_setup();
    let blob = write_origin(&dir, "docs/old.md", "one\ntwo\n");
    write_blame_file(&dir, "adrdom/blame/old.md", "docs/old.md", &blob, "1-1\tdecision: Is this also pass?\n2-2\tdropped: dead\n");
    let shown = show(&dir, &["adrdom/blame/old.md"]);
    assert!(shown.contains("+ 1-1\tdecision: Is this also pass? ["), "{}", shown);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// A repository whose objects are named by sha256 (64 hex characters): the
// approved text is found in its history all the same, and an edited
// question prints its real change, the approved line gone and the new one
// added.
#[test]
fn an_approved_text_is_found_in_a_sha256_repository() {
    let dir = init_repo();
    // The same repository, made again with sha256 object names.
    std::fs::remove_dir_all(dir.join(".git")).unwrap();
    assert!(run_git(&dir, &["init", "-q", "--object-format=sha256"]).status.success());
    run_git(&dir, &["config", "user.email", "test@example.com"]);
    run_git(&dir, &["config", "user.name", "Test User"]);
    run_git(&dir, &["config", "commit.gpgsign", "false"]);
    commit_all(&dir);
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget"]);
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}", gate_question(ONE, "the widget breaks.")));
    approve_record(&dir, REL);
    commit_all(&dir);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("the widget breaks.", "the widget breaks in use.", 1)).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(!shown.contains("in no commit"), "{}", shown);
    assert!(shown.contains("- Problem: the widget breaks.\n"), "{}", shown);
    assert!(shown.contains("+ Problem: the widget breaks in use.\n"), "{}", shown);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// approve signs a question's `id:` line with the rest of it, so the change
// show prints of a question holds that line too: an identifier written by
// hand under an approved question is shown before approve signs it.
#[test]
fn the_change_of_a_question_shows_its_id_line() {
    let (dir, path) = approved();
    let old = id_of(&path, ONE);
    let new = "0e0e0e0e-1111-4222-8333-444455556666";
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen(&format!("id: {}", old), &format!("id: {}", new), 1)).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains(&format!("- id: {}\n", old)), "{}", shown);
    assert!(shown.contains(&format!("+ id: {}\n", new)), "{}", shown);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// A blank line added at the end of the terms file or of a blame file
// changes what approve signs: show marks the blank line it added, so a
// changed file never prints with nothing marked.
#[test]
fn a_blank_line_added_at_the_end_of_a_file_is_marked() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget"]);
    commit_all(&dir);
    let terms = dir.join("adrdom/terms.md");
    let text = std::fs::read_to_string(&terms).unwrap();
    std::fs::write(&terms, format!("{}\n", text)).unwrap();
    let shown = show(&dir, &["adrdom/terms.md"]);
    assert!(shown.starts_with("[changed] adrdom/terms.md\n"), "{}", shown);
    assert!(shown.lines().any(|l| l.trim() == "+" || l.trim() == "-"), "{}", shown);

    let (dir, _, _) = blame_setup();
    let blob = write_origin(&dir, "docs/old.md", "one\n");
    let blame = write_blame_file(&dir, "adrdom/blame/old.md", "docs/old.md", &blob, "1-1\tdropped: dead\n");
    assert!(adr(&dir, &["approve", blame.to_str().unwrap()], &[("ADR_OWNER_BYPASS", "1")], "").status.success());
    commit_all(&dir);
    let text = std::fs::read_to_string(&blame).unwrap();
    std::fs::write(&blame, format!("{}\n", text)).unwrap();
    let shown = show(&dir, &["adrdom/blame/old.md"]);
    assert!(shown.contains("[changed]"), "{}", shown);
    assert!(shown.lines().any(|l| l.trim() == "+" || l.trim() == "-"), "{}", shown);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// A question struck by a plan change the owner approved and nobody has
// committed yet, then put back: no commit holds the approved plan, so the
// ledger tells which questions it held, and the one put back reads as new
// beyond it.
#[test]
fn a_question_put_back_after_an_uncommitted_strike_reads_as_new() {
    let (dir, path) = approved();
    let full = std::fs::read_to_string(&path).unwrap();
    let at = full.find(&format!("### {}\n", TWO)).unwrap();
    let kept: String = full[..at].lines().filter(|l| !l.starts_with("unrelated:")).map(|l| format!("{}\n", l)).collect();
    std::fs::write(&path, kept).unwrap();
    approve_record(&dir, REL);
    std::fs::write(&path, &full).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains(&format!("+ [not asked] {}\n", TWO)), "{}", shown);
}
