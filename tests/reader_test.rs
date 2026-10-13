// The reader and its file, adrdom/state/reader.tsv. Real git branches and
// merges run inside the harness's own throwaway temp repo.

mod common;

use adr::reader::{self, Kind, Pair};
use common::*;
use std::path::{Path, PathBuf};
use std::process::Output;

fn git_ok(dir: &Path, args: &[&str]) -> Output {
    let out = run_git(dir, args);
    assert!(
        out.status.success(),
        "git {:?} failed: stdout={} stderr={}",
        args,
        stdout(&out),
        stderr(&out)
    );
    out
}

fn commit_all(dir: &Path, msg: &str) {
    git_ok(dir, &["add", "-A"]);
    git_ok(dir, &["commit", "-q", "-m", msg]);
}

fn current_branch(dir: &Path) -> String {
    stdout(&git_ok(dir, &["rev-parse", "--abbrev-ref", "HEAD"])).trim().to_string()
}

/// An installed repo with the hook wiring switched off, so a plain commit
/// is never gated and `adr check` is run exactly where each case wants it.
fn new_repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    run_git(&dir, &["config", "--unset", "core.hooksPath"]);
    dir
}

fn row(word: &str, n: u32) -> reader::Row {
    reader::Row {
        time: format!("2026-10-05T10:00:{:02}Z", n),
        text_hash: adr::record::sha256_hex(&format!("text {}", n)),
        neighbour: "60341be0-fd05-4281-92d7-668584a8539a".to_string(),
        word: word.to_string(),
        reason: "A reason.".to_string(),
        prompt_hash: Kind::Body.prompt_hash(),
        model: reader::MODEL.to_string(),
        version: "0.1.0".to_string(),
    }
}

fn reader_failures(dir: &Path, extra: &[&str]) -> Vec<String> {
    let mut args = vec!["check"];
    args.extend_from_slice(extra);
    stdout(&adr(dir, &args, &[], ""))
        .lines()
        .filter(|l| l.contains("reader"))
        .map(String::from)
        .collect()
}

fn reader_path(dir: &Path) -> PathBuf {
    dir.join("adrdom/state/reader.tsv")
}

fn committed_repo_with_rows() -> PathBuf {
    let dir = new_repo();
    reader::append(&dir, &[row("same", 1), row("depends", 2)]).unwrap();
    commit_all(&dir, "base: two reader rows");
    dir
}

// adr: 30ca5d13-e68b-465f-aa84-ea4000c9330a
#[test]
fn check_passes_a_fresh_repo_and_an_appended_row() {
    let dir = new_repo();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
    // A missing file is fine while there are no rows.
    std::fs::remove_file(reader_path(&dir)).unwrap();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
    std::fs::write(reader_path(&dir), "").unwrap();

    let dir = committed_repo_with_rows();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
    reader::append(&dir, &[row("unrelated", 3)]).unwrap();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
}

// adr: 30ca5d13-e68b-465f-aa84-ea4000c9330a
#[test]
fn check_fails_an_edited_row_at_head_and_passes_a_row_gone_whole() {
    let dir = committed_repo_with_rows();
    let original = std::fs::read_to_string(reader_path(&dir)).unwrap();

    std::fs::write(reader_path(&dir), original.replacen("same", "depends", 1)).unwrap();
    let f = reader_failures(&dir, &[]);
    assert!(f.iter().any(|l| l.contains("edited by hand")), "{:?}", f);

    let first_only: String = original.lines().take(1).map(|l| format!("{}\n", l)).collect();
    std::fs::write(reader_path(&dir), first_only).unwrap();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());

    std::fs::write(reader_path(&dir), "").unwrap();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());

    std::fs::remove_file(reader_path(&dir)).unwrap();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());

    // A row may move.
    let swapped: String = original.lines().rev().map(|l| format!("{}\n", l)).collect();
    std::fs::write(reader_path(&dir), swapped).unwrap();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
}

