// What `adr show` prints, with no file named, with a file approve takes, and
// under its flags: what it reads from the records, statuses, findings and
// moves, and nothing else.

mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/gate.md";
const ONE: &str = "Is the widget gate open?";
const TWO: &str = "Is the widget gate close?";

const BODY: &str = "\nProblem: the widget breaks.\n\nStandard: plain words.\n\n\
- A) Yes the widget holds.\n  Why: reasons.\n  Costs: costs.\n\
- B) No the widget does not hold.\n  Why: other reasons.\n  Costs: other costs.\n";

fn q(heading: &str) -> String {
    format!("### {}\ndepends: widget\n{}\n", heading, BODY)
}

fn commit_all(dir: &Path, message: &str) {
    run_git(dir, &["add", "-A"]);
    let out = run_git(dir, &["commit", "-q", "-m", message]);
    assert!(
        out.status.success(),
        "commit failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn show(dir: &Path, args: &[&str]) -> String {
    let mut all = vec!["show"];
    all.extend_from_slice(args);
    let out = adr(dir, &all, &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    stdout(&out)
}

fn id_of(path: &Path, heading: &str) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    let at = text.find(&format!("### {}\n", heading)).unwrap();
    extract_ids(&text[at..])[0].clone()
}

/// A repository with the terms approved and the two questions of `REL`
/// approved and asked, the first answered; everything committed.
fn asked() -> (PathBuf, PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}{}", q(ONE), q(TWO)));
    approve_record(&dir, REL);
    let asked = adr(&dir, &["ask", path.to_str().unwrap()], &[], "");
    assert!(asked.status.success(), "{}", stderr(&asked));
    let one = id_of(&path, ONE);
    assert!(adr_answer(&dir, &one, "A").status.success());
    commit_all(&dir, "the gate, asked");
    (dir, path)
}

/// Whether `line` is one of the kinds `show` prints: a topic file's line, a
/// status line, a line under either (what it read), or a move.
fn is_a_kind_show_prints(line: &str) -> bool {
    line.starts_with("# ")
        || line.starts_with('[')
        || line.starts_with("  ")
        || line.starts_with("agent ")
        || line.starts_with("owner runs: ")
        || line.starts_with("not yet: ")
}

// adr: c84a4bd9-fc2d-44f0-b333-172978f61544
//
// With no file named: every current decision as its heading and the option
// picked, and every move; nothing about a question that is no decision.
#[test]
fn show_with_no_file_prints_every_current_decision_and_every_move() {
    let (dir, path) = asked();
    let two = id_of(&path, TWO);
    let version = current_version(&dir, &two);
    assert_eq!(
        show(&dir, &[]),
        format!(
            "# Gate ({rel})\n  {one}\n    Yes the widget holds.\n\
owner runs: adr answer {two_id}@{version} <letter>  # \"{two}\"\n\
not yet: \"{one}\" is answered and is built once \"{two}\" is answered\n",
            rel = REL,
            one = ONE,
            two = TWO,
            two_id = two,
            version = version
        )
    );
}

//
// With a file named: every question with its status and the lines it
// carries; the body of the asked one alone; then the moves of that file.
#[test]
fn show_with_a_file_prints_each_question_its_lines_and_an_asked_body() {
    let (dir, path) = asked();
    let two = id_of(&path, TWO);
    let version = current_version(&dir, &two);
    let shown = show(&dir, &[REL]);
    assert_eq!(
        shown,
        format!(
            "[approved] Gate ({rel})\n\
[answered A] {one}\n  depends: widget\n\
[asked @{version}] {two}\n  depends: widget\n  Problem: the widget breaks.\n  Standard: plain words.\n  \
- A) Yes the widget holds.\n    Why: reasons.\n    Costs: costs.\n  \
- B) No the widget does not hold.\n    Why: other reasons.\n    Costs: other costs.\n\
owner runs: adr answer {two_id}@{version} <letter>  # \"{two}\"\n\
not yet: \"{one}\" is answered and is built once \"{two}\" is answered\n",
            rel = REL,
            one = ONE,
            two = TWO,
            two_id = two,
            version = version
        )
    );
}

