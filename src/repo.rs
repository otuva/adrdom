use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The single directory every governed file lives under.
pub const ADRDOM_DIR: &str = "adrdom";
/// Where topic files live, relative to the repo root.
pub const DECISIONS_DIR: &str = "adrdom/decisions";
/// Where the tool's own state (ledger, ban list) lives, relative to the
/// repo root.
pub const STATE_DIR: &str = "adrdom/state";
/// The append-only ledger of every act (`ask`, `answer`, `approve`, ...).
pub const LEDGER_PATH: &str = "adrdom/state/ledger.tsv";
/// The repo's own terms file.
/// Where blame files live, relative to the repo root (per "What does a
/// blame file hold?", 34f0202a A): one `<stem>.md` per origin, holding its
/// line ranges and what each range went to. `do_approve` dispatches a path
/// under here the same way it dispatches `TERMS_PATH`, by prefix.
pub const BLAME_DIR: &str = "adrdom/blame";

pub const TERMS_PATH: &str = "adrdom/terms.md";
/// Where the hashed ban list lives, relative to the repo root: the one
/// literal for this path, used by every site that reads or writes it
/// (`ban`, `unban`, `ask`, `check --banned`, `terms_refusals`, `install`).
pub const BAN_LIST: &str = "adrdom/state/banned.sha256";

/// Where `adr install` writes the pre-commit hook, relative to the repo
/// root. Its content is `commands::install::PRE_COMMIT_HOOK`; `check`'s
/// `install` check reads this path the same way it reads AGENTS.md (per
/// "Does check hold the files install writes?", 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
/// answer A, extended by the owner to cover the git hooks).
pub const PRE_COMMIT_HOOK_PATH: &str = ".githooks/pre-commit";
/// Where `adr install` writes the commit-msg hook, relative to the repo
/// root. Its content is `commands::install::COMMIT_MSG_HOOK`; checked the
/// same way as `PRE_COMMIT_HOOK_PATH`.
pub const COMMIT_MSG_HOOK_PATH: &str = ".githooks/commit-msg";

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

/// The git blob sha `git hash-object --stdin` gives `content`, without
/// needing a real file on disk: lets a caller hash content it already
/// holds as a string -- read through `Source::Index` at a commit, or
/// `Source::WorkingTree` off disk -- the same way `check` reads every
/// other file it judges (per "Does check at a commit read the staged
/// files?"), rather than only ever hashing whatever sits in the working
/// tree right now. `None` on any failure to run git.
pub fn git_hash_object_stdin(root: &Path, content: &str) -> Option<String> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("git")
        .current_dir(root)
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(content.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
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
        .list_under(root, DECISIONS_DIR)
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

/// The other half of 76bd7ef0 A's naming rule ("is named by it"): the file
/// stem a topic file with h1 text `topic` must carry -- the text lowercased,
/// every run of characters that are not ASCII letters or digits turned into
/// one hyphen, leading and trailing hyphens dropped. "Ban" gives "ban",
/// "Two Words" gives "two-words". Check compares this to the file's own stem
/// only once `valid_topic_file_name` has accepted the name.
pub fn expected_topic_file_stem(topic: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in topic.chars() {
        if c.is_ascii_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('-');
            }
            gap = false;
            out.push(c.to_ascii_lowercase());
        } else {
            gap = true;
        }
    }
    out
}

#[cfg(test)]
mod topic_name_tests {
    use super::expected_topic_file_stem;

    #[test]
    fn expected_stem_lowercases_and_hyphenates_runs() {
        assert_eq!(expected_topic_file_stem("Ban"), "ban");
        assert_eq!(expected_topic_file_stem("Two Words"), "two-words");
        assert_eq!(expected_topic_file_stem("  A -- b_C  "), "a-b-c");
        assert_eq!(expected_topic_file_stem("Topic 2"), "topic-2");
        assert_eq!(expected_topic_file_stem(""), "");
    }
}

#[cfg(test)]
mod git_hash_object_stdin_tests {
    use super::git_hash_object_stdin;
    use std::path::Path;

    // adr: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6
    //
    // "the blob comparison must fail closed": `check`'s `blame` category
    // treats `None` from this function as a failure, never a silent pass.
    // Verified here at the primitive: a `root` git cannot even spawn in
    // (no such directory) returns `None`, not a made-up hash.
    #[test]
    fn returns_none_when_git_cannot_run_in_root() {
        let bogus = Path::new("/no/such/directory/at/all/adr-test");
        assert!(git_hash_object_stdin(bogus, "hello\n").is_none());
    }
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

    /// Whether `rel` (a path relative to `root`) names anything at all in
    /// this source -- a file or a directory alike -- regardless of whether
    /// its content can be read as text. Lets a caller tell "nothing at this
    /// path" apart from "something is here but failed to read" (`read`
    /// returns `None` for both), which `read` alone cannot.
    pub fn exists(&self, root: &Path, rel: &str) -> bool {
        match self {
            Source::WorkingTree => root.join(rel).exists(),
            Source::Index => {
                git(root, &["cat-file", "-e", &format!(":{}", rel)]).is_some()
                    || self.is_dir(root, rel)
            }
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
                    .map(|p| crate::state::rel_path(root, &p))
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