// adr: 30ca5d13-e68b-465f-aa84-ea4000c9330a
#[test]
fn check_fails_a_malformed_row() {
    let dir = new_repo();
    std::fs::write(reader_path(&dir), "not a row\n").unwrap();
    let f = reader_failures(&dir, &[]);
    assert!(f.iter().any(|l| l.contains("line 1: malformed row")), "{:?}", f);

    // Also a row with a word outside every set.
    let mut r = row("same", 1);
    r.word = "maybe".to_string();
    std::fs::write(reader_path(&dir), format!("{}\n", reader::row_line(&r))).unwrap();
    assert!(!reader_failures(&dir, &[]).is_empty());
}

// adr: 30ca5d13-e68b-465f-aa84-ea4000c9330a
#[test]
fn check_since_fails_a_row_the_commit_holds_edited_and_passes_one_gone_whole() {
    let dir = committed_repo_with_rows();
    let sha = stdout(&git_ok(&dir, &["rev-parse", "HEAD"])).trim().to_string();
    assert_eq!(reader_failures(&dir, &["--since", &sha]), Vec::<String>::new());
    let original = std::fs::read_to_string(reader_path(&dir)).unwrap();
    std::fs::write(reader_path(&dir), "").unwrap();
    commit_all(&dir, "drop the rows");
    assert_eq!(reader_failures(&dir, &["--since", &sha]), Vec::<String>::new());

    std::fs::write(reader_path(&dir), original.replacen("same", "depends", 1)).unwrap();
    commit_all(&dir, "edit a row");
    let f = reader_failures(&dir, &["--since", &sha]);
    assert!(f.iter().any(|l| l.contains(&format!("edits a row {} already holds", sha))), "{:?}", f);
}

// adr: 30ca5d13-e68b-465f-aa84-ea4000c9330a
#[test]
fn a_merge_keeps_both_branches_rows_and_check_fails_one_that_edits_them() {
    let dir = new_repo();
    commit_all(&dir, "base");
    let base = current_branch(&dir);

    git_ok(&dir, &["checkout", "-q", "-b", "a"]);
    reader::append(&dir, &[row("same", 1), row("same", 2)]).unwrap();
    commit_all(&dir, "a rows, one of them with another word than b's");
    git_ok(&dir, &["checkout", "-q", &base]);
    git_ok(&dir, &["checkout", "-q", "-b", "b"]);
    reader::append(&dir, &[row("depends", 2)]).unwrap();
    commit_all(&dir, "b row");

    // A merge that keeps the first parent's rows byte for byte passes, though
    // branch b holds row 2 with another word.
    git_ok(&dir, &["checkout", "-q", "a"]);
    git_ok(&dir, &["checkout", "-q", "-b", "keeps"]);
    git_ok(&dir, &["merge", "-q", "--no-ff", "-s", "ours", "--no-edit", "b"]);
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());

    // A merge whose file holds a line that no parent holds byte for byte, with
    // the identity of a row of a parent, fails.
    git_ok(&dir, &["checkout", "-q", "a"]);
    git_ok(&dir, &["checkout", "-q", "-b", "edits"]);
    git_ok(&dir, &["merge", "-q", "--no-ff", "--no-commit", "-s", "ours", "b"]);
    let edited = format!("{}\n{}\n", reader::row_line(&row("same", 1)), reader::row_line(&row("unrelated", 2)));
    std::fs::write(reader_path(&dir), edited).unwrap();
    commit_all(&dir, "merge b with row 2 edited");
    let f = reader_failures(&dir, &[]);
    assert!(f.iter().any(|l| l.contains("merge edits a reader row of a merged branch")), "{:?}", f);

    // A plain staged edit during a pending merge prints one finding, not two.
    git_ok(&dir, &["checkout", "-q", "a"]);
    git_ok(&dir, &["checkout", "-q", "-b", "pending"]);
    git_ok(&dir, &["merge", "-q", "--no-ff", "--no-commit", "-s", "ours", "b"]);
    std::fs::write(reader_path(&dir), format!("{}\n", reader::row_line(&row("unrelated", 2)))).unwrap();
    git_ok(&dir, &["add", "-A"]);
    let f = reader_failures(&dir, &[]);
    assert_eq!(f.iter().filter(|l| l.contains("edited by hand")).count(), 1, "{:?}", f);
    assert!(!f.iter().any(|l| l.contains("merge edits a reader row")), "{:?}", f);
    git_ok(&dir, &["merge", "--abort"]);

    // A merge that drops branch b's row whole passes.
    git_ok(&dir, &["checkout", "-q", "-f", "a"]);
    git_ok(&dir, &["checkout", "-q", "-b", "drops"]);
    std::fs::write(reader_path(&dir), format!("{}\n", reader::row_line(&row("same", 1)))).unwrap();
    commit_all(&dir, "a without its second row");
    git_ok(&dir, &["merge", "-q", "--no-ff", "-s", "ours", "--no-edit", "b"]);
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());

    // The union rule keeps both sides' rows with no conflict.
    git_ok(&dir, &["checkout", "-q", &base]);
    git_ok(&dir, &["checkout", "-q", "-b", "c"]);
    reader::append(&dir, &[row("same", 5)]).unwrap();
    commit_all(&dir, "c row");
    let out = run_git(&dir, &["merge", "--no-ff", "--no-edit", "b"]);
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    let rows = reader::read_file(&dir).rows;
    assert_eq!(rows.len(), 2, "{:?}", rows);
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
}

