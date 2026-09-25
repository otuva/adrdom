mod common;

use common::*;

const RECORD: &str = r#"# Example

### 1.1 Is this current?
depends: reasons

Problem: whether it is current.

Standard: clear yes or no.

- A) Yes it is current.
  Why: reasons.
  Costs: costs.
- B) No it is not.
  Why: other reasons.
  Costs: other costs.

### 1.2 Is this only asked?
depends: reasons

Problem: whether it is only asked.

Standard: clear yes or no.

- A) Yes only asked.
  Why: reasons.
  Costs: costs.
- B) No.
  Why: other reasons.
  Costs: other costs.
"#;

/// Every question is listed now (spec §7: `show` prints every derived
/// tier's questions with their state), not only the current, answered
/// ones — the old (pre-tier) `show` filtered down to just those. Both
/// headings here are root (no `depends:` heading entry, only the fixture
/// term `reasons`), so both land in tier 1.
#[test]
fn show_lists_every_question_with_its_state_and_tier() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "reasons");
    let record = write_record(&dir, "adrdom/decisions/example.md", RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    assert_eq!(ids.len(), 2);
    let answered_id = &ids[0];

    let answer_out = adr_owner(&dir, &["answer", answered_id, "A"], "");
    assert!(
        answer_out.status.success(),
        "answer failed: {}",
        stderr(&answer_out)
    );

    let out = adr(&dir, &["show"], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);

    assert!(shown.starts_with("# Example\n"), "{}", shown);
    assert!(shown.contains("tier 1\n"), "{}", shown);
    assert!(shown.contains("[answered A] 1.1 Is this current?"), "{}", shown);
    assert!(shown.contains("Yes it is current."), "{}", shown);
    // The second question is still open (asked, not answered): it is now
    // listed too, with its state, rather than left out.
    assert!(shown.contains("[open] 1.2 Is this only asked?"), "{}", shown);
    assert!(!shown.contains("Yes only asked."), "no chosen sentence for an unanswered question: {}", shown);
}

const TIERED_RECORD: &str = "# Tiered Topic\n\n\
### 1.1 Root?\n\n\
id: id-root\n\n\
Problem: p.\n\n\
Standard: s.\n\n\
- A) First choice.\n  Why: w.\n  Costs: c.\n\
- B) Second choice.\n  Why: w.\n  Costs: c.\n\n\
**Answer:** A\n\n\
### 1.2 Unasked root?\n\n\
Problem: p2.\n\n\
Standard: s2.\n\n\
- A) x.\n  Why: w.\n  Costs: c.\n\
- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.3 Skeleton?\n\n\
### 2.1 Depends on root?\n\
depends: 1.1 Root?=A\n\n\
id: id-open\n\n\
Problem: p3.\n\n\
Standard: s3.\n\n\
- A) x.\n  Why: w.\n  Costs: c.\n\
- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 2.2 Also depends?\n\
depends: 1.1 Root?=A\n\n\
id: id-refused\n\n\
Problem: p4.\n\n\
Standard: s4.\n\n\
- A) x.\n  Why: w.\n  Costs: c.\n\
- B) y.\n  Why: w.\n  Costs: c.\n\n\
**Answer:** refused\n";

/// Exercises tiers, every state `show` prints, the "stands on:" line, and
/// the chosen sentence, all via hand-crafted ledger rows and `**Answer:**`
/// lines (per the wave-2 brief) rather than `adr ask`/`adr answer`, which
/// are being rebuilt by other agents on this same branch.
// adr: aec860fd-22ad-44a7-99f7-222ea19537e7
//
// "show prints the tiers": the fixture below states no tier anywhere in
// its text (only `depends:` lines and hand-written ledger rows), yet
// `show` prints "tier 1" and "tier 2" derived from those depends lines.
//
// adr: b782664f-7465-4e1e-99e0-13f6dd2de774
//
// "show prints no decision" for a refused question: `id-refused` carries
// `**Answer:** refused`, and the expected output below lists it as
// `[refused] 2.2 Also depends?` with no chosen-sentence line under it,
// unlike `id-root`'s `[answered A]` line, which does print one.
#[test]
fn show_prints_tiers_states_stands_on_and_chosen_sentence() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/tiered.md", TIERED_RECORD);
    append_ledger_row(&dir, "asked", "id-root", "h-root", "1.1 Root?");
    append_ledger_row(&dir, "answered", "id-root", "h-root", "A");
    append_ledger_row(&dir, "asked", "id-open", "h-open", "2.1 Depends on root?");
    append_ledger_row(&dir, "asked", "id-refused", "h-refused", "2.2 Also depends?");
    append_ledger_row(&dir, "refused", "id-refused", "h-refused", "2.2 Also depends?");

    let out = adr(&dir, &["show", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);

    let expected = "\
# Tiered Topic
tier 1
  [answered A] 1.1 Root?
    First choice.
  [unasked] 1.2 Unasked root?
  [skeleton] 1.3 Skeleton?
tier 2
  [open] 2.1 Depends on root?
    stands on: 1.1 Root?=A
  [refused] 2.2 Also depends?
    stands on: 1.1 Root?=A
";
    assert_eq!(shown, expected, "full output: {}", shown);
}