// The unrelated line comes under its own flag.
#[test]
fn show_prints_the_unrelated_line_only_under_its_flag() {
    // adr: dce141e9-7d2e-4b70-b317-9f3d70bf5a69
    let (dir, path) = asked();
    let plain = show(&dir, &[REL]);
    assert!(plain.contains("  depends: widget\n"), "{}", plain);
    assert!(!plain.contains("unrelated:"), "{}", plain);
    let flagged = show(&dir, &[REL, "--unrelated"]);
    assert!(flagged.contains(&format!("  unrelated: {}\n", TWO)), "{}", flagged);

    // Edit the unrelated line of the first question: plain show keeps the
    // changed status and prints no marked line; the flag prints the mark.
    let text = std::fs::read_to_string(&path).unwrap();
    let edited = text.replacen(&format!("unrelated: {}", id_of(&path, TWO)), "unrelated: widget", 1);
    assert_ne!(edited, text);
    std::fs::write(&path, edited).unwrap();
    let plain = show(&dir, &[REL]);
    assert!(plain.contains("[changed"), "{}", plain);
    assert!(!plain.contains("unrelated:"), "{}", plain);
    assert!(!plain.lines().any(|l| l.trim_start().starts_with("+ ") || l.trim_start().starts_with("- ") && !l.contains(") ")), "{}", plain);
    let flagged = show(&dir, &[REL, "--unrelated"]);
    assert!(flagged.contains(&format!("- unrelated: {}\n", TWO)), "{}", flagged);
    assert!(flagged.contains("+ unrelated: widget\n"), "{}", flagged);
}

//
// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// Before approve the owner reads, in show, what approve would sign: a new
// file whole, marked; on a plan change, the question changed with the lines
// it lost and gained, and a question struck from the plan with its approved
// lines; a terms entry changed; a blame file's rows.
#[test]
fn show_marks_what_a_file_holds_beyond_its_latest_approval() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    // A new topic file: every question, its heading and every line, is
    // marked new.
    let path = write_record(&dir, REL, &format!("# Gate\n\nWhat the gate does.\n\n{}{}", q(ONE), q(TWO)));
    let shown = show(&dir, &[REL]);
    assert!(shown.starts_with("[new] Gate (adrdom/decisions/gate.md)\n+ [new] prose\n  + What the gate does.\n"), "{}", shown);
    assert!(shown.contains(&format!("+ [not asked] {}\n  + depends: widget\n  + Problem: the widget breaks.\n", ONE)), "{}", shown);
    assert!(shown.ends_with(&format!("; agent runs: adr plan {}\n", REL)), "{}", shown);

    // Approved and asked, then changed: ONE's body edited, TWO struck.
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    commit_all(&dir, "the gate, asked");
    let text = std::fs::read_to_string(&path).unwrap();
    let at = text.find(&format!("### {}\n", TWO)).unwrap();
    let edited = text[..at].replacen("the widget breaks.", "the widget breaks in use.", 1);
    std::fs::write(&path, edited).unwrap();
    let shown = show(&dir, &[REL]);
    // The `id:` line approve signs is part of the change, unmarked here.
    let id_one = extract_ids(&text[text.find(&format!("### {}\n", ONE)).unwrap()..])[0].clone();
    assert!(shown.starts_with("[changed] Gate (adrdom/decisions/gate.md)\n"), "{}", shown);
    assert!(
        shown.contains(&format!(
            "[changed since asked] {}\n    depends: widget\n    id: {}\n  - Problem: the widget breaks.\n  + Problem: the widget breaks in use.\n    Standard: plain words.\n",
            ONE, id_one
        )),
        "{}",
        shown
    );
    assert!(
        shown.contains(&format!(
            "- [removed] {}\n  - depends: widget\n  - id: {}\n  - Problem: the widget breaks.\n",
            TWO,
            extract_ids(&text[at..])[0]
        )),
        "{}",
        shown
    );
    // Nothing else of the file is marked: the prose stands as approved.
    assert!(!shown.contains("prose"), "{}", shown);
    std::fs::write(&path, text).unwrap();

    // A terms entry changed: the entry and the lines it lost and gained.
    std::fs::write(
        dir.join("adrdom/terms.md"),
        "# Terms\n\n## widget\nid: 99999999-9999-9999-9999-999999999999\ndefinition: A part of a gate.\naliases:\n\n## gate\ndefinition: A way through a wall.\naliases:\n",
    )
    .unwrap();
    let out = adr_owner(&dir, &["approve", "adrdom/terms.md"], "");
    assert!(out.status.success(), "{}", stderr(&out));
    commit_all(&dir, "the terms");
    let terms = std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap();
    std::fs::write(dir.join("adrdom/terms.md"), terms.replace("A part of a gate.", "A moving part of a gate.")).unwrap();
    let shown = show(&dir, &["adrdom/terms.md"]);
    assert!(shown.starts_with("[changed] adrdom/terms.md\n[changed] widget\n"), "{}", shown);
    assert!(
        shown.contains("  - definition: A part of a gate.\n  + definition: A moving part of a gate.\n    aliases:\n[approved] gate\n  definition: A way through a wall.\n"),
        "{}",
        shown
    );
    assert!(shown.ends_with("owner runs: adr approve adrdom/terms.md\n"), "{}", shown);

    // A new blame file: its lines, marked.
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdropped: dead\n");
    let shown = show(&dir, &["adrdom/blame/x.md"]);
    assert!(shown.starts_with("[new] adrdom/blame/x.md\n  + origin: docs/x.md\n"), "{}", shown);
    assert!(shown.contains("  + 1-1\tdropped: dead\n"), "{}", shown);
}

