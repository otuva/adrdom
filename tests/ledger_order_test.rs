// Decision 385a57d5-5f76-454a-ae5d-9b0d6f5c1492: "the latest row is the
// one with the latest time, and the place of a row in the file carries no
// meaning." These tests build ledger.tsv files by hand (never through
// `adr`'s own commands, which always append in real time order) so a
// row's file position can be pinned independently of its time -- the only
// way to actually exercise this decision.

mod common;

use common::*;
use adr::record;

const OLD_CONTENT: &str = r#"# Example

### 1.1 Does this pass, old wording?
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

const NEW_CONTENT: &str = r#"# Example

### 1.1 Does this pass, new wording?
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

/// A six-field ledger line, formatted the same way `ledger::row_line`
/// would, for a caller that wants to pin the exact bytes written (not go
/// through `append_ledger_row`, which always appends at the end).
fn line(time: &str, event: &str, id: &str, hash: &str, detail: &str) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\n",
        time, event, id, hash, detail, adr::version::VERSION
    )
}

// adr: 385a57d5-5f76-454a-ae5d-9b0d6f5c1492
//
// Two `approved` rows for the same path: the row with the later time is
// written earlier in the file. `state::plan_approved_hash` (via
// `Ledger::latest_for_path`) must still resolve to the later-time row --
// the one matching what's actually on disk now -- so `check` reports no
// "body under unapproved heading" failure. Read literally by file
// position, the physically-last (but chronologically older) row would win
// instead, and the file's one real section would come up unapproved.
#[test]
fn later_time_approved_row_wins_even_when_it_sits_earlier_in_the_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let rel = "adrdom/decisions/example.md";
    let path = write_record(&dir, rel, OLD_CONTENT);
    let old_hash = record::plan_hash(&record::load(&path).unwrap());

    // The content actually on disk from here on is NEW_CONTENT: this is
    // what `check` hashes "now" and compares the ledger against.
    std::fs::write(&path, NEW_CONTENT).unwrap();
    let new_rf = record::load(&path).unwrap();
    let new_hash = record::plan_hash(&new_rf);
    assert_ne!(old_hash, new_hash, "the two contents must hash differently");

    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let mut content = std::fs::read_to_string(&ledger_path).unwrap_or_default();
    // Physically first in the file, but the later time.
    content.push_str(&line("2024-01-01T00:00:02Z", "approved", "-", &new_hash, rel));
    // Physically second (last in the file), but the earlier time.
    content.push_str(&line("2024-01-01T00:00:01Z", "approved", "-", &old_hash, rel));
    std::fs::write(&ledger_path, &content).unwrap();

    // Context approved too, so only the plan-hash comparison this test
    // cares about is left open.
    let ctx_hash = record::context_hash(&new_rf);
    append_ledger_row(&dir, "context", "-", &ctx_hash, rel);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains("body under unapproved heading"),
        "the later-time approved row must win over the row that only sits later in the file: {}",
        text
    );
}

// adr: 385a57d5-5f76-454a-ae5d-9b0d6f5c1492
//
// The exact same rows, in two different physical file orders, must give
// the same `adr check` and the same `adr show`: nothing about the report
// may depend on where in the file a row happens to sit, only on the rows'
// times (and, for a tie, their own text).
#[test]
fn same_rows_different_file_order_give_the_same_check_and_show_output() {
    let rows = vec![
        line("2024-01-01T00:00:01Z", "approved", "-", "1111111111111111111111111111111111111111111111111111111111111111", "adrdom/decisions/x.md"),
        line("2024-01-01T00:00:05Z", "approved", "-", "2222222222222222222222222222222222222222222222222222222222222222", "adrdom/decisions/x.md"),
        line("2024-01-01T00:00:01Z", "banned", "-", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "nonsense1"),
        line("2024-01-01T00:00:03Z", "unbanned", "-", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "nonsense1"),
        line("2024-01-01T00:00:05Z", "asked", "33333333-3333-3333-3333-333333333333", "4444444444444444444444444444444444444444444444444444444444444444", "Some heading?"),
        line("2024-01-01T00:00:09Z", "answered", "33333333-3333-3333-3333-333333333333", "4444444444444444444444444444444444444444444444444444444444444444", "A"),
        line("2024-01-01T00:00:01Z", "context", "-", "5555555555555555555555555555555555555555555555555555555555555555", "adrdom/decisions/x.md"),
    ];

    let dir_a = init_repo();
    assert!(adr(&dir_a, &["install"], &[], "").status.success());
    std::fs::create_dir_all(dir_a.join("adrdom/state")).unwrap();
    let order_a = rows.clone();
    std::fs::write(dir_a.join("adrdom/state/ledger.tsv"), order_a.join("")).unwrap();

    let dir_b = init_repo();
    assert!(adr(&dir_b, &["install"], &[], "").status.success());
    std::fs::create_dir_all(dir_b.join("adrdom/state")).unwrap();
    let mut order_b = rows.clone();
    order_b.reverse();
    std::fs::write(dir_b.join("adrdom/state/ledger.tsv"), order_b.join("")).unwrap();
    assert_ne!(order_a, order_b, "the two files must actually differ in row order");

    let check_a = stdout(&adr(&dir_a, &["check"], &[], ""));
    let check_b = stdout(&adr(&dir_b, &["check"], &[], ""));
    assert_eq!(check_a, check_b, "check output must not depend on file order");

    let show_a = stdout(&adr(&dir_a, &["show"], &[], ""));
    let show_b = stdout(&adr(&dir_b, &["show"], &[], ""));
    assert_eq!(show_a, show_b, "show output must not depend on file order");
}