/// A single-file argument shows only that file, as before.
#[test]
fn show_with_a_file_argument_shows_only_that_file() {
    let dir = init_repo();
    let one = write_record(&dir, "adrdom/decisions/one.md", TIERED_RECORD);
    let two_content = TIERED_RECORD.replace("Tiered Topic", "Second Topic");
    write_record(&dir, "adrdom/decisions/two.md", &two_content);

    let out = adr(&dir, &["show", one.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);
    assert!(shown.contains("# Tiered Topic"), "{}", shown);
    assert!(!shown.contains("# Second Topic"), "{}", shown);
}

// adr: 48f4647d-8c81-4e8a-a936-6b32f5fb0699
//
// "there is no override": `override` names no subcommand the CLI
// recognizes, so clap itself refuses it (a non-zero exit, no file or
// ledger touched) rather than the tool running some bypass action. The
// other half of this decision -- every stop names the move that clears
// it -- is tied in src/msg.rs's `every_refusal_names_its_next_move`.
#[test]
fn override_is_not_a_recognized_command() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let out = adr(&dir, &["override", "some-stop", "because I said so"], &[], "");
    assert!(
        !out.status.success(),
        "an `override` subcommand must not exist: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("unrecognized subcommand") || err.contains("override"),
        "expected clap to name the unknown subcommand: {}",
        err
    );

    // Nothing was written: no override event of any kind lands in the
    // ledger.
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.to_lowercase().contains("override"),
        "no override event should ever reach the ledger: {}",
        ledger
    );
}

// adr: aec860fd-22ad-44a7-99f7-222ea19537e7
//
// "Does the tool derive tiers from the depends lines?": `show` derives
// tiers the same way `approve`/`check` do, including the cross-file
// pre-pass (`depends::normalize_cross_file_deps`) that rewrites a
// heading-shaped `depends:` entry naming a current decision's heading in
// another topic file to that decision's id before tiers are derived. File
// B here is never approved -- its depends line is still the written
// heading text, exactly as an agent first writes it -- so this only
// passes when `show` runs that same pre-pass itself instead of reading
// `rf.tiers` straight off an unnormalized file.
#[test]
fn show_derives_tiers_for_an_unapproved_file_whose_depends_line_names_a_cross_file_heading() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");

    // A: an already-answered, current decision living in a topic file of
    // its own.
    let a_id = "77777777-7777-7777-7777-777777777777";
    append_ledger_row(&dir, "asked", a_id, "h1", "1.1 Does A do a thing?");
    append_ledger_row(&dir, "answered", a_id, "h1", "A");
    let a_content = format!(
        "# Topic A\n\n### 1.1 Does A do a thing?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        a_id
    );
    write_record(&dir, "adrdom/decisions/topic-a.md", &a_content);

    // B: never approved -- its only question stands on A, named by A's
    // heading text, still unrewritten on disk.
    let b_content = "# Topic B\n\n### 1.1 Does B stand on A?\ndepends: 1.1 Does A do a thing?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    write_record(&dir, "adrdom/decisions/topic-b.md", b_content);

    let out = adr(&dir, &["show"], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);

    assert!(shown.contains("# Topic B"), "{}", shown);
    assert!(
        shown.contains("tier 1\n  [unasked] 1.1 Does B stand on A?"),
        "B's only question should land in tier 1 and be listed as unasked: {}",
        shown
    );
    assert!(
        shown.contains("stands on: 1.1 Does A do a thing?"),
        "the cross-file dependency should resolve back to A's heading text: {}",
        shown
    );
    assert!(
        !shown.contains("no heading in this file"),
        "an unapproved cross-file heading reference must not be reported as dangling: {}",
        shown
    );
}

// adr: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
//
// The cross-file half's edge, at `show` rather than `approve`: a heading
// text that names a current decision in more than one other topic file is
// ambiguous, so `show` must never guess which one a "stands on" line
// means. Before this fix, `normalize_cross_file_deps` silently substituted
// the first of the ambiguous matches (so tiers still derived and `show`
// printed a "stands on" line naming one of the two decisions, indistin-
// guishable from a real answer); now the ambiguity becomes the file's own
// `derive_error`, so `show` prints B's topic line and nothing else for it
// -- no tier, no question, no guessed "stands on" line -- the same silent
// drop every other derive error already gets from `show`.
#[test]
fn show_prints_no_guess_for_an_ambiguous_cross_file_heading_dependency() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");

    let a_id = "88888888-8888-8888-8888-888888888888";
    append_ledger_row(&dir, "asked", a_id, "h1", "1.1 Shared heading?");
    append_ledger_row(&dir, "answered", a_id, "h1", "A");
    let a_content = format!(
        "# Topic A\n\n### 1.1 Shared heading?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        a_id
    );
    write_record(&dir, "adrdom/decisions/topic-a.md", &a_content);

    let c_id = "99999999-8888-8888-8888-888888888888";
    append_ledger_row(&dir, "asked", c_id, "h1", "1.1 Shared heading?");
    append_ledger_row(&dir, "answered", c_id, "h1", "A");
    let c_content = format!(
        "# Topic C\n\n### 1.1 Shared heading?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        c_id
    );
    write_record(&dir, "adrdom/decisions/topic-c.md", &c_content);

    let b_content = "# Topic B\n\n### 1.1 Does B stand on the shared heading?\ndepends: 1.1 Shared heading?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    write_record(&dir, "adrdom/decisions/topic-b.md", b_content);

    let out = adr(&dir, &["show"], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);

    // Everything `show` prints for file B, from its topic line to the next
    // file's (or end of output): tier derivation failed on the ambiguous
    // entry, so this is the topic line alone -- no tier, no question, and
    // in particular no guessed "stands on" line.
    let b_start = shown.find("# Topic B").expect("topic B header");
    let b_end = shown[b_start..]
        .find("\n# ")
        .map(|i| b_start + i)
        .unwrap_or(shown.len());
    let b_block = shown[b_start..b_end].trim_end();
    assert_eq!(
        b_block, "# Topic B",
        "an ambiguous cross-file dependency must print nothing but the topic line, never a guessed tier or \"stands on\": {}",
        shown
    );
}
