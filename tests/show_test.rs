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

/// Every question is listed now (`show` prints every derived
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

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    assert_eq!(ids.len(), 2);
    let answered_id = &ids[0];

    let answer_out = adr_answer(&dir, answered_id, "A");
    assert!(
        answer_out.status.success(),
        "answer failed: {}",
        stderr(&answer_out)
    );

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);

    assert!(
        shown.starts_with("[approved] Example (adrdom/decisions/example.md)\n"),
        "{}",
        shown
    );
    assert!(!shown.contains("tier 1"), "{}", shown);
    assert!(
        shown.contains("[answered A] 1.1 Is this current?"),
        "{}",
        shown
    );
    // An answered question carries its status and, on the line under, the
    // option picked, as the listing with no file named prints it; its body
    // is not printed.
    assert!(
        shown.contains("[answered A] 1.1 Is this current?\n  Yes it is current.\n"),
        "{}",
        shown
    );
    assert!(!shown.contains("- A) Yes it is current."), "{}", shown);
    // The second question is still open (asked, not answered): it is now
    // listed too, with its state, rather than left out.
    //
    // adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
    //
    // It also carries its version ("show ... print[s] a short version of
    // each asked question"), so an answer to it can be checked against the
    // text the owner is reading right now.
    let version = current_version(&dir, &ids[1]);
    assert!(
        shown.contains(&format!("[asked @{}] 1.2 Is this only asked?", version)),
        "{}",
        shown
    );
    // The body of the asked question, options and all, for the owner who
    // answers it.
    assert!(shown.contains("  - A) Yes only asked."), "{}", shown);
}

const TIERED_RECORD: &str = "# Tiered Topic\n\n\
### 1.1 Root?\n\n\
id: aaaaaaaa-0000-4000-8000-000000000001\n\n\
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
id: aaaaaaaa-0000-4000-8000-000000000002\n\n\
Problem: p3.\n\n\
Standard: s3.\n\n\
- A) x.\n  Why: w.\n  Costs: c.\n\
- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 2.2 Also depends?\n\
depends: 1.1 Root?=A\n\n\
id: aaaaaaaa-0000-4000-8000-000000000003\n\n\
Problem: p4.\n\n\
Standard: s4.\n\n\
- A) x.\n  Why: w.\n  Costs: c.\n\
- B) y.\n  Why: w.\n  Costs: c.\n\n\
**Answer:** refused\n";

