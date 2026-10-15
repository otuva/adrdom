// What check holds the brief asset to: each sentence line ends in a
// from-comment naming a decision or term of the records.
mod common;

use common::*;
use std::path::Path;

const TERM_ID: &str = "99999999-9999-9999-9999-999999999999";
const Q1: &str = "# Example\n\n### 1.1 What is the unit of authority called?\ndepends: authority\n\nProblem: names leaked into decisions.\n\nStandard: plain words.\n\n- A) The authoritative sentence is a decision.\n  Why: plainest word.\n  Costs: two nouns.\n- B) The whole unit is a block.\n  Why: matches predecessors.\n  Costs: names a region.\n";
const Q2: &str = "\n### 1.2 What is the second unit of authority called?\n\nsupersedes: OLD\ndepends: authority\n\nProblem: naming the second unit.\n\nStandard: plain words.\n\n- A) The second thing is a widget.\n  Why: reasons.\n  Costs: costs.\n- B) The second thing is a gadget.\n  Why: other reasons.\n  Costs: other costs.\n";

/// A repo with the term `authority` and one answered decision; returns its id.
fn repo_with_decision() -> (std::path::PathBuf, String) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", Q1);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    (dir, id)
}

fn put_brief(dir: &Path, text: &str) {
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("assets/brief.md"), text).unwrap();
    assert!(run_git(dir, &["add", "assets/brief.md"]).status.success());
}

// adr: de5ed5b1-3a83-4403-8953-941cb7e94390
#[test]
fn a_fully_marked_brief_passes_and_headings_blanks_fences_and_imports_need_no_comment() {
    let (dir, id) = repo_with_decision();
    let text = format!(
        "# Brief\n\n@terms-brief.md\n\n```\nunmarked inside a fence\n```\n\nA sentence. <!-- from: {id}, {TERM_ID} -->\n"
    );
    put_brief(&dir, &text);
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(!stdout(&out).contains("assets/brief.md"), "{}", stdout(&out));
}

// adr: de5ed5b1-3a83-4403-8953-941cb7e94390
#[test]
fn a_sentence_line_with_no_from_comment_fails_check_at_its_line() {
    let (dir, id) = repo_with_decision();
    put_brief(&dir, &format!("# Brief\n\nMarked. <!-- from: {id} -->\nNot marked.\n"));
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("assets/brief.md:4 is a sentence with no from-comment; agent writes"),
        "{}",
        stdout(&out)
    );
    assert!(!stdout(&out).contains("assets/brief.md:3"), "{}", stdout(&out));
}

// adr: de5ed5b1-3a83-4403-8953-941cb7e94390
#[test]
fn an_unknown_uuid_a_non_uuid_and_a_misplaced_comment_fail_check() {
    let (dir, id) = repo_with_decision();
    let ghost = "12345678-1234-4234-8234-123456789012";
    put_brief(
        &dir,
        &format!(
            "Ok. <!-- from: {id} -->\nGhost. <!-- from: {ghost} -->\nWord. <!-- from: nonsense -->\nEmpty. <!-- from: -->\nMid <!-- from: {id} --> text.\n"
        ),
    );
    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!out.status.success(), "{}", text);
    assert!(text.contains(&format!("assets/brief.md:2 holds \"{ghost}\" in its from-comment, which names no decision or term; agent rewrites")), "{}", text);
    assert!(text.contains("assets/brief.md:3 holds \"nonsense\" in its from-comment, which is no identifier"), "{}", text);
    assert!(text.contains("assets/brief.md:4 holds a from-comment that is empty"), "{}", text);
    assert!(text.contains("assets/brief.md:5 holds a from-comment that does not end the line"), "{}", text);
}

// adr: de5ed5b1-3a83-4403-8953-941cb7e94390
#[test]
fn a_superseded_decision_in_a_from_comment_is_a_note_with_the_successor_move() {
    let (dir, old_id) = repo_with_decision();
    let record = dir.join("adrdom/decisions/example.md");
    let mut content = std::fs::read_to_string(&record).unwrap();
    content.push_str(&Q2.replace("OLD", &old_id));
    std::fs::write(&record, &content).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let new_id = extract_ids(&std::fs::read_to_string(&record).unwrap())
        .into_iter()
        .find(|i| i != &old_id)
        .unwrap();
    assert!(adr_answer(&dir, &new_id, "A").status.success());

    put_brief(&dir, &format!("Old. <!-- from: {old_id} -->\n"));
    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(out.status.success(), "{}", text);
    assert!(text.contains(&format!("assets/brief.md:1 names {old_id}")), "{}", text);
    assert!(text.contains("not a current decision; agent rewrites the from-comment to"), "{}", text);
    assert!(text.contains(&new_id), "{}", text);
}

// adr: de5ed5b1-3a83-4403-8953-941cb7e94390
#[test]
fn no_brief_asset_or_one_with_no_from_comment_is_left_alone() {
    let (dir, _) = repo_with_decision();
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(!stdout(&out).contains("assets/brief.md"), "{}", stdout(&out));

    put_brief(&dir, "# Someone else's brief\n\nNo comments here.\n");
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(!stdout(&out).contains("assets/brief.md"), "{}", stdout(&out));
}

// adr: de5ed5b1-3a83-4403-8953-941cb7e94390
#[test]
fn every_sentence_line_of_this_repositorys_brief_names_a_decision_or_term() {
    let mut fenced = false;
    for (i, line) in adr::commands::install::BRIEF.lines().enumerate() {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced || line.trim().is_empty() || line.starts_with('#') || line.starts_with('@') {
            continue;
        }
        let at = line.find(" <!-- from: ").unwrap_or_else(|| panic!("line {} has no from-comment: {}", i + 1, line));
        assert!(line.ends_with(" -->"), "line {}: {}", i + 1, line);
        let ids = &line[at + " <!-- from: ".len()..line.len() - " -->".len()];
        assert!(ids.split(", ").all(|id| id.len() == 36), "line {}: {}", i + 1, line);
    }
    assert!(!adr::commands::install::brief().contains("<!--"));
}
