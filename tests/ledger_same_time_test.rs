// Decision T (ce19c883-c61d-477b-a6d4-fee5134c6e9a): "does check fail two
// ledger rows on one subject with the same time?" -- check fails two rows
// that carry one time and compete for one subject; the command run again
// writes a later row that stands; rows of one command that share a time do
// not compete.
//
// Cases (a)/(b)/(d) hand-craft ledger rows directly (`append_ledger_row_at`)
// rather than driving the real commands, so each pins one exact shape
// (a hand tie, never a real one) without depending on how any other
// category of `check` happens to read the rest of the fixture. Case (e)
// drives the real harness -- `answer_one_question`, lifted from
// `tests/check_test.rs` (read, not edited) -- since its whole point is that
// no real command run ever produces the shape the other cases hand-craft.

mod common;

use adr::record;
use common::*;

const COMPETES_TEXT: &str = "compete for";

const ONE_QUESTION: &str = r#"# Example

### 3.1 Does this pass?
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

/// Writes `ONE_QUESTION`, approves its plan/context via a hand-written
/// ledger row, asks, then owner-answers option A. Lifted from
/// `tests/check_test.rs`'s own helper of the same shape (read, not
/// edited) -- test (e) below needs exactly "one command run" of each of
/// `ask`/`approve`/`answer`, not a fixture of its own invention.
fn answer_one_question(dir: &std::path::Path) -> (std::path::PathBuf, String) {
    let terms_content =
        "# Terms\n\n## pass\nid: 11111111-1111-1111-1111-111111111111\ndefinition: Whether something is accepted.\naliases: \n";
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();
    approve_terms_via_ledger(dir, terms_content);
    let record = write_record(dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_record(dir, "adrdom/decisions/example.md");
    assert!(adr(dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    let arg = id_at_version(dir, &id);
    let out = adr_owner(dir, &["answer", &arg, "A"], "");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    (record, id)
}

// adr: ce19c883-c61d-477b-a6d4-fee5134c6e9a
//
// A hand-tied `banned`/`unbanned` pair for one hash, at one time, fails;
// running `ban` for real (the file doesn't yet hold the hash, so this is a
// genuinely new entry) writes a later, single row for that hash, and check
// passes.
#[test]
fn a_banned_and_unbanned_for_one_hash_at_one_time_fails_then_a_later_ban_row_passes() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let hash = record::sha256_hex("sametimehashwordalpha");
    let tie_time = "2024-01-01T00:00:00Z";
    // Row text ties are broken alphabetically ("banned" < "unbanned"), so
    // `banned` sorts first and `unbanned` -- processed last by
    // `banned::currently_banned` -- is the one that (correctly) matches
    // `adrdom/state/banned.sha256` still holding nothing for this hash.
    append_ledger_row_at(&dir, "banned", "-", &hash, "", tie_time);
    append_ledger_row_at(&dir, "unbanned", "-", &hash, "", tie_time);

    let tied = adr(&dir, &["check"], &[], "");
    assert!(!tied.status.success(), "a hand-tied banned/unbanned pair must fail check");
    let text = stdout(&tied);
    assert!(text.contains("ledger:"), "{}", text);
    assert!(text.contains(COMPETES_TEXT), "{}", text);
    assert!(text.contains(&hash[..12]), "{}", text);

    let ban = adr(&dir, &["ban"], &[], "sametimehashwordalpha\n");
    assert!(ban.status.success(), "ban failed: {}", stderr(&ban));

    let after = adr(&dir, &["check"], &[], "");
    assert!(
        after.status.success(),
        "check must pass once the later ban row settles the tie: {}",
        stdout(&after)
    );
}

// adr: ce19c883-c61d-477b-a6d4-fee5134c6e9a
//
// Two state rows (asked, answered) sharing one id at one time fail.
#[test]
fn b_two_state_rows_for_one_id_at_one_time_fails() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let id = "22222222-2222-2222-2222-222222222222";
    let tie_time = "2024-01-01T00:00:00Z";
    append_ledger_row_at(&dir, "asked", id, "hash-1", "Does this pass?", tie_time);
    append_ledger_row_at(&dir, "answered", id, "hash-1", "A", tie_time);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success(), "two state rows sharing an id at one time must fail check");
    let text = stdout(&out);
    assert!(text.contains("ledger:"), "{}", text);
    assert!(text.contains(COMPETES_TEXT), "{}", text);
    assert!(text.contains(&format!("id {}", id)), "{}", text);
}

// adr: ce19c883-c61d-477b-a6d4-fee5134c6e9a
//
// An asked row and its own assumes rows, sharing an id at one time, never
// compete: `assumes` is not one of the state events the id rule matches.
#[test]
fn c_an_asked_row_with_its_assumes_rows_at_one_time_passes() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let id = "33333333-3333-3333-3333-333333333333";
    let tie_time = "2024-01-01T00:00:00Z";
    append_ledger_row_at(&dir, "asked", id, "hash-1", "Does this pass?", tie_time);
    append_ledger_row_at(&dir, "assumes", id, "hash-2", "Dependent A?", tie_time);
    append_ledger_row_at(&dir, "assumes", id, "hash-3", "Dependent B?", tie_time);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains(&format!("compete for id {}", id)),
        "an asked row and its own assumes rows must not compete: {}",
        text
    );
}

// adr: ce19c883-c61d-477b-a6d4-fee5134c6e9a
//
// Two byte-identical rows at one time never compete, however many of them
// there are.
#[test]
fn d_two_byte_identical_rows_at_one_time_passes() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let id = "44444444-4444-4444-4444-444444444444";
    let tie_time = "2024-01-01T00:00:00Z";
    append_ledger_row_at(&dir, "asked", id, "hash-1", "Does this pass?", tie_time);
    append_ledger_row_at(&dir, "asked", id, "hash-1", "Does this pass?", tie_time);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains(&format!("compete for id {}", id)),
        "two byte-identical rows must not compete: {}",
        text
    );
}

// adr: ce19c883-c61d-477b-a6d4-fee5134c6e9a
//
// The real flow of one command run each (`ask`, `approve`, `answer`,
// through the harness) never trips the rule: `check` passes clean.
#[test]
fn e_the_real_flow_of_one_command_run_never_trips_the_rule() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let (_record, id) = answer_one_question(&dir);
    write_test_comment(&dir, &id);

    let out = adr(&dir, &["check"], &[], "");
    assert!(
        out.status.success(),
        "the real ask/approve/answer flow must never trip the same-time rule: {}",
        stdout(&out)
    );
    assert!(!stdout(&out).contains(COMPETES_TEXT), "{}", stdout(&out));
}
