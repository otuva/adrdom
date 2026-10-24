// The program the reader runs: the one of the agent named at install.

mod common;

use adr::reader::{self, Kind, Pair, Program};
use common::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

fn installed(flags: &[&str]) -> PathBuf {
    let dir = init_repo();
    let mut args = vec!["install"];
    args.extend_from_slice(flags);
    let out = adr(&dir, &args, &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    dir
}

fn pair() -> Pair {
    Pair {
        kind: Kind::Plan,
        new_text: "Does the widget glow?".to_string(),
        neighbour: "id-1".to_string(),
        neighbour_text: "Does the widget shine?".to_string(),
        letters: Vec::new(),
    }
}

// adr: 5418b256-dfae-439a-96c1-57d82380cd1e
// "What does the reader run?": installed for claude, the reader is claude.
#[test]
fn a_repository_installed_for_claude_reads_with_claude() {
    let dir = installed(&["--claude"]);
    assert_eq!(reader::program(&dir), Some(Program::Claude));
}

// adr: 5418b256-dfae-439a-96c1-57d82380cd1e
// "What does the reader run?": installed for codex, the reader is codex.
#[test]
fn a_repository_installed_for_codex_reads_with_codex() {
    let dir = installed(&["--codex"]);
    assert_eq!(reader::program(&dir), Some(Program::Codex));
}

// adr: 5418b256-dfae-439a-96c1-57d82380cd1e
// "What does the reader run?": with two agents installed the tool picks claude.
#[test]
fn with_both_agents_installed_the_tool_picks_claude() {
    let dir = installed(&["--claude", "--codex"]);
    assert_eq!(reader::program(&dir), Some(Program::Claude));
}

// adr: 5418b256-dfae-439a-96c1-57d82380cd1e
// "What does the reader run?": a repository installed for no agent has no reader.
#[test]
fn a_repository_installed_for_no_agent_has_no_reader() {
    let dir = installed(&[]);
    assert_eq!(reader::program(&dir), None);
    let e = reader::load_in(&dir, &OsString::from("/nonexistent"))
        .err()
        .expect("load refuses");
    assert!(
        e.contains("reader: no agent is installed for this repository"),
        "{}",
        e
    );
    assert!(e.contains("agent runs: adr install --claude"), "{}", e);
}

// adr: 5418b256-dfae-439a-96c1-57d82380cd1e
// "What does the reader run?": the program is the installed agent's, found on PATH, else named missing.
#[test]
fn the_missing_program_named_is_the_installed_agents() {
    for (flag, name) in [("--claude", "claude"), ("--codex", "codex")] {
        let dir = installed(&[flag]);
        let e = reader::load_in(&dir, &OsString::from("/nonexistent"))
            .err()
            .expect("load refuses");
        assert!(
            e.contains(&format!("reader: the {} program is missing", name)),
            "{}",
            e
        );
        assert!(
            e.contains(&format!("agent installs the {} program", name)),
            "{}",
            e
        );
    }
}

// adr: 5418b256-dfae-439a-96c1-57d82380cd1e
// "What does the reader run?": on Windows the program is `<name>.exe`, else `<name>.cmd`.
#[test]
fn the_program_file_is_looked_up_with_each_suffix_in_order() {
    let dir = init_repo();
    let path = std::env::join_paths([&dir]).unwrap();
    let file = |n: &str| {
        let f = dir.join(n);
        std::fs::write(&f, "").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    };
    file("codex.cmd");
    assert_eq!(
        reader::find_program("codex", &path, &[".exe", ".cmd"]),
        Some(dir.join("codex.cmd"))
    );
    file("codex.exe");
    assert_eq!(
        reader::find_program("codex", &path, &[".exe", ".cmd"]),
        Some(dir.join("codex.exe"))
    );
    assert_eq!(reader::find_program("codex", &path, &[""]), None);
    assert_eq!(
        reader::find_program("claude", &path, &[".exe", ".cmd"]),
        None
    );
}

// adr: 5418b256-dfae-439a-96c1-57d82380cd1e
// "What does the reader run?": a file with no executable bit is not the program; the search goes on.
#[cfg(unix)]
#[test]
fn a_file_that_cannot_run_is_skipped() {
    use std::os::unix::fs::PermissionsExt;
    let a = init_repo();
    let b = init_repo();
    std::fs::write(a.join("claude"), "").unwrap();
    std::fs::set_permissions(a.join("claude"), std::fs::Permissions::from_mode(0o644)).unwrap();
    let path = std::env::join_paths([&a, &b]).unwrap();
    assert_eq!(reader::find_program("claude", &path, &[""]), None);
    std::fs::write(b.join("claude"), "").unwrap();
    std::fs::set_permissions(b.join("claude"), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        reader::find_program("claude", &path, &[""]),
        Some(b.join("claude"))
    );
}

#[cfg(unix)]
mod stubs {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Mutex;

    // A script written and run at once can meet a fork of another test
    // thread holding its file open; one at a time.
    static LOCK: Mutex<()> = Mutex::new(());

    fn stub(dir: &Path, name: &str, script: &str) {
        let f = dir.join(name);
        std::fs::write(&f, script).unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    // adr: 5418b256-dfae-439a-96c1-57d82380cd1e
    // "What does the reader run?": the claude program is run with the message on its standard input.
    #[test]
    fn the_claude_program_is_run_and_its_reply_read() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = installed(&["--claude"]);
        let bin = init_repo();
        stub(&bin, "claude", &format!("#!/bin/sh\ncat > {0}/stdin.txt\nprintf '%s\\n' \"$@\" > {0}/args.txt\nprintf '%s\\n' '{{\"result\":\"They ask the same.\\nsame\"}}'\n", bin.display()));
        let r = reader::load_in(&dir, bin.as_os_str()).expect("claude found");
        assert_eq!(r.model(), reader::MODEL);
        let rows = reader::read_with(&dir, r.as_ref(), &[pair()])
            .unwrap()
            .all()
            .unwrap();
        assert_eq!(rows[0].word, "same");
        assert_eq!(rows[0].model, reader::MODEL);
        let got = std::fs::read_to_string(bin.join("stdin.txt")).unwrap();
        assert!(
            got.contains("Does the widget glow?") && got.contains("Does the widget shine?"),
            "{}",
            got
        );
        // The system prompt goes by file, never as an argument.
        let args = std::fs::read_to_string(bin.join("args.txt")).unwrap();
        assert!(
            args.lines().any(|l| l == "--system-prompt-file"),
            "{}",
            args
        );
        assert!(!args.lines().any(|l| l == "--system-prompt"), "{}", args);
    }

    // adr: 5418b256-dfae-439a-96c1-57d82380cd1e
    // "What does the reader run?": the codex program is run with the prompt on its standard input and its reply file read.
    #[test]
    fn the_codex_program_is_run_and_its_reply_file_read() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = installed(&["--codex"]);
        let bin = init_repo();
        stub(
            &bin,
            "codex",
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > {0}/args.txt\nwhile [ $# -gt 0 ]; do [ \"$1\" = -o ] && out=$2; shift; done\ncat > {0}/stdin.txt\nprintf 'They ask the same.\\nsame\\n' > \"$out\"\n",
                bin.display()
            ),
        );
        let r = reader::load_in(&dir, bin.as_os_str()).expect("codex found");
        assert_eq!(r.model(), reader::CODEX_MODEL);
        let rows = reader::read_with(&dir, r.as_ref(), &[pair()])
            .unwrap()
            .all()
            .unwrap();
        assert_eq!(rows[0].word, "same");
        assert_eq!(rows[0].model, reader::CODEX_MODEL);
        let got = std::fs::read_to_string(bin.join("stdin.txt")).unwrap();
        assert!(
            got.contains("Does the widget glow?") && got.contains("Does the widget shine?"),
            "{}",
            got
        );
        assert!(got.contains("====="), "the prompt is set apart: {}", got);
        let args = std::fs::read_to_string(bin.join("args.txt")).unwrap();
        for flag in [
            "--ignore-user-config",
            "--ignore-rules",
            "shell_tool",
            reader::CODEX_MODEL,
        ] {
            assert!(
                args.lines().any(|l| l == flag),
                "{} missing: {}",
                flag,
                args
            );
        }
    }
}

// adr: 5418b256-dfae-439a-96c1-57d82380cd1e
// "What does the reader run?": the real codex program reads one pair.
#[test]
#[ignore]
fn the_real_codex_program_reads_one_pair() {
    let dir = installed(&["--codex"]);
    let path = std::env::var_os("PATH").unwrap_or_default();
    let r = match reader::load_in(&dir, &path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("skipped: {}", e);
            return;
        }
    };
    let rows = reader::read_with(&dir, r.as_ref(), &[pair()])
        .unwrap()
        .all()
        .unwrap();
    eprintln!("word: {} reason: {}", rows[0].word, rows[0].reason);
}
