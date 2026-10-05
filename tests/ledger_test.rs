mod common;

use common::*;

const ONE_QUESTION: &str = r#"# Example

### 5.1 Is this pass?
depends: pass

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes.
  Why: reasons.
  Costs: costs.
- B) No.
  Why: other reasons.
  Costs: other costs.
"#;

// adr: bf6697e3-8832-4b90-b036-8fecedb55406
//
// "every row present at HEAD must still be present, byte for byte,
// somewhere in the working copy -- its place in the file is free (per
// "Does check fail a ledger whose rows changed place?",
// c10c3a49-e993-4c9f-a46b-4a1ba3646dcf A)": tampering with an
// already-committed row (this test) fails `check` with a `ledger:`
// "missing or changed" failure;
// `ledger_deleted_committed_row_fails_check_but_a_new_appended_row_still_passes`
// below covers the sibling clauses -- a deleted row fails the same way,
// and a genuinely new appended row still passes.
#[test]
fn ledger_append_only_violation_is_detected_after_a_commit() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = "# Terms\n\n## pass\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A clear yes.\naliases: passes\n";
    std::fs::write(dir.join("adrdom/terms.md"), terms).unwrap();
    approve_terms_via_ledger(&dir, terms);
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    // Commit everything install wrote too, not just adrdom/: the
    // pre-commit hook now judges the same staged tree `check --staged`
    // would (per "Does check at a commit read the staged files?"), and
    // AGENTS.md sitting on disk but unstaged would otherwise refuse this
    // commit before it ever reaches the ledger.
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "ask"]);

    // A clean check should still pass the ledger check (no divergence yet).
    let out = adr(&dir, &["check"], &[], "");
    let out_text = stdout(&out);
    assert!(
        !out_text.contains("ledger:"),
        "unexpected ledger failure before tampering: {}",
        out_text
    );

    // Now tamper with an already-committed line instead of only appending.
    // Change the detail column so the row stays well-formed (still a valid
    // `asked` row with a real hash) but the working copy is no longer a
    // byte-for-byte prefix match of what's on HEAD.
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let content = std::fs::read_to_string(&ledger_path).unwrap();
    let mut lines: Vec<&str> = content.lines().collect();
    let asked_idx = lines
        .iter()
        .position(|l| l.contains("\tasked\t"))
        .expect("an asked row");
    let tampered = lines[asked_idx].replace("Is this pass?", "rewritten after the fact");
    lines[asked_idx] = &tampered;
    let new_content = lines.join("\n") + "\n";
    std::fs::write(&ledger_path, new_content).unwrap();

    let out2 = adr(&dir, &["check"], &[], "");
    let out2_text = stdout(&out2);
    assert!(!out2.status.success());
    assert!(out2_text.contains("ledger:"), "{}", out2_text);
    assert!(out2_text.contains("missing or changed"), "{}", out2_text);
}

