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

/// The naming half of 76bd7ef0 A ("each file holds one topic and is named
/// by it in lowercase words joined by hyphens, with no number or prefix"):
/// whether `name` -- a file's own base name, e.g. from `Path::file_name`,
/// never a whole path -- is one or more lowercase ASCII letter words joined
/// by single hyphens, ending in `.md`. Equivalent to the regex
/// `^[a-z]+(-[a-z]+)*\.md$`: a digit, an underscore, an uppercase letter, or
/// a leading, trailing, or doubled hyphen all fail it. `list_decision_files`
/// walks into subdirectories a topic file might sit under; this judges only
/// the file's own base name, never the subdirectory part of its path, since
/// the decision names "the file", not the file's place in the tree.
pub fn valid_topic_file_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".md") else {
        return false;
    };
    if stem.is_empty() {
        return false;
    }
    stem.split('-')
        .all(|word| !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase()))
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
/// being an interactive terminal. No env var is consulted (per "What does
/// an owner command look for before it runs?": an agent
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