/// Exercises every status `show` prints and the lines a question carries,
/// all via hand-crafted ledger rows and `**Answer:**` lines, on a file whose
/// plan and prose equal their latest approval: nothing is beyond it, so no
/// line is marked.
//
// "show prints no decision" for a refused question: the refused one carries
// `**Answer:** refused`, and the expected output below lists it as
// `[refused] 2.2 Also depends?` and nothing else: the reference lines of an answered question are hidden.
//
// adr: b782664f-7465-4e1e-99e0-13f6dd2de774
// adr: d1d6fed2-4e1a-44bd-b9cb-22f779a00bce
#[test]
fn show_prints_each_status_and_the_lines_each_question_carries() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    // "root" is a word the unasked questions already carry in their
    // headings, so they pair a term without a change of text.
    approve_terms_with(&dir, &["root"]);
    let record = write_record(&dir, "adrdom/decisions/tiered-topic.md", TIERED_RECORD);
    approve_via_ledger(&dir, "adrdom/decisions/tiered-topic.md", TIERED_RECORD);
    // Every row carries the hash of its section's text as it stands: a
    // stand-in hash reads as "edited since", for the answered root as for
    // the open question (show prints no version and no owner command then).
    let rf = adr::record::load(&record).unwrap();
    let root = rf
        .sections
        .iter()
        .find(|s| s.id.as_deref() == Some("aaaaaaaa-0000-4000-8000-000000000001"))
        .unwrap();
    let root_hash = adr::record::section_hash(&rf.lines, root);
    append_ledger_row_at(
        &dir,
        "asked",
        "aaaaaaaa-0000-4000-8000-000000000001",
        &root_hash,
        "1.1 Root?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(
        &dir,
        "answered",
        "aaaaaaaa-0000-4000-8000-000000000001",
        &root_hash,
        "A",
        "2024-01-01T00:00:01Z",
    );
    let open = rf
        .sections
        .iter()
        .find(|s| s.id.as_deref() == Some("aaaaaaaa-0000-4000-8000-000000000002"))
        .unwrap();
    let open_hash = adr::record::section_hash(&rf.lines, open);
    append_ledger_row(
        &dir,
        "asked",
        "aaaaaaaa-0000-4000-8000-000000000002",
        &open_hash,
        "2.1 Depends on root?",
    );
    // The refused rows carry the section's real hash, as answer writes it.
    let refused = rf
        .sections
        .iter()
        .find(|s| s.id.as_deref() == Some("aaaaaaaa-0000-4000-8000-000000000003"))
        .unwrap();
    let refused_hash = adr::record::section_hash(&rf.lines, refused);
    append_ledger_row_at(
        &dir,
        "asked",
        "aaaaaaaa-0000-4000-8000-000000000003",
        &refused_hash,
        "2.2 Also depends?",
        "2024-01-01T00:00:02Z",
    );
    append_ledger_row_at(
        &dir,
        "refused",
        "aaaaaaaa-0000-4000-8000-000000000003",
        &refused_hash,
        "2.2 Also depends?",
        "2024-01-01T00:00:03Z",
    );
    let version = adr::record::version_of(&rf.lines, open);

    let out = adr(&dir, &["show", record.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);

    // The asked question's version is the current section hash's own prefix (per
    // "Does an answer carry the version of the question the owner read?").
    // No tier heading and no "dependents:" line: what is computed from the
    // records comes under a flag. The body is printed for the asked
    // question alone.
    let expected = "\
[approved] Tiered Topic (adrdom/decisions/tiered-topic.md)
[answered A] 1.1 Root?
  First choice.
[not asked] 1.2 Unasked root?
[no body] 1.3 Skeleton?
[asked @{V}] 2.1 Depends on root?
  depends: 1.1 Root?=A
  Problem: p3.
  Standard: s3.
  - A) x.
    Why: w.
    Costs: c.
  - B) y.
    Why: w.
    Costs: c.
[refused] 2.2 Also depends?
adrdom/decisions/tiered-topic.md: \"1.1 Root?\" has no depends line; agent writes the depends line of \"1.1 Root?\"
adrdom/decisions/tiered-topic.md: \"1.2 Unasked root?\" has no depends line; agent writes the depends line of \"1.2 Unasked root?\"
adrdom/decisions/tiered-topic.md: \"1.3 Skeleton?\" has no depends line; agent writes the depends line of \"1.3 Skeleton?\"
adrdom/decisions/tiered-topic.md: \"2.2 Also depends?\" is [refused] and its depends line was edited; agent restores the line from git
agent runs: adr plan adrdom/decisions/tiered-topic.md
owner runs: adr approve adrdom/decisions/tiered-topic.md
owner runs: adr answer aaaaaaaa-0000-4000-8000-000000000002@{V} <letter>  # \"2.1 Depends on root?\"
agent writes the plan change that drops or replaces \"2.2 Also depends?\"
not yet: \"1.1 Root?\" is answered and is built once \"1.2 Unasked root?\" is answered
";
    let expected = expected.replace("{V}", &version);
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
    assert!(
        shown.contains("Tiered Topic (adrdom/decisions/one.md)"),
        "{}",
        shown
    );
    assert!(!shown.contains("Second Topic"), "{}", shown);
}

// adr: 48f4647d-8c81-4e8a-a936-6b32f5fb0699
//
// "there is no override": `override` names no subcommand the CLI
// recognizes, so clap itself refuses it (a non-zero exit, no file or
// ledger touched) rather than the tool running some bypass action. The
// other half of this decision -- every stop names the move that clears
// it -- is tied in src/msg's `every_refusal_names_its_next_move`.
#[test]
fn override_is_not_a_recognized_command() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let out = adr(
        &dir,
        &["override", "some-stop", "because I said so"],
        &[],
        "",
    );
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