// adr: dfd32ef6-c2ac-40ef-b8b3-9c58732516b1
//
// "an owner command writes the file and the ledger and commits nothing":
// real `adr approve` and `adr answer` calls below each write to disk and
// append a ledger row, but HEAD never moves and `git status` still shows
// the write as an uncommitted change -- twice, once per command. Only the
// commit the test makes afterward moves HEAD, and `check` run at that new
// commit (the working tree now equal to it) verifies the file and the
// ledger still match.
#[test]
fn approve_and_answer_write_without_committing_and_the_next_commit_carries_them() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    run_git(&dir, &["add", "-A"]);
    let install_commit = run_git(&dir, &["commit", "-q", "-m", "install"]);
    assert!(install_commit.status.success(), "{:?}", install_commit);
    let head_before = stdout(&run_git(&dir, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();

    // A real, owner-gated `approve` of adrdom/terms.md: writes the file
    // (already on disk, unchanged content here) and appends a `terms`
    // ledger row.
    let terms = "# Terms\n\n## pass\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A clear yes.\naliases: passes\n";
    let terms_path = write_record(&dir, "adrdom/terms.md", terms);
    let approve_terms_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(
        approve_terms_out.status.success(),
        "approve failed: {}",
        stderr(&approve_terms_out)
    );

    let head_after_terms = stdout(&run_git(&dir, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    assert_eq!(head_before, head_after_terms, "approve must not commit");
    let status_after_terms = stdout(&run_git(&dir, &["status", "--porcelain"]));
    assert!(
        status_after_terms.contains("adrdom/terms.md"),
        "terms.md should sit as an uncommitted change: {}",
        status_after_terms
    );
    assert!(
        status_after_terms.contains("adrdom/state/ledger.tsv"),
        "the ledger should sit as an uncommitted change: {}",
        status_after_terms
    );

    // A real, owner-gated `approve` of a topic file: same shape.
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    let head_after_record_approve = stdout(&run_git(&dir, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    assert_eq!(
        head_before, head_after_record_approve,
        "approving the topic file must not commit either"
    );

    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();

    // A real, owner-gated `answer`: writes the `**Answer:**` line into the
    // topic file and appends an `answered` ledger row.
    let answer_out = adr_answer(&dir, &id, "A");
    assert!(
        answer_out.status.success(),
        "answer failed: {}",
        stderr(&answer_out)
    );

    let head_after_answer = stdout(&run_git(&dir, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    assert_eq!(head_before, head_after_answer, "answer must not commit");
    // `adrdom/decisions/` is a whole new, untracked directory at this
    // point, so plain `git status --porcelain` names the directory rather
    // than the file inside it -- `-uall` expands it to the actual path.
    let status_after_answer = stdout(&run_git(&dir, &["status", "--porcelain", "-uall"]));
    assert!(
        status_after_answer.contains("adrdom/decisions/example.md"),
        "the answered topic file should still be an uncommitted change: {}",
        status_after_answer
    );

    // The next commit is what carries all of it: write the test tie the
    // decision requires, commit everything, and confirm HEAD actually
    // moved and check is satisfied with what that commit now holds.
    write_test_comment(&dir, &id);
    run_git(&dir, &["add", "-A"]);
    let final_commit = run_git(&dir, &["commit", "-q", "-m", "answer 3.1"]);
    assert!(final_commit.status.success(), "{:?}", final_commit);
    let head_final = stdout(&run_git(&dir, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    assert_ne!(head_before, head_final, "the next commit should move HEAD");

    let check_out = adr(&dir, &["check"], &[], "");
    assert!(
        check_out.status.success(),
        "check at the carrying commit should verify the file and ledger match: {}",
        stdout(&check_out)
    );
}

// adr: bf6697e3-8832-4b90-b036-8fecedb55406
//
// The deletion half and the still-passes half of "every row present at
// HEAD must still be present, byte for byte, somewhere in the working
// copy; its place in the file is free": removing a committed row fails
// `check` exactly like editing one does, and restoring HEAD's content plus
// a genuinely new appended row clears the failure.
#[test]
fn ledger_deleted_committed_row_fails_check_but_a_new_appended_row_still_passes() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = "# Terms\n\n## pass\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A clear yes.\naliases: passes\n";
    std::fs::write(dir.join("adrdom/terms.md"), terms).unwrap();
    approve_terms_via_ledger(&dir, terms);
    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    // Everything install wrote staged too (see the comment in
    // `ledger_append_only_violation_is_detected_after_a_commit`): the
    // pre-commit hook now reads the same staged tree `check --staged`
    // does.
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "ask"]);

    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let committed = std::fs::read_to_string(&ledger_path).unwrap();

    let clean_check = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&clean_check).contains("ledger:"),
        "unexpected ledger failure before deleting anything: {}",
        stdout(&clean_check)
    );

    // Delete one committed line entirely (not just edit it): HEAD's row is
    // then missing from the working copy altogether, wherever it might
    // otherwise have sat.
    let mut lines: Vec<&str> = committed.lines().collect();
    let asked_idx = lines
        .iter()
        .position(|l| l.contains("\tasked\t"))
        .expect("an asked row");
    lines.remove(asked_idx);
    let with_deletion = lines.join("\n") + "\n";
    std::fs::write(&ledger_path, &with_deletion).unwrap();

    let deleted_check = adr(&dir, &["check"], &[], "");
    assert!(!deleted_check.status.success());
    let deleted_text = stdout(&deleted_check);
    assert!(deleted_text.contains("ledger:"), "{}", deleted_text);
    assert!(deleted_text.contains("missing or changed"), "{}", deleted_text);

    // Restore HEAD's content untouched and append a genuinely new row on
    // top: every HEAD row is still present, so the check must pass again.
    let mut restored = committed.clone();
    restored.push_str("2024-01-01T00:00:00Z\tcontext\t-\t-\tsomedetail\t0.1.0\n");
    std::fs::write(&ledger_path, &restored).unwrap();

    let restored_check = adr(&dir, &["check"], &[], "");
    assert!(
        !stdout(&restored_check).contains("missing or changed"),
        "every HEAD row still present should pass the ledger check: {}",
        stdout(&restored_check)
    );
}

// adr: f3e64c4e-21ab-442f-8c40-6060591e2b77
//
// "one tab-separated row per event in one append-only file: the time, the
// event, the identifier, a hash of the text it covers, a detail, and the
// tool version": drives real terms/record `approve`, `ask`, and (owner)
// `answer` commands and checks every resulting row is exactly six
// tab-separated fields, clause by clause -- an RFC3339 time, a real event
// name, the right identifier column (`-` for file-level events, the
// question's id for `asked`/`answered`), a 64-hex-char sha256 hash of the
// text each event covers, the right detail, and the crate's VERSION.
#[test]
fn ledger_rows_from_ask_approve_answer_are_six_fields_with_correct_meaning() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = "# Terms\n\n## pass\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A clear yes.\naliases: passes\n";
    let terms_path = write_record(&dir, "adrdom/terms.md", terms);
    let approve_terms_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(
        approve_terms_out.status.success(),
        "{}",
        stderr(&approve_terms_out)
    );

    let record = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let id = extract_ids(&content)[0].clone();
    let answer_out = adr_answer(&dir, &id, "A");
    assert!(answer_out.status.success(), "{}", stderr(&answer_out));

    let ledger_content = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let version_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("VERSION");
    let version = std::fs::read_to_string(version_path).unwrap();
    let version = version.trim();
    assert!(!version.is_empty(), "VERSION file should not be empty");

    let mut saw = std::collections::HashSet::new();
    let mut asked_hash = None;
    for line in ledger_content.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            6,
            "every row must be exactly six tab-separated fields: {}",
            line
        );
        let (time, event, ident, hash, detail, ver) =
            (fields[0], fields[1], fields[2], fields[3], fields[4], fields[5]);
        assert!(
            time.contains('T') && time.ends_with('Z'),
            "time field should be RFC3339: {}",
            time
        );
        assert_eq!(ver, version, "every row this run wrote should carry the tool version: {}", line);

        match event {
            "terms" => {
                saw.insert("terms");
                assert_eq!(ident, "-", "a file-level event's identifier column is '-'");
                assert_eq!(hash.len(), 64, "hash should be a sha256 hex digest: {}", hash);
                assert_eq!(detail, "adrdom/terms.md", "detail should name the file the event covers");
            }
            "approved" => {
                saw.insert("approved");
                // one row per question (id column = question id) and one
                // file row (id column '-').
                assert!(ident == "-" || ident == id, "{}", line);
                assert_eq!(hash.len(), 64);
                assert_eq!(detail, "adrdom/decisions/example.md");
            }
            "context" => {
                saw.insert("context");
                assert_eq!(ident, "-");
                assert_eq!(hash.len(), 64);
                assert_eq!(detail, "adrdom/decisions/example.md");
            }
            "asked" => {
                saw.insert("asked");
                assert_eq!(ident, id, "asked row's identifier is the question's id");
                assert_eq!(hash.len(), 64);
                assert_eq!(detail, "5.1 Is this pass?", "detail is the heading text asked covers");
                asked_hash = Some(hash.to_string());
            }
            "answered" => {
                saw.insert("answered");
                assert_eq!(ident, id, "answered row's identifier is the question's id");
                assert_eq!(hash.len(), 64);
                assert_eq!(detail, "A", "detail is the chosen letter");
                assert_eq!(
                    Some(hash.to_string()),
                    asked_hash,
                    "answered's hash is the hash of the same text asked covered"
                );
            }
            _ => {}
        }
    }
    for expected in ["terms", "approved", "context", "asked", "answered"] {
        assert!(saw.contains(expected), "expected a {} row from this run", expected);
    }
}

// adr: 7a1e7b81-0967-4ed2-8a8a-c7153056836f
//
// "the identifier is assigned once, never reused, and independent of the
// file, the heading and the wording; the text is hashed separately, so an
// edit is still caught": a heading's id, once minted by `approve`, does
// not change when its heading text is reworded and re-approved (the hash
// in its `asked` row does change -- the edit is still caught), and the
// exact same id survives a plain copy of the section into a brand-new
// topic file, with no id re-minted for the copy.
#[test]
fn question_identifier_survives_reword_and_a_move_to_another_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "pass");

    let record_path = write_record(&dir, "adrdom/decisions/example.md", ONE_QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");

    let content = std::fs::read_to_string(&record_path).unwrap();
    let ids = extract_ids(&content);
    assert_eq!(ids.len(), 1, "approve should have minted exactly one id");
    let id = ids[0].clone();
    assert!(
        uuid::Uuid::parse_str(&id).is_ok(),
        "the identifier should be a uuid: {}",
        id
    );

    assert!(adr(&dir, &["ask", record_path.to_str().unwrap()], &[], "")
        .status
        .success());
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let after_first_ask = std::fs::read_to_string(&ledger_path).unwrap();
    let first_row = after_first_ask
        .lines()
        .filter(|l| l.contains("\tasked\t") && l.contains(&id))
        .next_back()
        .expect("a first asked row for this id")
        .to_string();
    let first_fields: Vec<&str> = first_row.split('\t').collect();
    let (first_hash, first_detail) = (first_fields[3].to_string(), first_fields[4].to_string());

    // Reword the heading text only; approve's own id-minting only fires
    // for a heading with no id line at all, so the existing id line is
    // left untouched by this re-approve.
    let reworded = content.replace("Is this pass?", "Does this still pass, reworded?");
    std::fs::write(&record_path, &reworded).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let after_reword_ids = extract_ids(&std::fs::read_to_string(&record_path).unwrap());
    assert_eq!(after_reword_ids, vec![id.clone()], "reword must not mint a new id or drop the old one");

    assert!(adr(&dir, &["ask", record_path.to_str().unwrap()], &[], "")
        .status
        .success());
    let after_reword_ask = std::fs::read_to_string(&ledger_path).unwrap();
    let second_row = after_reword_ask
        .lines()
        .filter(|l| l.contains("\tasked\t") && l.contains(&id))
        .next_back()
        .expect("a second asked row for this id")
        .to_string();
    let second_fields: Vec<&str> = second_row.split('\t').collect();
    let (second_id, second_hash, second_detail) = (
        second_fields[2].to_string(),
        second_fields[3].to_string(),
        second_fields[4].to_string(),
    );

    assert_eq!(second_id, id, "the re-asked row must carry the exact same identifier as before the reword");
    assert_ne!(second_hash, first_hash, "the reworded text must hash differently -- an edit is still caught");
    assert_ne!(second_detail, first_detail, "the detail should reflect the new heading wording");

    // Move the same section, id intact, into a brand-new topic file: the
    // identifier does not depend on which file it lives in.
    let moved_content = std::fs::read_to_string(&record_path).unwrap();
    write_record(&dir, "adrdom/decisions/moved.md", &moved_content);
    std::fs::remove_file(&record_path).unwrap();

    let moved_ids = extract_ids(&moved_content);
    assert_eq!(
        moved_ids,
        vec![id.clone()],
        "the moved copy must keep the exact same id, not mint a new one"
    );

    let check_out = adr(&dir, &["check"], &[], "");
    let check_text = stdout(&check_out);
    assert!(
        !check_text.contains(&msg_ledger_asked_id_missing(&id)),
        "moving the section to a new file must not orphan its identifier in the ledger: {}",
        check_text
    );
}

