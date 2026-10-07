// What `adr show` prints under a topic file as "next": one line per move,
// each naming who makes it. One plan is walked from its first draft to the
// last test tie, and at every step the lines are read. The agent reads its
// next move there and never works it out from the tiers.
//
// Decision f8d92841-aef7-4cd7-bd94-0e07c03e4cb6: "Does the agent build on a
// decision before its tier is fully answered?" Answer: "No: a tier is built
// once every question in it is answered, and nothing in a tier is built
// while a sibling is open."
// Decision fa90692a-150e-4a89-91e2-1d575cb59d0f: "Does a depends entry
// carry the answers it holds under?"
// Decision 11be073c-ce91-44d8-833a-ef0b5e0b1672: "Does an answer carry the
// version of the question the owner read?"

mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/gate.md";
const ONE: &str = "Is the widget gate open?";
const TWO: &str = "Is the widget gate close?";
const THREE: &str = "Is the widget gate lock once it closes?";

const BODY: &str = "\nProblem: the widget breaks.\n\nStandard: plain words.\n\n\
- A) Yes the widget holds.\n  Why: reasons.\n  Costs: costs.\n\
- B) No the widget does not hold.\n  Why: other reasons.\n  Costs: other costs.\n";

/// A heading, its depends line and a full body, as written before approval.
fn q(heading: &str, depends: &str) -> String {
    format!("### {}\ndepends: {}\n{}\n", heading, depends, BODY)
}

fn setup() -> (PathBuf, PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let plan = format!(
        "# Gate\n\n{}{}{}",
        q(ONE, "widget"),
        q(TWO, "widget"),
        q(THREE, &format!("widget, {}=A", TWO))
    );
    let path = write_record(&dir, REL, &plan);
    (dir, path)
}

