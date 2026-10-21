// One question heading stands in one topic file only: approve refuses a
// heading that also stands in another topic file and writes nothing; check
// prints it as a note while one of the files awaits approval and fails it
// once both are approved.
mod common;

use common::*;
use std::path::Path;

const A: &str = "adrdom/decisions/a.md";
const B: &str = "adrdom/decisions/b.md";

const OPTS: &str = "- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";

fn file(title: &str) -> String {
    format!(
        "# {}\n\n### Does the widget hold?\ndepends: widget\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n{}",
        title, OPTS
    )
}

fn repo() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget"]);
    dir
}

fn failures(dir: &Path) -> String {
    check_failures(dir)
}

fn ledger(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default()
}

const LINE: &str = "duplicate heading \"Does the widget hold?\" in adrdom/decisions/a.md and adrdom/decisions/b.md";

#[test]
fn approve_refuses_a_heading_that_stands_in_another_topic_file_and_writes_nothing() {
    // adr: 23f21671-e34c-44aa-8804-6163d975807f
    let dir = repo();
    write_record(&dir, A, &file("A"));
    approve_record(&dir, A);
    let record = write_record(&dir, B, &file("B"));

    let before = ledger(&dir);
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains(LINE) && err.contains("agent rewrites one of the two sections"),
        "{}",
        err
    );
    assert_eq!(ledger(&dir), before, "a refused approve writes no row");
    assert!(
        !std::fs::read_to_string(&record).unwrap().contains("id: "),
        "a refused approve mints no id"
    );

    // The same refusal from the other file's side.
    let dir = repo();
    write_record(&dir, B, &file("B"));
    approve_record(&dir, B);
    let record = write_record(&dir, A, &file("A"));
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success() && stderr(&out).contains(LINE),
        "{}",
        stderr(&out)
    );

    // Renamed, it approves.
    let renamed = file("A").replace("Does the widget hold?", "Does the widget stay?");
    std::fs::write(dir.join(A), renamed).unwrap();
    let out = adr_owner(&dir, &["approve", dir.join(A).to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn a_copied_heading_is_never_asked_because_approve_refuses_it() {
    // adr: 23f21671-e34c-44aa-8804-6163d975807f
    let dir = repo();
    write_record(&dir, A, &file("A"));
    approve_record(&dir, A);
    let asked_before = asked_row_count(&dir);
    let record = write_record(&dir, B, &file("B"));
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(
        !out.status.success() && stderr(&out).contains(LINE),
        "{}",
        stderr(&out)
    );
    assert_eq!(asked_row_count(&dir), asked_before);
}

#[test]
fn approve_of_an_unchanged_question_passes_while_a_draft_copies_its_heading() {
    // adr: 23f21671-e34c-44aa-8804-6163d975807f
    let dir = repo();
    let a = format!("# A\n\n{}", answered(&dir, ID1));
    approve_via_ledger(&dir, A, &a);
    let b = write_record(&dir, B, &file("B"));
    // A prose edit above the first question; the question itself is unchanged.
    let edited = a.replacen("# A\n\n", "# A\n\nA line of prose.\n\n", 1);
    let record = write_record(&dir, A, &edited);
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let out = adr_owner(&dir, &["approve", b.to_str().unwrap()], "");
    assert!(
        !out.status.success() && stderr(&out).contains(LINE),
        "{}",
        stderr(&out)
    );
}

#[test]
fn check_notes_a_heading_in_two_files_while_one_awaits_approval() {
    // adr: 23f21671-e34c-44aa-8804-6163d975807f
    let dir = repo();
    write_record(&dir, A, &file("A"));
    write_record(&dir, B, &file("B"));
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains(LINE), "{}", stdout(&out));
    assert!(failures(&dir).is_empty());

    let dir = repo();
    write_record(&dir, A, &file("A"));
    approve_record(&dir, A);
    write_record(&dir, B, &file("B"));
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains(LINE), "{}", stdout(&out));
}

#[test]
fn check_fails_a_heading_in_two_files_once_both_are_approved() {
    // adr: 23f21671-e34c-44aa-8804-6163d975807f
    let dir = repo();
    approve_via_ledger(&dir, A, &file("A"));
    approve_via_ledger(&dir, B, &file("B"));
    let failed = failures(&dir);
    assert!(failed.contains(LINE), "{}", failed);
}

