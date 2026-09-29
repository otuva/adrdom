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
const ONE: &str = "Does the widget gate open?";
const TWO: &str = "Does the widget gate close?";
const THREE: &str = "Does the widget gate lock once it closes?";

const BODY: &str = "\nProblem: the widget breaks.\n\nStandard: plain words.\n\n\
- A) Yes the widget holds.\n  Why: reasons.\n  Costs: costs.\n\
- B) No the widget does not hold.\n  Why: other reasons.\n  Costs: other costs.\n";

fn setup() -> (PathBuf, PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let plan = format!(
        "# Gate\n\n### {}\ndepends: widget\n\n### {}\ndepends: widget\n\n### {}\ndepends: widget, {}\n",
        ONE, TWO, THREE, TWO
    );
    let path = write_record(&dir, REL, &plan);
    (dir, path)
}

/// The lines `show` prints under "next" for the one topic file.
fn next(dir: &Path) -> Vec<String> {
    let out = adr(dir, &["show"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    stdout(&out)
        .lines()
        .skip_while(|l| *l != "next")
        .skip(1)
        .take_while(|l| l.starts_with("  "))
        .map(|l| l.trim().to_string())
        .collect()
}

/// Puts a body under `heading`, after its id line.
fn write_body(path: &Path, heading: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    let at = text.find(&format!("### {}\n", heading)).unwrap();
    let id_line = at + text[at..].find("\nid: ").unwrap() + 1;
    let end = id_line + text[id_line..].find('\n').unwrap() + 1;
    std::fs::write(path, format!("{}{}{}", &text[..end], BODY, &text[end..])).unwrap();
}

fn id_of(path: &Path, heading: &str) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    let at = text.find(&format!("### {}\n", heading)).unwrap();
    extract_ids(&text[at..])[0].clone()
}

fn body_of(heading: &str) -> String {
    format!("agent writes the body of \"{}\"", heading)
}

fn ask_for(headings: &[&str]) -> String {
    let names: Vec<String> = headings.iter().map(|h| format!("\"{}\"", h)).collect();
    format!("agent runs adr ask {} for {}", REL, names.join(", "))
}

fn answer_for(dir: &Path, id: &str, heading: &str) -> String {
    format!(
        "owner runs: adr answer {} <letter> for \"{}\"",
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

fn later(heading: &str) -> String {
    format!("not yet, after the questions named above: \"{}\"", heading)
}

// adr: f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
// adr: fa90692a-150e-4a89-91e2-1d575cb59d0f
// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
#[test]
fn show_names_the_next_move_at_every_step() {
    let (dir, path) = setup();
    let p = path.to_str().unwrap();

    // A plan that waits for its approval: nothing is written under it.
    assert_eq!(next(&dir), vec![format!("owner runs: adr approve {}", REL)]);

    approve_record(&dir, REL);
    assert_eq!(next(&dir), vec![body_of(ONE), body_of(TWO), later(THREE)]);

    write_body(&path, ONE);
    assert_eq!(next(&dir), vec![body_of(TWO), ask_for(&[ONE]), later(THREE)]);

    // THREE names TWO on its depends line, and TWO now has its options:
    // the letters are written before TWO is asked.
    write_body(&path, TWO);
    let set = format!(
        "agent writes =A, =A|B or =* on the depends line of \"{}\" where it names \"{}\"",
        THREE, TWO
    );
    assert_eq!(next(&dir), vec![set, ask_for(&[ONE, TWO]), later(THREE)]);

    let two = id_of(&path, TWO);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace(&format!(", {}\n", two), &format!(", {}=A\n", two))).unwrap();
    assert_eq!(next(&dir), vec![ask_for(&[ONE, TWO]), later(THREE)]);

    let asked = adr(&dir, &["ask", p], &[], "");
    assert!(asked.status.success(), "{}", stderr(&asked));
    let one = id_of(&path, ONE);
    let waiting = vec![
        answer_for(&dir, &one, ONE),
        answer_for(&dir, &two, TWO),
        later(THREE),
    ];
    assert_eq!(next(&dir), waiting);

    // `ask` with nothing to ask names the same moves.
    let again = adr(&dir, &["ask", p], &[], "");
    assert!(again.status.success());
    let said = stdout(&again);
    assert!(said.contains("nothing to ask in"), "{}", said);
    for line in &waiting {
        assert!(said.contains(&format!("  {}", line)), "{}", said);
    }

    // A body edited after it was asked: the owner's command would be
    // refused, so the move is the agent's, to ask it again.
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("the widget breaks.", "the widget breaks in use.", 1)).unwrap();
    assert_eq!(
        next(&dir),
        vec![ask_for(&[ONE]), answer_for(&dir, &two, TWO), later(THREE)]
    );
    // Its line in the listing carries no version for the owner to use.
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert!(shown.contains(&format!("  [open] {}\n", ONE)), "{}", shown);
    assert!(adr(&dir, &["ask", p], &[], "").status.success());
    assert_eq!(
        next(&dir),
        vec![answer_for(&dir, &one, ONE), answer_for(&dir, &two, TWO), later(THREE)]
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
            later(THREE),
        ]
    );

    // Both answered: they are built first, then the next body is written.
    assert!(adr_answer(&dir, &two, "A").status.success());
    assert_eq!(
        next(&dir),
        vec![build_of(ONE, &one), build_of(TWO, &two), body_of(THREE)]
    );

    write_test_comment(&dir, &one);
    write_test_comment(&dir, &two);
    assert_eq!(next(&dir), vec![body_of(THREE)]);

    write_body(&path, THREE);
    assert_eq!(next(&dir), vec![ask_for(&[THREE])]);
    assert!(adr(&dir, &["ask", p], &[], "").status.success());
    let three = id_of(&path, THREE);
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

// adr: f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
//
// The question beside an answered one is refused: no answer is coming, and
// the line says what the answered one waits for.
#[test]
fn show_names_the_plan_change_an_answered_decision_waits_for() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    const FOUR: &str = "Does the widget gate stay shut?";
    let plan = format!(
        "# Gate\n\n### {}\ndepends: widget\n\n### {}\ndepends: widget\n\n### {}\ndepends: widget\n",
        ONE, TWO, FOUR
    );
    let path = write_record(&dir, REL, &plan);
    approve_record(&dir, REL);
    write_body(&path, ONE);
    write_body(&path, TWO);
    write_body(&path, FOUR);
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
// adr: 15d82e53-e344-4347-b639-3bf6eb9ef1b4
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
        plan.push_str(&format!("\n### Does the widget gate {} hold?\ndepends: widget\n", n));
    }
    let path = write_record(&dir, REL, &plan);
    approve_record(&dir, REL);
    for n in ["one", "two", "three", "four"] {
        write_body(&path, &format!("Does the widget gate {} hold?", n));
    }
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    for id in extract_ids(&std::fs::read_to_string(&path).unwrap()) {
        assert!(adr_answer(&dir, &id, "A").status.success());
        write_test_comment(&dir, &id);
    }

    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("\n### Does the widget gate five hold?\ndepends: widget\n");
    std::fs::write(&path, text).unwrap();

    let refusal = format!(
        "{}: 5 questions stand together, the most is 4: 4 answered, and \"Does the widget gate five hold?\"; \
agent finds a question among the ones named that assumes the answer of another one and names that one on its depends line, \
or moves it to another file when none does",
        REL
    );
    assert_eq!(next(&dir), vec![format!("agent clears: {}", refusal)]);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let plan_lines: Vec<String> = stdout(&out)
        .lines()
        .filter(|l| l.starts_with("plan: ") && !l.contains("awaiting approval"))
        .map(|l| l.to_string())
        .collect();
    assert_eq!(
        plan_lines,
        vec![
            format!("plan: {}", refusal),
            format!(
                "plan: {}: 4 bodies under unapproved headings; owner runs: adr approve {}",
                REL, REL
            ),
        ]
    );
}

