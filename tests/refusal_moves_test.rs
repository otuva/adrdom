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
        "{}: \"Is the widget beta hold?\" depends on \"Is the widget alpha hold?\" ({}) which is not answered yet; a question stands only on decisions in force; not yet, after it is answered",
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
        assert!(!stdout(&out).contains("no tracked file holds its tie"), "{:?}: {}", args, stdout(&out));
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
            content.push_str(&format!("{}\n", question(&format!("Is the widget {} hold?", n), "widget", "")));
        }
        content.push_str(&question(FIFTH, fifth_depends, ""));
        content
    };
    let rel = "adrdom/decisions/cap.md";
    let path = write_record(&dir, rel, &plan("widget"));

    let refusal = format!(
        "{}: 5 questions in one tier, the most is 4: \
\"Is the widget one hold?\", \"Is the widget two hold?\", \"Is the widget three hold?\", \
\"Is the widget four hold?\", \"{}\"; \
no tier holds more than four questions; agent rewrites the depends lines of these questions",
        rel, FIFTH
    );
    let out = adr(&dir, &["check"], &[], "");
    assert!(stdout(&out).contains(&refusal), "{}", stdout(&out));
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains(&refusal), "{}", stderr(&out));

    write_record(&dir, rel, &plan("widget, Is the widget one hold?=A"));
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
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
        "{}: 5 questions in one tier, 4 answered, the most is 4: \"Is the widget five hold?\"; \
no tier holds more than four questions; agent rewrites the depends lines of these questions",
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
            "{}: {} is answered and no tracked file holds its tie; a current decision is reported until a test names it; agent builds \"{}\" and puts \"adr: {}\" in a test that shows it",
            A, ALPHA, ALPHA, alpha
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

    // Under alpha, show names under its flag the question of the other file
    // that stands on it: what would lose its ground with alpha.
    let shown = stdout(&adr(&dir, &["show", "--all", "--dependents"], &[], ""));
    assert!(
        shown.contains(&format!(
            "[answered A] {}\n  depends: widget\n  dependents: {} ({})\n",
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
            "{} (\"{}\") tied to a test and not a current decision, at tested.txt:1; every test tie names a current decision; agent removes the tie comment from the test",
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
            "{}: \"Is the widget delta hold?\": supersedes {} (\"{}\"), which is not a current decision; a question supersedes only a current decision; agent rewrites the supersedes entry",
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

    // The reading end is closed before the tool starts, so its first print
    // meets the closed pipe whatever the timing.
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let out = Command::new(adr_bin())
        .current_dir(&dir)
        .arg("show")
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.is_empty(), "{}", err);
    assert_eq!(out.status.code(), Some(141), "{:?}", out.status);
}

/// The clauses of a refusal's move, split at `, ` and `; `: no clause may
/// come twice, and none may be the command that just failed by itself.
fn assert_one_clearing_move(line: &str, failed: &str) {
    let (_, mv) = line.split_once("; ").unwrap_or_else(|| panic!("a refusal with no move: {}", line));
    // Clauses are joined by "; " and, as `Next` renders them, by ", " before
    // the next actor's clause.
    let joined = mv.replace(", agent ", "\u{0}agent ").replace(", owner ", "\u{0}owner ").replace(", not yet", "\u{0}not yet");
    let clauses: Vec<&str> = joined.split(['\n', '\u{0}']).flat_map(|l| l.split("; ")).collect();
    for (i, c) in clauses.iter().enumerate() {
        assert!(!clauses[..i].contains(c), "a clause named twice: {}", line);
    }
    assert_ne!(mv.trim(), format!("owner runs: {}", failed), "the move is the failed command: {}", line);
    assert_ne!(mv.trim(), format!("agent runs: {}", failed), "the move is the failed command: {}", line);
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// A settings file install cannot read names the fix once, in install's
// refusal and in check's finding alike; uninstall names the fix and its own
// run once.
#[test]
fn a_settings_file_that_does_not_parse_names_its_fix_once() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());
    std::fs::write(dir.join(".claude/settings.json"), "{ not json").unwrap();

    let out = adr(&dir, &["install", "--claude"], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    let fix = "agent fixes .claude/settings.json so it parses as JSON, then runs: adr install --claude";
    assert!(err.contains(fix), "{}", err);
    assert_eq!(err.matches("agent ").count(), 1, "{}", err);
    assert_one_clearing_move(err.trim_end(), "adr install --claude");

    let check = stdout(&adr(&dir, &["check"], &[], ""));
    let line = check.lines().find(|l| l.contains("settings.json")).unwrap_or_else(|| panic!("{}", check));
    assert!(line.ends_with("agent rewrites the file, then runs: adr install --claude"), "check names another move: {}", line);

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("agent fixes .claude/settings.json so it parses as JSON, owner runs: adr uninstall"), "{}", err);
    assert_eq!(err.matches("owner runs: adr uninstall").count(), 1, "{}", err);
    assert_one_clearing_move(err.trim_end(), "adr uninstall");
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// Outside a git repository every command names the step that clears it,
// changing into one, ahead of the command, and nothing twice.
#[test]
fn outside_a_repository_every_command_names_the_step_that_clears_it() {
    let outside = OutsideDir::new("outside-moves");
    let outside = outside.path();
    // Every command, with the arguments and flags it was run with: the move
    // names that same command line to run again, the command's name leads
    // the line once.
    for (args, owner) in [
        (vec!["approve", "x.md"], true),
        (vec!["answer", "id@abcd1234", "A"], true),
        (vec!["unban"], true),
        (vec!["uninstall"], true),
        (vec!["serve", "--port", "0"], true),
        (vec!["show"], false),
        (vec!["show", "x.md"], false),
        (vec!["show", "--all", "--neighbours", "--dependents"], false),
        (vec!["show", "x.md", "--questions"], false),
        (vec!["check"], false),
        (vec!["check", "--staged"], false),
        (vec!["check", "--message", "m.txt"], false),
        (vec!["check", "--since", "abc123"], false),
        (vec!["check", "--push"], false),
        (vec!["ask", "x.md"], false),
        (vec!["plan", "x.md"], false),
        (vec!["read"], false),
        (vec!["install"], false),
        (vec!["install", "--claude"], false),
        (vec!["install", "--claude", "--codex"], false),
        (vec!["ban"], false),
        (vec!["ban", "--term", "widget"], false),
        (vec!["ban", "--term", "answer set"], false),
        (vec!["ban", "--rebuild"], false),
    ] {
        let out = if owner { adr_owner(outside, &args, "") } else { adr(outside, &args, &[], "") };
        assert!(!out.status.success(), "{:?}", args);
        let err = stderr(&out);
        let line = err.trim_end();
        assert!(line.contains("not inside a git repository; "), "{:?}: {}", args, err);
        // The move is the command line as given, so it runs as printed: an
        // argument of several words is quoted.
        let quoted: Vec<String> =
            args.iter().map(|a| if a.contains(' ') { format!("'{}'", a) } else { a.to_string() }).collect();
        let command = format!("adr {}", quoted.join(" "));
        assert!(line.ends_with(&format!("changes into a git repository and runs: {}", command)), "{:?}: {}", args, err);
        assert!(line.starts_with(&format!("{}: not inside", args[0])), "{:?}: {}", args, err);
        assert_one_clearing_move(line, &command);
    }
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// An owner command refused to an agent names the command line the owner
// runs, as it can be run: serve with its port.
#[test]
fn an_owner_refusal_names_the_command_as_it_can_be_run() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    for (args, command) in [
        (vec!["serve", "--port", "8123"], "adr serve --port 8123"),
        (vec!["unban"], "adr unban"),
    ] {
        let out = adr(&dir, &args, &[], "");
        assert_eq!(out.status.code(), Some(2), "{:?}", args);
        let err = stderr(&out);
        let line = err.trim_end();
        // The owner running it is what clears an agent's refusal here, so the
        // move is the command itself, named once.
        assert!(line.ends_with(&format!("; owner runs: {}", command)), "{:?}: {}", args, err);
        assert_eq!(line.matches("owner runs:").count(), 1, "{:?}: {}", args, err);
    }
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// ask and plan on a topic file that does not exist say, in words, what to
// give the command (the file meant is not known, so no command line is
// printed that would not run as printed), once.
#[test]
fn ask_and_plan_on_a_missing_topic_file_name_the_command_that_clears_it() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let missing = dir.join("adrdom/decisions/nofile.md");
    for cmd in ["ask", "plan"] {
        let out = adr(&dir, &[cmd, missing.to_str().unwrap()], &[], "");
        assert!(!out.status.success(), "{}: refused", cmd);
        let err = stderr(&out);
        let line = err.lines().next().unwrap_or_default();
        assert!(
            line.ends_with(&format!("agent gives adr {} a topic file under adrdom/decisions that exists", cmd)),
            "{}: {}",
            cmd,
            err
        );
        assert_one_clearing_move(line, &format!("adr {}", cmd));
    }
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// A refusal of several lines names its move on every line: ask on a terms
// file with two faulty entries prints each finding with its own agent move
// and no command name in front.
#[test]
fn a_refusal_of_several_lines_names_the_command_on_every_line() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::write(dir.join("adrdom/terms.md"), "# Terms\n\n## widget\naliases: widgets\n\n## gadget\naliases: gadgets\n").unwrap();
    let rec = write_record(&dir, "adrdom/decisions/x.md", "# X\n\n### Does the widget run?\n");
    let out = adr(&dir, &["ask", rec.to_str().unwrap()], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    let lines: Vec<&str> = err.lines().collect();
    assert!(lines.len() >= 2, "{}", err);
    assert!(lines.iter().all(|l| l.starts_with("adrdom/terms.md:") && l.contains("; agent ")), "{}", err);
}
