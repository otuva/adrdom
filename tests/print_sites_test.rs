// Guards against the ways output drifted apart before the prints decisions:
// a command printing lines of its own beside the one printer, a move worded
// outside the one move computation, and two commands naming two moves for
// one state.
mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/gate.md";

// adr: 848e1b76-9cfe-483b-94dd-094eb9b2f995
// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// Every line a command prints goes through the one printer in src/out.rs,
// so what a command prints is a `Line` (what it read, a status, a finding,
// a move, an event) and nothing else: no `print!` and no handle on
// standard output or error anywhere else in the tool.
#[test]
fn no_source_file_but_the_printer_writes_to_the_terminal() {
    let writers = ["print!(", "println!(", "eprint!(", "eprintln!(", "io::stdout()", "io::stderr()"];
    let mut found = Vec::new();
    for (rel, text) in source_files() {
        if rel == "src/out.rs" {
            continue;
        }
        for (n, line) in code_lines(&text) {
            if writers.iter().any(|w| line.contains(w)) {
                found.push(format!("{}:{}: {}", rel, n, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "printing outside src/out.rs:\n{}", found.join("\n"));
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
//
// The words of a move that is no refusal's own clause (who builds, who
// answers, what waits) are written once, in src/moves.rs. msg.rs's `Next`
// is the renderer moves.rs calls for a clause; it writes "agent" and
// "owner" and never a move of its own.
#[test]
fn the_words_of_a_move_are_written_only_in_moves() {
    let phrases = ["not yet", "owner runs:", "agent builds", "builds \\\"", "next: "];
    let mut found = Vec::new();
    for (rel, text) in source_files() {
        if rel == "src/moves.rs" {
            continue;
        }
        for (n, line) in code_lines(&text) {
            for literal in string_literals(line) {
                if phrases.iter().any(|p| literal.contains(p)) {
                    found.push(format!("{}:{}: {}", rel, n, line.trim()));
                }
            }
        }
    }
    assert!(found.is_empty(), "move words outside src/moves.rs:\n{}", found.join("\n"));
    let msg = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/msg.rs")).unwrap();
    assert!(!msg.contains("pub fn next_"), "msg.rs builds a next line of its own");
}

/// The output of `adr <args>` in `dir`, standard output and error together:
/// where a command names the move, whichever stream carries it.
fn all_of(dir: &Path, args: &[&str]) -> String {
    let out = adr(dir, args, &[], "");
    stdout(&out) + &stderr(&out)
}

/// The move a refusal line names: what follows its finding.
fn move_of(refusal: &str) -> &str {
    refusal.split_once("; ").map(|(_, m)| m).unwrap_or_else(|| panic!("a refusal with no move: {}", refusal))
}

/// Asserts every command that reads the state names the move of each line
/// `refused` holds, word for word: `show`, `show <path>`, and `plan` and
/// `ask` when the state is a topic file's; `check` names the whole line,
/// finding and move.
fn assert_every_command_names(dir: &Path, path: &Path, refused: &[String], case: &str, topic: bool) {
    assert!(!refused.is_empty(), "{}: approve must refuse", case);
    let rel = path.strip_prefix(dir).unwrap().to_str().unwrap().to_string();
    let mut readers: Vec<(&str, String)> = vec![("show", all_of(dir, &["show"])), ("show <path>", all_of(dir, &["show", &rel]))];
    if topic {
        readers.push(("plan", all_of(dir, &["plan", path.to_str().unwrap()])));
        readers.push(("ask", all_of(dir, &["ask", path.to_str().unwrap()])));
    }
    let check = all_of(dir, &["check"]);
    for r in refused {
        assert!(check.contains(r.as_str()), "{}: check does not name what approve refused:\n{}\n--- check:\n{}", case, r, check);
        for (who, out) in &readers {
            assert!(
                out.contains(move_of(r)),
                "{}: {} names another move than approve:\n{}\n--- {}:\n{}",
                case,
                who,
                move_of(r),
                who,
                out
            );
        }
    }
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
//
// One state, one move: for each state approve refuses in, show (with no
// file and with the file), check, plan and ask name the finding and its
// move in the same words approve refuses with.
#[test]
fn every_command_names_the_move_approve_refuses_with() {
    let cases: Vec<(&str, String)> = vec![
        (
            "loaded word in a draft",
            format!("# Gate\n\n{}", gate_question("Does the widget hold?", "the widget currently breaks.")),
        ),
        (
            "heading with no body",
            format!(
                "# Gate\n\n{}\n### Does the widget stay?\ndepends: widget\n",
                gate_question("Does the widget hold?", "the widget breaks.")
            ),
        ),
        ("neighbour on no line", format!("# Gate\n\n{}", gate_question("Does the widget hold?", "the widget breaks."))),
    ];
    for (case, content) in cases {
        let dir = if case == "neighbour on no line" {
            let dir = init_repo();
            assert!(adr(&dir, &["install"], &[], "").status.success());
            approve_terms_with(&dir, &["widget", "paint"]);
            dir
        } else {
            widget_repo()
        };
        let rec = write_record(&dir, REL, &content);
        if case == "neighbour on no line" {
            assert!(adr(&dir, &["plan", rec.to_str().unwrap()], &[], "").status.success());
        } else {
            plan_record(&dir, REL);
        }
        let out = approve_unplanned(&dir, &rec);
        assert!(!out.status.success(), "{}: approve refuses", case);
        assert_every_command_names(&dir, &rec, &refusal_lines(&out, "approve: "), case, true);
    }
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
#[test]
fn every_command_names_the_move_for_an_asked_question_edited_with_a_loaded_word() {
    let dir = widget_repo();
    let rec = write_record(&dir, REL, &format!("# Gate\n\n{}", gate_question("Does the widget hold?", "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", rec.to_str().unwrap()], &[], "").status.success());
    let id = extract_ids(&std::fs::read_to_string(&rec).unwrap()).remove(0);
    let version = current_version(&dir, &id);
    let text = std::fs::read_to_string(&rec).unwrap().replace("the widget breaks.", "the widget currently breaks.");
    std::fs::write(&rec, text).unwrap();
    let out = approve_unplanned(&dir, &rec);
    let refused = refusal_lines(&out, "approve: ");
    assert_every_command_names(&dir, &rec, &refused, "asked question edited with a loaded word", true);
    // The owner's answer on the edited question names the same move.
    let answered = adr_owner(&dir, &["answer", &format!("{}@{}", id, version), "A"], "");
    assert!(!answered.status.success());
    for r in &refused {
        assert!(stderr(&answered).contains(move_of(r)), "answer names another move:\n{}\n--- answer:\n{}", r, stderr(&answered));
    }
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
#[test]
fn every_command_names_the_move_for_a_removed_file_with_an_asked_question() {
    let dir = widget_repo();
    let rec = write_record(&dir, REL, &format!("# Gate\n\n{}", gate_question("Does the widget hold?", "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", rec.to_str().unwrap()], &[], "").status.success());
    std::fs::remove_file(&rec).unwrap();
    let out = approve_unplanned(&dir, &rec);
    let refused = refusal_lines(&out, "approve: ");
    assert!(!refused.is_empty(), "approve refuses the removal");
    let (show, check) = (all_of(&dir, &["show"]), all_of(&dir, &["check"]));
    for r in &refused {
        assert!(check.contains(r.as_str()), "check names another finding:\n{}\n--- check:\n{}", r, check);
        assert!(show.contains(move_of(r)), "show names another move:\n{}\n--- show:\n{}", move_of(r), show);
    }
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
#[test]
fn every_command_names_the_move_for_a_blame_row_naming_no_question() {
    let (dir, _, _) = blame_setup();
    let blob = write_origin(&dir, "docs/old.md", "one\ntwo\n");
    let blame = write_blame_file(&dir, "adrdom/blame/old.md", "docs/old.md", &blob, "1-2\tdecision: Is there no such thing?\n");
    let out = approve_unplanned(&dir, &blame);
    assert_every_command_names(&dir, &blame, &refusal_lines(&out, "approve: "), "blame row naming no question", false);
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
//
// An asked question edited since it was asked, its file awaiting approval:
// check's finding names the move show, plan, ask and answer name for it.
#[test]
fn every_command_names_the_move_for_an_asked_question_edited_since_asked() {
    let dir = widget_repo();
    let rec = write_record(&dir, REL, &format!("# Gate\n\n{}", gate_question("Does the widget hold?", "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", rec.to_str().unwrap()], &[], "").status.success());
    let id = extract_ids(&std::fs::read_to_string(&rec).unwrap()).remove(0);
    let version = current_version(&dir, &id);
    let text = std::fs::read_to_string(&rec).unwrap().replace("the widget breaks.", "the widget snaps.");
    std::fs::write(&rec, text).unwrap();

    let check = all_of(&dir, &["check"]);
    let edited = check
        .lines()
        .find(|l| l.contains("edited after asking"))
        .unwrap_or_else(|| panic!("check names the edit:\n{}", check))
        .to_string();
    let mv = move_of(&edited).to_string();
    for (who, out) in [
        ("show", all_of(&dir, &["show"])),
        ("show <path>", all_of(&dir, &["show", REL])),
        ("plan", all_of(&dir, &["plan", rec.to_str().unwrap()])),
        ("ask", all_of(&dir, &["ask", rec.to_str().unwrap()])),
    ] {
        assert!(out.contains(&mv), "{} names another move than check:\n{}\n--- {}:\n{}", who, mv, who, out);
    }
    let answered = adr_owner(&dir, &["answer", &format!("{}@{}", id, version), "A"], "");
    assert!(!answered.status.success());
    assert!(stderr(&answered).contains(&mv), "answer names another move:\n{}\n--- answer:\n{}", mv, stderr(&answered));
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
//
// Every move that names a command reads `<actor> runs: <command>`: no
// string in the tool words it another way.
#[test]
fn every_move_that_names_a_command_reads_runs_colon() {
    let mut found = Vec::new();
    for (rel, text) in source_files() {
        for (n, line) in code_lines(&text) {
            for literal in string_literals(line) {
                if literal.contains("runs adr ") || literal.contains("runs `adr") {
                    found.push(format!("{}:{}: {}", rel, n, line.trim()));
                }
            }
        }
    }
    assert!(found.is_empty(), "a move naming a command without `runs:`:\n{}", found.join("\n"));
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// Every refusal names the command that clears it: no move in the tool tells
// the agent to look at the error and try again without naming what to run.
#[test]
fn no_move_says_retry_without_naming_the_command() {
    let mut found = Vec::new();
    for (rel, text) in source_files() {
        for (n, line) in code_lines(&text) {
            for literal in string_literals(line) {
                if ["checks the error", "checks the path", "and retries"].iter().any(|w| literal.contains(w)) {
                    found.push(format!("{}:{}: {}", rel, n, line.trim()));
                }
            }
        }
    }
    assert!(found.is_empty(), "a move that names no command:\n{}", found.join("\n"));
}

/// The findings and moves a command printed: every line at the left edge
/// that is no status line, no topic line and no question `plan` read.
fn report_of(out: &str) -> Vec<String> {
    out.lines().filter(|l| !l.starts_with([' ', '[', '"', '#', '+', '-'])).map(str::to_string).collect()
}

/// Asserts show, plan (when `plan_out` is given) and check print one set of
/// findings for the file, and show and plan one list of moves: a finding
/// renders one way in every command (per "Does a command compute a move of
/// its own?", aac5c313 A, and "What does a command that writes nothing
/// print?", 848e1b76 A).
fn assert_one_report(dir: &Path, plan_out: Option<&str>, stage: &str) -> Vec<String> {
    let show = report_of(&stdout(&adr(dir, &["show", REL], &[], "")));
    let check: Vec<String> = stdout(&adr(dir, &["check"], &[], "")).lines().filter(|l| l.contains(REL)).map(str::to_string).collect();
    let findings: Vec<String> = show.iter().filter(|l| !(l.starts_with("agent ") || l.starts_with("owner runs:") || l.starts_with("not yet"))).cloned().collect();
    let mut sorted_check = check.clone();
    sorted_check.sort();
    let mut sorted_findings = findings.clone();
    sorted_findings.sort();
    assert_eq!(sorted_findings, sorted_check, "{}: show and check print other findings", stage);
    if let Some(p) = plan_out {
        assert_eq!(report_of(p), show, "{}: plan and show print another report", stage);
    }
    findings
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
// adr: 848e1b76-9cfe-483b-94dd-094eb9b2f995
//
// At every planning stage -- headings only, the lines written, the bodies
// written, ready -- show, plan and check print the same findings, each
// with its move, and show and plan the same moves.
#[test]
fn every_planning_stage_prints_one_report_in_every_command() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget", "paint"]);
    let (h1, h2) = ("Does the widget hold?", "Does the widget bend?");
    let path = write_record(&dir, REL, &format!("# Gate\n\n### {}\ndepends: widget\n\n### {}\ndepends: widget\n", h1, h2));
    let plan = |dir: &Path| stdout(&adr(dir, &["plan", path.to_str().unwrap()], &[], ""));

    assert!(!assert_one_report(&dir, None, "headings, not planned").is_empty());
    let out = plan(&dir);
    assert!(!assert_one_report(&dir, Some(&out), "headings planned").is_empty());
    assert!(report_of(&out).iter().any(|l| l.contains("is on no line")), "{}", out);

    std::fs::write(&path, add_unrelated(&std::fs::read_to_string(&path).unwrap(), &lines_wanted(&out))).unwrap();
    let out = plan(&dir);
    assert!(!assert_one_report(&dir, Some(&out), "lines written").is_empty());
    assert!(report_of(&out).iter().any(|l| l.contains("[no body]")), "{}", out);

    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace(&format!("### {}\n", h1), &format!("### {}\n{}", h1, "\nProblem: the widget holds.\n\nStandard: plain words.\n\n"))
        .replace(&format!("### {}\n", h2), &format!("### {}\n{}", h2, "\nProblem: the widget bends.\n\nStandard: plain words.\n\n"));
    let text = text.replace("\nStandard: plain words.\n\n", &format!("\nStandard: plain words.\n\n{}", GATE_OPTIONS));
    std::fs::write(&path, text).unwrap();
    let out = plan(&dir);
    assert!(!assert_one_report(&dir, Some(&out), "bodies written").is_empty(), "{}", out);

    std::fs::write(&path, add_unrelated(&std::fs::read_to_string(&path).unwrap(), &lines_wanted(&out))).unwrap();
    let out = plan(&dir);
    assert!(assert_one_report(&dir, Some(&out), "ready").is_empty(), "{}", out);
    assert_eq!(report_of(&out), vec![format!("owner runs: adr approve {}", REL)], "{}", out);

    // Asked, then one question struck from the plan before the owner
    // approves the change: the one finding, in every command.
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    let text = std::fs::read_to_string(&path).unwrap();
    let at = text.find(&format!("### {}", h2)).unwrap();
    std::fs::write(&path, &text[..at]).unwrap();
    let out = plan(&dir);
    let findings = assert_one_report(&dir, Some(&out), "struck asked question");
    assert_eq!(findings.len(), 1, "one finding for the struck question: {:?}", findings);
    assert!(findings[0].starts_with("ids: ") && findings[0].contains(h2) && findings[0].contains("was removed from the plan"), "{:?}", findings);
    assert!(findings[0].ends_with(&format!("; owner runs: adr approve {}", REL)), "{:?}", findings);
}