// adr: 30ca5d13-e68b-465f-aa84-ea4000c9330a
#[test]
fn install_writes_the_second_merge_rule_and_an_empty_reader_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let attrs = std::fs::read_to_string(dir.join("adrdom/.gitattributes")).unwrap();
    assert_eq!(attrs, "state/ledger.tsv merge=union\nstate/reader.tsv merge=union\n");
    assert_eq!(std::fs::read_to_string(reader_path(&dir)).unwrap(), "");

    // A second install leaves rows alone, and check holds the file's content.
    reader::append(&dir, &[row("same", 1)]).unwrap();
    let before = std::fs::read_to_string(reader_path(&dir)).unwrap();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert_eq!(std::fs::read_to_string(reader_path(&dir)).unwrap(), before);

    std::fs::write(dir.join("adrdom/.gitattributes"), "state/ledger.tsv merge=union\n").unwrap();
    let out = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(out.contains("adrdom/.gitattributes differs"), "{}", out);
}

// adr: 30ca5d13-e68b-465f-aa84-ea4000c9330a
#[test]
fn the_permission_file_forbids_edits_of_the_reader_file() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());
    let settings = std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&settings).unwrap();
    let deny: Vec<&str> = v["permissions"]["deny"].as_array().unwrap().iter().filter_map(|d| d.as_str()).collect();
    // The glob covers adrdom/state/reader.tsv.
    for tool in ["Edit", "Write"] {
        assert!(deny.contains(&format!("{}(adrdom/state/**)", tool).as_str()), "{:?}", deny);
    }
}

fn pair(kind: Kind, new_text: &str, neighbour: &str) -> Pair {
    Pair {
        kind,
        new_text: new_text.to_string(),
        neighbour: neighbour.to_string(),
        neighbour_text: format!("text of {}", neighbour),
        letters: if kind == Kind::Body { vec!['A', 'B'] } else { Vec::new() },
    }
}

/// The records the fake reader's rows stand on: topic file `live.md` with the
/// questions "Does X?" (id-1, a body that makes the fake reader say `same`),
/// "Does Y?" (id-2), "Does W?" (id-3) and "Does Z?" (a body that makes it
/// say `inconsistent`).
fn repo_with_records() -> (PathBuf, adr::record::RecordFile) {
    let dir = new_repo();
    let body = |word: &str| format!("\nProblem: p.\n\nStandard: s.\n\n- A) y [word:{}]\n  Why: w.\n  Costs: c.\n- B) n\n  Why: w.\n  Costs: c.\n", word);
    let content = format!(
        "# Live\n\nTexts for the reader file.\n\n### Does X?\nid: id-1\n{}\n### Does Y?\nid: id-2\n\n### Does W?\nid: id-3\n\n### Does Z?\n{}",
        body("same"),
        body("inconsistent")
    );
    let path = write_record(&dir, "adrdom/decisions/live.md", &content);
    let rf = adr::record::load(&path).unwrap();
    (dir, rf)
}

fn section<'a>(rf: &'a adr::record::RecordFile, heading: &str) -> &'a adr::record::Section {
    rf.sections.iter().find(|s| s.heading_text == heading).unwrap()
}

