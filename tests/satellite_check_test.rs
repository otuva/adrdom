// What check and show do in a satellite and in its primary: the satellite's
// own files are judged against the primary's record, read from the primary's
// working tree, and the primary counts the ties its satellites hold.
mod common;

use common::*;
use std::path::{Path, PathBuf};

const SECOND: &str = "# Second\n\n### 2.1 What is the second unit of authority called?\ndepends: authority\n\nProblem: naming the second unit.\n\nStandard: plain words.\n\n- A) The second thing is a widget.\n  Why: reasons.\n  Costs: costs.\n- B) The second thing is a gadget.\n  Why: other reasons.\n  Costs: other costs.\n";
const STRAY: &str = "11111111-2222-3333-4444-555555555555";

fn status(dir: &Path) -> String {
    stdout(&run_git(dir, &["status", "--porcelain"]))
}

/// What the satellite's `adrdom/primary` holds.
fn primary_line(sat: &Path) -> String {
    std::fs::read_to_string(sat.join("adrdom/primary"))
        .unwrap()
        .trim()
        .to_string()
}

/// The satellite's path from the primary, as its row holds it.
fn sat_path(primary: &Path, sat: &Path) -> String {
    relative_between(primary, sat)
}

/// Writes `text` at `rel` in `dir`, stages it and commits it; the commit
/// runs the repository's hooks.
fn commit_file(dir: &Path, rel: &str, text: &str) -> std::process::Output {
    std::fs::write(dir.join(rel), text).unwrap();
    assert!(run_git(dir, &["add", rel]).status.success());
    run_git(dir, &["commit", "-q", "-m", "chore: add a file"])
}

fn ban(primary: &Path, word: &str) {
    let out = adr(primary, &["ban"], &[], &format!("{}\n", word));
    assert!(out.status.success(), "{}", stderr(&out));
}

/// A second decision answered in the primary and left uncommitted.
fn second_decision(primary: &Path) -> String {
    let record = write_record(primary, "adrdom/decisions/second.md", SECOND);
    approve_record(primary, "adrdom/decisions/second.md");
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(primary, &id, "A").status.success());
    id
}

/// The satellite moved to a sibling name, as a checkout that is not there.
fn move_away(dir: &Path) -> PathBuf {
    let moved = PathBuf::from(format!("{}-moved", dir.display()));
    std::fs::rename(dir, &moved).unwrap();
    moved
}

// adr: cb15b115-610f-4c82-a8f7-6bae9c131dc0
#[test]
fn a_word_banned_in_the_primary_and_not_committed_refuses_a_satellite_commit() {
    let (primary, sat, _) = satellite_pair();
    ban(&primary, "gadgetword");
    let c = commit_file(&sat, "notes.txt", "a gadgetword here\n");
    assert!(!c.status.success(), "{}", stderr(&c));
    assert!(stderr(&c).contains("banned"), "{}", stderr(&c));
    let ok = commit_file(&sat, "notes.txt", "a plain line\n");
    assert!(ok.status.success(), "{}", stderr(&ok));
}

