// What a command that writes prints: one line per ledger row it appends,
// each the line `out::Event::from_row` gives that row, or the finding that
// stopped it on standard error and nothing on standard output.
mod common;

use common::*;
use std::path::Path;
use std::process::Output;

const REL: &str = "adrdom/decisions/example.md";

const RECORD: &str = r#"# Example

### 1.1 What color is the widget?
depends: widget

Problem: the widget's color is undecided.

Standard: plain words.

- A) The widget is red.
  Why: reasons.
  Costs: costs.
- B) The widget is blue.
  Why: other reasons.
  Costs: other costs.

### 1.2 Does the widget's paint match its color?
depends: 1.1 What color is the widget?=A

Problem: the paint might not match.

Standard: plain words.

- A) Yes.
  Why: reasons.
  Costs: costs.
- B) No.
  Why: other reasons.
  Costs: other costs.
"#;

fn ledger_text(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default()
}

fn ledger_rows(dir: &Path) -> Vec<adr::ledger::Row> {
    adr::ledger::parse(&ledger_text(dir)).rows
}

// Runs `act` and holds what it printed to the rows it appended: one line
// per row, each the line of that row's event (an answer's line carries the
// picked option's sentence after it). The place of a row in the ledger
// carries no meaning, so lines and rows are matched as sets. Returns the
// output.
fn prints_one_line_per_row(dir: &Path, act: impl FnOnce() -> Output) -> Output {
    // Names from before the act (a dropped question leaves the file) and
    // after it (approve gives a heading its identifier).
    let before = ledger_text(dir);
    let mut names = adr::actions::event_names(dir);
    let out = act();
    assert!(out.status.success(), "{}", stderr(&out));
    names.extend(adr::actions::event_names(dir));
    let after = ledger_text(dir);
    assert!(after.starts_with(&before), "the ledger is append-only");
    let appended = adr::ledger::parse(&after[before.len()..]).rows;
    let printed = stdout(&out);
    let mut lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), appended.len(), "one line per row: {}\n{:?}", printed, appended);
    for row in &appended {
        let want = adr::out::Event::from_row(row, &names).expect("every row an act writes is an event").text();
        let at = lines
            .iter()
            .position(|l| *l == want || l.starts_with(&format!("{}: ", want)))
            .unwrap_or_else(|| panic!("no line for {:?} in {}", row, printed));
        lines.remove(at);
    }
    out
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// "What does a command that writes print?" A: the event it wrote, one line
// each, or the finding that stopped it, and nothing else. Walks approve,
// ask, answer (with a drop), ban and unban, and a stop of each kind.
#[test]
fn every_writing_command_prints_one_line_per_row_it_writes() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, REL, RECORD);
    let path = record.to_str().unwrap().to_string();

    // approve: one row per question, the file row and the prose row.
    let out = prints_one_line_per_row(&dir, || adr_owner(&dir, &["approve", &path], ""));
    assert_eq!(stdout(&out).lines().count(), 4, "{}", stdout(&out));

    // Approve again: no row, nothing printed.
    let out = prints_one_line_per_row(&dir, || adr_owner(&dir, &["approve", &path], ""));
    assert_eq!(stdout(&out), "");

    // ask: an asked row per question and the assumes row.
    let out = prints_one_line_per_row(&dir, || adr(&dir, &["ask", &path], &[], ""));
    assert_eq!(stdout(&out).lines().count(), 3, "{}", stdout(&out));

    // ask with nothing left to ask: no row, nothing printed.
    let out = prints_one_line_per_row(&dir, || adr(&dir, &["ask", &path], &[], ""));
    assert_eq!(stdout(&out), "");

    // answer B: the answered row and the dropped row of 1.2, whose set lacks B.
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    let q1 = ids[0].clone();
    let out = prints_one_line_per_row(&dir, || adr_answer(&dir, &q1, "B"));
    assert_eq!(
        stdout(&out),
        "answered: \"1.1 What color is the widget?\" B: The widget is blue.\ndropped: \"1.2 Does the widget's paint match its color?\"\n"
    );

    // ban and unban: one row each.
    let out = prints_one_line_per_row(&dir, || adr(&dir, &["ban"], &[], "zebra\n"));
    assert_eq!(stdout(&out), "banned: a word\n");
    let out = prints_one_line_per_row(&dir, || adr_owner(&dir, &["unban"], "zebra\n"));
    assert_eq!(stdout(&out), "unbanned: a word\n");

    // A stop: nothing on standard output, the finding with its move on
    // standard error, and no row.
    let before = ledger_rows(&dir).len();
    let out = adr_answer(&dir, &q1, "A");
    assert!(!out.status.success());
    assert_eq!(stdout(&out), "", "{}", stdout(&out));
    assert!(!stderr(&out).is_empty());
    assert_eq!(ledger_rows(&dir).len(), before);
}

// The lines a writing command no longer prints: no tier, no `next` and no
// move line, whatever it wrote.
#[test]
fn a_writing_command_prints_no_tier_and_no_move() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, REL, RECORD);
    let path = record.to_str().unwrap().to_string();
    let approved = adr_owner(&dir, &["approve", &path], "");
    let asked = adr(&dir, &["ask", &path], &[], "");
    for out in [&approved, &asked] {
        assert!(out.status.success(), "{}", stderr(out));
        let text = stdout(out);
        for word in ["tier", "next", "agent ", "owner runs", "not yet"] {
            assert!(!text.contains(word), "{:?} in {}", word, text);
        }
    }
}