/// The moves `show` prints for the one topic file: every line that is not
/// the file's topic line, a status line or a decision under it.
fn next(dir: &Path) -> Vec<String> {
    let out = adr(dir, &["show"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    moves_in(&stdout(&out))
}

fn moves_in(shown: &str) -> Vec<String> {
    shown
        .lines()
        .filter(|l| !l.starts_with(' ') && !l.starts_with("# ") && !l.starts_with(['[', '+', '-']))
        .map(str::to_string)
        .collect()
}

fn id_of(path: &Path, heading: &str) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    let at = text.find(&format!("### {}\n", heading)).unwrap();
    extract_ids(&text[at..])[0].clone()
}

fn ask_for(headings: &[&str]) -> String {
    let names: Vec<String> = headings.iter().map(|h| format!("\"{}\"", h)).collect();
    format!("agent runs: adr ask {}  # {}", REL, names.join(", "))
}

fn plan_for() -> String {
    format!("agent runs: adr plan {}", REL)
}

fn lines_for(headings: &[&str]) -> String {
    let names: Vec<String> = headings.iter().map(|h| format!("\"{}\"", h)).collect();
    format!("agent writes lines for {}", names.join(", "))
}

/// Asserts `lines` are the findings of one planning step: one or more per
/// heading of `headings`, each a `plan:` finding about its heading that
/// ends with the move `mv` names for it.
fn assert_each_names(lines: &[String], headings: &[&str], mv: impl Fn(&str) -> String) {
    assert!(!lines.is_empty(), "no finding");
    for l in lines {
        assert!(l.starts_with(&format!("plan: {}: ", REL)), "{}", l);
        assert!(headings.iter().any(|h| l.ends_with(&format!("; {}", mv(h)))), "{}", l);
    }
    for h in headings {
        assert!(lines.iter().any(|l| l.contains(&format!("\"{}\"", h))), "no finding for {}: {:?}", h, lines);
    }
}

fn answer_for(dir: &Path, id: &str, heading: &str) -> String {
    format!(
        "owner runs: adr answer {} <letter>  # \"{}\"",
        id_at_version(dir, id),
        heading
    )
}

fn build_of(heading: &str, id: &str) -> String {
    format!(
        "agent builds \"{}\" and puts \"adr: {}\" in a test that shows it",
        heading, id
    )
}

/// The line for an asked question `answer` would refuse: it names what the
/// question waits on.
fn waits(heading: &str, on: &str) -> String {
    format!("not yet: \"{}\" is asked and waits for \"{}\"", heading, on)
}

// adr: f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
#[test]
fn show_names_the_next_move_at_every_step() {
    let (dir, path) = setup();
    let p = path.to_str().unwrap();

    // A plan that waits for its approval follows the planning steps: the
    // agent plans it, writes the lines of what it read, and only then does
    // the owner approve.
    // Each step's findings carry the step's move (per "Does a command
    // compute a move of its own?", aac5c313 A): one per question.
    assert_each_names(&next(&dir), &[ONE, TWO, THREE], |_| plan_for());
    assert!(adr(&dir, &["plan", p], &[], "").status.success());
    assert_each_names(&next(&dir), &[ONE, TWO, THREE], |h| lines_for(&[h]));
    plan_record(&dir, REL);
    assert_eq!(next(&dir), vec![format!("owner runs: adr approve {}", REL)]);

    approve_record(&dir, REL);
    // Every question is asked in one run, whatever its tier.
    assert_eq!(next(&dir), vec![ask_for(&[ONE, TWO, THREE])]);
    let two = id_of(&path, TWO);

    let asked = adr(&dir, &["ask", p], &[], "");
    assert!(asked.status.success(), "{}", stderr(&asked));
    let one = id_of(&path, ONE);
    let three = id_of(&path, THREE);
    // `answer` takes THREE only once TWO has a decision: it stands on it.
    let waiting = vec![
        answer_for(&dir, &one, ONE),
        answer_for(&dir, &two, TWO),
        waits(THREE, TWO),
    ];
    assert_eq!(next(&dir), waiting);

    // `ask` with nothing to ask writes nothing and prints nothing (per
    // "What does a command that writes print?", 069a6b4a A): the moves are
    // show's.
    let again = adr(&dir, &["ask", p], &[], "");
    assert!(again.status.success());
    assert_eq!(stdout(&again), "", "{}", stdout(&again));

    // A body edited after it was asked: the owner approves the edit, then
    // the move is the agent's, to ask it again.
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("the widget breaks.", "the widget breaks in use.", 1)).unwrap();
    approve_record(&dir, REL);
    assert_eq!(
        next(&dir),
        vec![ask_for(&[ONE]), answer_for(&dir, &two, TWO), waits(THREE, TWO)]
    );
    // Its line in the listing carries no version for the owner to use.
    let shown = stdout(&adr(&dir, &["show", "--all"], &[], ""));
    assert!(shown.contains(&format!("\n[asked] {}\n", ONE)), "{}", shown);
    assert!(adr(&dir, &["ask", p], &[], "").status.success());
    assert_eq!(
        next(&dir),
        vec![answer_for(&dir, &one, ONE), answer_for(&dir, &two, TWO), waits(THREE, TWO)]
    );

    // One of two answered: it is not built while the other is open.
    assert!(adr_answer(&dir, &one, "A").status.success());
    assert_eq!(
        next(&dir),
        vec![
            answer_for(&dir, &two, TWO),
            format!(
                "not yet: \"{}\" is answered and is built once \"{}\" is answered",
                ONE, TWO
            ),
            waits(THREE, TWO),
        ]
    );

    // Both answered: they are built first, and the owner's answer to THREE
    // is named, since what it stands on has its decision.
    assert!(adr_answer(&dir, &two, "A").status.success());
    assert_eq!(
        next(&dir),
        vec![build_of(ONE, &one), build_of(TWO, &two), answer_for(&dir, &three, THREE)]
    );

    write_test_comment(&dir, &one);
    write_test_comment(&dir, &two);
    assert_eq!(next(&dir), vec![answer_for(&dir, &three, THREE)]);

    assert!(adr_answer(&dir, &three, "refuse").status.success());
    assert_eq!(
        next(&dir),
        vec![format!(
            "agent changes the plan: \"{}\" is refused and holds the questions that come after it",
            THREE
        )]
    );
}

// adr: 7327cd58-ac58-40a4-8ef2-00072fb552b2
// adr: 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9
//
// `next` and `answer` read one rule: an owner line is printed only for a
// question `answer` takes now. THREE stands on TWO and sits a tier above
// ONE. While TWO has no decision, THREE is under `not yet` and `answer`
// refuses it. With ONE refused, THREE waits on it too, though it does not
// stand on ONE; TWO stays answerable (same tier as the refused one).
#[test]
fn next_names_an_owner_answer_only_where_answer_takes_the_question() {
    let (dir, path) = setup();
    let p = path.to_str().unwrap();
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", p], &[], "").status.success());
    let (one, two, three) = (id_of(&path, ONE), id_of(&path, TWO), id_of(&path, THREE));
    assert_eq!(
        next(&dir),
        vec![answer_for(&dir, &one, ONE), answer_for(&dir, &two, TWO), waits(THREE, TWO)]
    );
    assert!(!adr_answer(&dir, &three, "A").status.success());

    // ONE refused: THREE waits on ONE as well; TWO is still taken.
    assert!(adr_answer(&dir, &one, "refuse").status.success());
    let lines = next(&dir);
    assert!(lines.contains(&answer_for(&dir, &two, TWO)), "{:?}", lines);
    assert!(
        lines.contains(&format!("not yet: \"{}\" is asked and waits for \"{}\", \"{}\"", THREE, TWO, ONE)),
        "{:?}",
        lines
    );
    assert!(!lines.iter().any(|l| l.contains(&format!("adr answer {}@", three))), "{:?}", lines);

    // TWO answered: THREE still waits, for the refused ONE alone.
    assert!(adr_answer(&dir, &two, "A").status.success());
    let lines = next(&dir);
    assert!(lines.contains(&waits(THREE, ONE)), "{:?}", lines);
    let refused = adr_answer(&dir, &three, "A");
    assert!(!refused.status.success());
    assert!(stderr(&refused).contains("is refused and holds the tiers above it"), "{}", stderr(&refused));
}

