// A finding carries the rule it breaks (src/rule.rs), the rule carries the
// decisions it comes from, and the line a command prints names what was
// found and the move, from the tool's fixed set of acts.
mod common;

use adr::rule;
use common::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The tags the lines of findings opened with before a finding carried a
/// rule of its own.
const OLD_TAGS: [&str; 14] = [
    "parse",
    "plan",
    "plan_frozen",
    "tells",
    "answers",
    "ids",
    "terms",
    "blame",
    "banned",
    "ledger",
    "reader",
    "install",
    "tested",
    "untested",
];

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Whether `s` holds an identifier written as eight, four, four, four and
/// twelve hexadecimal digits.
fn holds_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    let shape = [8usize, 4, 4, 4, 12];
    let total = 36;
    if b.len() < total {
        return false;
    }
    (0..=b.len() - total).any(|start| {
        let mut at = start;
        for (i, len) in shape.iter().enumerate() {
            if i > 0 {
                if b[at] != b'-' {
                    return false;
                }
                at += 1;
            }
            if !b[at..at + len].iter().all(|c| c.is_ascii_hexdigit()) {
                return false;
            }
            at += len;
        }
        true
    })
}

//
// Every rule a finding can carry comes from at least one decision, and every
// decision it names is a current decision of this repository's records.
// Rules with no decision yet fail here until their decision is answered.
#[test]
fn every_rule_comes_from_current_decisions() {
    // adr: 24bdcbde-d9f4-45a0-8c3d-37e53569e950
    let root = manifest();
    let records = adr::record::load_all(&root);
    let led = adr::ledger::read(&root.join(adr::repo::LEDGER_PATH));
    let held: BTreeSet<&str> = records
        .iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.as_deref()))
        .collect();

    let without: Vec<&str> = rule::ALL
        .iter()
        .filter(|r| r.decisions.is_empty())
        .map(|r| r.name)
        .collect();
    let mut not_current: Vec<String> = Vec::new();
    for r in rule::ALL {
        for id in r.decisions {
            if !held.contains(id) || !adr::state::is_current(&led, id) {
                not_current.push(format!("{} names {}", r.name, id));
            }
        }
    }
    assert!(
        without.is_empty() && not_current.is_empty(),
        "rules with no decision: {:?}\ndecisions that are not current: {:?}",
        without,
        not_current
    );
}

