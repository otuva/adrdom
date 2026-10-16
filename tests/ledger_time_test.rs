// Decision aefa7f3c-70c3-412e-8d73-16ff9e2b703a: "a command writes its rows
// with one time, the later of the clock and one second after the last row
// of the ledger." These tests exercise `ledger::write_time` through the
// real commands (`ask`, `answer`) rather than calling it directly, so a
// regression in any call site -- not just the function itself -- would
// show up here too.

mod common;

use common::*;

const ONE_QUESTION: &str = r#"# Example

### Is this pass?
depends: pass

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.
"#;

// Two tier-1 questions, neither depending on the other -- both become
// candidates on the same `adr ask` run, so that run writes two `asked` rows
// in one command, nothing shared between them but the run itself.
const TWO_QUESTIONS: &str = r#"# Example

### Is this first one pass?
depends: pass

Problem: whether the first one passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.

### Is this second one pass?
depends: pass

Problem: whether the second one passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.
"#;

/// A six-field ledger line, formatted the same way `ledger::row_line`
/// would (mirrors the helper `ledger_order_test.rs` already uses for the
/// same reason: pin the exact bytes written by hand).
fn line(time: &str, event: &str, id: &str, hash: &str, detail: &str) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\n",
        time,
        event,
        id,
        hash,
        detail,
        adr::version::VERSION
    )
}

/// The `time` field of the last (by file position -- real commands only
/// ever append, so file order is also true write order here) row matching
/// `needle` (e.g. `"\tasked\t"`) in the ledger at `dir`.
fn last_row_time(dir: &std::path::Path, needle: &str) -> String {
    let content = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let row = content
        .lines()
        .filter(|l| l.contains(needle))
        .next_back()
        .unwrap_or_else(|| panic!("no row matching {} in: {}", needle, content));
    row.split('\t').next().unwrap().to_string()
}

// adr: aefa7f3c-70c3-412e-8d73-16ff9e2b703a
//
// A ledger whose last row already carries a time an hour ahead of the
// clock: the next command to write a row (here, `adr ask`) must not write
// the clock's own time -- it would sit *before* that future row -- but one
// second past it.
#[test]
fn a_command_writes_one_second_past_a_future_last_row() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "pass");

    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");

    // A throwaway row (never read by anything `ask` checks) whose only job
    // is to sit at a time far in the future, so it is the ledger's latest
    // row regardless of the clock.
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let mut content = std::fs::read_to_string(&ledger_path).unwrap();
    content.push_str(&line(
        "2099-01-01T00:00:00Z",
        "banned",
        "-",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "",
    ));
    std::fs::write(&ledger_path, &content).unwrap();

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));

    assert_eq!(
        last_row_time(&dir, "\tasked\t"),
        "2099-01-01T00:00:01Z",
        "the asked row must sit exactly one second past the future row, not at the clock's own time"
    );
}

// adr: aefa7f3c-70c3-412e-8d73-16ff9e2b703a
//
// Two commands, run one after the other through the real harness (`ask`
// then, as the owner, `answer`): the second's rows carry a later time than
// the first's, whether or not the two happened to run inside the same
// wall-clock second.
#[test]
fn two_commands_run_back_to_back_write_different_times() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "pass");

    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");

    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let asked_time = last_row_time(&dir, "\tasked\t");

    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    let answer_out = adr_answer(&dir, &id, "A");
    assert!(
        answer_out.status.success(),
        "answer failed: {}",
        stderr(&answer_out)
    );
    let answered_time = last_row_time(&dir, "\tanswered\t");

    assert_ne!(
        asked_time, answered_time,
        "the second command's row must not share the first command's time"
    );
    assert!(
        answered_time.as_str() > asked_time.as_str(),
        "the second command's row must be later: asked {}, answered {}",
        asked_time,
        answered_time
    );
}

// adr: aefa7f3c-70c3-412e-8d73-16ff9e2b703a
//
// One `adr ask` run, asking two independent tier-1 questions at once: both
// `asked` rows it writes carry the exact same time -- the one value taken
// once for the whole run, not a fresh one per row.
#[test]
fn one_commands_rows_share_one_time() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "pass");

    let record = write_record(&dir, "adrdom/decisions/example.md", TWO_QUESTIONS);
    approve_record(&dir, "adrdom/decisions/example.md");

    let out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "ask failed: {}", stderr(&out));

    let content = std::fs::read_to_string(&dir.join("adrdom/state/ledger.tsv")).unwrap();
    let times: Vec<&str> = content
        .lines()
        .filter(|l| l.contains("\tasked\t"))
        .map(|l| l.split('\t').next().unwrap())
        .collect();
    assert_eq!(
        times.len(),
        2,
        "both questions should have been asked: {}",
        content
    );
    assert_eq!(
        times[0], times[1],
        "both asked rows from the one run must carry the same time"
    );
}
