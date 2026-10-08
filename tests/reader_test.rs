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
        neighbour: "62e50382-db1b-4649-9f26-7f44c379d6fe".to_string(),
        word: word.to_string(),
        reason: "A reason.".to_string(),
        prompt_hash: Kind::Heading.prompt_hash(),
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

// adr: 26c5352f-3f82-4376-9dc2-8ccc0074d080
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

// adr: 26c5352f-3f82-4376-9dc2-8ccc0074d080
#[test]
fn check_fails_an_edited_or_removed_row_at_head() {
    let dir = committed_repo_with_rows();
    let original = std::fs::read_to_string(reader_path(&dir)).unwrap();

    std::fs::write(reader_path(&dir), original.replacen("same", "depends", 1)).unwrap();
    let f = reader_failures(&dir, &[]);
    assert!(f.iter().any(|l| l.contains("missing or changed")), "{:?}", f);

    let first_only: String = original.lines().take(1).map(|l| format!("{}\n", l)).collect();
    std::fs::write(reader_path(&dir), first_only).unwrap();
    let f = reader_failures(&dir, &[]);
    assert!(f.iter().any(|l| l.contains("missing or changed")), "{:?}", f);

    std::fs::remove_file(reader_path(&dir)).unwrap();
    let f = reader_failures(&dir, &[]);
    assert!(f.iter().any(|l| l.contains("missing or changed")), "{:?}", f);

    // A row may move.
    let swapped: String = original.lines().rev().map(|l| format!("{}\n", l)).collect();
    std::fs::write(reader_path(&dir), swapped).unwrap();
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
}

// adr: 26c5352f-3f82-4376-9dc2-8ccc0074d080
#[test]
fn check_fails_a_malformed_row() {
    let dir = new_repo();
    std::fs::write(reader_path(&dir), "not a row\n").unwrap();
    let f = reader_failures(&dir, &[]);
    assert!(f.iter().any(|l| l.contains("line 1: malformed row")), "{:?}", f);

    // Also a row with a word outside both sets.
    let mut r = row("same", 1);
    r.word = "maybe".to_string();
    std::fs::write(reader_path(&dir), format!("{}\n", reader::row_line(&r))).unwrap();
    assert!(!reader_failures(&dir, &[]).is_empty());
}

// adr: 26c5352f-3f82-4376-9dc2-8ccc0074d080
#[test]
fn check_since_fails_a_row_the_commit_holds_and_the_file_lacks() {
    let dir = committed_repo_with_rows();
    let sha = stdout(&git_ok(&dir, &["rev-parse", "HEAD"])).trim().to_string();
    assert_eq!(reader_failures(&dir, &["--since", &sha]), Vec::<String>::new());
    std::fs::write(reader_path(&dir), "").unwrap();
    commit_all(&dir, "drop the rows");
    // HEAD is a commit without the rows; the failure is the one against --since.
    let f = reader_failures(&dir, &["--since", &sha]);
    assert!(f.iter().any(|l| l.contains(&format!("drops a row {} already holds", sha))), "{:?}", f);
}

// adr: 26c5352f-3f82-4376-9dc2-8ccc0074d080
#[test]
fn a_merge_keeps_both_branches_rows_and_check_fails_one_that_drops_them() {
    let dir = new_repo();
    commit_all(&dir, "base");
    let base = current_branch(&dir);

    git_ok(&dir, &["checkout", "-q", "-b", "a"]);
    reader::append(&dir, &[row("same", 1)]).unwrap();
    commit_all(&dir, "a row");
    git_ok(&dir, &["checkout", "-q", &base]);
    git_ok(&dir, &["checkout", "-q", "-b", "b"]);
    reader::append(&dir, &[row("depends", 2)]).unwrap();
    commit_all(&dir, "b row");

    // A merge that drops branch b's row fails.
    git_ok(&dir, &["checkout", "-q", "a"]);
    git_ok(&dir, &["checkout", "-q", "-b", "drops"]);
    git_ok(&dir, &["merge", "-q", "--no-ff", "-s", "ours", "--no-edit", "b"]);
    let f = reader_failures(&dir, &[]);
    assert!(f.iter().any(|l| l.contains("merge drops a reader row of a merged branch")), "{:?}", f);

    // The union rule keeps both sides' rows with no conflict.
    git_ok(&dir, &["checkout", "-q", "a"]);
    let out = run_git(&dir, &["merge", "--no-ff", "--no-edit", "b"]);
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    let rows = reader::read_file(&dir).rows;
    assert_eq!(rows.len(), 2, "{:?}", rows);
    assert_eq!(reader_failures(&dir, &[]), Vec::<String>::new());
}

