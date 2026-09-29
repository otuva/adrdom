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