// adr: f4f667ad-8714-4e65-897b-c7002e423c28
// adr: 1b181f10-97f4-4bbe-8939-d5842192126c
//
// A superseded question stays in its topic file, and show names under it
// the question that took its place.
#[test]
fn show_names_what_replaced_a_superseded_question() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let path = write_record(&dir, REL, &format!("# Gate\n\n### {}\ndepends: widget\n", ONE));
    approve_record(&dir, REL);
    write_body(&path, ONE);
    let p = path.to_str().unwrap();
    assert!(adr(&dir, &["ask", p], &[], "").status.success());
    let one = id_of(&path, ONE);
    assert!(adr_answer(&dir, &one, "A").status.success());

    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(&format!("\n### {}\ndepends: widget\n", TWO));
    std::fs::write(&path, text).unwrap();
    approve_record(&dir, REL);
    let text = std::fs::read_to_string(&path).unwrap();
    let two = id_of(&path, TWO);
    let id_line = format!("id: {}\n", two);
    std::fs::write(
        &path,
        text.replace(&id_line, &format!("{}supersedes: {}\n{}", id_line, one, BODY)),
    )
    .unwrap();
    let asked = adr(&dir, &["ask", p], &[], "");
    assert!(asked.status.success(), "{}", stderr(&asked));
    assert!(adr_answer(&dir, &two, "A").status.success());

    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    let expected = format!(
        "  [superseded] {}\n    stands on: widget\n    replaced by: {}\n",
        ONE, TWO
    );
    assert!(shown.contains(&expected), "{}", shown);
}

// adr: f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
//
// Every question answered and tied to a test: show prints no "next" at all.
#[test]
fn show_prints_no_next_when_nothing_is_left() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let path = write_record(&dir, REL, &format!("# Gate\n\n### {}\ndepends: widget\n", ONE));
    approve_record(&dir, REL);
    write_body(&path, ONE);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    let one = id_of(&path, ONE);
    assert!(adr_answer(&dir, &one, "A").status.success());
    write_test_comment(&dir, &one);

    let out = adr(&dir, &["show"], &[], "");
    assert!(!stdout(&out).lines().any(|l| l == "next"), "{}", stdout(&out));
}
