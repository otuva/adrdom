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
    list_decision_files_via(root, &Source::WorkingTree)
}

/// Like `list_decision_files`, but through `source`: every `.md` path
/// under `adrdom/decisions/` that `source` considers to exist. `check`'s
/// only caller of this form -- everything else always wants the working
/// tree, so it keeps calling the plain function above.
pub fn list_decision_files_via(root: &Path, source: &Source) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = source
        .list_under(root, "adrdom/decisions")
        .into_iter()
        .filter(|p| p.ends_with(".md"))
        .map(|rel| root.join(rel))
        .collect();
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

/// Every file under `dir` (any extension), found recursively on disk.
/// `list_decision_files` filters this to `.md` itself; kept unfiltered
/// here so `Source::WorkingTree`'s other uses (the `.claude`/`.codex`
/// directory checks) can share it.
fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_files(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// Where `check` reads a file's bytes, and which files it considers to
/// exist, for every check that reads one (per "Does check at a commit
/// read the staged files?", 34e81f57 A): `Index` reads the git index --
/// exactly the content and file set about to enter the commit -- and is
/// used whenever `check` is running at a commit (`--staged`, or
/// `--message`, since the commit-msg hook's full report judges the same
/// staged tree the pre-commit hook already saw, not just the message
/// text). `WorkingTree` reads the filesystem: used outside a commit, and
/// unconditionally by every other command (`ask`, `approve`, `show`,
/// `serve`, ...), which never take this type at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    WorkingTree,
    Index,
}

impl Source {
    pub fn for_check(staged: bool, message: bool) -> Source {
        if staged || message {
            Source::Index
        } else {
            Source::WorkingTree
        }
    }

    /// `rel`'s content (a path relative to `root`, forward slashes), or
    /// `None` when it doesn't exist in this source at all.
    pub fn read(&self, root: &Path, rel: &str) -> Option<String> {
        match self {
            Source::WorkingTree => std::fs::read_to_string(root.join(rel)).ok(),
            Source::Index => git(root, &["show", &format!(":{}", rel)]),
        }
    }

    /// Whether `rel` (a path relative to `root`) is a directory holding at
    /// least one file in this source.
    pub fn is_dir(&self, root: &Path, rel: &str) -> bool {
        match self {
            Source::WorkingTree => root.join(rel).is_dir(),
            Source::Index => !self.list_under(root, rel).is_empty(),
        }
    }

    /// Every file path (relative to `root`, forward slashes) this source
    /// considers to exist under `rel_dir`, unsorted.
    pub fn list_under(&self, root: &Path, rel_dir: &str) -> Vec<String> {
        match self {
            Source::WorkingTree => {
                let mut out = Vec::new();
                walk_files(&root.join(rel_dir), &mut out);
                out.into_iter()
                    .map(|p| {
                        p.strip_prefix(root)
                            .unwrap_or(&p)
                            .to_string_lossy()
                            .replace('\\', "/")
                    })
                    .collect()
            }
            Source::Index => git(root, &["ls-files", "--", rel_dir])
                .map(|s| s.lines().map(|l| l.to_string()).collect())
                .unwrap_or_default(),
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
