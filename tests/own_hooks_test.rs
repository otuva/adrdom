// A repository keeps hooks of its own beside the tool's: install moves the
// hook it finds into `.githooks/<name>.own`, the tool's hook runs it once
// check passes, install stops when another program holds core.hooksPath,
// and check holds that setting.
mod common;

use common::*;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Every text an earlier install wrote for a hook.
const EARLIER: [(&str, &str); 5] = [
    ("pre-commit", "#!/bin/sh\nadr check --staged\nexit $?\n"),
    (
        "pre-commit",
        "#!/bin/sh\nadr check --staged --quiet\nexit $?\n",
    ),
    (
        "commit-msg",
        "#!/bin/sh\nadr check --message \"$1\"\nexit $?\n",
    ),
    (
        "commit-msg",
        "#!/bin/sh\nadr check --message \"$1\" --quiet\nexit $?\n",
    ),
    ("pre-push", "#!/bin/sh\nadr check --push\nexit $?\n"),
];

fn write_mode(path: &Path, text: &str, mode: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn hooks_path(dir: &Path) -> String {
    stdout(&run_git(dir, &["config", "core.hooksPath"]))
        .trim()
        .to_string()
}

fn install_ok(dir: &Path) -> String {
    let out = adr(dir, &["install"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    stdout(&out)
}

/// An installed repository whose state is committed, so a later commit runs
/// the hooks on a passing check.
fn ready() -> PathBuf {
    let dir = init_repo();
    install_ok(&dir);
    approve_empty_terms(&dir);
    run_git(&dir, &["add", "-A"]);
    let c = run_git(&dir, &["commit", "-q", "-m", "chore: install"]);
    assert!(c.status.success(), "{}{}", stdout(&c), stderr(&c));
    dir
}

/// An own hook that records its arguments and input under `.git` and exits
/// with the number in `.git/own-exit`.
fn write_own(dir: &Path, name: &str) {
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > .git/own-{name}-args\ncat > .git/own-{name}-stdin\nexit $(cat .git/own-exit 2>/dev/null || echo 0)\n"
    );
    write_mode(&dir.join(format!(".githooks/{name}.own")), &script, 0o755);
}

fn ran(dir: &Path, name: &str) -> bool {
    dir.join(format!(".git/own-{name}-args")).exists()
}

/// Stages a malformed ledger row, which check refuses.
fn stage_a_failure(dir: &Path) {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(dir.join("adrdom/state/ledger.tsv"))
        .unwrap();
    writeln!(f, "not a ledger row").unwrap();
    run_git(dir, &["add", "-A"]);
}

fn top_level(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .filter(|n| n != ".git")
        .collect();
    v.sort();
    v
}

fn bare_remote() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "adr-test-bare-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(run_git(&dir, &["init", "-q", "--bare"]).status.success());
    dir
}

// adr: d6112bd7-1649-4a47-ab3f-0681e4d0e8f6
#[test]
fn a_repository_keeps_an_own_hook_beside_the_tools_and_check_passes() {
    let dir = ready();
    write_own(&dir, "pre-commit");
    for (name, text) in [
        ("pre-commit", PRE_COMMIT_TEXT),
        ("commit-msg", COMMIT_MSG_TEXT),
        ("pre-push", PRE_PUSH_TEXT),
    ] {
        let hook = std::fs::read_to_string(dir.join(format!(".githooks/{}", name))).unwrap();
        assert_eq!(hook, text);
        assert!(
            hook.contains(&format!("{}.own", name)),
            "the hook text calls the own hook: {}",
            hook
        );
    }
    let out = adr(&dir, &["check"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
    assert_eq!(stdout(&out), "");
}

// adr: 6ac1dde1-fe11-47af-861a-334dbf917b11
#[test]
fn the_own_hook_runs_after_a_passing_check_with_gits_arguments() {
    let dir = ready();
    write_own(&dir, "pre-commit");
    write_own(&dir, "commit-msg");
    std::fs::write(dir.join("a.txt"), "a\n").unwrap();
    run_git(&dir, &["add", "a.txt"]);
    let c = run_git(&dir, &["commit", "-q", "-m", "feat: a"]);
    assert!(c.status.success(), "{}{}", stdout(&c), stderr(&c));
    assert!(ran(&dir, "pre-commit"));
    assert!(ran(&dir, "commit-msg"));
    let args = std::fs::read_to_string(dir.join(".git/own-commit-msg-args")).unwrap();
    assert!(args.trim_end().ends_with("COMMIT_EDITMSG"), "{}", args);
}

// adr: 6ac1dde1-fe11-47af-861a-334dbf917b11
#[test]
fn the_own_hook_does_not_run_after_a_failing_check() {
    let dir = ready();
    write_own(&dir, "pre-commit");
    write_own(&dir, "commit-msg");
    stage_a_failure(&dir);
    let c = run_git(&dir, &["commit", "-q", "-m", "feat: a"]);
    assert!(!c.status.success(), "{}", stdout(&c));
    assert!(!ran(&dir, "pre-commit"), "{}", stdout(&c));
    assert!(!ran(&dir, "commit-msg"));
}

// adr: 6ac1dde1-fe11-47af-861a-334dbf917b11
#[test]
fn a_failing_own_hook_fails_the_commit() {
    let dir = ready();
    write_own(&dir, "pre-commit");
    std::fs::write(dir.join(".git/own-exit"), "3\n").unwrap();
    std::fs::write(dir.join("a.txt"), "a\n").unwrap();
    run_git(&dir, &["add", "a.txt"]);
    let c = run_git(&dir, &["commit", "-q", "-m", "feat: a"]);
    assert!(!c.status.success());
    assert!(ran(&dir, "pre-commit"));
    let log = stdout(&run_git(&dir, &["log", "--oneline"]));
    assert_eq!(log.lines().count(), 2, "{}", log);
}

// adr: 6ac1dde1-fe11-47af-861a-334dbf917b11
#[test]
fn a_non_executable_own_hook_is_skipped() {
    let dir = ready();
    write_own(&dir, "pre-commit");
    std::fs::set_permissions(
        dir.join(".githooks/pre-commit.own"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    std::fs::write(dir.join("a.txt"), "a\n").unwrap();
    run_git(&dir, &["add", "a.txt"]);
    let c = run_git(&dir, &["commit", "-q", "-m", "feat: a"]);
    assert!(c.status.success(), "{}", stderr(&c));
    assert!(!ran(&dir, "pre-commit"));
}

// adr: 6ac1dde1-fe11-47af-861a-334dbf917b11
#[test]
fn the_own_pre_push_hook_gets_gits_arguments_and_input_after_check() {
    let dir = ready();
    let branch = stdout(&run_git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]))
        .trim()
        .to_string();
    let remote = bare_remote();
    run_git(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
    write_own(&dir, "pre-push");

    let push = run_git(&dir, &["push", "-q", "-u", "origin", &branch]);
    assert!(push.status.success(), "{}{}", stdout(&push), stderr(&push));
    let head = stdout(&run_git(&dir, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let input = std::fs::read(dir.join(".git/own-pre-push-stdin")).unwrap();
    let zeros = "0".repeat(head.len());
    assert_eq!(
        String::from_utf8(input).unwrap(),
        format!("refs/heads/{b} {head} refs/heads/{b} {zeros}\n", b = branch)
    );
    let args = std::fs::read_to_string(dir.join(".git/own-pre-push-args")).unwrap();
    assert_eq!(args.lines().next(), Some("origin"), "{}", args);

    // A failing own hook fails the push.
    std::fs::remove_file(dir.join(".git/own-pre-push-args")).unwrap();
    std::fs::write(dir.join(".git/own-exit"), "4\n").unwrap();
    std::fs::write(dir.join("a.txt"), "a\n").unwrap();
    run_git(&dir, &["add", "a.txt"]);
    // The commit hook has no own hook here and the exit file is for pre-push.
    let c = run_git(&dir, &["commit", "-q", "-m", "feat: a"]);
    assert!(c.status.success(), "{}", stderr(&c));
    let push = run_git(&dir, &["push", "-q", "origin", &branch]);
    assert!(!push.status.success());
    assert!(ran(&dir, "pre-push"));
}

// adr: 6ac1dde1-fe11-47af-861a-334dbf917b11
#[test]
fn the_pre_push_hook_hands_an_empty_input_on_as_empty() {
    let dir = ready();
    write_own(&dir, "pre-push");
    let bin_dir = adr_bin().parent().unwrap().to_path_buf();
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let status = scrubbed(Command::new(dir.join(".githooks/pre-push")))
        .current_dir(&dir)
        .env("PATH", path)
        .env("ADR_FAKE_MODEL", "1")
        .args(["origin", "url"])
        .stdin(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(ran(&dir, "pre-push"));
    assert_eq!(
        std::fs::read(dir.join(".git/own-pre-push-stdin")).unwrap(),
        b""
    );
    assert_eq!(
        std::fs::read_to_string(dir.join(".git/own-pre-push-args")).unwrap(),
        "origin\nurl\n"
    );
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn install_moves_a_git_hooks_hook_into_the_own_file_keeping_its_mode() {
    let dir = init_repo();
    assert_eq!(hooks_path(&dir), "");
    write_mode(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\necho mine\n",
        0o750,
    );
    write_mode(
        &dir.join(".git/hooks/commit-msg"),
        "#!/bin/sh\necho off\n",
        0o644,
    );
    let sample = std::fs::read_dir(dir.join(".git/hooks"))
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".sample")
        })
        .count();

    let text = install_ok(&dir);
    assert_eq!(
        std::fs::read_to_string(dir.join(".githooks/pre-commit.own")).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
    assert_eq!(mode(&dir.join(".githooks/pre-commit.own")), 0o750);
    assert_eq!(mode(&dir.join(".githooks/commit-msg.own")), 0o644);
    assert!(!dir.join(".git/hooks/pre-commit").exists());
    assert!(!dir.join(".git/hooks/commit-msg").exists());
    assert!(
        text.contains("written: .githooks/pre-commit.own\n"),
        "{}",
        text
    );
    assert!(
        text.contains("removed: .git/hooks/pre-commit\n"),
        "{}",
        text
    );
    assert_eq!(
        std::fs::read_to_string(dir.join(".githooks/pre-commit")).unwrap(),
        PRE_COMMIT_TEXT
    );
    assert_eq!(hooks_path(&dir), ".githooks");
    let after = std::fs::read_dir(dir.join(".git/hooks"))
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".sample")
        })
        .count();
    assert_eq!(sample, after, "sample files stay");
    assert!(!dir.join(".githooks/pre-push.own").exists());
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn a_moved_hook_runs_after_check_at_the_next_commit() {
    let dir = init_repo();
    write_mode(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\ntouch .git/old-hook-ran\n",
        0o755,
    );
    install_ok(&dir);
    approve_empty_terms(&dir);
    run_git(&dir, &["add", "-A"]);
    let c = run_git(&dir, &["commit", "-q", "-m", "chore: install"]);
    assert!(c.status.success(), "{}{}", stdout(&c), stderr(&c));
    assert!(dir.join(".git/old-hook-ran").exists());
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn install_moves_a_foreign_githooks_hook_and_leaves_the_tools_own_texts() {
    let dir = init_repo();
    run_git(&dir, &["config", "core.hooksPath", ".githooks"]);
    write_mode(
        &dir.join(".githooks/pre-commit"),
        "#!/bin/sh\necho mine\n",
        0o755,
    );
    let text = install_ok(&dir);
    assert_eq!(
        std::fs::read_to_string(dir.join(".githooks/pre-commit.own")).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join(".githooks/pre-commit")).unwrap(),
        PRE_COMMIT_TEXT
    );
    assert!(
        text.contains("written: .githooks/pre-commit.own\n"),
        "{}",
        text
    );
    assert!(!text.contains("removed:"), "{}", text);

    // A second install moves nothing: the hook is the tool's now.
    let again = install_ok(&dir);
    assert_eq!(again, "");
    assert_eq!(
        std::fs::read_to_string(dir.join(".githooks/pre-commit.own")).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn a_hook_an_earlier_install_wrote_is_overwritten_never_moved() {
    for (name, old) in EARLIER {
        let dir = init_repo();
        run_git(&dir, &["config", "core.hooksPath", ".githooks"]);
        write_mode(&dir.join(format!(".githooks/{}", name)), old, 0o755);
        install_ok(&dir);
        assert!(
            !dir.join(format!(".githooks/{}.own", name)).exists(),
            "{} moved",
            old
        );
        let now = std::fs::read_to_string(dir.join(format!(".githooks/{}", name))).unwrap();
        assert_ne!(now, old);
        assert!(now.contains(&format!("{}.own", name)), "{}", now);
    }
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn a_tools_hook_with_carriage_returns_is_rewritten_never_moved() {
    let texts = EARLIER
        .iter()
        .copied()
        .chain([
            ("pre-commit", common::PRE_COMMIT_TEXT),
            ("commit-msg", common::COMMIT_MSG_TEXT),
            ("pre-push", common::PRE_PUSH_TEXT),
        ])
        .collect::<Vec<_>>();
    for (name, text) in texts {
        let dir = init_repo();
        run_git(&dir, &["config", "core.hooksPath", ".githooks"]);
        let crlf = text.replace('\n', "\r\n");
        write_mode(&dir.join(format!(".githooks/{}", name)), &crlf, 0o755);
        install_ok(&dir);
        assert!(
            !dir.join(format!(".githooks/{}.own", name)).exists(),
            "{:?} moved",
            crlf
        );
        let now = std::fs::read_to_string(dir.join(format!(".githooks/{}", name))).unwrap();
        assert!(!now.contains('\r'), "{:?}", now);
    }
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn git_hooks_is_left_alone_when_hooks_path_is_already_githooks() {
    let dir = init_repo();
    run_git(&dir, &["config", "core.hooksPath", "./.githooks"]);
    write_mode(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\necho mine\n",
        0o755,
    );
    install_ok(&dir);
    assert!(dir.join(".git/hooks/pre-commit").exists());
    assert!(!dir.join(".githooks/pre-commit.own").exists());
    assert_eq!(hooks_path(&dir), "./.githooks", "an own value is kept");
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn a_collision_with_an_existing_own_file_stops_install_before_any_write() {
    let dir = init_repo();
    write_mode(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\necho mine\n",
        0o755,
    );
    write_mode(
        &dir.join(".githooks/pre-commit.own"),
        "#!/bin/sh\necho kept\n",
        0o755,
    );
    let before = top_level(&dir);
    let out = adr(&dir, &["install"], &[], "");
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains(".git/hooks/pre-commit") && err.contains(".githooks/pre-commit.own"),
        "{}",
        err
    );
    assert_eq!(top_level(&dir), before);
    assert_eq!(
        std::fs::read_to_string(dir.join(".githooks/pre-commit.own")).unwrap(),
        "#!/bin/sh\necho kept\n"
    );
    assert!(dir.join(".git/hooks/pre-commit").exists());
    assert!(!dir.join(".githooks/pre-commit").exists());
    assert_eq!(hooks_path(&dir), "");
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn two_hooks_of_one_name_stop_install_before_any_write() {
    let dir = init_repo();
    write_mode(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\necho a\n",
        0o755,
    );
    write_mode(
        &dir.join(".githooks/pre-commit"),
        "#!/bin/sh\necho b\n",
        0o755,
    );
    let before = top_level(&dir);
    let out = adr(&dir, &["install"], &[], "");
    assert_eq!(out.status.code(), Some(1));
    // The own hook file is not there yet: the agent writes it.
    assert!(
        stderr(&out).contains("; agent writes .githooks/pre-commit.own, then removes"),
        "{}",
        stderr(&out)
    );
    assert_eq!(top_level(&dir), before);
    assert!(dir.join(".git/hooks/pre-commit").exists());
    assert_eq!(hooks_path(&dir), "");
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn install_from_the_primary_moves_a_satellites_hook() {
    let primary = init_repo();
    install_ok(&primary);
    let sat = init_repo();
    write_mode(
        &sat.join(".git/hooks/pre-push"),
        "#!/bin/sh\necho mine\n",
        0o755,
    );
    let r = relative_between(&primary, &sat);
    let out = adr(&primary, &["install", "--satellite", &r], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(&format!("written: {}/.githooks/pre-push.own\n", r)),
        "{}",
        text
    );
    assert!(
        text.contains(&format!("removed: {}/.git/hooks/pre-push\n", r)),
        "{}",
        text
    );
    assert_eq!(
        std::fs::read_to_string(sat.join(".githooks/pre-push.own")).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
    assert!(!sat.join(".git/hooks/pre-push").exists());
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn install_in_a_satellite_moves_a_foreign_hook_and_stops_on_a_collision() {
    let (_primary, sat, _id) = satellite_pair();
    write_mode(
        &sat.join(".githooks/commit-msg"),
        "#!/bin/sh\necho mine\n",
        0o755,
    );
    let out = adr(&sat, &["install"], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        std::fs::read_to_string(sat.join(".githooks/commit-msg.own")).unwrap(),
        "#!/bin/sh\necho mine\n"
    );

    // The own file is there now: a second foreign hook cannot move onto it.
    write_mode(
        &sat.join(".githooks/commit-msg"),
        "#!/bin/sh\necho other\n",
        0o755,
    );
    let out = adr(&sat, &["install"], &[], "");
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains(".githooks/commit-msg.own"),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        std::fs::read_to_string(sat.join(".githooks/commit-msg")).unwrap(),
        "#!/bin/sh\necho other\n"
    );
    assert_eq!(
        std::fs::read_to_string(sat.join(".githooks/commit-msg.own")).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
}

// adr: 897b52a7-f6d2-467e-ba14-9f4ce4865664
#[test]
fn install_stops_while_another_program_holds_hooks_path() {
    let dir = init_repo();
    run_git(&dir, &["config", "core.hooksPath", ".husky"]);
    let before = top_level(&dir);
    let out = adr(&dir, &["install"], &[], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stderr(&out).trim(),
        "install: core.hooksPath is .husky, not .githooks; owner removes the hook manager that set core.hooksPath, then runs: adr install"
    );
    assert_eq!(top_level(&dir), before);
    assert_eq!(hooks_path(&dir), ".husky");
    assert_eq!(stdout(&out), "");
}

// adr: 897b52a7-f6d2-467e-ba14-9f4ce4865664
#[test]
fn install_stops_for_a_satellite_that_another_program_holds() {
    let primary = init_repo();
    install_ok(&primary);
    let sat = init_repo();
    run_git(&sat, &["config", "core.hooksPath", ".husky"]);
    let before = top_level(&sat);
    let tsv_before = std::fs::read_to_string(primary.join("adrdom/state/satellites.tsv")).ok();
    let r = relative_between(&primary, &sat);
    let out = adr(&primary, &["install", "--satellite", &r], &[], "");
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("core.hooksPath is .husky, not .githooks"));
    assert_eq!(top_level(&sat), before);
    assert_eq!(hooks_path(&sat), ".husky");
    assert_eq!(
        std::fs::read_to_string(primary.join("adrdom/state/satellites.tsv")).ok(),
        tsv_before
    );

    // Inside an installed satellite.
    let (_p, sat, _id) = satellite_pair();
    run_git(&sat, &["config", "core.hooksPath", ".husky"]);
    let before = top_level(&sat);
    let out = adr(&sat, &["install"], &[], "");
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("core.hooksPath is .husky, not .githooks"));
    assert_eq!(top_level(&sat), before);
    assert_eq!(hooks_path(&sat), ".husky");
}

// adr: db76c9a0-b4fc-4aa2-8afb-47cd78ef7138
#[test]
fn check_reports_a_hooks_path_that_is_not_set_or_is_foreign() {
    let dir = init_repo();
    install_ok(&dir);
    approve_empty_terms(&dir);
    assert!(adr(&dir, &["check"], &[], "").status.success());

    run_git(&dir, &["config", "--unset", "core.hooksPath"]);
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("core.hooksPath is not set; agent runs: adr install"),
        "{}",
        stdout(&out)
    );

    run_git(&dir, &["config", "core.hooksPath", ".husky"]);
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("core.hooksPath is .husky, not .githooks; owner removes the hook manager that set core.hooksPath, then runs: adr install"),
        "{}",
        stdout(&out)
    );
}

// adr: db76c9a0-b4fc-4aa2-8afb-47cd78ef7138
#[test]
fn check_takes_every_spelling_of_the_tools_directory() {
    let dir = init_repo();
    install_ok(&dir);
    approve_empty_terms(&dir);
    let absolute = dir.join(".githooks").display().to_string();
    for value in [".githooks", "./.githooks", absolute.as_str()] {
        run_git(&dir, &["config", "core.hooksPath", value]);
        let out = adr(&dir, &["check"], &[], "");
        assert!(out.status.success(), "{}: {}", value, stdout(&out));
        assert_eq!(stdout(&out), "", "{}", value);
    }
}

// adr: db76c9a0-b4fc-4aa2-8afb-47cd78ef7138
#[test]
fn check_reports_a_foreign_hooks_path_in_a_satellite() {
    let (_p, sat, _id) = satellite_pair();
    run_git(&sat, &["config", "core.hooksPath", ".husky"]);
    let out = adr(&sat, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("core.hooksPath is .husky, not .githooks; owner removes the hook manager that set core.hooksPath, then runs: adr install"),
        "{}",
        stdout(&out)
    );
}

// adr: 292359a4-0512-465b-b6a7-79838e66e5a5
#[test]
fn uninstall_leaves_the_own_hook_file_in_place() {
    let dir = init_repo();
    write_mode(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\necho mine\n",
        0o755,
    );
    install_ok(&dir);
    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!dir.join(".githooks/pre-commit").exists());
    assert_eq!(
        std::fs::read_to_string(dir.join(".githooks/pre-commit.own")).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
}

/// Installs over a repository whose hooks were set up by the caller, then
/// commits once so the hooks run.
fn install_and_commit(dir: &Path) {
    install_ok(dir);
    approve_empty_terms(dir);
    run_git(dir, &["add", "-A"]);
    let c = run_git(dir, &["commit", "-q", "-m", "chore: install"]);
    assert!(c.status.success(), "{}{}", stdout(&c), stderr(&c));
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn a_moved_symlink_to_another_moved_hook_runs_that_hook_once_per_name() {
    let dir = init_repo();
    write_mode(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\necho \"$(basename \"$0\") $#\" >> .git/runs\n",
        0o755,
    );
    std::os::unix::fs::symlink("pre-commit", dir.join(".git/hooks/commit-msg")).unwrap();
    install_and_commit(&dir);
    assert_eq!(
        std::fs::read_link(dir.join(".githooks/commit-msg.own")).unwrap(),
        PathBuf::from("pre-commit.own")
    );
    let mut runs: Vec<String> = std::fs::read_to_string(dir.join(".git/runs"))
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    runs.sort();
    assert_eq!(runs, ["commit-msg.own 1", "pre-commit.own 0"]);
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn a_moved_symlink_to_a_work_tree_script_still_runs_the_script() {
    let dir = init_repo();
    write_mode(
        &dir.join("scripts/lint.sh"),
        "#!/bin/sh\ntouch .git/lint-ran\n",
        0o755,
    );
    std::os::unix::fs::symlink("../../scripts/lint.sh", dir.join(".git/hooks/pre-commit")).unwrap();
    install_and_commit(&dir);
    assert!(dir.join(".git/lint-ran").exists());
    assert!(!dir.join(".git/hooks/pre-commit").exists());
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn a_dangling_symlink_is_not_a_hook() {
    let dir = init_repo();
    std::os::unix::fs::symlink("missing", dir.join(".git/hooks/pre-commit")).unwrap();
    install_ok(&dir);
    assert!(std::fs::symlink_metadata(dir.join(".git/hooks/pre-commit")).is_ok());
    assert!(std::fs::symlink_metadata(dir.join(".githooks/pre-commit.own")).is_err());
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn a_directory_in_githooks_is_not_a_hook_and_stays() {
    let dir = init_repo();
    run_git(&dir, &["config", "core.hooksPath", ".githooks"]);
    std::fs::create_dir_all(dir.join(".githooks/pre-commit")).unwrap();
    let out = adr(&dir, &["install"], &[], "");
    assert!(dir.join(".githooks/pre-commit").is_dir());
    assert!(!dir.join(".githooks/pre-commit.own").exists());
    assert!(!out.status.success());
}

// adr: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
#[test]
fn the_tools_own_text_in_git_hooks_is_left_where_it_is() {
    let dir = init_repo();
    write_mode(&dir.join(".git/hooks/pre-commit"), PRE_COMMIT_TEXT, 0o755);
    write_mode(&dir.join(".git/hooks/commit-msg"), EARLIER[2].1, 0o755);
    install_ok(&dir);
    assert!(!dir.join(".githooks/pre-commit.own").exists());
    assert!(!dir.join(".githooks/commit-msg.own").exists());
    assert!(dir.join(".git/hooks/pre-commit").exists());
    assert!(dir.join(".git/hooks/commit-msg").exists());
}

// adr: db76c9a0-b4fc-4aa2-8afb-47cd78ef7138
#[test]
fn check_expands_a_home_relative_hooks_path_as_git_does() {
    let dir = init_repo();
    install_ok(&dir);
    approve_empty_terms(&dir);
    let home = dir.parent().unwrap();
    let rel = dir.file_name().unwrap().to_string_lossy().to_string();
    run_git(
        &dir,
        &["config", "core.hooksPath", &format!("~/{}/.githooks", rel)],
    );
    let out = adr(
        &dir,
        &["check"],
        &[("HOME", &home.display().to_string())],
        "",
    );
    assert!(out.status.success(), "{}", stdout(&out));
    assert_eq!(stdout(&out), "");
}