// adr: f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
//
// The question beside an answered one is refused: no answer is coming, and
// the line says what the answered one waits for.
#[test]
fn show_names_the_plan_change_an_answered_decision_waits_for() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    const FOUR: &str = "Is the widget gate stay shut?";
    let plan = format!(
        "# Gate\n\n{}{}{}",
        q(ONE, "widget"),
        q(TWO, "widget"),
        q(FOUR, "widget")
    );
    let path = write_record(&dir, REL, &plan);
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    assert!(adr_answer(&dir, &id_of(&path, ONE), "A").status.success());
    assert!(adr_answer(&dir, &id_of(&path, TWO), "refuse").status.success());

    let refused = format!(
        "agent changes the plan: \"{}\" is refused and holds the questions that come after it",
        TWO
    );
    let waits_for_plan = |heading: &str| {
        format!(
            "not yet: \"{}\" is answered and is built once the plan changes, \"{}\" beside it is refused",
            heading, TWO
        )
    };

    // One question beside it is refused and one still waits for its
    // answer: the answer is named, the refused one has its own line.
    let four = id_of(&path, FOUR);
    assert_eq!(
        next(&dir),
        vec![
            answer_for(&dir, &four, FOUR),
            refused.clone(),
            format!(
                "not yet: \"{}\" is answered and is built once \"{}\" is answered",
                ONE, FOUR
            ),
        ]
    );

    assert!(adr_answer(&dir, &four, "A").status.success());
    assert_eq!(
        next(&dir),
        vec![refused, waits_for_plan(ONE), waits_for_plan(FOUR)]
    );
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// A heading added to an approved file, beside four answered questions. The
// refusal is the agent's next move: show prints it whole and names no
// owner command, and check prints it ahead of one line for the file.
#[test]
fn show_prints_the_refusal_of_a_plan_as_the_next_move() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let mut plan = String::from("# Gate\n");
    for n in ["one", "two", "three", "four"] {
        plan.push_str(&format!("\n{}", q(&format!("Is the widget gate {} hold?", n), "widget")));
    }
    let path = write_record(&dir, REL, &plan);
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    for id in extract_ids(&std::fs::read_to_string(&path).unwrap()) {
        assert!(adr_answer(&dir, &id, "A").status.success());
        write_test_comment(&dir, &id);
    }

    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("\n### Is the widget gate five hold?\ndepends: widget\n");
    std::fs::write(&path, text).unwrap();

    let refusal = format!(
        "{}: 5 questions stand together, the most is 4: 4 answered, and \"Is the widget gate five hold?\"; \
agent finds a question among the ones named that assumes the answer of another one and names that one on its depends line, \
or moves it to another file when none does",
        REL
    );
    let five = "Is the widget gate five hold?";
    assert_eq!(
        next(&dir),
        vec![
            format!("plan: {}", refusal),
            format!("plan: {}: \"{}\" has no reader rows for its heading; {}", REL, five, plan_for()),
        ]
    );

    let out = adr(&dir, &["check"], &[], "");
    // The edited file awaits approval again: check prints the refusal as a
    // note (912f4f3b-1213-435a-af0d-7617017fcff4), beside what else approve
    // refuses in it (fe14991d): the planning step the file is at, its new
    // heading not yet read, is one more note.
    assert!(out.status.success());
    let plan_lines: Vec<String> = stdout(&out)
        .lines()
        .filter(|l| l.starts_with("plan: "))
        .map(|l| l.to_string())
        .collect();
    assert_eq!(plan_lines.first(), Some(&format!("plan: {}", refusal)), "{:?}", plan_lines);
    assert!(plan_lines.iter().any(|l| l.contains("has no reader rows for its heading")), "{:?}", plan_lines);
}

