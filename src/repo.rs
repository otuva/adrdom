use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Runs `git <args>` in `root`, returning stdout on success and None on
/// any non-zero exit (e.g. `git show HEAD:...` for a path that doesn't
/// exist yet on HEAD).
pub fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        None
    }
}

pub fn list_decision_files(root: &Path) -> Vec<PathBuf> {
    let dir = root.join("adrdom/decisions");
    let mut out = Vec::new();
    walk_md(&dir, &mut out);
    out.sort();
    out
}

fn walk_md(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_md(&p, out);
        } else if p.extension().map(|e| e == "md").unwrap_or(false) {
            out.push(p);
        }
    }
}

/// The owner-authorization gate shared by every owner command (`answer`,
/// `approve`, `unban`, `serve`, `uninstall`): the owner's sign is stdin
/// being an interactive terminal. No env var is consulted (9.2 A: an agent
/// can set or unset any env var, so one is not evidence of who is typing).
/// Built with the test-only `owner-bypass` feature and `ADR_OWNER_BYPASS=1`
/// bypasses this for tests, which never run with a real TTY on stdin.
pub fn owner_authorized() -> bool {
    #[cfg(feature = "owner-bypass")]
    {
        if std::env::var("ADR_OWNER_BYPASS").as_deref() == Ok("1") {
            return true;
        }
    }
    std::io::stdin().is_terminal()
}

/// The refusal line every owner command prints when `owner_authorized`
/// fails: `<cmd>: refusing: this is the owner's command; owner runs: adr
/// <cmd> <args>` -- `args` is the command line exactly as the agent typed
/// it (empty for a command that takes none), never mentioning a terminal,
/// TTY, or env var.
pub fn owner_refusal(cmd: &str, args: &str) -> String {
    if args.is_empty() {
        format!(
            "{}: refusing: this is the owner's command; owner runs: adr {}",
            cmd, cmd
        )
    } else {
        format!(
            "{}: refusing: this is the owner's command; owner runs: adr {} {}",
            cmd, cmd, args
        )
    }
}