// adr: 55a6a2d8-03af-4db9-a4c7-e2790e38357e
#[test]
fn the_fake_reader_writes_one_row_per_pair_and_skips_pairs_on_record() {
    let (dir, rf) = repo_with_records();
    let x_body = adr::record::body_text(&rf, section(&rf, "Does X?"));
    let z_body = adr::record::body_text(&rf, section(&rf, "Does Z?"));
    let p = vec![
        pair(Kind::Body, &x_body, "id-1"),
        pair(Kind::Body, &x_body, "id-2"),
        pair(Kind::Body, &z_body, "id-1"),
        pair(Kind::Plan, "Does W?", &reader::neighbour_key_of_text(&z_body)),
    ];
    let rows = reader::read_with(&dir, &reader::Fake, &p).unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows.iter().map(|r| r.word.as_str()).collect::<Vec<_>>(),
        vec!["same", "same", "inconsistent", "unrelated"]
    );
    let r = &rows[0];
    assert_eq!(r.text_hash, adr::record::sha256_hex(&x_body));
    assert_eq!(r.neighbour, "id-1");
    assert_eq!(r.prompt_hash, Kind::Body.prompt_hash());
    assert_eq!(r.model, reader::MODEL);
    assert!(!r.reason.is_empty());
    assert!(rows[3].neighbour.starts_with("text:"));
    // The text hash is that of the new side alone: the question's heading for a plan pair.
    assert_eq!(rows[3].text_hash, adr::record::sha256_hex("Does W?"));

    let file = reader::read_file(&dir);
    assert_eq!(file.rows.len(), 4);
    assert!(file.malformed.is_empty());
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());

    // The same pairs again are not read again: the file does not grow.
    let again = reader::read_with(&dir, &reader::Fake, &p).unwrap();
    assert_eq!(again, rows);
    assert_eq!(reader::read_file(&dir).rows.len(), 4);

    // A new neighbour is read; the others are not.
    let mut more = p.clone();
    more.push(pair(Kind::Body, &x_body, "id-3"));
    reader::read_with(&dir, &reader::Fake, &more).unwrap();
    assert_eq!(reader::read_file(&dir).rows.len(), 5);
}

/// A row on the text `text` and the neighbour `neighbour`.
fn live_row(text: &str, neighbour: &str, word: &str, n: u32) -> reader::Row {
    let mut r = row(word, n);
    r.text_hash = adr::record::sha256_hex(text);
    r.neighbour = neighbour.to_string();
    r
}

// adr: a8034401-420c-45d7-8f43-60c1b9f3baf9
#[test]
fn an_append_drops_rows_whose_text_or_neighbour_is_gone_and_keeps_the_rest_byte_for_byte() {
    let (dir, rf) = repo_with_records();
    let z_body = adr::record::body_text(&rf, section(&rf, "Does Z?"));
    let live_heading = live_row("Does W?", "id-1", "same", 1);
    let live_body = live_row(&z_body, &reader::neighbour_key_of_text(&z_body), "depends", 2);
    let live_term_free = live_row("Does Y?", "id-3", "unrelated", 3);
    let text_gone = live_row("Does a question that was struck?", "id-1", "same", 4);
    let neighbour_gone = live_row("Does W?", "id-struck", "same", 5);
    let text_key_gone = live_row("Does W?", &reader::neighbour_key_of_text("a body that was edited"), "same", 6);
    let kept = [&live_heading, &live_body, &live_term_free];
    let mut file = String::new();
    for r in [&text_gone, &live_heading, &neighbour_gone, &live_body, &text_key_gone, &live_term_free] {
        file.push_str(&reader::row_line(r));
        file.push('\n');
    }
    file.push_str("a line that is no row\n");
    std::fs::write(reader_path(&dir), file).unwrap();

    let new = live_row("Does W?", "id-2", "same", 7);
    reader::append(&dir, &[new.clone()]).unwrap();

    let mut want = String::new();
    for r in kept {
        want.push_str(&reader::row_line(r));
        want.push('\n');
    }
    want.push_str("a line that is no row\n");
    want.push_str(&reader::row_line(&new));
    want.push('\n');
    assert_eq!(std::fs::read_to_string(reader_path(&dir)).unwrap(), want);

    // With no row to append nothing is dropped.
    std::fs::write(reader_path(&dir), format!("{}\n", reader::row_line(&text_gone))).unwrap();
    reader::append(&dir, &[]).unwrap();
    assert_eq!(std::fs::read_to_string(reader_path(&dir)).unwrap(), format!("{}\n", reader::row_line(&text_gone)));
}