// adr: f4f667ad-8714-4e65-897b-c7002e423c28
//
// adr: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
// adr: 3b8da790-ea0c-4ca5-860a-980c7bc5f361
//
// A superseded question stays in its topic file, and show names under it
// the question that took its place. The agent names the decision it
// replaces by its heading, under the heading of the plan; approve turns the
// heading into the identifier and show turns it back.
#[test]
fn show_names_what_replaced_a_superseded_question() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}", q(ONE, "widget")));
    approve_record(&dir, REL);
    let p = path.to_str().unwrap();
    assert!(adr(&dir, &["ask", p], &[], "").status.success());
    let one = id_of(&path, ONE);
    assert!(adr_answer(&dir, &one, "A").status.success());

    // The supersedes line is written with the plan, before approval.
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(&format!("\n### {}\ndepends: widget\nsupersedes: {}\n{}", TWO, ONE, BODY));
    std::fs::write(&path, text).unwrap();
    approve_record(&dir, REL);
    let two = id_of(&path, TWO);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains(&format!("\nsupersedes: {}\nid: {}\n", one, two)),
        "{}",
        text
    );
    let asked = adr(&dir, &["ask", p], &[], "");
    assert!(asked.status.success(), "{}", stderr(&asked));
    assert!(adr_answer(&dir, &two, "A").status.success());

    // What replaced a superseded question is computed from the records:
    // it comes under its flag.
    let shown = stdout(&adr(&dir, &["show", "--all", "--dependents"], &[], ""));
    let expected = format!(
        "[superseded] {one}\n  depends: widget\n  replaced by: {two}\n\
[answered A] {two}\n  depends: widget\n  supersedes: {one}\n",
        one = ONE,
        two = TWO
    );
    assert!(shown.contains(&expected), "{}", shown);

    // A heading that names no current decision: approve refuses it and
    // names the move.
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(&format!("\n### {}\ndepends: widget\nsupersedes: {}\n{}", THREE, ONE, BODY));
    std::fs::write(&path, text).unwrap();
    let approved = adr_owner(&dir, &["approve", p], "");
    assert!(!approved.status.success());
    assert!(
        stderr(&approved).contains(&format!(
            "approve: {}: \"{}\": supersedes \"{}\", which is not a current decision; agent removes it from the supersedes line or names a current decision",
            REL, THREE, ONE
        )),
        "{}",
        stderr(&approved)
    );
}

// adr: f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
//
// Every question answered and tied to a test: show prints no "next" at all.
#[test]
fn show_prints_no_next_when_nothing_is_left() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}", q(ONE, "widget")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    let one = id_of(&path, ONE);
    assert!(adr_answer(&dir, &one, "A").status.success());
    write_test_comment(&dir, &one);

    let out = adr(&dir, &["show"], &[], "");
    assert!(!stdout(&out).lines().any(|l| l == "next"), "{}", stdout(&out));
}

//
// With no topic file and no flag show prints the short listing: each
// current decision as its heading and its answer, and the moves. With a
// topic file it prints the file form: each question with its status, its
// lines and the body of an asked one; --all prints that for every file.
#[test]
fn show_is_short_until_it_is_asked_for_the_whole() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let plan = format!(
        "# Gate\n\n{}{}",
        q(ONE, "widget"),
        q(THREE, &format!("widget, {}=A", ONE))
    );
    let path = write_record(&dir, REL, &plan);
    approve_record(&dir, REL);
    let one = id_of(&path, ONE);
    let p = path.to_str().unwrap();
    // One run asks both questions, THREE (tier 2) as well.
    assert!(adr(&dir, &["ask", p], &[], "").status.success());
    let three = id_of(&path, THREE);
    let version = current_version(&dir, &three);
    assert!(adr_answer(&dir, &one, "A").status.success());
    write_test_comment(&dir, &one);

    let short = stdout(&adr(&dir, &["show"], &[], ""));
    assert_eq!(
        short,
        format!(
            "# Gate ({rel})\n  {one}\n    Yes the widget holds.\nowner runs: adr answer {id}@{version} <letter>  # \"{three}\"\n",
            rel = REL,
            one = ONE,
            three = THREE,
            id = three,
            version = version
        )
    );

    let whole = format!(
        "[approved] Gate ({rel})\n[answered A] {one}\n  depends: widget\n  unrelated: {three}\n\
[asked @{version}] {three}\n  depends: widget, {one}=A\n  Problem: the widget breaks.\n  Standard: plain words.\n  \
- A) Yes the widget holds.\n    Why: reasons.\n    Costs: costs.\n  \
- B) No the widget does not hold.\n    Why: other reasons.\n    Costs: other costs.\n\
owner runs: adr answer {id}@{version} <letter>  # \"{three}\"\n",
        rel = REL,
        one = ONE,
        three = THREE,
        id = three,
        version = version
    );
    assert_eq!(stdout(&adr(&dir, &["show", p], &[], "")), whole);
    assert!(stdout(&adr(&dir, &["show", "--all"], &[], "")).starts_with(&whole));
}

