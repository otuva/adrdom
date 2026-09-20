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

/// The owner-authorization gate shared by `adr lock` and `adr unban`:
/// refuse unless running interactively as a human (no CLAUDECODE, stdin is
/// a TTY), or built with the test-only `owner-bypass` feature and
/// `ADR_OWNER_BYPASS=1`.
pub fn owner_authorized() -> bool {
    #[cfg(feature = "owner-bypass")]
    {
        if std::env::var("ADR_OWNER_BYPASS").as_deref() == Ok("1") {
            return true;
        }
    }
    if std::env::var_os("CLAUDECODE").is_some() {
        return false;
    }
    std::io::stdin().is_terminal()
}