// adr: a8034401-420c-45d7-8f43-60c1b9f3baf9
#[test]
fn a_read_drops_rows_of_gone_texts() {
    let (dir, _rf) = repo_with_records();
    let old = live_row("Does a question that was struck?", "id-1", "same", 1);
    std::fs::write(reader_path(&dir), format!("{}\n", reader::row_line(&old))).unwrap();
    let rows = reader::read_with(&dir, &reader::Fake, &[pair(Kind::Plan, "Does X?", "id-2")]).unwrap();
    let file = reader::read_file(&dir);
    assert_eq!(file.rows, rows);
}

// adr: a8034401-420c-45d7-8f43-60c1b9f3baf9
#[test]
fn a_union_merge_brings_back_a_dropped_row_and_check_passes() {
    let (dir, _rf) = repo_with_records();
    let old = live_row("Does a question that was struck?", "id-1", "same", 1);
    std::fs::write(reader_path(&dir), format!("{}\n", reader::row_line(&old))).unwrap();
    commit_all(&dir, "base: a row whose text will be gone");
    let base = current_branch(&dir);

    // Branch a appends, which drops the row.
    git_ok(&dir, &["checkout", "-q", "-b", "a"]);
    let x = "Does X?".to_string();
    reader::append(&dir, &[live_row(&x, "id-2", "same", 2)]).unwrap();
    assert!(!std::fs::read_to_string(reader_path(&dir)).unwrap().contains(&reader::row_line(&old)));
    commit_all(&dir, "a: append, row dropped");

    // Branch b keeps it and adds a row of its own.
    git_ok(&dir, &["checkout", "-q", &base]);
    git_ok(&dir, &["checkout", "-q", "-b", "b"]);
    reader::append(&dir, &[live_row(&x, "id-3", "same", 3)]).unwrap();
    // Appending dropped it on b too; put it back as the file held it.
    let mut file = std::fs::read_to_string(reader_path(&dir)).unwrap();
    file.push_str(&format!("{}\n", reader::row_line(&old)));
    std::fs::write(reader_path(&dir), file).unwrap();
    commit_all(&dir, "b: row kept");

    git_ok(&dir, &["checkout", "-q", "a"]);
    let out = run_git(&dir, &["merge", "--no-ff", "--no-edit", "b"]);
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    let merged = std::fs::read_to_string(reader_path(&dir)).unwrap();
    assert!(merged.contains(&reader::row_line(&old)), "{}", merged);
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
}

// adr: 30ca5d13-e68b-465f-aa84-ea4000c9330a
#[test]
fn check_judges_a_new_line_by_its_shape_alone() {
    let dir = committed_repo_with_rows();
    let original = std::fs::read_to_string(reader_path(&dir)).unwrap();

    // A row written by hand with a good shape passes.
    let by_hand = reader::row_line(&row("unrelated", 9));
    std::fs::write(reader_path(&dir), format!("{}{}\n", original, by_hand)).unwrap();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());

    // A line of a bad shape fails.
    std::fs::write(reader_path(&dir), format!("{}{}\n", original, by_hand.replacen("unrelated", "maybe", 1))).unwrap();
    let f = reader_failures(&dir, &[]);
    assert!(f.iter().any(|l| l.contains("malformed row")), "{:?}", f);

    // An edit of the reason or the version alone is an edit too.
    for (from, to) in [("A reason.", "Another."), ("0.1.0", "0.2.0")] {
        std::fs::write(reader_path(&dir), original.replacen(from, to, 1)).unwrap();
        let f = reader_failures(&dir, &[]);
        assert!(f.iter().any(|l| l.contains("edited by hand")), "{} {:?}", from, f);
    }
}