/// Mirrors `msg::ledger_asked_id_missing`'s distinguishing text (see
/// src/msg.rs) closely enough to search `check`'s output for it -- the
/// crate has no lib target for tests to call the real formatter.
fn msg_ledger_asked_id_missing(id: &str) -> String {
    format!("asked row id {} not found in any topic file", id)
}

// adr: 99619e27-19b6-47ef-83b9-c79ac30ee2f0
//
// "a VERSION file at the repository root is the one source of the
// version; the binary embeds it, `adr --version` prints it, every ledger
// row carries it": `adr --version` prints exactly the repo-root VERSION
// file's content, and a real `approve`-written ledger row's version
// column is that same string. Cargo.toml's own `version` field is an
// explicit placeholder, not a second source, so there is no sibling
// clause to cover here.
#[test]
fn version_flag_and_ledger_rows_use_the_repository_version_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let version_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("VERSION");
    let version = std::fs::read_to_string(version_path).unwrap();
    let version = version.trim().to_string();
    assert!(!version.is_empty(), "VERSION file should not be empty");

    let version_out = adr(&dir, &["--version"], &[], "");
    assert!(version_out.status.success(), "{}", stderr(&version_out));
    let printed = stdout(&version_out);
    assert!(
        printed.trim().ends_with(version.as_str()),
        "`adr --version` should print the repo-root VERSION file's content: printed {:?}, VERSION file {:?}",
        printed,
        version
    );
    // A build that takes the owner bypass says so; the build the owner
    // installs, with no feature, prints the bare number.
    let marked = printed.contains("test build with the owner bypass");
    assert_eq!(marked, cfg!(feature = "owner-bypass"), "{:?}", printed);

    let terms = "# Terms\n\n## pass\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A clear yes.\naliases: passes\n";
    let terms_path = write_record(&dir, "adrdom/terms.md", terms);
    let approve_terms_out = adr_owner(&dir, &["approve", terms_path.to_str().unwrap()], "");
    assert!(approve_terms_out.status.success(), "{}", stderr(&approve_terms_out));

    let ledger_content = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let terms_row = ledger_content
        .lines()
        .filter(|l| l.contains("\tterms\t"))
        .next_back()
        .expect("a terms ledger row from the real approve above");
    let fields: Vec<&str> = terms_row.split('\t').collect();
    assert_eq!(fields.len(), 6, "{}", terms_row);
    assert_eq!(
        fields[5], version,
        "the ledger row's version column should be the repo-root VERSION file's content"
    );
}

#[test]
fn ledger_accepts_new_event_kinds() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let mut content = std::fs::read_to_string(&ledger_path).unwrap();
    for event in ["withdrawn", "approved", "context", "terms", "unbanned"] {
        content.push_str(&format!(
            "2024-01-01T00:00:00Z\t{}\t-\t-\tsomedetail\n",
            event
        ));
    }
    std::fs::write(&ledger_path, content).unwrap();

    let out = adr(&dir, &["check"], &[], "");
    let out_text = stdout(&out);
    assert!(
        !out_text.contains("malformed row"),
        "new event kinds should not be treated as malformed: {}",
        out_text
    );
}