//
// Every constant of src/rule.rs is in `rule::ALL`, and a finding is built
// from a rule constant alone: no call that builds one passes anything else.
#[test]
fn every_finding_is_built_from_a_rule_constant() {
    // adr: 24bdcbde-d9f4-45a0-8c3d-37e53569e950
    let source = std::fs::read_to_string(manifest().join("src/rule.rs")).unwrap();
    let declared: BTreeSet<String> = source
        .lines()
        .filter_map(|l| l.strip_prefix("pub const "))
        .filter(|l| l.contains(": Rule ="))
        .map(|l| l.split(':').next().unwrap().to_string())
        .collect();
    let listed: BTreeSet<String> = rule::ALL.iter().map(|r| r.name.to_uppercase()).collect();
    assert_eq!(
        declared, listed,
        "a constant of src/rule.rs missing from ALL, or the reverse"
    );
    assert_eq!(
        rule::ALL.len(),
        declared.len(),
        "a rule listed twice in ALL"
    );

    let mut bad = Vec::new();
    for (rel, text) in source_files() {
        if rel == "src/rule.rs" {
            continue;
        }
        for (n, line) in code_lines(&text) {
            if let Some(at) = line.find("Finding::new(") {
                let arg = line[at + "Finding::new(".len()..].trim_start();
                let ok = ["&rule::", "&crate::rule::", "said.rule"]
                    .iter()
                    .any(|p| arg.starts_with(p));
                // The call split over lines names its rule on the next one.
                if !ok && !arg.is_empty() {
                    bad.push(format!("{}:{}: {}", rel, n, line.trim()));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "a finding built from something but a rule constant:\n{}",
        bad.join("\n")
    );
}

/// Every heading of the questions in this repository's own records.
fn own_headings() -> Vec<String> {
    adr::record::load_all(&manifest())
        .iter()
        .flat_map(|rf| rf.sections.iter().map(|s| s.heading_text.clone()))
        .collect()
}

fn question(heading: &str, depends: &str, option: &str) -> String {
    format!(
        "### {}\ndepends: {}\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n\
- A) {}\n  Why: reasons.\n  Costs: costs.\n\
- B) No the widget does not hold.\n  Why: other reasons.\n  Costs: other costs.\n",
        heading, depends, option
    )
}

/// The openings of a move clause: who takes it, or a clause that follows
/// one by the same actor.
const OPENINGS: [&str; 4] = ["agent ", "builder ", "owner ", "then "];

/// The lines of `out` that are findings: the ones holding a move after `; `.
fn finding_lines(out: &std::process::Output) -> Vec<String> {
    let text = stdout(out) + &stderr(out);
    text.lines()
        .filter(|l| {
            l.split("; ").skip(1).any(|p| {
                OPENINGS[..3].iter().any(|o| p.starts_with(o)) || p.starts_with(adr::msg::NOT_YET)
            })
        })
        .map(str::to_string)
        .collect()
}

/// Every clause of `mv` opens with who takes it, or `then`, and goes on with
/// one act of the tool's fixed set; a clause nobody takes yet opens with
/// `not yet: `.
fn assert_move_is_acts(command: &str, mv: &str, line: &str) {
    let mut at = 0;
    let mut clauses = 0;
    while at < mv.len() {
        let rest = &mv[at..];
        let clause_starts = at == 0 || mv[..at].ends_with(", ");
        if clause_starts && rest.starts_with(adr::msg::NOT_YET) {
            clauses += 1;
        } else if clause_starts {
            if let Some(o) = OPENINGS.iter().find(|o| rest.starts_with(**o)) {
                let act = &rest[o.len()..];
                assert!(
                    adr::msg::ACTS.iter().any(|a| act.starts_with(a)),
                    "{}: a clause opens with no act of the set: {:?} in {}",
                    command,
                    rest,
                    line
                );
                clauses += 1;
            }
        }
        at += rest.chars().next().map_or(1, char::len_utf8);
    }
    assert!(
        clauses > 0,
        "{}: no clause in the move of {}",
        command,
        line
    );
    assert!(
        OPENINGS.iter().any(|o| mv.starts_with(o)) || mv.starts_with(adr::msg::NOT_YET),
        "{}: the move opens with no clause: {}",
        command,
        line
    );
}

fn assert_finding_shape(command: &str, line: &str, headings: &[String]) {
    let parts: Vec<&str> = line.split("; ").collect();
    assert_eq!(
        parts.len(),
        2,
        "{}: found; move expected, nothing on what the rule protects: {}",
        command,
        line
    );
    let mv = parts[1];
    assert_move_is_acts(command, mv, line);
    if let Some((tag, _)) = line.split_once(": ") {
        assert!(
            !OLD_TAGS.contains(&tag),
            "{}: the line opens with a category tag: {}",
            command,
            line
        );
    }
    // A move that runs a command names the question it acts on by identifier
    // (`adr answer <id>@<version> <letter>`): the found part carries none.
    let before_move = parts[..parts.len() - 1].join("; ");
    assert!(
        !holds_uuid(&before_move),
        "{}: an identifier before the move: {}",
        command,
        line
    );
    for h in headings {
        assert!(
            !line.contains(h.as_str()),
            "{}: a heading of this repository's decisions on the line {}",
            command,
            line
        );
    }
}

fn ready_repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    dir
}

fn first_unanswered_waits(dir: &Path) -> std::process::Output {
    let rel = "adrdom/decisions/chain.md";
    let text = format!(
        "# Chain\n\n{}\n{}",
        question("Is the widget one hold?", "widget", "Yes the widget holds."),
        question(
            "Is the widget two hold?",
            "widget, Is the widget one hold?=A",
            "Yes the widget holds."
        )
    );
    let path = write_record(dir, rel, &text);
    approve_record(dir, rel);
    let ids = extract_ids(&std::fs::read_to_string(&path).unwrap());
    adr_answer(dir, &ids[1], "A")
}

//
// The line a command prints for a finding is one line of two parts: what
// was found and the move, every clause of it one act of the tool's fixed
// set; nothing on what the rule protects. It opens with no category tag and
// names neither an identifier nor a heading of this repository's own
// decisions. check, show, plan, approve and answer print it the same
// way.
#[test]
fn a_finding_names_what_it_found_and_the_move() {
    // adr: 50b31713-4fea-4aa3-842a-79c2750660ba
    let dir = ready_repo();
    let headings = own_headings();

    let rel = "adrdom/decisions/bad.md";
    let long_option = "Yes the widget holds and every one of the widgets in the whole of the building is held by it for as long as anyone looks at them closely.";
    let bad = format!(
        "# Bad\n\n### Is the widget hold?\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n\
- A) {}\n  Why: reasons.\n  Costs: costs.\n- B) No the widget does not hold.\n  Why: other reasons.\n  Costs: other costs.\n",
        long_option
    );
    let path = write_record(&dir, rel, &bad);
    let p = path.to_str().unwrap();

    let mut seen: Vec<(&str, usize)> = Vec::new();
    let mut take = |command: &'static str, out: std::process::Output| {
        let lines = finding_lines(&out);
        for l in &lines {
            assert_finding_shape(command, l, &headings);
        }
        seen.push((command, lines.len()));
    };
    take("check", adr(&dir, &["check"], &[], ""));
    take("show", adr(&dir, &["show", rel], &[], ""));
    take("plan", adr(&dir, &["plan", p], &[], ""));
    take("approve", adr_owner(&dir, &["approve", p], ""));
    take("answer", first_unanswered_waits(&dir));

    let empty: Vec<&str> = seen
        .iter()
        .filter(|(_, n)| *n == 0)
        .map(|(c, _)| *c)
        .collect();
    assert!(
        empty.is_empty(),
        "the fixture raised no finding under: {:?}",
        empty
    );
}