// adr: 9afe335c-2a7a-4625-b115-f74d3f091cac
//
// "What does a depends entry name?": a term, or a question of the same topic
// file. File B here is never approved -- its depends line is still the
// written heading text of a current decision of another file -- and `show`
// prints the one finding that names the entry, not a dangling reference.
#[test]
fn show_names_a_heading_of_another_file_in_the_depends_line_of_an_unapproved_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");

    // A: an already-answered, current decision living in a topic file of
    // its own.
    let a_id = "77777777-7777-7777-7777-777777777777";
    append_ledger_row_at(
        &dir,
        "asked",
        a_id,
        "h1",
        "1.1 Is A do a thing?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(&dir, "answered", a_id, "h1", "A", "2024-01-01T00:00:01Z");
    let a_content = format!(
        "# Topic A\n\n### 1.1 Is A do a thing?\ndepends: decision\n\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        a_id
    );
    write_record(&dir, "adrdom/decisions/topic-a.md", &a_content);

    // B: never approved -- its only question stands on A, named by A's
    // heading text, still unrewritten on disk.
    let b_content = "# Topic B\n\n### 1.1 Is B stand on A?\ndepends: 1.1 Is A do a thing?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    write_record(&dir, "adrdom/decisions/topic-b.md", b_content);

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);

    assert!(
        shown.contains("[new] Topic B (adrdom/decisions/topic-b.md)"),
        "{}",
        shown
    );
    assert!(
        shown.contains("[not asked] 1.1 Is B stand on A?\n  + depends: 1.1 Is A do a thing?"),
        "B's only question should be listed as unasked, its new lines marked: {}",
        shown
    );
    assert!(
        shown.contains("\"1.1 Is B stand on A?\" depends on \"1.1 Is A do a thing?\", a decision of another topic file; agent rewrites the depends line"),
        "{}",
        shown
    );
    assert!(
        !shown.contains("no heading in this file"),
        "a heading of another file's decision is one finding, not a dangling reference too: {}",
        shown
    );
}

// adr: 9afe335c-2a7a-4625-b115-f74d3f091cac
//
// A heading text that is a current decision in more than one other topic
// file is no question of the file either: `show` names the entry and the
// move, and guesses no "stands on" line.
#[test]
fn show_prints_no_guess_for_a_heading_naming_decisions_of_two_other_files() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");

    let a_id = "88888888-8888-8888-8888-888888888888";
    append_ledger_row_at(
        &dir,
        "asked",
        a_id,
        "h1",
        "1.1 Shared heading?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(&dir, "answered", a_id, "h1", "A", "2024-01-01T00:00:01Z");
    let a_content = format!(
        "# Topic A\n\n### 1.1 Shared heading?\ndepends: decision\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        a_id
    );
    write_record(&dir, "adrdom/decisions/topic-a.md", &a_content);

    let c_id = "99999999-8888-8888-8888-888888888888";
    append_ledger_row_at(
        &dir,
        "asked",
        c_id,
        "h1",
        "1.1 Shared heading?",
        "2024-01-01T00:00:02Z",
    );
    append_ledger_row_at(&dir, "answered", c_id, "h1", "A", "2024-01-01T00:00:03Z");
    let c_content = format!(
        "# Topic C\n\n### 1.1 Shared heading?\ndepends: decision\nid: {}\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        c_id
    );
    write_record(&dir, "adrdom/decisions/topic-c.md", &c_content);

    let b_content = "# Topic B\n\n### 1.1 Does B stand on the shared heading?\ndepends: 1.1 Shared heading?\n\nProblem: p2 of the decision.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
    write_record(&dir, "adrdom/decisions/topic-b.md", b_content);

    let out = adr(&dir, &["show", "--all"], &[], "");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let shown = stdout(&out);

    // Everything `show` prints for file B, from its status line to the next
    // file's: its question as written, marked new, and the finding whose
    // move is the agent naming the decision by id -- never a guess at which
    // of the two decisions the heading means.
    let b_start = shown.find("[new] Topic B").expect("topic B status line");
    let b_end = shown
        .find("Topic C (")
        .map(|i| shown[..i].rfind('\n').unwrap())
        .unwrap_or(shown.len());
    let b_block = shown[b_start..b_end].trim_end();
    assert!(
        b_block.contains("  + depends: 1.1 Shared heading?"),
        "{}",
        shown
    );
    assert!(
        b_block.contains("adrdom/decisions/topic-b.md: \"1.1 Does B stand on the shared heading?\" depends on \"1.1 Shared heading?\", a decision of another topic file; agent rewrites the depends line of"),
        "an ambiguous cross-file dependency names the agent's move and no guess: {}",
        shown
    );
}