#[test]
fn a_word_outside_the_plan_set_is_an_error_and_keeps_no_row() {
    let dir = new_repo();
    let p = vec![
        pair(Kind::Plan, "Does X?", "id-1"),
        pair(Kind::Plan, "Does Y? [word:consistent]", "id-2"),
    ];
    let err = reader::read_with(&dir, &reader::Fake, &p).unwrap_err();
    assert!(err.contains("id-2") && err.contains("not one of same, unrelated"), "{}", err);
    // The whole batch is kept or none of it: the pair that was fine is not written.
    assert!(reader::read_file(&dir).rows.is_empty());
}

// adr: d10dab89-1d42-4764-9d72-22d6fd6a9260
#[test]
fn the_body_set_accepts_its_four_words_and_closes_with_a_letter_of_the_question() {
    let dir = new_repo();
    let mut p: Vec<Pair> = reader::BODY_WORDS
        .iter()
        .enumerate()
        .map(|(i, w)| pair(Kind::Body, &format!("Q{} [word:{}]", i, w), "id-1"))
        .collect();
    p.push(pair(Kind::Body, "Q closes [word:closes B]", "id-1"));
    let rows = reader::read_with(&dir, &reader::Fake, &p).unwrap();
    let mut want = reader::BODY_WORDS.to_vec();
    want.push("closes B");
    assert_eq!(rows.iter().map(|r| r.word.as_str()).collect::<Vec<_>>(), want);
    assert_eq!(reader::read_file(&dir).rows.len(), 5);
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
}

// adr: d10dab89-1d42-4764-9d72-22d6fd6a9260
#[test]
fn a_body_word_outside_the_set_is_an_error_and_keeps_no_row() {
    for word in ["same-ish", "depends", "closes", "closes C", "closes AB"] {
        let dir = new_repo();
        let p = vec![
            pair(Kind::Body, "Q fine [word:same]", "id-1"),
            pair(Kind::Body, &format!("Q bad [word:{}]", word), "id-2"),
        ];
        let err = reader::read_with(&dir, &reader::Fake, &p).unwrap_err();
        assert!(
            err.contains("id-2") && err.contains("not one of same, consistent, inconsistent, unrelated, closes and the letter of an option"),
            "{}: {}",
            word,
            err
        );
        assert!(reader::read_file(&dir).rows.is_empty(), "{}", word);
    }
}

// adr: 30ca5d13-e68b-465f-aa84-ea4000c9330a
#[test]
fn check_passes_the_words_depends_and_closes_with_a_letter_and_fails_closes_alone() {
    let dir = new_repo();
    let line = |w: &str, n| format!("{}\n", reader::row_line(&row(w, n)));
    std::fs::write(reader_path(&dir), format!("{}{}", line("depends", 1), line("closes B", 2))).unwrap();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
    commit_all(&dir, "rows with depends and closes B");
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());

    for bad in ["closes", "closes b"] {
        std::fs::write(reader_path(&dir), line(bad, 3)).unwrap();
        let f = reader_failures(&dir, &[]);
        assert!(f.iter().any(|l| l.contains("malformed row")), "{} {:?}", bad, f);
    }
}

// adr: bd94a89f-7142-4dab-b757-6482a718a0a0
// The only test of this binary that touches the environment.
#[test]
fn without_the_reader_the_call_stops_with_a_message_and_writes_nothing() {
    let dir = new_repo();
    std::env::set_var("ADR_NO_READER", "1");
    let loaded = reader::load();
    let read = reader::read(&dir, &[pair(Kind::Body, "Does X?", "id-1")]);
    std::env::remove_var("ADR_NO_READER");
    for e in [loaded.err().expect("load refuses"), read.unwrap_err()] {
        assert!(e.contains("the claude program is missing"), "{}", e);
        assert!(e.contains("installs the claude CLI, or runs it on another machine"), "{}", e);
    }
    assert!(reader::read_file(&dir).rows.is_empty());
    // Every other command runs without it.
    let out = adr(&dir, &["check"], &[("ADR_NO_READER", "1")], "");
    assert!(!stdout(&out).contains("reader:"), "{}", stdout(&out));
}