// adr: 26c5352f-3f82-4376-9dc2-8ccc0074d080
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

// adr: 26c5352f-3f82-4376-9dc2-8ccc0074d080
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
    }
}

// adr: 08a1b488-32fc-4dc7-b4b3-93a6e768725f
#[test]
fn the_fake_reader_writes_one_row_per_pair_and_skips_pairs_on_record() {
    let dir = new_repo();
    let p = vec![
        pair(Kind::Heading, "Does X? [word:same]", "id-1"),
        pair(Kind::Heading, "Does X? [word:same]", "id-2"),
        pair(Kind::Body, "Does X?\n- A) y [word:inconsistent]", "id-1"),
        pair(Kind::Plan, "Does Y?", &reader::neighbour_key_of_text("Does Z?\nbody")),
    ];
    let rows = reader::read_with(&dir, &reader::Fake, &p).unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows.iter().map(|r| r.word.as_str()).collect::<Vec<_>>(),
        vec!["same", "same", "inconsistent", "unrelated"]
    );
    let r = &rows[0];
    assert_eq!(r.text_hash, adr::record::sha256_hex("Does X? [word:same]"));
    assert_eq!(r.neighbour, "id-1");
    assert_eq!(r.prompt_hash, Kind::Heading.prompt_hash());
    assert_eq!(r.model, reader::MODEL);
    assert!(!r.reason.is_empty());
    assert!(rows[3].neighbour.starts_with("text:"));
    // The text hash is that of the new side alone, the heading and body for a body.
    assert_eq!(rows[2].text_hash, adr::record::sha256_hex("Does X?\n- A) y [word:inconsistent]"));

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
    more.push(pair(Kind::Heading, "Does X? [word:same]", "id-3"));
    reader::read_with(&dir, &reader::Fake, &more).unwrap();
    assert_eq!(reader::read_file(&dir).rows.len(), 5);
}

// adr: 62e50382-db1b-4649-9f26-7f44c379d6fe
#[test]
fn a_word_outside_the_heading_set_is_an_error_and_keeps_no_row() {
    let dir = new_repo();
    let p = vec![
        pair(Kind::Heading, "Does X?", "id-1"),
        pair(Kind::Heading, "Does Y? [word:consistent]", "id-2"),
    ];
    let err = reader::read_with(&dir, &reader::Fake, &p).unwrap_err();
    assert!(err.contains("id-2") && err.contains("not one of same, depends, unrelated"), "{}", err);
    // The whole batch is kept or none of it: the pair that was fine is not written.
    assert!(reader::read_file(&dir).rows.is_empty());
}

// adr: c6e0b7d6-cdec-46da-b2bc-cda218673868
#[test]
fn the_body_set_accepts_its_five_words_and_nothing_else() {
    let dir = new_repo();
    let p: Vec<Pair> = reader::BODY_WORDS
        .iter()
        .enumerate()
        .map(|(i, w)| pair(Kind::Body, &format!("Q{} [word:{}]", i, w), "id-1"))
        .collect();
    let rows = reader::read_with(&dir, &reader::Fake, &p).unwrap();
    assert_eq!(
        rows.iter().map(|r| r.word.as_str()).collect::<Vec<_>>(),
        reader::BODY_WORDS.to_vec()
    );
    let err = reader::read_with(&dir, &reader::Fake, &[pair(Kind::Body, "Q [word:same-ish]", "id-1")]).unwrap_err();
    assert!(err.contains("not one of same, consistent, inconsistent, depends, unrelated"), "{}", err);
}

// adr: 3dba486c-a399-4ce3-a794-e38c4f733403
// The only test of this binary that touches the environment.
#[test]
fn without_the_reader_the_call_stops_with_a_message_and_writes_nothing() {
    let dir = new_repo();
    std::env::set_var("ADR_NO_READER", "1");
    let loaded = reader::load();
    let read = reader::read(&dir, &[pair(Kind::Heading, "Does X?", "id-1")]);
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