// adr: cb15b115-610f-4c82-a8f7-6bae9c131dc0
#[test]
fn a_decision_answered_in_the_primary_and_not_committed_is_current_in_the_satellite() {
    let (primary, sat, _) = satellite_pair();
    let id = second_decision(&primary);
    let c = commit_file(&sat, "tied.txt", &format!("adr: {}\n", id));
    assert!(c.status.success(), "{}", stderr(&c));
    let out = adr(&sat, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(!stdout(&out).contains(&id), "{}", stdout(&out));
    // A tie to an id no decision holds is reported there.
    std::fs::write(sat.join("tied.txt"), format!("adr: {}\n", STRAY)).unwrap();
    let out = adr(&sat, &["check"], &[], "");
    assert!(stdout(&out).contains(STRAY), "{}", stdout(&out));
}

// adr: cb15b115-610f-4c82-a8f7-6bae9c131dc0
#[test]
fn the_primary_working_tree_wins_over_its_index_at_a_staged_satellite_commit() {
    let (primary, sat, _) = satellite_pair();
    ban(&primary, "gadgetword");
    assert!(run_git(&primary, &["add", "-A"]).status.success());
    let c = run_git(&primary, &["commit", "-q", "-m", "chore: ban"]);
    assert!(c.status.success(), "{}{}", stdout(&c), stderr(&c));
    // The index and HEAD of the primary still hold the hash; the working
    // tree does not.
    let list = primary.join("adrdom/state/banned.sha256");
    assert!(!std::fs::read_to_string(&list).unwrap().trim().is_empty());
    std::fs::write(&list, "").unwrap();
    let c = commit_file(&sat, "notes.txt", "a gadgetword here\n");
    assert!(c.status.success(), "{}", stderr(&c));
}

// adr: cb15b115-610f-4c82-a8f7-6bae9c131dc0
#[test]
fn with_the_primary_moved_away_check_in_the_satellite_stops() {
    let (primary, sat, _) = satellite_pair();
    let held = primary_line(&sat);
    move_away(&primary);
    for args in [&["check"][..], &["check", "--staged"], &["check", "--push"]] {
        let out = adr(&sat, args, &[], "");
        assert!(!out.status.success(), "{:?}", args);
        let err = stderr(&out);
        assert!(err.contains("record not found at"), "{:?}: {}", args, err);
        assert!(err.contains(&held), "{:?}: {}", args, err);
    }
}

// adr: cb15b115-610f-4c82-a8f7-6bae9c131dc0
#[test]
fn a_fault_in_the_primary_record_fails_the_primary_check_and_not_the_satellite() {
    let (primary, sat, _) = satellite_pair();
    let topic = primary.join("adrdom/decisions/example.md");
    let text = std::fs::read_to_string(&topic).unwrap();
    std::fs::write(&topic, text.replace("plain words", "other words")).unwrap();
    let out = adr(&primary, &["check"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    let out = adr(&sat, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    let c = commit_file(&sat, "notes.txt", "a plain line\n");
    assert!(c.status.success(), "{}", stderr(&c));
}

// adr: 13ddec51-ce03-4645-99ae-2bbf4f30712d
#[test]
fn a_tie_in_a_satellite_clears_the_untested_note_and_the_build_move_of_the_primary() {
    let (primary, sat, id) = satellite_pair();
    let out = adr(&primary, &["check"], &[], "");
    assert!(
        stdout(&out).contains("is answered and no tracked file holds its tie"),
        "{}",
        stdout(&out)
    );
    let shown = adr(&primary, &["show"], &[], "");
    assert!(
        stdout(&shown).contains("in a test that shows it"),
        "{}",
        stdout(&shown)
    );
    let c = commit_file(&sat, "tied.txt", &format!("adr: {}\n", id));
    assert!(c.status.success(), "{}", stderr(&c));
    let out = adr(&primary, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        !stdout(&out).contains("no tracked file holds its tie"),
        "{}",
        stdout(&out)
    );
    let shown = adr(&primary, &["show"], &[], "");
    assert!(
        !stdout(&shown).contains("in a test that shows it"),
        "{}",
        stdout(&shown)
    );
}

// adr: 13ddec51-ce03-4645-99ae-2bbf4f30712d
#[test]
fn a_stale_tie_in_a_satellite_is_reported_by_the_primary_with_the_satellite_path() {
    let (primary, sat, _) = satellite_pair();
    assert!(
        commit_file(&sat, "tied.txt", &format!("one\nadr: {}\n", STRAY))
            .status
            .success()
    );
    let out = adr(&primary, &["check"], &[], "");
    let want = format!("{}/tied.txt:2", sat_path(&primary, &sat));
    assert!(stdout(&out).contains(&want), "{}", stdout(&out));
}

// adr: 30063002-420c-4309-bae2-4175b993f8f1
#[test]
fn show_in_a_satellite_names_its_primary_first_and_lists_its_decisions() {
    let (primary, sat, id) = satellite_pair();
    let first = format!(
        "record: {}",
        std::fs::canonicalize(&primary).unwrap().display()
    );
    for args in [
        &["show"][..],
        &["show", "--ids"],
        &["show", "adrdom/decisions/example.md"],
        &["show", "--all"],
    ] {
        let out = adr(&sat, args, &[], "");
        assert!(out.status.success(), "{:?}: {}", args, stderr(&out));
        let text = stdout(&out);
        assert_eq!(
            text.lines().next(),
            Some(first.as_str()),
            "{:?}: {}",
            args,
            text
        );
        assert!(
            text.contains("What is the unit of authority called?"),
            "{:?}: {}",
            args,
            text
        );
    }
    let out = adr(&sat, &["show", "--ids"], &[], "");
    assert!(stdout(&out).contains(&id), "{}", stdout(&out));
}

#[test]
fn show_file_in_a_satellite_takes_a_path_from_the_satellite_or_through_the_primary() {
    let (_, sat, _) = satellite_pair();
    let through = format!("{}/adrdom/decisions/example.md", primary_line(&sat));
    for given in ["adrdom/decisions/example.md", through.as_str()] {
        let out = adr(&sat, &["show", given], &[], "");
        assert!(out.status.success(), "{}: {}", given, stderr(&out));
        assert!(
            stdout(&out).starts_with("record: "),
            "{}: {}",
            given,
            stdout(&out)
        );
        assert!(
            stdout(&out).contains("What is the unit of authority called?"),
            "{}: {}",
            given,
            stdout(&out)
        );
    }
}

// adr: 563ba307-4c7d-4945-9c89-46dfa61cb88d
#[test]
fn an_untested_decision_is_not_reported_in_a_satellite_and_a_stale_tie_is() {
    let (_, sat, _) = satellite_pair();
    let out = adr(&sat, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(!stdout(&out).contains("no tracked file holds its tie"));
    let shown = adr(&sat, &["show"], &[], "");
    assert!(
        !stdout(&shown).contains("in a test that shows it"),
        "{}",
        stdout(&shown)
    );
    assert!(
        !stdout(&shown).contains("no tracked file holds its tie"),
        "{}",
        stdout(&shown)
    );
    assert!(commit_file(&sat, "tied.txt", &format!("adr: {}\n", STRAY))
        .status
        .success());
    for cmd in ["check", "show"] {
        let out = adr(&sat, &[cmd], &[], "");
        assert!(stdout(&out).contains(STRAY), "{}: {}", cmd, stdout(&out));
        assert!(
            stdout(&out).contains("tied.txt:1"),
            "{}: {}",
            cmd,
            stdout(&out)
        );
    }
    // A stale tie is a note: check still passes.
    assert!(adr(&sat, &["check"], &[], "").status.success());
}

// adr: fc118154-4172-48db-bddb-75a691894edf
#[test]
fn a_satellite_missing_at_its_path_is_a_note_with_its_url_and_clears_nothing_else() {
    let (primary, sat, _) = satellite_pair();
    let url = "https://example.com/sat.git";
    assert!(run_git(&sat, &["remote", "add", "origin", url])
        .status
        .success());
    let rel = sat_path(&primary, &sat);
    let out = adr(&primary, &["install", "--satellite", &rel], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let moved = move_away(&sat);
    let out = adr(&primary, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(
        text.contains(&format!("satellite {} not found, url {}", rel, url)),
        "{}",
        text
    );
    assert!(
        text.contains(&format!("agent runs: git clone {} {}", url, rel)),
        "{}",
        text
    );
    assert!(!text.contains("no tracked file holds its tie"), "{}", text);
    assert!(!sat.exists(), "check cloned the satellite");
    let shown = stdout(&adr(&primary, &["show"], &[], ""));
    assert!(
        shown.contains(&format!("satellite {} not found, url {}", rel, url)),
        "{}",
        shown
    );
    assert!(!shown.contains("in a test that shows it"), "{}", shown);
    assert!(!sat.exists());
    assert!(moved.exists());
}

// adr: fc118154-4172-48db-bddb-75a691894edf
#[test]
fn a_missing_satellite_with_no_url_says_so() {
    let (primary, sat, _) = satellite_pair();
    let rel = sat_path(&primary, &sat);
    move_away(&sat);
    let out = adr(&primary, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains(&format!("satellite {} not found, no url", rel)),
        "{}",
        stdout(&out)
    );
    assert!(!stdout(&out).contains("no tracked file holds its tie"));
}

// adr: 03f47941-43c5-479f-bf64-2d030361092d
#[test]
fn a_drifted_satellite_brief_fails_check_and_a_commit_until_install_runs() {
    let (_, sat, _) = satellite_pair();
    let brief = sat.join("adrdom/state/brief.md");
    let mut text = std::fs::read_to_string(&brief).unwrap();
    text.push_str("a drifted line\n");
    std::fs::write(&brief, text).unwrap();
    let want = "adrdom/state/brief.md differs from the primary's; agent runs: adr install";
    let out = adr(&sat, &["check"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(stdout(&out).contains(want), "{}", stdout(&out));
    assert!(run_git(&sat, &["add", "-A"]).status.success());
    let c = run_git(&sat, &["commit", "-q", "-m", "chore: drift"]);
    assert!(!c.status.success());
    assert!(stderr(&c).contains(want), "{}", stderr(&c));
    let out = adr(&sat, &["install"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let out = adr(&sat, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(!stdout(&out).contains("differs"), "{}", stdout(&out));
}

#[test]
fn check_show_and_install_in_a_satellite_leave_the_primary_untouched() {
    let (primary, sat, _) = satellite_pair();
    let before = status(&primary);
    adr(&sat, &["check"], &[], "");
    adr(&sat, &["show"], &[], "");
    assert!(adr(&sat, &["install"], &[], "").status.success());
    assert_eq!(status(&primary), before);
}

#[test]
fn a_satellite_without_an_ignore_or_attributes_file_passes_check() {
    let (_, sat, _) = satellite_pair();
    assert!(!sat.join("adrdom/.gitignore").exists());
    assert!(!sat.join("adrdom/.gitattributes").exists());
    let out = adr(&sat, &["check", "--staged"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    let out = adr(&sat, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
}

#[test]
fn a_terms_brief_copy_that_differs_from_the_primary_terms_fails_until_install_runs() {
    let (_, sat, _) = satellite_pair();
    let copy = sat.join("adrdom/state/terms-brief.md");
    assert!(copy.exists());
    std::fs::write(&copy, "- authority: another definition.\n").unwrap();
    let out = adr(&sat, &["check"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("adrdom/state/terms-brief.md differs from the primary's adrdom/terms.md; agent runs: adr install"),
        "{}",
        stdout(&out)
    );
    assert!(adr(&sat, &["install"], &[], "").status.success());
    let out = adr(&sat, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
}

#[test]
fn terms_brief_drift_is_found_in_a_satellite_when_the_terms_were_approved_one_entry_at_a_time() {
    let primary = init_repo();
    assert!(adr(&primary, &["install"], &[], "").status.success());
    std::fs::write(
        primary.join("adrdom/terms.md"),
        "# Terms\n\n## owner\ndefinition: The person the repository belongs to.\naliases: owners\n\n## greeting\ndefinition: The line the program prints.\naliases: greetings\n",
    )
    .unwrap();
    for name in ["owner", "greeting"] {
        let out = adr_owner(&primary, &["approve", "adrdom/terms.md", name], "");
        assert!(out.status.success(), "{}", stderr(&out));
    }
    let satellite = init_repo();
    let rel = relative_between(&primary, &satellite);
    let out = adr(&primary, &["install", "--satellite", &rel], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let out = adr(&satellite, &["check"], &[], "");
    assert!(!stdout(&out).contains("terms-brief"), "{}", stdout(&out));
    let copy = satellite.join("adrdom/state/terms-brief.md");
    let mut text = std::fs::read_to_string(&copy).unwrap();
    text.push_str("drift\n");
    std::fs::write(&copy, text).unwrap();
    let out = adr(&satellite, &["check"], &[], "");
    assert!(
        stdout(&out)
            .contains("adrdom/state/terms-brief.md differs from the primary's adrdom/terms.md"),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
}

/// A linked worktree of `main` at `<main>-wt/<name>/deep`, a place no layout
/// of siblings reaches.
fn linked_worktree(main: &Path, name: &str) -> PathBuf {
    let wt = PathBuf::from(format!("{}-wt", main.display()))
        .join(name)
        .join("deep");
    std::fs::create_dir_all(wt.parent().unwrap()).unwrap();
    let out = run_git(
        main,
        &["worktree", "add", "-q", "-b", name, wt.to_str().unwrap()],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    wt
}

// adr: 0320a987-f8b2-4105-9b27-64bf484026e6
#[test]
fn a_linked_worktree_of_a_satellite_finds_the_primary_from_the_main_working_tree() {
    let (primary, sat, id) = satellite_pair();
    let wt = linked_worktree(&sat, "satwt");
    for args in [&["check"][..], &["show"], &["show", "--ids"]] {
        let out = adr(&wt, args, &[], "");
        assert!(out.status.success(), "{:?}: {}", args, stderr(&out));
        assert!(
            !stderr(&out).contains("record not found"),
            "{}",
            stderr(&out)
        );
    }
    let shown = stdout(&adr(&wt, &["show", "--ids"], &[], ""));
    assert!(shown.contains(&id), "{}", shown);
    assert!(
        shown.contains(&primary.file_name().unwrap().to_string_lossy().to_string()),
        "{}",
        shown
    );
}

// adr: 0320a987-f8b2-4105-9b27-64bf484026e6
#[test]
fn a_linked_worktree_of_the_primary_finds_its_satellite_and_counts_the_tie_in_it() {
    let (primary, sat, id) = satellite_pair();
    let wt = linked_worktree(&primary, "primwt");
    let out = adr(&wt, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("not found"), "{}", text);
    assert!(text.contains("no tracked file holds its tie"), "{}", text);
    assert!(commit_file(&sat, "tied.txt", &format!("adr: {}\n", id))
        .status
        .success());
    for args in [&["check"][..], &["show"]] {
        let out = adr(&wt, args, &[], "");
        let text = stdout(&out);
        assert!(out.status.success(), "{:?}: {}", args, text);
        assert!(!text.contains("not found"), "{:?}: {}", args, text);
        assert!(
            !text.contains("no tracked file holds its tie")
                && !text.contains("in a test that shows it"),
            "{:?}: {}",
            args,
            text
        );
    }
}

// adr: 0320a987-f8b2-4105-9b27-64bf484026e6
#[test]
fn a_commit_in_a_linked_worktree_of_a_satellite_runs_its_hook_against_the_primary() {
    let (primary, sat, _) = satellite_pair();
    let wt = linked_worktree(&sat, "satcommit");
    let ok = commit_file(&wt, "notes.txt", "a plain line\n");
    assert!(ok.status.success(), "{}", stderr(&ok));
    ban(&primary, "gadgetword");
    let c = commit_file(&wt, "more.txt", "a gadgetword here\n");
    assert!(!c.status.success(), "{}", stderr(&c));
    assert!(stderr(&c).contains("banned"), "{}", stderr(&c));
}

// adr: 0320a987-f8b2-4105-9b27-64bf484026e6
#[test]
fn a_commit_in_a_linked_worktree_of_the_primary_counts_the_tie_in_the_satellite() {
    let (primary, sat, id) = satellite_pair();
    assert!(commit_file(&sat, "tied.txt", &format!("adr: {}\n", id))
        .status
        .success());
    let wt = linked_worktree(&primary, "primcommit");
    // The hook scans the satellite from the primary's main working tree; a
    // scan that read the worktree's index would find no tie and no satellite.
    let c = commit_file(&wt, "notes.txt", "a plain line\n");
    assert!(c.status.success(), "{}", stderr(&c));
    let text = format!("{}{}", stdout(&c), stderr(&c));
    assert!(!text.contains("not found"), "{}", text);
    assert!(!text.contains("no tracked file holds its tie"), "{}", text);
    // A stale tie in the satellite is reported with the satellite's path.
    assert!(commit_file(&sat, "stale.txt", &format!("adr: {}\n", STRAY))
        .status
        .success());
    let c = commit_file(&wt, "more.txt", "another line\n");
    let text = format!("{}{}", stdout(&c), stderr(&c));
    assert!(
        text.contains(&format!("{}/stale.txt:1", sat_path(&primary, &sat))),
        "{}",
        text
    );
}

// adr: 7e248514-25f8-46ed-936e-2849cd27cf86
#[test]
fn show_in_a_satellite_and_in_its_linked_worktree_print_the_directory_the_record_was_read_from() {
    let (primary, sat, _) = satellite_pair();
    let want = format!(
        "record: {}",
        std::fs::canonicalize(&primary).unwrap().display()
    );
    let wt = PathBuf::from(format!("{}-wt", sat.display())).join("deep");
    std::fs::create_dir_all(wt.parent().unwrap()).unwrap();
    let made = run_git(
        &sat,
        &["worktree", "add", "-q", "-b", "wt", wt.to_str().unwrap()],
    );
    assert!(made.status.success(), "{}", stderr(&made));
    for dir in [&sat, &wt] {
        let out = adr(dir, &["show"], &[], "");
        assert!(out.status.success(), "{}", stderr(&out));
        assert_eq!(
            stdout(&out).lines().next(),
            Some(want.as_str()),
            "{}",
            stdout(&out)
        );
    }
}

// adr: 7e248514-25f8-46ed-936e-2849cd27cf86
#[test]
fn show_in_a_primary_prints_no_record_line() {
    let (primary, _, _) = satellite_pair();
    let out = adr(&primary, &["show"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("record: "), "{}", stdout(&out));
}
