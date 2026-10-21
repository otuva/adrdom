// What install and uninstall do for a satellite, and the commands that
// refuse in one: the satellite's files and the primary's row are written
// together and removed together.
mod common;

use common::*;
use std::path::{Path, PathBuf};

fn status(dir: &Path) -> String {
    stdout(&run_git(dir, &["status", "--porcelain"]))
}

fn rel(primary: &Path, satellite: &Path) -> String {
    relative_between(primary, satellite)
}

fn rel_back(primary: &Path, satellite: &Path) -> String {
    relative_between(satellite, primary)
}

fn tracked(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = stdout(&run_git(
        dir,
        &["ls-files", "--cached", "--others", "--exclude-standard"],
    ))
    .lines()
    .map(String::from)
    .collect();
    v.sort();
    v
}

fn satellites_tsv(primary: &Path) -> String {
    std::fs::read_to_string(primary.join("adrdom/state/satellites.tsv")).unwrap_or_default()
}

/// A primary holding a record and a second repository with no record.
fn primary_and_plain() -> (PathBuf, PathBuf) {
    let primary = init_repo();
    assert!(adr(&primary, &["install"], &[], "").status.success());
    (primary, init_repo())
}

// adr: 937bc0a6-691d-4d67-91c6-cc9e956ebc96
#[test]
fn one_install_satellite_writes_both_sides() {
    let (primary, sat) = primary_and_plain();
    let r = rel(&primary, &sat);
    let out = adr(&primary, &["install", "--satellite", &r], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(&format!("written: {}/adrdom/primary", r)),
        "{}",
        text
    );
    assert!(
        text.contains("written: adrdom/state/satellites.tsv"),
        "{}",
        text
    );
    assert_eq!(
        std::fs::read_to_string(sat.join("adrdom/primary")).unwrap(),
        format!("{}\n", rel_back(&primary, &sat))
    );
    assert!(sat.join(".githooks/pre-commit").exists());
    assert!(satellites_tsv(&primary).starts_with(&format!("{}\t", r)));
    // The primary itself is not installed again.
    assert_eq!(status(&primary).lines().count(), 3, "{}", status(&primary));
}

#[test]
fn install_satellite_refuses_a_path_that_is_not_a_repository() {
    let (primary, _) = primary_and_plain();
    let out = adr(
        &primary,
        &["install", "--satellite", "../no-such-dir"],
        &[],
        "",
    );
    assert!(!out.status.success());
    assert!(
        stderr(&out)
            .contains("install: ../no-such-dir is not a git repository; agent rewrites the path"),
        "{}",
        stderr(&out)
    );
    assert_eq!(satellites_tsv(&primary), "");
}