// adr: cd83e277-ebf0-4458-a77a-c526ab4cfeca
// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// `show` and `answer` read one rule: a question is answerable when its
// latest ledger row is `asked` and holds the hash of its text now. Asked
// and unedited, `next` names the owner's answer and the answer succeeds;
// asked and then edited, `next` names the agent's `ask` and `answer` on
// the version the owner read refuses.
#[test]
fn show_and_answer_agree_on_whether_the_asked_text_stands() {
    let (dir, path) = setup();
    let p = path.to_str().unwrap();
    approve_record(&dir, REL);
    let two = id_of(&path, TWO);
    assert!(adr(&dir, &["ask", p], &[], "").status.success());
    let one = id_of(&path, ONE);
    assert_eq!(
        next(&dir),
        vec![answer_for(&dir, &one, ONE), answer_for(&dir, &two, TWO), waits(THREE, TWO)]
    );

    // Edited after it was asked: `next` hands the question back to the
    // agent, and the version the owner read is refused.
    let old_version = current_version(&dir, &two);
    let text = std::fs::read_to_string(&path).unwrap();
    let at = text.find(&format!("### {}\n", TWO)).unwrap();
    let edited = format!(
        "{}{}",
        &text[..at],
        text[at..].replacen("the widget breaks.", "the widget breaks in use.", 1)
    );
    std::fs::write(&path, edited).unwrap();
    approve_record(&dir, REL);
    assert_eq!(
        next(&dir),
        vec![ask_for(&[TWO]), answer_for(&dir, &one, ONE), waits(THREE, TWO)]
    );
    let refused = adr_owner(&dir, &["answer", &format!("{}@{}", two, old_version), "A"], "");
    assert!(!refused.status.success());
    // The tool knows the agent's `ask` is the move: the refusal names it and
    // gives the owner nothing to run.
    assert_eq!(
        stderr(&refused).trim_end(),
        format!("answer: \"{}\" is edited since it was asked; {}", TWO, ask_for(&[TWO]))
    );

    // The unedited question is still answerable.
    assert!(adr_answer(&dir, &one, "A").status.success());
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// `answer` on a question that has a body and was never asked: show prints
// no version for it, so the refusal names the agent's `ask`, not show. A
// question re-asked after an edit gets the owner's line whole, with the
// version the tool now holds.
#[test]
fn answer_on_a_question_never_asked_names_the_ask() {
    let (dir, path) = setup();
    approve_record(&dir, REL);
    let one = id_of(&path, ONE);
    let out = adr_owner(&dir, &["answer", &format!("{}@12345678", one), "A"], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        format!("answer: \"{}\" is not asked; {}", ONE, ask_for(&[ONE]))
    );

    // Asked, edited and asked again: the version read is old, the tool holds
    // the current one and prints the owner's command with it.
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    let old = current_version(&dir, &one);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("the widget breaks.", "the widget breaks in use.", 1)).unwrap();
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    let out = adr_owner(&dir, &["answer", &format!("{}@{}", one, old), "A"], "");
    assert_eq!(
        stderr(&out).trim_end(),
        format!(
            "answer: \"{}\" has moved since that version was read; show prints it as it is now; owner runs: adr show {}",
            ONE,
            REL
        )
    );
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// `show <file>` on a file approve does not take says so and names the
// files it takes, instead of sending the agent to fix a topic file; the
// terms file is one approve takes, so show prints it.
#[test]
fn show_refuses_a_file_approve_does_not_take() {
    let (dir, _path) = setup();
    approve_record(&dir, REL);
    std::fs::write(dir.join("README.md"), "# Readme\n").unwrap();
    let out = adr(&dir, &["show", "README.md"], &[], "");
    assert!(!out.status.success());
    assert_eq!(
        stderr(&out).trim_end(),
        "show: README.md is not a file approve takes; agent names a topic file under adrdom/decisions, adrdom/terms.md or a blame file under adrdom/blame"
    );
    let terms = adr(&dir, &["show", "adrdom/terms.md"], &[], "");
    assert!(terms.status.success(), "{}", stderr(&terms));
    assert!(stdout(&terms).starts_with("[approved] adrdom/terms.md\n"), "{}", stdout(&terms));
}