// adr: 36dfbeaa-698d-4a57-b3b0-d425348e14a5
//
// The neighbours with the reader's word, what stands on a question and what
// replaced it are computed from the records: the plain listing has none of
// them, each comes under its flag, and a flag needs a file or --all.
#[test]
fn a_view_computed_from_the_records_comes_under_its_flag() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let three = "Is the widget gate lock once it closes?";
    let path = write_record(
        &dir,
        REL,
        &format!("# Gate\n\n{}{}### {}\ndepends: widget, {}=A\n{}\n", q(ONE), q(TWO), three, ONE, BODY),
    );
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());

    let plain = show(&dir, &[REL]);
    for word in ["neighbour", "dependents:", "replaced by"] {
        assert!(!plain.contains(word), "{}: {}", word, plain);
    }
    let all = show(&dir, &["--all"]);
    assert!(!all.contains("neighbour") && !all.contains("dependents:"), "{}", all);

    let neighbours = show(&dir, &[REL, "--neighbours"]);
    assert!(neighbours.contains("  term \"widget\": heading "), "{}", neighbours);
    assert!(!plain.contains("  term \"widget\": heading "), "{}", plain);
    let dependents = show(&dir, &[REL, "--dependents"]);
    assert!(dependents.contains(&format!("  dependents: {}\n", three)), "{}", dependents);
    assert!(!dependents.contains("  term \"widget\": heading "), "{}", dependents);

    let out = adr(&dir, &["show", "--neighbours"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
}

// adr: 848e1b76-9cfe-483b-94dd-094eb9b2f995
//
// Show writes nothing, so it prints what it read, statuses, findings and
// moves: no count, no tier heading, no line on how to read the rest.
#[test]
fn show_prints_no_count_tier_or_hint_line() {
    let (dir, _path) = asked();
    let blob = write_origin(&dir, "docs/x.md", "one\n");
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdropped: dead\n");
    for args in [vec![], vec![REL], vec!["--all"], vec!["adrdom/terms.md"], vec!["adrdom/blame/x.md"]] {
        let shown = show(&dir, &args);
        for line in shown.lines() {
            assert!(is_a_kind_show_prints(line), "show {:?} printed {:?}:\n{}", args, line, shown);
            assert!(!line.starts_with("tier ") && !line.starts_with("next"), "{:?}: {}", args, shown);
        }
        assert!(!shown.contains("adr show <topic file>"), "{}", shown);
        assert!(!shown.contains("dropped: dead 1") && !shown.contains("from: "), "{}", shown);
    }
}

// adr: f53ac3b9-efde-427c-8027-19988aa8fda7
//
// A removed topic file is `[removed]`; each question of its approved plan
// keeps the status of its own list.
#[test]
fn a_removed_topic_file_shows_each_question_with_its_own_status() {
    let (dir, path) = asked();
    let two = id_of(&path, TWO);
    let version = current_version(&dir, &two);
    std::fs::remove_file(&path).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.starts_with(&format!("[removed] {}\n", REL)), "{}", shown);
    assert!(shown.contains(&format!("[answered A] {}\n", ONE)), "{}", shown);
    assert!(shown.contains(&format!("[asked @{}] {}\n", version, TWO)), "{}", shown);
    assert!(!shown.contains(&format!("[removed] {}", ONE)), "{}", shown);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// Approved and not yet committed, then a question struck: no committed text
// holds the approved plan, and the struck question still shows, by its
// identifier.
#[test]
fn a_question_struck_from_an_uncommitted_approval_still_shows() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}{}", q(ONE), q(TWO)));
    approve_record(&dir, REL);
    let two = id_of(&path, TWO);
    let text = std::fs::read_to_string(&path).unwrap();
    let at = text.find(&format!("### {}\n", TWO)).unwrap();
    std::fs::write(&path, &text[..at]).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains(&format!("[removed] {}\n", two)), "{}", shown);
}