// adr: a8034401-420c-45d7-8f43-60c1b9f3baf9
#[test]
fn an_append_drops_nothing_when_a_topic_file_cannot_be_read() {
    let (dir, _rf) = repo_with_records();
    let old = live_row("Does a question that was struck?", "id-1", "same", 1);
    std::fs::write(reader_path(&dir), format!("{}\n", reader::row_line(&old))).unwrap();
    // Invalid UTF-8: the records cannot read this file, so no text is gone.
    std::fs::write(dir.join("adrdom/decisions/broken.md"), [0xff, 0xfe, 0x23, 0x20, 0x0a]).unwrap();
    let new = live_row("Does W?", "id-2", "same", 2);
    reader::append(&dir, &[new.clone()]).unwrap();
    let want = format!("{}\n{}\n", reader::row_line(&old), reader::row_line(&new));
    assert_eq!(std::fs::read_to_string(reader_path(&dir)).unwrap(), want);
}

// adr: a8034401-420c-45d7-8f43-60c1b9f3baf9
#[test]
fn an_append_starts_a_new_line_when_the_file_does_not_end_with_one() {
    let (dir, _rf) = repo_with_records();
    let first = live_row("Does W?", "id-1", "same", 1);
    let second = live_row("Does W?", "id-2", "same", 2);
    std::fs::write(reader_path(&dir), reader::row_line(&first)).unwrap();
    reader::append(&dir, &[second.clone()]).unwrap();
    let want = format!("{}\n{}\n", reader::row_line(&first), reader::row_line(&second));
    assert_eq!(std::fs::read_to_string(reader_path(&dir)).unwrap(), want);
    assert_eq!(reader::read_file(&dir).rows.len(), 2);
}

// adr: a8034401-420c-45d7-8f43-60c1b9f3baf9
#[test]
fn concurrent_appends_with_drops_lose_no_row() {
    use std::io::Write;
    let (dir, _rf) = repo_with_records();
    let threads = 4;
    let each = 25;
    std::thread::scope(|s| {
        for t in 0..threads {
            let dir = &dir;
            s.spawn(move || {
                for i in 0..each {
                    // A row whose text is gone, so the next append rewrites the file.
                    let mut gone = live_row("Does a question that was struck?", "id-1", "same", 1);
                    gone.reason = format!("gone t{} i{}", t, i);
                    let mut f = std::fs::OpenOptions::new().append(true).open(reader_path(dir)).unwrap();
                    f.write_all(format!("{}\n", reader::row_line(&gone)).as_bytes()).unwrap();
                    drop(f);
                    let mut live = live_row("Does W?", "id-1", "same", 1);
                    live.reason = format!("live t{} i{}", t, i);
                    reader::append(dir, &[live]).unwrap();
                }
            });
        }
    });
    let got = std::fs::read_to_string(reader_path(&dir)).unwrap();
    for t in 0..threads {
        for i in 0..each {
            assert!(got.contains(&format!("live t{} i{}", t, i)), "row t{} i{} lost", t, i);
        }
    }
    let state = dir.join("adrdom/state");
    let stray: Vec<_> = std::fs::read_dir(state).unwrap().filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().ends_with(".tmp")).collect();
    assert!(stray.is_empty());
}

// adr: a8034401-420c-45d7-8f43-60c1b9f3baf9
#[test]
fn an_append_removes_the_temporary_files_a_killed_rewrite_left() {
    let (dir, _rf) = repo_with_records();
    let stale = dir.join("adrdom/state/reader.tsv.00000000-0000-0000-0000-000000000000.tmp");
    std::fs::write(&stale, "half a rewrite").unwrap();
    reader::append(&dir, &[live_row("Does W?", "id-1", "same", 1)]).unwrap();
    assert!(!stale.exists());
    // Also when the append rewrites the file.
    std::fs::write(&stale, "half a rewrite").unwrap();
    let gone = live_row("Does a question that was struck?", "id-1", "same", 1);
    let mut f = std::fs::OpenOptions::new().append(true).open(reader_path(&dir)).unwrap();
    std::io::Write::write_all(&mut f, format!("{}\n", reader::row_line(&gone)).as_bytes()).unwrap();
    drop(f);
    reader::append(&dir, &[live_row("Does W?", "id-2", "same", 2)]).unwrap();
    assert!(!stale.exists());
}
