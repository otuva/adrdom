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
        stderr(&out).contains(
            "install: ../no-such-dir is not a git repository; agent rewrites the path after --satellite"
        ),
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
            "install: {} holds a record of its own; agent rewrites the path after --satellite",
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
    let agent_cmds: Vec<Vec<&str>> = vec![
        vec!["plan", topic],
        vec!["ask", topic],
        vec!["ban"],
        vec!["read"],
    ];
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
            "install: record not found at {}; agent restores the primary's checkout at {} from git",
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