#[test]
fn install_satellite_refuses_a_repository_with_a_record_and_this_one() {
    let (primary, _) = primary_and_plain();
    let other = init_repo();
    assert!(adr(&other, &["install"], &[], "").status.success());
    let r = rel(&primary, &other);
    let out = adr(&primary, &["install", "--satellite", &r], &[], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains(&format!(
            "install: {} holds a record of its own; agent rewrites the path",
            r
        )),
        "{}",
        stderr(&out)
    );
    assert!(!other.join("adrdom/primary").exists());
    let out = adr(&primary, &["install", "--satellite", "."], &[], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("install: . holds a record of its own;"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn install_satellite_refuses_where_no_record_is_held() {
    let (a, b) = (init_repo(), init_repo());
    let r = rel(&a, &b);
    let out = adr(&a, &["install", "--satellite", &r], &[], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains(&format!(
            "install: this repository holds no record; agent runs: adr install, then runs: adr install --satellite {}",
            r
        )),
        "{}",
        stderr(&out)
    );
    assert!(!b.join("adrdom").exists());
}

#[test]
fn install_satellite_in_a_satellite_names_the_path_from_the_primary() {
    let (primary, sat, _) = satellite_pair();
    let third = init_repo();
    let out = adr(
        &sat,
        &["install", "--satellite", third.to_str().unwrap()],
        &[],
        "",
    );
    assert!(!out.status.success());
    let want = format!(
        "install: the record is at {}; agent changes into {}, then runs: adr install --satellite {}",
        rel_back(&primary, &sat),
        rel_back(&primary, &sat),
        rel(&primary, &third)
    );
    assert!(stderr(&out).contains(&want), "{}", stderr(&out));
}

// adr: 7e756e04-dde0-4137-ad4f-646e52073398
#[test]
fn the_row_holds_the_relative_path_and_the_origin_url() {
    let (primary, sat) = primary_and_plain();
    let r = rel(&primary, &sat);
    assert!(adr(&primary, &["install", "--satellite", &r], &[], "")
        .status
        .success());
    assert_eq!(satellites_tsv(&primary), format!("{}\t\n", r));

    run_git(
        &sat,
        &["remote", "add", "origin", "https://example.com/a.git"],
    );
    let out = adr(&primary, &["install", "--satellite", &r], &[], "");
    assert!(out.status.success());
    assert_eq!(
        satellites_tsv(&primary),
        format!("{}\thttps://example.com/a.git\n", r)
    );

    // Run again: still one row, and no write is printed.
    let out = adr(&primary, &["install", "--satellite", &r], &[], "");
    assert_eq!(stdout(&out), "", "{}", stdout(&out));
    assert_eq!(satellites_tsv(&primary).lines().count(), 1);

    run_git(
        &sat,
        &["remote", "set-url", "origin", "https://example.com/b.git"],
    );
    assert!(adr(&primary, &["install", "--satellite", &r], &[], "")
        .status
        .success());
    assert_eq!(
        satellites_tsv(&primary),
        format!("{}\thttps://example.com/b.git\n", r)
    );
}

// adr: 7e756e04-dde0-4137-ad4f-646e52073398
#[test]
fn the_satellites_file_is_tracked_not_ignored() {
    let (primary, sat) = primary_and_plain();
    let r = rel(&primary, &sat);
    assert!(adr(&primary, &["install", "--satellite", &r], &[], "")
        .status
        .success());
    let ignored = run_git(
        &primary,
        &["check-ignore", "-q", "adrdom/state/satellites.tsv"],
    );
    assert!(!ignored.status.success());
    assert!(tracked(&primary).contains(&"adrdom/state/satellites.tsv".to_string()));
}

// adr: 0e16fbf3-1401-4538-bd12-25303994af10
#[test]
fn a_writing_command_in_a_satellite_refuses_and_writes_nothing() {
    let (primary, sat, id) = satellite_pair();
    let back = rel_back(&primary, &sat);
    let before = (status(&primary), status(&sat));
    let topic = primary.join("adrdom/decisions/example.md");
    let topic = topic.to_str().unwrap();
    let answer_id = format!("{}@x", id);
    let agent_cmds: Vec<Vec<&str>> = vec![vec!["plan", topic], vec!["ban"], vec!["read"]];
    for args in agent_cmds {
        let out = adr(&sat, &args, &[], "somewordtoban\n");
        assert!(!out.status.success(), "{:?}", args);
        let err = stderr(&out);
        assert!(
            err.contains(&format!("{}: the record is at {}", args[0], back)),
            "{:?}: {}",
            args,
            err
        );
        assert!(
            err.contains(&format!(
                "agent changes into {}, then runs: adr {}",
                back, args[0]
            )),
            "{}",
            err
        );
    }
    let owner_cmds: Vec<Vec<&str>> = vec![
        vec!["approve", topic],
        vec!["answer", &answer_id, "A"],
        vec!["unban"],
        vec!["serve", "--port", "0"],
    ];
    for args in owner_cmds {
        let out = adr_owner(&sat, &args, "x\n");
        assert!(!out.status.success(), "{:?}", args);
        let err = stderr(&out);
        assert!(
            err.contains(&format!("{}: the record is at {}", args[0], back)),
            "{:?}: {}",
            args,
            err
        );
        assert!(
            err.contains(&format!(
                "owner changes into {}, then runs: adr {}",
                back, args[0]
            )),
            "{}",
            err
        );
    }
    assert_eq!((status(&primary), status(&sat)), before);
}

// adr: 0e16fbf3-1401-4538-bd12-25303994af10
#[test]
fn check_show_install_and_uninstall_are_not_refused_by_the_writing_gate() {
    let (_, sat, _) = satellite_pair();
    for args in [vec!["check"], vec!["show"], vec!["install"]] {
        let out = adr(&sat, &args, &[], "");
        assert!(
            !stderr(&out).contains("the record is at"),
            "{:?}: {}",
            args,
            stderr(&out)
        );
        assert!(
            !stdout(&out).contains("the record is at"),
            "{:?}: {}",
            args,
            stdout(&out)
        );
    }
    let out = adr_owner(&sat, &["uninstall"], "");
    assert!(
        !stderr(&out).contains("the record is at"),
        "{}",
        stderr(&out)
    );
}

// adr: a8280dd1-4bfa-4eeb-9656-1cd06381590a
#[test]
fn install_writes_the_satellite_files_and_nothing_else() {
    let (primary, sat, _) = satellite_pair();
    assert_eq!(
        tracked(&sat),
        vec![
            ".githooks/commit-msg",
            ".githooks/pre-commit",
            ".githooks/pre-push",
            "AGENTS.md",
            "README.md",
            "adrdom/primary",
            "adrdom/state/brief.md",
            "adrdom/state/terms-brief.md",
        ]
    );
    for rel in ["adrdom/state/brief.md", "adrdom/state/terms-brief.md"] {
        assert_eq!(
            std::fs::read(sat.join(rel)).unwrap(),
            std::fs::read(primary.join(rel)).unwrap(),
            "{}",
            rel
        );
    }
    let hooks = stdout(&run_git(&sat, &["config", "core.hooksPath"]));
    assert_eq!(hooks.trim(), ".githooks");
    let agents = std::fs::read_to_string(sat.join("AGENTS.md")).unwrap();
    assert!(agents.contains("<!-- adrdom brief begin -->"), "{}", agents);
    for absent in [
        "adrdom/state/ledger.tsv",
        "adrdom/.gitattributes",
        "adrdom/.gitignore",
        "adrdom/state/banned.sha256",
        "adrdom/state/reader.tsv",
        "CLAUDE.md",
        ".claude",
    ] {
        assert!(!sat.join(absent).exists(), "{}", absent);
    }
}

// adr: a8280dd1-4bfa-4eeb-9656-1cd06381590a
#[test]
fn install_in_a_satellite_rewrites_a_stale_copy_and_no_row() {
    let (primary, sat, _) = satellite_pair();
    let rows = satellites_tsv(&primary);
    std::fs::write(sat.join("adrdom/state/brief.md"), "stale\n").unwrap();
    std::fs::write(sat.join("adrdom/state/terms-brief.md"), "stale\n").unwrap();
    let out = adr(&sat, &["install"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("written: adrdom/state/brief.md"),
        "{}",
        stdout(&out)
    );
    for rel in ["adrdom/state/brief.md", "adrdom/state/terms-brief.md"] {
        assert_eq!(
            std::fs::read(sat.join(rel)).unwrap(),
            std::fs::read(primary.join(rel)).unwrap()
        );
    }
    assert_eq!(satellites_tsv(&primary), rows);
    // Run again: nothing to write.
    assert_eq!(stdout(&adr(&sat, &["install"], &[], "")), "");
}

// adr: a8280dd1-4bfa-4eeb-9656-1cd06381590a
#[test]
fn install_with_a_flag_in_a_satellite_refuses_and_writes_nothing() {
    let (_, sat, _) = satellite_pair();
    let before = status(&sat);
    for flag in ["--claude", "--codex", "--planning"] {
        let out = adr(&sat, &["install", flag], &[], "");
        assert!(!out.status.success());
        assert!(
            stderr(&out).contains(&format!(
                "install: a satellite takes no {}; agent runs: adr install",
                flag
            )),
            "{}",
            stderr(&out)
        );
    }
    assert_eq!(status(&sat), before);
    assert!(!sat.join("CLAUDE.md").exists());
}

// adr: a8280dd1-4bfa-4eeb-9656-1cd06381590a
#[test]
fn install_in_a_satellite_whose_record_is_missing_refuses_and_writes_nothing() {
    let (primary, sat, _) = satellite_pair();
    let moved = primary.with_file_name(format!(
        "{}-moved",
        primary.file_name().unwrap().to_string_lossy()
    ));
    std::fs::rename(&primary, &moved).unwrap();
    std::fs::write(sat.join("adrdom/state/brief.md"), "stale\n").unwrap();
    let out = adr(&sat, &["install"], &[], "");
    assert!(!out.status.success());
    let back = std::fs::read_to_string(sat.join("adrdom/primary")).unwrap();
    assert!(
        stderr(&out).contains(&format!(
            "install: record not found at {}; agent restores {}",
            back.trim(),
            back.trim()
        )),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        std::fs::read_to_string(sat.join("adrdom/state/brief.md")).unwrap(),
        "stale\n"
    );
}

// adr: da777c6e-f22a-41fa-ba6f-1f1544382710
#[test]
fn uninstall_in_a_satellite_removes_its_files_and_its_row() {
    let (primary, sat, _) = satellite_pair();
    let back = rel_back(&primary, &sat);
    let out = adr_owner(&sat, &["uninstall"], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains(&format!("written: {}/adrdom/state/satellites.tsv", back)),
        "{}",
        stdout(&out)
    );
    assert_eq!(satellites_tsv(&primary), "");
    assert!(!sat.join(".githooks").exists());
    assert!(!sat.join("AGENTS.md").exists());
    assert!(sat.join("adrdom/primary").exists());
    for rel in ["adrdom/state/brief.md", "adrdom/state/terms-brief.md"] {
        assert!(sat.join(rel).exists(), "{}", rel);
    }
    assert!(stdout(&run_git(&sat, &["config", "core.hooksPath"]))
        .trim()
        .is_empty());
}

// adr: da777c6e-f22a-41fa-ba6f-1f1544382710
#[test]
fn uninstall_in_a_satellite_with_the_record_missing_removes_nothing() {
    let (primary, sat, _) = satellite_pair();
    let moved = primary.with_file_name(format!(
        "{}-moved",
        primary.file_name().unwrap().to_string_lossy()
    ));
    std::fs::rename(&primary, &moved).unwrap();
    let out = adr_owner(&sat, &["uninstall"], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("uninstall: record not found at"),
        "{}",
        stderr(&out)
    );
    assert!(sat.join(".githooks/pre-commit").exists());
    assert!(sat.join("AGENTS.md").exists());
}

// adr: 0e16fbf3-1401-4538-bd12-25303994af10
#[test]
fn the_refusal_of_a_writing_command_prints_a_file_argument_as_the_primary_runs_it() {
    let (primary, sat, _) = satellite_pair();
    let back = rel_back(&primary, &sat);
    let inside = format!("{}/adrdom/decisions/example.md", back);
    let absolute = primary.join("adrdom/decisions/example.md");
    for (cmd, given) in [
        ("plan", inside.as_str()),
        ("plan", absolute.to_str().unwrap()),
    ] {
        let out = adr(&sat, &[cmd, given], &[], "");
        assert!(!out.status.success());
        let want = format!(
            "changes into {}, then runs: adr {} adrdom/decisions/example.md",
            back, cmd
        );
        assert!(stderr(&out).contains(&want), "{}: {}", cmd, stderr(&out));
    }
    let out = adr_owner(&sat, &["approve", &inside, "authority"], "");
    let want = format!(
        "changes into {}, then runs: adr approve adrdom/decisions/example.md authority",
        back
    );
    assert!(stderr(&out).contains(&want), "{}", stderr(&out));
    // A path outside the primary is printed as given.
    let out = adr(&sat, &["plan", "elsewhere/x.md"], &[], "");
    assert!(
        stderr(&out).contains("then runs: adr plan elsewhere/x.md"),
        "{}",
        stderr(&out)
    );
}

// adr: 7e756e04-dde0-4137-ad4f-646e52073398
#[test]
fn the_row_holds_no_credentials_of_an_http_origin_and_keeps_an_ssh_user() {
    let (primary, sat) = primary_and_plain();
    let r = rel(&primary, &sat);
    run_git(
        &sat,
        &[
            "remote",
            "add",
            "origin",
            "https://user:SECRETTOKEN@example.com/a.git",
        ],
    );
    let out = adr(&primary, &["install", "--satellite", &r], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        satellites_tsv(&primary),
        format!("{}\thttps://example.com/a.git\n", r)
    );
    run_git(
        &sat,
        &["remote", "set-url", "origin", "git@example.com:o/a.git"],
    );
    assert!(adr(&primary, &["install", "--satellite", &r], &[], "")
        .status
        .success());
    assert_eq!(
        satellites_tsv(&primary),
        format!("{}\tgit@example.com:o/a.git\n", r)
    );
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
#[test]
fn a_write_failure_in_install_satellite_names_the_command_with_the_path() {
    let (primary, sat) = primary_and_plain();
    let r = rel(&primary, &sat);
    // A file where the hooks directory belongs, so it cannot be created.
    std::fs::write(sat.join(".githooks"), "x").unwrap();
    let out = adr(&primary, &["install", "--satellite", &r], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains(&format!("adr install --satellite {}", r)),
        "{}",
        err
    );
    assert_eq!(satellites_tsv(&primary), "");
}

// adr: 0320a987-f8b2-4105-9b27-64bf484026e6
#[test]
fn install_satellite_from_a_linked_worktree_of_the_primary_writes_paths_between_main_working_trees()
{
    let (primary, sat) = primary_and_plain();
    for args in [
        &["add", "-A"][..],
        &["commit", "-q", "-m", "chore: install"],
    ] {
        assert!(run_git(&primary, args).status.success());
    }
    let wt = PathBuf::from(format!("{}-wt", primary.display())).join("deep");
    std::fs::create_dir_all(wt.parent().unwrap()).unwrap();
    let made = run_git(
        &primary,
        &["worktree", "add", "-q", "-b", "wt", wt.to_str().unwrap()],
    );
    assert!(made.status.success(), "{}", stderr(&made));
    let out = adr(
        &wt,
        &["install", "--satellite", sat.to_str().unwrap()],
        &[],
        "",
    );
    assert!(out.status.success(), "{}", stderr(&out));
    // The row is written where the command ran, from the main working tree.
    let row = std::fs::read_to_string(wt.join("adrdom/state/satellites.tsv")).unwrap();
    assert!(
        row.starts_with(&format!("{}\t", rel(&primary, &sat))),
        "{}",
        row
    );
    assert_eq!(
        std::fs::read_to_string(sat.join("adrdom/primary")).unwrap(),
        format!("{}\n", rel_back(&primary, &sat))
    );
}

/// A satellite installed under a first primary, and a second primary with a
/// record.
fn held_satellite() -> (PathBuf, PathBuf, PathBuf) {
    let (p1, sat) = primary_and_plain();
    let out = adr(&p1, &["install", "--satellite", &rel(&p1, &sat)], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let p2 = init_repo();
    assert!(adr(&p2, &["install"], &[], "").status.success());
    (p1, p2, sat)
}

// adr: 490cdbb6-0482-41ba-8f57-36a6136b166f
#[test]
fn install_satellite_refuses_a_satellite_held_by_another_primary_and_writes_nothing() {
    let (p1, p2, sat) = held_satellite();
    let before = (
        status(&p2),
        status(&sat),
        satellites_tsv(&p2),
        std::fs::read_to_string(sat.join("adrdom/primary")).unwrap(),
    );
    let out = adr(&p2, &["install", "--satellite", &rel(&p2, &sat)], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    let other = std::fs::canonicalize(&p1).unwrap();
    assert!(err.contains(&other.display().to_string()), "{}", err);
    assert!(err.contains("adr uninstall"), "{}", err);
    assert_eq!(satellites_tsv(&p2), "");
    assert_eq!(
        before,
        (
            status(&p2),
            status(&sat),
            satellites_tsv(&p2),
            std::fs::read_to_string(sat.join("adrdom/primary")).unwrap(),
        )
    );
    assert!(satellites_tsv(&p1).starts_with(&format!("{}\t", rel(&p1, &sat))));
}

// adr: 490cdbb6-0482-41ba-8f57-36a6136b166f
#[test]
fn install_satellite_takes_a_satellite_after_uninstall_in_it() {
    let (_, p2, sat) = held_satellite();
    assert!(adr_owner(&sat, &["uninstall"], "").status.success());
    let out = adr(&p2, &["install", "--satellite", &rel(&p2, &sat)], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        std::fs::read_to_string(sat.join("adrdom/primary")).unwrap(),
        format!("{}\n", rel_back(&p2, &sat))
    );
    assert!(satellites_tsv(&p2).starts_with(&format!("{}\t", rel(&p2, &sat))));
}

// adr: 490cdbb6-0482-41ba-8f57-36a6136b166f
#[test]
fn install_satellite_takes_a_satellite_whose_primary_is_gone() {
    let (p1, p2, sat) = held_satellite();
    std::fs::rename(&p1, format!("{}-moved", p1.display())).unwrap();
    let out = adr(&p2, &["install", "--satellite", &rel(&p2, &sat)], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        std::fs::read_to_string(sat.join("adrdom/primary")).unwrap(),
        format!("{}\n", rel_back(&p2, &sat))
    );
}

// adr: 490cdbb6-0482-41ba-8f57-36a6136b166f
#[test]
fn install_satellite_again_from_the_same_primary_is_not_refused() {
    let (p1, _, sat) = held_satellite();
    let out = adr(&p1, &["install", "--satellite", &rel(&p1, &sat)], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
}

/// The primary of a pair with its satellite moved to a sibling name, and the
/// path the primary lists for the satellite.
fn moved_satellite() -> (PathBuf, PathBuf, String) {
    let (primary, sat, _) = satellite_pair();
    let listed = rel(&primary, &sat);
    let gone = sat.with_file_name(format!(
        "{}-gone",
        sat.file_name().unwrap().to_string_lossy()
    ));
    std::fs::rename(&sat, &gone).unwrap();
    (primary, gone, listed)
}

fn primary_files(primary: &Path) -> Vec<(String, Vec<u8>)> {
    tracked(primary)
        .into_iter()
        .map(|f| {
            let bytes = std::fs::read(primary.join(&f)).unwrap_or_default();
            (f, bytes)
        })
        .collect()
}

// adr: 59e73422-5d55-4b4c-9793-25f78f5f9586
#[test]
fn uninstall_satellite_drops_the_row_of_a_moved_satellite_and_nothing_else() {
    let (primary, _gone, listed) = moved_satellite();
    let before = primary_files(&primary);
    let hooks = stdout(&run_git(&primary, &["config", "core.hooksPath"]));
    let out = adr_owner(&primary, &["uninstall", "--satellite", &listed], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out).trim(),
        "written: adrdom/state/satellites.tsv",
        "{}",
        stdout(&out)
    );
    assert_eq!(satellites_tsv(&primary), "");
    let after = primary_files(&primary);
    let changed: Vec<_> = before
        .iter()
        .zip(&after)
        .filter(|(b, a)| b != a)
        .map(|(b, _)| b.0.clone())
        .collect();
    assert_eq!(before.len(), after.len());
    assert_eq!(changed, vec!["adrdom/state/satellites.tsv".to_string()]);
    assert!(primary.join(".githooks/pre-commit").exists());
    assert!(primary.join("AGENTS.md").exists());
    assert_eq!(
        stdout(&run_git(&primary, &["config", "core.hooksPath"])),
        hooks
    );
}

// adr: 59e73422-5d55-4b4c-9793-25f78f5f9586
#[test]
fn uninstall_satellite_refuses_a_satellite_still_at_its_path() {
    let (primary, _, _) = satellite_pair();
    let listed = rel(&primary, &sat_of(&primary));
    let before = primary_files(&primary);
    let out = adr_owner(&primary, &["uninstall", "--satellite", &listed], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("a checkout is at"), "{}", err);
    assert!(err.contains("adr uninstall"), "{}", err);
    assert_eq!(primary_files(&primary), before);
}

/// The directory the only row of `primary` names.
fn sat_of(primary: &Path) -> PathBuf {
    let row = satellites_tsv(primary);
    let path = row.lines().next().unwrap().split('\t').next().unwrap();
    std::fs::canonicalize(primary.join(path)).unwrap()
}

// adr: 59e73422-5d55-4b4c-9793-25f78f5f9586
#[test]
fn uninstall_satellite_refuses_a_path_with_no_row() {
    let (primary, _gone, _) = moved_satellite();
    let before = primary_files(&primary);
    let out = adr_owner(&primary, &["uninstall", "--satellite", "../nowhere"], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("no satellite is listed at ../nowhere"),
        "{}",
        stderr(&out)
    );
    assert_eq!(primary_files(&primary), before);
}

// adr: 59e73422-5d55-4b4c-9793-25f78f5f9586
#[test]
fn uninstall_satellite_in_a_satellite_names_the_primary_and_writes_nothing() {
    let (primary, sat, _) = satellite_pair();
    let listed = rel(&primary, &sat);
    let before = primary_files(&primary);
    let sat_before = primary_files(&sat);
    let out = adr_owner(&sat, &["uninstall", "--satellite", "../elsewhere"], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("uninstall: the record is at"), "{}", err);
    assert!(err.contains("adr uninstall --satellite"), "{}", err);
    assert_eq!(primary_files(&primary), before);
    assert_eq!(primary_files(&sat), sat_before);
    assert!(satellites_tsv(&primary).starts_with(&listed));
}

// adr: 59e73422-5d55-4b4c-9793-25f78f5f9586
#[test]
fn uninstall_satellite_is_refused_to_an_agent() {
    let (primary, _gone, listed) = moved_satellite();
    let before = primary_files(&primary);
    let out = adr(&primary, &["uninstall", "--satellite", &listed], &[], "");
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("refusing: this is the owner's command"),
        "{}",
        stderr(&out)
    );
    assert_eq!(primary_files(&primary), before);
}

// adr: 59e73422-5d55-4b4c-9793-25f78f5f9586
#[test]
fn uninstall_satellite_reads_the_path_from_a_subdirectory_and_through_a_gone_parent() {
    let (primary, _gone, listed) = moved_satellite();
    let deep = primary.join("sub/deeper");
    std::fs::create_dir_all(&deep).unwrap();
    // From a subdirectory: two more `..` to reach the primary's parent.
    let from_sub = format!("../../{}", listed);
    let out = adr_owner(&deep, &["uninstall", "--satellite", &from_sub], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(satellites_tsv(&primary), "");

    // Through components that do not exist: `x/missing/child/../..` folds to `x`.
    let (primary, _gone, listed) = moved_satellite();
    let through = format!("{}/missing/child/../..", listed);
    let out = adr_owner(&primary, &["uninstall", "--satellite", &through], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(satellites_tsv(&primary), "");
}

// adr: 490cdbb6-0482-41ba-8f57-36a6136b166f
// adr: da777c6e-f22a-41fa-ba6f-1f1544382710
#[test]
fn install_satellite_refusal_clears_when_the_row_names_the_directory_in_another_form() {
    let (p1, p2, sat) = held_satellite();
    // The same directory through a segment that goes down into the
    // primary and back up: `./x/../../<name>` for `../<name>`.
    let listed = rel(&p1, &sat);
    let name = sat.file_name().unwrap().to_string_lossy().to_string();
    let parent = p1.parent().unwrap();
    assert!(parent.join(&name).exists());
    let other_form = format!("./x/../{}", listed);
    std::fs::create_dir_all(p1.join("x")).unwrap();
    let tsv = p1.join("adrdom/state/satellites.tsv");
    let text = std::fs::read_to_string(&tsv).unwrap();
    let url = text.trim_end().split('\t').nth(1).unwrap_or("").to_string();
    let row = if url.is_empty() {
        format!("{}\n", other_form)
    } else {
        format!("{}\t{}\n", other_form, url)
    };
    std::fs::write(&tsv, row).unwrap();
    assert_ne!(other_form, listed);

    let out = adr(&p2, &["install", "--satellite", &rel(&p2, &sat)], &[], "");
    assert!(!out.status.success());
    let other = std::fs::canonicalize(&p1).unwrap();
    assert!(stderr(&out).contains(&other.display().to_string()));

    let un = adr_owner(&sat, &["uninstall"], "");
    assert!(un.status.success(), "{}", stderr(&un));
    assert!(
        !satellites_tsv(&p1).contains(&name),
        "{}",
        satellites_tsv(&p1)
    );

    let out = adr(&p2, &["install", "--satellite", &rel(&p2, &sat)], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
}

/// The path a refusal prints after `marker`, up to the next whitespace.
fn path_after(text: &str, marker: &str) -> String {
    let at = text.find(marker).unwrap_or_else(|| panic!("{}", text)) + marker.len();
    text[at..]
        .split_whitespace()
        .next()
        .unwrap()
        .trim_end_matches(';')
        .to_string()
}

/// `base` joined with `rel`, with `.` and `..` folded away without the disk.
fn folded(base: &Path, rel: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for c in base.join(rel).components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// A linked worktree of `sat` at `<sat>-far/away/wt`, placed away from any
/// sibling layout.
fn far_worktree(sat: &Path) -> PathBuf {
    let wt = PathBuf::from(format!("{}-far", sat.display())).join("away/wt");
    std::fs::create_dir_all(wt.parent().unwrap()).unwrap();
    let made = run_git(
        sat,
        &["worktree", "add", "-q", "-b", "far", wt.to_str().unwrap()],
    );
    assert!(made.status.success(), "{}", stderr(&made));
    std::fs::canonicalize(&wt).unwrap()
}

// adr: 0320a987-f8b2-4105-9b27-64bf484026e6
// adr: 0e16fbf3-1401-4538-bd12-25303994af10
#[test]
fn a_writing_command_in_a_far_linked_worktree_of_a_satellite_names_a_path_that_leads_to_the_primary(
) {
    let (primary, sat, _) = satellite_pair();
    let wt = far_worktree(&sat);
    let topic = "adrdom/decisions/x.md";
    let out = adr(&wt, &["plan", topic], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    let shown = path_after(&err, "the record is at ");
    assert_eq!(
        std::fs::canonicalize(wt.join(&shown)).unwrap(),
        std::fs::canonicalize(&primary).unwrap(),
        "{}",
        err
    );
    // In the main working tree the line is printed as adrdom/primary holds it.
    let held = std::fs::read_to_string(sat.join("adrdom/primary")).unwrap();
    let main = adr(&sat, &["plan", topic], &[], "");
    assert!(!main.status.success());
    assert!(
        stderr(&main).contains(&format!("the record is at {}", held.trim_end())),
        "{}",
        stderr(&main)
    );
}

// adr: 0320a987-f8b2-4105-9b27-64bf484026e6
#[test]
fn check_in_a_far_linked_worktree_with_the_primary_moved_names_where_the_primary_was() {
    let (primary, sat, _) = satellite_pair();
    let wt = far_worktree(&sat);
    let was = std::fs::canonicalize(&primary).unwrap();
    std::fs::rename(&primary, format!("{}-moved", primary.display())).unwrap();
    let out = adr(&wt, &["check"], &[], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    let shown = path_after(&err, "record not found at ");
    assert_eq!(folded(&wt, &shown), was, "{}", err);
}

// adr: 490cdbb6-0482-41ba-8f57-36a6136b166f
#[test]
fn install_satellite_refuses_a_linked_worktree_whose_main_working_tree_holds_the_line() {
    let (p1, sat) = primary_and_plain();
    std::fs::write(sat.join("README"), "x\n").unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "chore: seed"]] {
        assert!(run_git(&sat, args).status.success());
    }
    let out = adr(&p1, &["install", "--satellite", &rel(&p1, &sat)], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    // adrdom/primary stays uncommitted, so the worktree lacks it.
    let wt = far_worktree(&sat);
    assert!(!wt.join("adrdom/primary").exists());
    let p2 = init_repo();
    assert!(adr(&p2, &["install"], &[], "").status.success());
    let before = (status(&p2), status(&wt), satellites_tsv(&p2), tracked(&wt));
    let out = adr(
        &p2,
        &["install", "--satellite", wt.to_str().unwrap()],
        &[],
        "",
    );
    assert!(!out.status.success());
    let err = stderr(&out);
    let other = std::fs::canonicalize(&p1).unwrap();
    assert!(err.contains(&other.display().to_string()), "{}", err);
    assert_eq!(satellites_tsv(&p2), "");
    assert!(!wt.join("adrdom/primary").exists());
    assert_eq!(
        before,
        (status(&p2), status(&wt), satellites_tsv(&p2), tracked(&wt))
    );
}
