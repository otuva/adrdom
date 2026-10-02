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
fn an_entry_on_an_unanswered_question_of_another_file_says_it_waits() {
    let dir = setup();
    let a = write_record(
        &dir,
        A,
        &format!("# A\n\n{}", question("Is the widget alpha hold?", "widget", "")),
    );
    approve_record(&dir, A);
    assert!(adr(&dir, &["ask", a.to_str().unwrap()], &[], "").status.success());

    let b = write_record(
        &dir,
        B,
        &format!(
            "# B\n\n{}",
            question("Is the widget beta hold?", "widget, Is the widget alpha hold?", "")
        ),
    );
    // The tool holds alpha's state in the other file: the line says whose
    // turn it is (nobody's but the owner's, through alpha), not a move the
    // agent cannot make.
    let refusal = format!(
        "{}: \"Is the widget beta hold?\" depends on \"Is the widget alpha hold?\" ({}) which is not answered yet; not yet, after it is answered",
        B, A
    );
    let out = adr(&dir, &["check"], &[], "");
    assert!(stdout(&out).contains(&refusal), "{}", stdout(&out));
    let out = adr_owner(&dir, &["approve", b.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains(&refusal), "{}", stderr(&out));

    // Once alpha is answered it is a current decision: the entry holds.
    let alpha = extract_ids(&std::fs::read_to_string(&a).unwrap())[0].clone();
    assert!(adr_answer(&dir, &alpha, "A").status.success());
    let out = adr_owner(&dir, &["approve", b.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
}

// adr: 4319a653-6901-446c-94ab-6420ef472134
// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
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
        &format!("# A\n\n{}", question("Is the widget alpha hold?", "widget", "")),
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
    const FIFTH: &str = "Is the widget hold where the widget one holds?";
    let plan = |fifth_depends: &str| {
        let mut content = String::from("# Cap\n\n");
        for n in ["one", "two", "three", "four"] {
            content.push_str(&format!("### Is the widget {} hold?\ndepends: widget\n\n", n));
        }
        content.push_str(&format!("### {}\ndepends: {}\n\n", FIFTH, fifth_depends));
        content
    };
    let rel = "adrdom/decisions/cap.md";
    let path = write_record(&dir, rel, &plan("widget"));

    let refusal = format!(
        "{}: 5 questions would be put to the owner together, the most is 4: \
\"Is the widget one hold?\", \"Is the widget two hold?\", \"Is the widget three hold?\", \
\"Is the widget four hold?\", \"{}\"; \
agent finds a question among these that assumes the answer of another one and names that one on its depends line, \
or splits the file when none does",
        rel, FIFTH
    );
    let out = adr(&dir, &["check"], &[], "");
    assert!(stdout(&out).contains(&refusal), "{}", stdout(&out));
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains(&refusal), "{}", stderr(&out));

    write_record(&dir, rel, &plan("widget, Is the widget one hold?"));
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
            question(&format!("Is the widget {} hold?", n), "widget", "")
        ));
    }
    let path = write_record(&dir, rel, &content);
    approve_record(&dir, rel);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    for id in extract_ids(&std::fs::read_to_string(&path).unwrap()) {
        assert!(adr_answer(&dir, &id, "A").status.success());
    }

    let mut content = std::fs::read_to_string(&path).unwrap();
    content.push_str("\n### Is the widget five hold?\ndepends: widget\n");
    std::fs::write(&path, content).unwrap();

    let refusal = format!(
        "{}: 5 questions stand together, the most is 4: 4 answered, and \"Is the widget five hold?\"; \
agent finds a question among the ones named that assumes the answer of another one and names that one on its depends line, \
or moves it to another file when none does",
        rel
    );
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains(&refusal), "{}", stderr(&out));
}

