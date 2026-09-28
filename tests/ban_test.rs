mod common;

use common::*;

// adr: 4f04bae4-98e3-48eb-9403-3257268e2d87
//
// "Does ban refuse a word that appears in the repository?": ban scans
// every tracked file first, and a hit refuses naming the file and line --
// never the matched word itself -- and writes nothing to either the
// banned-word hash file or the ledger.
#[test]
fn ban_refuses_on_a_word_already_in_use_and_writes_nothing() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    std::fs::write(
        dir.join("notes.txt"),
        "this line mentions gadgetword right here\n",
    )
    .unwrap();
    run_git(&dir, &["add", "notes.txt"]);

    let out = adr(&dir, &["ban"], &[], "gadgetword\n");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("notes.txt:1"), "{}", err);
    assert!(
        !err.to_lowercase().contains("gadgetword"),
        "the matched word must never reach stdout/stderr: {}",
        err
    );

    let banned = std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert!(
        banned.trim().is_empty(),
        "ban must write nothing on refusal: {}",
        banned
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.lines().any(|l| l.contains("\tbanned\t")));
}

#[test]
fn ban_writes_two_column_lines_with_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let term_id = "11111111-1111-1111-1111-111111111111";
    let terms_content = format!(
        "# Terms\n\n## widget\nid: {}\ndefinition: A small thing.\naliases: gadget\n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();

    let out = adr(&dir, &["ban", "--term", "widget"], &[], "forbiddenword\n");
    assert!(out.status.success(), "ban failed: {}", stderr(&out));

    let banned = std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert!(
        banned.lines().any(|l| l.ends_with(&format!(" {}", term_id))),
        "expected a two-column line carrying the term uuid: {}",
        banned
    );

    // adr: fcc5b6fe-9072-42ea-a25b-2be72e094bac
    // The repository holds only the hash of a banned word, never the word
    // itself: no plaintext of the banned word appears in the file, and
    // every hash column is a 64-character hex sha256 digest.
    assert!(
        !banned.to_lowercase().contains("forbiddenword"),
        "banned list must hold only a hash, never the plaintext word: {}",
        banned
    );
    for line in banned.lines() {
        let hash = line.split(' ').next().unwrap();
        assert_eq!(hash.len(), 64, "expected a sha256 hex digest: {}", line);
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "expected a hex digest, not plaintext: {}",
            line
        );
    }

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger
            .lines()
            // detail is the 5th (not last) tab-separated field now that
            // every row carries a trailing version column.
            .any(|l| l.contains("\tbanned\t") && l.contains(&format!("\t{}\t", term_id))),
        "expected a banned row with the term uuid as detail: {}",
        ledger
    );

    // Matching by alias, case-insensitively, resolves to the same term.
    let out2 = adr(&dir, &["ban", "--term", "GADGET"], &[], "anotherword\n");
    assert!(out2.status.success(), "ban failed: {}", stderr(&out2));
    let banned2 = std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert_eq!(
        banned2
            .lines()
            .filter(|l| l.ends_with(&format!(" {}", term_id)))
            .count(),
        2,
        "{}",
        banned2
    );
}

#[test]
fn ban_refuses_unknown_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let out = adr(&dir, &["ban", "--term", "nosuchterm"], &[], "someword\n");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("nosuchterm"), "{}", stderr(&out));

    let banned = std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert!(banned.trim().is_empty());

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.lines().any(|l| l.contains("\tbanned\t")));
}

// adr: c6da863d-5085-402f-815a-9bb832b4df7e
// One hashed ban list carries both a term-tagged word (banned with
// `--term`) and a plain one (banned with no term), and one `check` scan
// over that single list catches both.
#[test]
fn check_banned_scanner_names_the_term() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let term_id = "22222222-2222-2222-2222-222222222222";
    let terms_content = format!(
        "# Terms\n\n## widget\nid: {}\ndefinition: A small thing.\naliases: \n",
        term_id
    );
    std::fs::write(dir.join("adrdom/terms.md"), terms_content).unwrap();

    let ban_out = adr(&dir, &["ban", "--term", "widget"], &[], "forbiddenword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));
    let ban_out2 = adr(&dir, &["ban"], &[], "plainword\n");
    assert!(ban_out2.status.success(), "ban failed: {}", stderr(&ban_out2));

    // Both words live in the one ban list file.
    let banned = std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert_eq!(banned.lines().count(), 2, "expected one list, two lines: {}", banned);

    std::fs::write(
        dir.join("notes.txt"),
        "this mentions forbiddenword and plainword right here\n",
    )
    .unwrap();
    run_git(&dir, &["add", "notes.txt"]);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    let hits: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("banned: notes.txt:1: banned word"))
        .collect();
    assert_eq!(hits.len(), 2, "expected one hit per word: {}", text);
    assert!(
        hits.iter().any(|l| l.contains("(term widget)")),
        "{}",
        text
    );
    assert!(
        hits
            .iter()
            .any(|l| l.starts_with("banned: notes.txt:1: banned word;")),
        "{}",
        text
    );
    assert!(!text.to_lowercase().contains("forbiddenword"), "{}", text);
    assert!(!text.to_lowercase().contains("plainword"), "{}", text);
}

#[test]
fn check_fails_when_banned_file_and_ledger_disagree_both_ways() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    // The ledger says a hash is currently banned; the file is missing it.
    append_ledger_row(
        &dir,
        "banned",
        "-",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "",
    );

    // The file has a hash the ledger never recorded a `banned` row for.
    std::fs::write(
        dir.join("adrdom/state/banned.sha256"),
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n",
    )
    .unwrap();

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("banned: aaaaaaaaaaaa in ledger, missing from adrdom/state/banned.sha256"),
        "{}",
        text
    );
    assert!(
        text.contains("banned: bbbbbbbbbbbb in file, no ledger row"),
        "{}",
        text
    );
}

#[test]
fn ban_ignores_a_word_present_only_in_a_ledger_row_detail() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    // As `adr ask` would have written for a question heading that
    // mentioned "gadgetword", followed by the row `adr answer` writes once
    // that section is superseded (the section itself then deleted from
    // the record, per the fixture's premise). The word now lives only in
    // this `asked` row's detail column.
    append_ledger_row_at(
        &dir,
        "asked",
        "33333333-3333-3333-3333-333333333333",
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        "1.1 What handles gadgetword?",
        "2024-01-01T00:00:00Z",
    );
    append_ledger_row_at(
        &dir,
        "superseded",
        "33333333-3333-3333-3333-333333333333",
        "-",
        "44444444-4444-4444-4444-444444444444",
        "2024-01-01T00:00:01Z",
    );

    // history is not scanned: the ledger copy of the word never blocks `ban`.
    let out = adr(&dir, &["ban"], &[], "gadgetword\n");
    assert!(out.status.success(), "ban failed: {}", stderr(&out));

    let banned = std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert!(!banned.trim().is_empty(), "{}", banned);

    // ...nor does `check`'s scanner report it once banned.
    let out2 = adr(&dir, &["check"], &[], "");
    assert!(out2.status.success(), "check failed: {}", stdout(&out2));
    let text = stdout(&out2);
    assert!(!text.to_lowercase().contains("gadgetword"), "{}", text);
}

#[test]
fn check_passes_when_banned_file_and_ledger_agree() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let ban_out = adr(&dir, &["ban"], &[], "quietword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains("in ledger, missing from") && !text.contains("in file, no ledger row"),
        "{}",
        text
    );
}
