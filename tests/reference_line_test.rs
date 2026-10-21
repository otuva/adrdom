// A reference line (`depends:`, `supersedes:`) holds its list on one line:
// a list continued on a second line is refused by approve and
// reported by check.
mod common;

use common::*;

const CLEAN: &str = "# Example\n\n### 1.1 What is a decision?\ndepends: decision\n\nProblem: p.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n";

/// Approves CLEAN, then rewrites the approved file's depends line (approve
/// has turned its entries into ids) with `edit`, which gets that line and
/// returns its replacement text; returns (repo, record path).
fn approved_then_edited(edit: impl Fn(&str) -> String) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", CLEAN);
    approve_record(&dir, "adrdom/decisions/example.md");
    let approved = std::fs::read_to_string(&record).unwrap();
    let dep = approved
        .lines()
        .find(|l| l.starts_with("depends: "))
        .unwrap()
        .to_string();
    let edited = approved.replacen(&format!("{}\n", dep), &format!("{}\n", edit(&dep)), 1);
    assert_ne!(
        edited, approved,
        "the line to edit must exist: {}",
        approved
    );
    std::fs::write(&record, edited).unwrap();
    (dir, record)
}

fn refused_everywhere(dir: &std::path::Path, record: &std::path::Path, needle: &str) {
    let apr = adr_owner(dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !apr.status.success(),
        "approve should refuse: {}",
        stdout(&apr)
    );
    assert!(
        stderr(&apr).contains(needle),
        "approve output missing {:?}: {}",
        needle,
        stderr(&apr)
    );

    // The edited file awaits approval, so check prints the finding as a note.
    let chk = adr(dir, &["check"], &[], "");
    let text = format!("{}{}", stdout(&chk), stderr(&chk));
    assert!(
        text.contains(needle),
        "check output missing {:?}: {}",
        needle,
        text
    );
}

#[test]
fn a_depends_list_continued_on_a_second_line_is_refused() {
    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    let (dir, record) = approved_then_edited(|d| format!("{d}\n{d}"));
    refused_everywhere(&dir, &record, "depends second line");
}

#[test]
fn a_depends_list_continued_on_a_bare_following_line_is_refused() {
    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    let (dir, record) =
        approved_then_edited(|d| format!("{d},\n  {}", d.trim_start_matches("depends: ")));
    refused_everywhere(&dir, &record, "stray line");
}

#[test]
fn a_supersedes_list_continued_on_a_second_line_is_refused() {
    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    let (dir, record) = approved_then_edited(|d| format!("{d}\nsupersedes: aaa\nsupersedes: bbb"));
    refused_everywhere(&dir, &record, "supersedes second line");
}

#[test]
fn a_supersedes_list_continued_on_a_bare_following_line_is_refused() {
    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    let (dir, record) = approved_then_edited(|d| format!("{d}\nsupersedes: aaa,\n  bbb"));
    refused_everywhere(&dir, &record, "stray line");
}

#[test]
fn a_second_blame_line_is_refused() {
    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    // The second blame line is line 6 of the file: the heading is line 3, the
    // depends line 4, the first blame line 5.
    let (dir, record) = approved_then_edited(|d| format!("{d}\nblame: aaa 1-2\nblame: bbb 1-2"));
    refused_everywhere(
        &dir,
        &record,
        "adrdom/decisions/example.md: \"1.1 What is a decision?\": blame second line, line 6; agent rewrites the blame line",
    );
}