// `show <file> --all` names two forms at once: clap refuses it rather than
// print one of them.
#[test]
fn show_refuses_a_file_and_all_together() {
    let (dir, _) = asked();
    let out = adr(&dir, &["show", REL, "--all"], &[], "");
    assert!(!out.status.success());
    assert!(stdout(&out).is_empty(), "{}", stdout(&out));
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// Lines that cannot reach standard output (a full device) leave the output
// incomplete: the command stops with 1, and its refusal names what failed
// and the command to run again ("Does a refusal name the command that
// clears it?").
#[cfg(target_os = "linux")]
#[test]
fn show_into_a_full_device_does_not_pass() {
    let (dir, _) = asked();
    let full = std::fs::OpenOptions::new().write(true).open("/dev/full").unwrap();
    let out = std::process::Command::new(adr_bin())
        .args(["show", "--all"])
        .current_dir(&dir)
        .env("ADR_FAKE_MODEL", "1")
        .stdin(std::process::Stdio::null())
        .stdout(full)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("standard output cannot be written"), "{}", err);
    assert!(err.contains("runs: adr show --all"), "{}", err);
}

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// A question new since the latest approval is marked as a whole, its
// heading line too, at every stage: a heading alone, its lines written, its
// body written, and a heading added to an approved plan.
#[test]
fn a_question_new_since_approval_marks_its_heading_at_every_stage() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let path = write_record(&dir, REL, &format!("# Gate\n\n### {}\n", ONE));
    assert!(show(&dir, &[REL]).contains(&format!("\n+ [no body] {}\n", ONE)), "headings only: {}", show(&dir, &[REL]));
    std::fs::write(&path, format!("# Gate\n\n### {}\ndepends: widget\n", ONE)).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains(&format!("\n+ [no body] {}\n  + depends: widget\n", ONE)), "lines written: {}", shown);
    std::fs::write(&path, format!("# Gate\n\n{}", q(ONE))).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains(&format!("\n+ [not asked] {}\n  + depends: widget\n", ONE)), "body written: {}", shown);

    approve_record(&dir, REL);
    commit_all(&dir, "the gate");
    let shown = show(&dir, &[REL]);
    assert!(!shown.lines().any(|l| l.starts_with("+ ") || l.starts_with("- ")), "approved: {}", shown);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{}\n### {}\ndepends: widget\n", text, TWO)).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains(&format!("\n+ [no body] {}\n  + depends: widget\n", TWO)), "plan change: {}", shown);
    assert!(shown.contains(&format!("\n[not asked] {}\n", ONE)), "the approved question is not marked: {}", shown);
}

// adr: 36dfbeaa-698d-4a57-b3b0-d425348e14a5
//
// What stands on a term entry is computed from the records: the current
// decisions whose terms line or depends line names the term come under
// `--dependents`, and the plain form of the terms file has none of them.
#[test]
fn the_decisions_standing_on_a_term_come_under_the_dependents_flag() {
    let (dir, _) = asked();
    let plain = show(&dir, &["adrdom/terms.md"]);
    assert!(!plain.contains("dependents:"), "{}", plain);
    let dependents = show(&dir, &["adrdom/terms.md", "--dependents"]);
    let at = dependents.find("] widget\n").unwrap_or_else(|| panic!("{}", dependents));
    let entry = &dependents[at..];
    let entry = &entry[..entry[1..].find("\n[").map(|i| i + 1).unwrap_or(entry.len())];
    // ONE is answered, a current decision; TWO is asked, not yet one.
    assert!(entry.contains(&format!("  dependents: {} ({})\n", ONE, REL)), "{}", dependents);
    assert!(!entry.contains(TWO), "{}", dependents);
}
