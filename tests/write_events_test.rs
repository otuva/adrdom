// What a command that writes prints and leaves behind at the edges: a file
// written with no row to cover it, a ledger that refuses the rows, the
// words given to unban, and the files install and uninstall write (per
// "What does a command that writes print?", 069a6b4a A).

mod common;

use common::*;
use std::path::Path;

const REL: &str = "adrdom/decisions/gate.md";
const ONE: &str = "Is the widget gate open?";

fn commit_all(dir: &Path) {
    run_git(dir, &["add", "-A"]);
    assert!(run_git(dir, &["commit", "-q", "-m", "step"]).status.success());
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// Approve that rewrites a topic file and writes no row (a reference line
// written back in words, its hash the approved one) names the file it wrote.
#[test]
fn approve_names_a_file_it_rewrote_with_no_row() {
    let dir = widget_repo();
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}", gate_question(ONE, "the widget breaks.")));
    approve_record(&dir, REL);
    let text = std::fs::read_to_string(&path).unwrap();
    // CRLF line ends: the hash reads the lines, the bytes differ.
    std::fs::write(&path, text.replace('\n', "\r\n")).unwrap();
    let out = approve_unplanned(&dir, &path);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), format!("written: {}\n", REL));
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// An asked question's terms line deleted by hand: ask writes it back, a
// file written with no row, and says so in one line; check then passes it.
#[test]
fn ask_writes_back_a_terms_line_and_names_the_file() {
    let dir = widget_repo();
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}", gate_question(ONE, "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    let text = std::fs::read_to_string(&path).unwrap();
    let without: String = text.lines().filter(|l| !l.starts_with("terms: ")).map(|l| format!("{}\n", l)).collect();
    std::fs::write(&path, without).unwrap();
    let out = adr(&dir, &["ask", path.to_str().unwrap()], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), format!("written: {}\n", REL));
    assert!(std::fs::read_to_string(&path).unwrap().contains("\nterms: "));
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// A ledger that refuses the rows leaves the file as it was: ask writes no
// terms line for rows no ledger holds, and running it again asks.
#[cfg(unix)]
#[test]
fn ask_leaves_the_file_as_it_was_when_the_ledger_refuses() {
    let dir = widget_repo();
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}", gate_question(ONE, "the widget breaks.")));
    approve_record(&dir, REL);
    let before = std::fs::read_to_string(&path).unwrap();
    let ledger = dir.join("adrdom/state/ledger.tsv");
    set_mode(&ledger, 0o444);
    let out = adr(&dir, &["ask", path.to_str().unwrap()], &[], "");
    set_mode(&ledger, 0o644);
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    let again = adr(&dir, &["ask", path.to_str().unwrap()], &[], "");
    assert!(again.status.success(), "{}", stderr(&again));
    assert_eq!(stdout(&again), format!("asked: \"{}\"\n", ONE));
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// Approve of the terms file that the ledger refuses leaves the terms file
// and the terms brief as they were.
#[cfg(unix)]
#[test]
fn terms_approve_leaves_its_files_as_they_were_when_the_ledger_refuses() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = "# Terms\n\n## widget\ndefinition: A part of a gadget.\naliases: \n";
    std::fs::write(dir.join("adrdom/terms.md"), terms).unwrap();
    let ledger = dir.join("adrdom/state/ledger.tsv");
    set_mode(&ledger, 0o444);
    let out = adr_owner(&dir, &["approve", dir.join("adrdom/terms.md").to_str().unwrap()], "");
    set_mode(&ledger, 0o644);
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap(), terms);
    assert!(!dir.join("adrdom/state/terms-brief.md").exists());
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// A supersedes line that names one decision twice is found before
// approval, as a finding of the file.
#[test]
fn a_supersedes_line_naming_one_decision_twice_is_found() {
    let dir = widget_repo();
    let path = write_record(&dir, REL, &format!("# Gate\n\n{}", gate_question(ONE, "the widget breaks.")));
    approve_record(&dir, REL);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    let text = std::fs::read_to_string(&path).unwrap();
    let id = extract_ids(&text)[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    let two = "Is the widget gate shut?";
    let q2 = gate_question(two, "the widget breaks.")
        .replacen("depends: widget\n", &format!("depends: widget\nsupersedes: {}, {}\n", ONE, id), 1);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{}\n{}", text, q2)).unwrap();
    let shown = stdout(&adr(&dir, &["show", REL], &[], ""));
    assert!(shown.contains("names what an earlier entry of the line names"), "{}", shown);
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// Unban counts the words given, not the hashes of each: two words, none
// banned, read as two; what is banned is what the ledger says, whatever the
// ban list holds.
#[test]
fn unban_counts_words_and_reads_the_ledger() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let out = adr_owner(&dir, &["unban"], "alpha beta\ngamma\n");
    assert!(stderr(&out).contains("2 of the words given were not banned"), "{}", stderr(&out));

    assert!(adr(&dir, &["ban"], &[], "zorblax\n").status.success());
    // The list loses the word by hand: the ledger still says it is banned.
    std::fs::write(dir.join("adrdom/state/banned.sha256"), "").unwrap();
    let out = adr_owner(&dir, &["unban"], "zorblax\n");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "unbanned: a word\n");
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// A word given to ban that stands in the repository: each place is a
// finding with the move that clears it, as check names a banned word's
// place.
#[test]
fn ban_names_each_place_a_word_stands_with_its_move() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::write(dir.join("notes.md"), "a zorblax here\n").unwrap();
    commit_all(&dir);
    let out = adr(&dir, &["ban"], &[], "zorblax\n");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("ban: notes.md:1: holds a word given; agent rewrites the line without the word, then runs: adr ban"), "{}", err);
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// `ban` with nothing new and `ban --rebuild` bring a missing ban list to
// the ledger's set alike: both write it, once.
#[test]
fn ban_and_rebuild_bring_a_missing_list_back_alike() {
    for args in [vec!["ban"], vec!["ban", "--rebuild"]] {
        let dir = init_repo();
        assert!(adr(&dir, &["install"], &[], "").status.success());
        std::fs::remove_file(dir.join("adrdom/state/banned.sha256")).unwrap();
        let out = adr(&dir, &args, &[], "");
        assert!(out.status.success(), "{:?}: {}", args, stderr(&out));
        assert_eq!(stdout(&out), "written: adrdom/state/banned.sha256\n", "{:?}", args);
        assert_eq!(stdout(&adr(&dir, &args, &[], "")), "", "{:?}", args);
    }
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// Install names git's configuration it sets, and uninstall the key it
// unsets, each one line; a hook edited by hand keeps running where git
// looks for it.
#[test]
fn install_and_uninstall_name_the_configuration_they_change() {
    let dir = init_repo();
    let out = adr(&dir, &["install"], &[], "");
    assert!(stdout(&out).lines().any(|l| l == "set: core.hooksPath .githooks"), "{}", stdout(&out));
    let again = adr(&dir, &["install"], &[], "");
    assert!(!stdout(&again).contains("core.hooksPath"), "{}", stdout(&again));

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).lines().any(|l| l == "unset: core.hooksPath"), "{}", stdout(&out));

    // A hook edited by hand is left, and so is the configuration that runs it.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    std::fs::write(dir.join(".githooks/pre-push"), "#!/bin/sh\necho mine\n").unwrap();
    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("core.hooksPath"), "{}", stdout(&out));
    let path = run_git(&dir, &["config", "core.hooksPath"]);
    assert_eq!(String::from_utf8_lossy(&path.stdout).trim(), ".githooks");
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// AGENTS.md that cannot be read as text is a failure of uninstall, named,
// never skipped as if install had written nothing there.
#[test]
fn uninstall_names_an_agents_file_it_cannot_read() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let mut bytes = std::fs::read(dir.join("AGENTS.md")).unwrap();
    bytes.extend_from_slice(&[0xff, 0xfe, b'\n']);
    std::fs::write(dir.join("AGENTS.md"), bytes).unwrap();
    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("AGENTS.md"), "{}", stderr(&out));
}

// adr: 069a6b4a-de35-45f3-8c6c-8e254bf40565
//
// A hook saved with CRLF line ends is no hook git can run: check names it
// and install writes it again; the brief's markers saved with CRLF are
// found, so install adds no second brief.
#[test]
fn a_crlf_hook_is_written_again_and_a_crlf_brief_is_found() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let hook = dir.join(".githooks/pre-push");
    let text = std::fs::read_to_string(&hook).unwrap();
    std::fs::write(&hook, text.replace('\n', "\r\n")).unwrap();
    let check = adr(&dir, &["check"], &[], "");
    assert!(stdout(&check).contains(".githooks/pre-push differs from what install writes"), "{}", stdout(&check));
    let out = adr(&dir, &["install"], &[], "");
    assert!(stdout(&out).contains("written: .githooks/pre-push"), "{}", stdout(&out));

    let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    std::fs::write(dir.join("AGENTS.md"), agents.replace('\n', "\r\n")).unwrap();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let after = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert_eq!(after.matches("adrdom brief begin").count(), 1, "{}", after);
}