/// Writes `content` to `rel`, approves it, asks it and answers the question
/// at `index` with A. Returns that question's id.
fn answered(dir: &std::path::Path, rel: &str, content: &str, index: usize) -> String {
    let path = write_record(dir, rel, content);
    approve_record(dir, rel);
    let out = adr(dir, &["ask", path.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask {} failed: {}", rel, stderr(&out));
    let id = extract_ids(&std::fs::read_to_string(&path).unwrap())[index].clone();
    let out = adr_answer(dir, &id, "A");
    assert!(out.status.success(), "answer {} failed: {}", rel, stderr(&out));
    id
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// A line that names an identifier names the question beside it, and a
// stale tag is named with the file and line it sits at: the tool holds
// both, the reader does not look them up.
#[test]
fn a_refusal_names_the_question_beside_its_identifier() {
    let dir = setup();
    const ALPHA: &str = "Is the widget alpha hold?";
    const BETA: &str = "Is the widget beta hold?";
    const GAMMA: &str = "Is the widget gamma hold?";
    let alpha = answered(&dir, A, &format!("# A\n\n{}", question(ALPHA, "widget", "")), 0);

    // Answered, no test tied to it.
    let out = adr(&dir, &["check"], &[], "");
    assert!(
        stdout(&out).contains(&format!(
            "untested: {}: \"{}\"; no tracked file holds \"adr: {}\"",
            A, ALPHA, alpha
        )),
        "{}",
        stdout(&out)
    );

    // Beta stands on alpha; then gamma takes alpha's place.
    write_test_comment(&dir, &alpha);
    let beta = answered(
        &dir,
        B,
        &format!("# B\n\n{}", question(BETA, &format!("widget, {}", ALPHA), "")),
        0,
    );
    write_test_comment(&dir, &beta);

    // Under alpha, show names the question of the other file that stands
    // on it: what would lose its ground with alpha.
    let shown = stdout(&adr(&dir, &["show", "--all"], &[], ""));
    assert!(
        shown.contains(&format!(
            "  [answered A] {}\n    stands on: widget\n    stood on by: {} ({})\n",
            ALPHA, BETA, B
        )),
        "{}",
        shown
    );

    // Gamma takes alpha's place; beta stands on alpha, so the line names
    // beta too (per "Does a supersedes line name every current decision
    // standing on its target?").
    let mut a = std::fs::read_to_string(dir.join(A)).unwrap();
    a.push_str(&format!(
        "\n{}",
        question(GAMMA, "widget", &format!("supersedes: {}, {}\n", alpha, BETA))
    ));
    let gamma = answered(&dir, A, &a, 1);
    write_test_comment(&dir, &gamma);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        text.contains(&format!(
            "tested: {} (\"{}\"): tied to a test and not a current decision at tested.txt:1; agent removes",
            alpha, ALPHA
        )),
        "{}",
        text
    );

    // A question that names alpha on its supersedes line after alpha left:
    // approve refuses it and names the question beside the identifier.
    let mut a = std::fs::read_to_string(dir.join(A)).unwrap();
    a.push_str(&format!(
        "\n### Is the widget delta hold?\ndepends: widget\nsupersedes: {}\n",
        alpha
    ));
    std::fs::write(dir.join(A), &a).unwrap();
    let out = adr_owner(&dir, &["approve", dir.join(A).to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains(&format!(
            "approve: {}: \"Is the widget delta hold?\": supersedes {} (\"{}\"), which is not a current decision; agent removes it from the supersedes line or names a current decision",
            A,
            alpha, ALPHA
        )),
        "{}",
        stderr(&out)
    );
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// The reader of `adr show` goes away while the tool still prints (`adr show
// | head`). The tool ends without a word: a crash report there reads as a
// fault of the repository.
#[test]
fn show_ends_without_a_word_when_its_reader_goes_away() {
    use std::process::{Command, Stdio};
    let dir = setup();
    let mut plan = String::from("# A\n");
    for n in ["one", "two", "three", "four"] {
        plan.push_str(&format!("\n### Is the widget {} hold?\ndepends: widget\n", n));
    }
    write_record(&dir, A, &plan);

    let mut child = Command::new(adr_bin())
        .current_dir(&dir)
        .arg("show")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The reading end is closed before the tool has read a file, so its
    // first print meets the closed pipe.
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.is_empty(), "{}", err);
    assert_eq!(out.status.code(), Some(141), "{:?}", out.status);
}