const ID1: &str = "11111111-1111-1111-1111-111111111111";
const ID2: &str = "22222222-2222-2222-2222-222222222222";

/// The widget question as an answered decision with `id`, ledger rows
/// written, ready to put in a topic file.
fn answered(dir: &Path, id: &str) -> String {
    let term = std::fs::read_to_string(dir.join("adrdom/terms.md"))
        .unwrap()
        .lines()
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .trim()
        .to_string();
    let section = format!(
        "### Does the widget hold?\ndepends: {t}\n\nid: {id}\n\nterms: widget={t}\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n{o}",
        t = term,
        id = id,
        o = OPTS.trim_end_matches('\n')
    );
    let hash = mirror_section_hash(&section);
    append_ledger_row_at(
        dir,
        "asked",
        id,
        &hash,
        "Does the widget hold?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(dir, "answered", id, &hash, "A", "2024-01-01T00:00:01Z");
    format!("{}\n\n**Answer:** A\n", section)
}

#[test]
fn a_draft_question_copying_an_answered_heading_is_a_note_and_two_answered_ones_fail() {
    // adr: 05141f49-d9d2-407c-b60c-ebb195a05864
    let dir = repo();
    let a = format!("# A\n\n{}", answered(&dir, ID1));
    approve_via_ledger(&dir, A, &a);
    write_record(&dir, B, &file("B"));
    let failed = failures(&dir);
    assert!(!failed.contains("duplicate heading"), "{}", failed);
    let all = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(all.contains(LINE), "{}", all);

    // Both questions answered: nobody can still rename, so it fails, though
    // the second file awaits approval.
    let b = format!("# B\n\n{}", answered(&dir, ID2));
    write_record(&dir, B, &b);
    let failed = failures(&dir);
    assert!(failed.contains(LINE), "{}", failed);
}

/// `file(title)` with an id on its question, approved by ledger rows the way
/// `approve` writes them: one row for the question's hash, one for the file.
fn approved_with_id(dir: &Path, rel: &str, title: &str, id: &str) -> String {
    let text = file(title).replacen(
        "depends: widget\n",
        &format!("depends: widget\n\nid: {}\n", id),
        1,
    );
    std::fs::write(dir.join(rel), &text).unwrap();
    let rf = adr::record::load(&dir.join(rel)).unwrap();
    for s in &rf.sections {
        append_ledger_row(
            dir,
            "approved",
            id,
            &adr::record::question_hash(&rf, s),
            rel,
        );
    }
    approve_via_ledger(dir, rel, &text);
    text
}

#[test]
fn check_fails_a_heading_in_two_files_when_the_draft_holds_it_unchanged() {
    // adr: 23f21671-e34c-44aa-8804-6163d975807f
    // adr: 05141f49-d9d2-407c-b60c-ebb195a05864
    let dir = repo();
    let a = approved_with_id(&dir, A, "A", ID1);
    approved_with_id(&dir, B, "B", ID2);
    // A prose edit makes A a draft; its question is unchanged, so it is no copy.
    write_record(
        &dir,
        A,
        &a.replacen("# A\n\n", "# A\n\nA line of prose.\n\n", 1),
    );
    let failed = failures(&dir);
    assert!(failed.contains(LINE), "{}", failed);
}

#[test]
fn check_notes_a_new_heading_in_a_draft_that_copies_an_approved_one() {
    // adr: 23f21671-e34c-44aa-8804-6163d975807f
    // adr: 05141f49-d9d2-407c-b60c-ebb195a05864
    let dir = repo();
    approved_with_id(&dir, A, "A", ID1);
    let b = file("B").replacen("### Does the widget hold?", "### Does the widget stay?", 1);
    approve_via_ledger(&dir, B, &b);
    // B gains a question copying A's heading: new, forceless, in a draft.
    write_record(
        &dir,
        B,
        &format!("{}\n{}", b, file("B").split_once("\n\n").unwrap().1),
    );
    let failed = failures(&dir);
    assert!(!failed.contains(LINE), "{}", failed);
    let all = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(all.contains(LINE), "{}", all);
}
