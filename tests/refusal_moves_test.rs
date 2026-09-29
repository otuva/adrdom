// Decision b0e865b4-9204-4c55-9f5b-dcb755b3a03b: "Does a refusal name the
// command that clears it?" Answer: "Yes: every refusal names what failed,
// the command that clears it and who runs that command, the agent or the
// owner." Each test here walks one case to the refusal, reads the move it
// names, and makes that move where one move is enough to clear it.

mod common;

use common::*;
use std::path::PathBuf;

fn question(heading: &str, depends: &str, extra: &str) -> String {
    format!(
        "### {}\ndepends: {}\n{}\nProblem: the widget breaks.\n\nStandard: plain words.\n\n\
- A) Yes the widget holds.\n  Why: reasons.\n  Costs: costs.\n\
- B) No the widget does not hold.\n  Why: other reasons.\n  Costs: other costs.\n",
        heading, depends, extra
    )
}

fn setup() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    dir
}

const A: &str = "adrdom/decisions/a.md";
const B: &str = "adrdom/decisions/b.md";

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
// adr: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf
//
// "What does a depends line name?": "a term, a question in the same topic
// file, or a current decision". An entry that names a question of another
// file which is not answered names none of the three. "adds the heading"
// would copy that question into this file.
#[test]
fn an_entry_on_an_unanswered_question_of_another_file_names_what_a_line_may_name() {
    let dir = setup();
    let a = write_record(
        &dir,
        A,
        &format!("# A\n\n{}", question("Does the widget alpha hold?", "widget", "")),
    );
    approve_record(&dir, A);
    assert!(adr(&dir, &["ask", a.to_str().unwrap()], &[], "").status.success());

    let b = write_record(
        &dir,
        B,
        &format!(
            "# B\n\n{}",
            question("Does the widget beta hold?", "widget, Does the widget alpha hold?", "")
        ),
    );
    let refusal = format!(
        "{}: \"Does the widget beta hold?\" depends on \"Does the widget alpha hold?\" which is no heading in this file and no current decision; agent names a heading of this file or a current decision",
        B
    );
    let out = adr(&dir, &["check"], &[], "");
    assert!(stdout(&out).contains(&refusal), "{}", stdout(&out));
    let out = adr_owner(&dir, &["approve", b.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains(&refusal), "{}", stderr(&out));

    // Once alpha is answered it is a current decision: the same line holds.
    let alpha = extract_ids(&std::fs::read_to_string(&a).unwrap())[0].clone();
    assert!(adr_answer(&dir, &alpha, "A").status.success());
    let out = adr_owner(&dir, &["approve", b.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
// adr: 0b962bc8-05a8-48da-9fe8-bf77c7aa6e4a
//
// "What ties a test to a decision?": "found by text search in every tracked
// file outside adrdom". A file whose name holds a letter outside ASCII is a
// tracked file; "untested" named a move the agent had made.
#[test]
fn a_tie_in_a_file_named_outside_ascii_counts() {
    let dir = setup();
    let path = write_record(
        &dir,
        A,
        &format!("# A\n\n{}", question("Does the widget alpha hold?", "widget", "")),
    );
    approve_record(&dir, A);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    let alpha = extract_ids(&std::fs::read_to_string(&path).unwrap())[0].clone();
    assert!(adr_answer(&dir, &alpha, "A").status.success());

    std::fs::write(dir.join("tëst.txt"), format!("adr: {}\n", alpha)).unwrap();
    run_git(&dir, &["add", "-A"]);

    for args in [&["check"][..], &["check", "--staged"][..]] {
        let out = adr(&dir, args, &[], "");
        assert!(out.status.success(), "{:?}: {}", args, stdout(&out));
        assert!(stdout(&out).contains("tested: 1"), "{:?}: {}", args, stdout(&out));
    }
}

// adr: d8a9dfff-de89-442c-a08c-8276efab3f36
//
// The same listing feeds the ban scan: a word in a tracked file named
// outside ASCII is a word in use.
#[test]
fn the_ban_scan_reads_a_file_named_outside_ascii() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::write(dir.join("nötes.txt"), "this line mentions gadgetword right here\n").unwrap();
    run_git(&dir, &["add", "-A"]);

    let out = adr(&dir, &["ban"], &[], "gadgetword\n");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(stderr(&out).contains("nötes.txt:1"), "{}", stderr(&out));
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
// adr: 15d82e53-e344-4347-b639-3bf6eb9ef1b4
//
// Five questions written as if none stood on another. The fifth asks about
// the widget "where the widget one holds": it assumes the answer of the
// first and its depends line leaves that out. The refusal names the five
// and the two moves; the first move, made, clears it.
#[test]
fn the_cap_refusal_names_the_questions_and_the_move() {
    let dir = setup();
    const FIFTH: &str = "Does the widget hold where the widget one holds?";
    let plan = |fifth_depends: &str| {
        let mut content = String::from("# Cap\n\n");
        for n in ["one", "two", "three", "four"] {
            content.push_str(&format!("### Does the widget {} hold?\ndepends: widget\n\n", n));
        }
        content.push_str(&format!("### {}\ndepends: {}\n\n", FIFTH, fifth_depends));
        content
    };
    let rel = "adrdom/decisions/cap.md";
    let path = write_record(&dir, rel, &plan("widget"));

    let refusal = format!(
        "{}: 5 questions would be put to the owner together, the most is 4: \
\"Does the widget one hold?\", \"Does the widget two hold?\", \"Does the widget three hold?\", \
\"Does the widget four hold?\", \"{}\"; \
agent finds a question among these that assumes the answer of another one and names that one on its depends line, \
or splits the file when none does",
        rel, FIFTH
    );
    let out = adr(&dir, &["check"], &[], "");
    assert!(stdout(&out).contains(&refusal), "{}", stdout(&out));
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains(&refusal), "{}", stderr(&out));

    write_record(&dir, rel, &plan("widget, Does the widget one hold?"));
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
// adr: 15d82e53-e344-4347-b639-3bf6eb9ef1b4
//
// Four answered questions hold their place, and their depends lines are
// never edited again. A fifth beside them is over the cap: the refusal
// counts the answered ones and names only the one the agent can move.
#[test]
fn the_cap_refusal_names_only_the_questions_not_answered() {
    let dir = setup();
    let rel = "adrdom/decisions/cap.md";
    let mut content = String::from("# Cap\n");
    for n in ["one", "two", "three", "four"] {
        content.push_str(&format!(
            "\n{}",
            question(&format!("Does the widget {} hold?", n), "widget", "")
        ));
    }
    let path = write_record(&dir, rel, &content);
    approve_record(&dir, rel);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    for id in extract_ids(&std::fs::read_to_string(&path).unwrap()) {
        assert!(adr_answer(&dir, &id, "A").status.success());
    }

    let mut content = std::fs::read_to_string(&path).unwrap();
    content.push_str("\n### Does the widget five hold?\ndepends: widget\n");
    std::fs::write(&path, content).unwrap();

    let refusal = format!(
        "{}: 5 questions stand together, the most is 4: 4 answered, and \"Does the widget five hold?\"; \
agent finds a question among the ones named that assumes the answer of another one and names that one on its depends line, \
or moves it to another file when none does",
        rel
    );
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains(&refusal), "{}", stderr(&out));
}
