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
/// The roles file: an untracked file holding one `<identifier>\t<word>` line
/// for each session that took a role ("Does the ledger hold the planners?",
/// 1cec44fc: no; an untracked file under adrdom/state holds the identifier
/// and the word). `install`'s gitignore keeps it out of git.
pub const ROLES_PATH: &str = "adrdom/state/roles.tsv";
/// The reader's file: one row for each word the reader gave on a pair of
/// texts (`reader::Row`). `check` holds each row at HEAD byte for byte or gone whole; the tool drops
/// a row whose text or neighbour is gone from the records when it appends.
pub const READER_PATH: &str = "adrdom/state/reader.tsv";
/// The neighbours file: an untracked file holding one row for each neighbour
/// the tool brought for a heading, the heading hash and the neighbour's
/// identifier. The reader reads none of them ("Does the reader read a
/// neighbour of a heading?", 60341be0: no), so the row holds no word.
/// `install`'s gitignore keeps it out of git.
pub const NEIGHBOURS_PATH: &str = "adrdom/state/neighbours.tsv";
/// The repo's own terms file.
/// Where blame files live, relative to the repo root (per "What does a
/// blame file hold of its origin?", d2851ea1 A): one `<name>.md` per origin,
/// holding its line ranges and whether each holds a decision or only detail.
/// `do_approve` dispatches a path under here the same way it dispatches
/// `TERMS_PATH`, by prefix.
pub const BLAME_DIR: &str = "adrdom/blame";

pub const TERMS_PATH: &str = "adrdom/terms.md";
/// The terms brief (per "Does the tool write a terms brief for the agent?",
/// ec0b9293 A): the file `approve` writes from the terms file, `check`
/// holds to it, and the brief names for the agent to read.
pub const TERMS_BRIEF_PATH: &str = "adrdom/state/terms-brief.md";
/// The brief's text (per "Does install write outside adrdom?", 4f925b13 A):
/// the file `install` writes under adrdom, which the marked section of
/// AGENTS.md names with an @ line, and `check` holds to.
pub const BRIEF_PATH: &str = "adrdom/state/brief.md";
/// The file of a satellite holding the path to its primary, relative to the
/// satellite's root (per "Does install take the path of a satellite?").
pub const PRIMARY_PATH: &str = "adrdom/primary";
/// The primary's list of its satellites, one `<path>\t<url>` row each (per
/// "What does the primary hold of a satellite?").
pub const SATELLITES_PATH: &str = "adrdom/state/satellites.tsv";
/// Where the hashed ban list lives, relative to the repo root: the one
/// literal for this path, used by every site that reads or writes it
/// (`ban`, `unban`, `ask`, `check --banned`, `terms_refusals`, `install`).
pub const BAN_LIST: &str = "adrdom/state/banned.sha256";

/// Where `adr install` writes the ledger's merge rule, relative to the
/// repo root (Decision 7, db56a285-946f-48b1-b073-bb1e90192185 -- install
/// writes a git attributes file under adrdom that keeps the ledger rows of
/// both branches at a merge; the ban list gets no such rule). Its content
/// is `commands::install::GITATTRIBUTES_CONTENT`; `check`'s `install`
/// check reads this path the same way it reads the git hooks.
pub const GITATTRIBUTES_PATH: &str = "adrdom/.gitattributes";

/// Where `adr install` writes the rule that keeps the embedding index out
/// of git (the model is outside the repository, in the user cache directory:
/// "Does install put the embedding model outside the repository?", 459c1b77 A;
/// "Does git track a file the tool can get again without the owner?")
/// and the roles file too (1cec44fc).
/// Its content is `commands::install::GITIGNORE_CONTENT`; `check`'s `install`
/// check reads this path the same way it reads `GITATTRIBUTES_PATH`.
pub const GITIGNORE_PATH: &str = "adrdom/.gitignore";
/// The directory under adrdom that holds the embedding index.
pub const CACHE_DIR: &str = "adrdom/cache";

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
/// Where `adr install` writes the pre-push hook, relative to the repo root
/// (Decision P, 19e1c023-a678-4842-a414-bb0e45c41601 -- check fails a push
/// unless every ledger row of the remote branch is in the ledger being
/// pushed). Its content is `commands::install::PRE_PUSH_HOOK`; checked the
/// same way as `PRE_COMMIT_HOOK_PATH`.
pub const PRE_PUSH_HOOK_PATH: &str = ".githooks/pre-push";

/// Whether `s` is git's all-zero sha, of whatever length (40 hex digits for
/// sha-1, 64 for sha-256): the value a pre-push line carries for "this ref
/// doesn't exist yet" (as the local sha, a deleted branch; as the remote
/// sha, a branch new on the remote), and the value a workflow's "commit
/// before the push" carries when there was none (the branch's first push).
/// Checked by shape, not a fixed-width literal, so it reads either sha
/// width the same way.
pub fn is_zero_sha(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c == '0')
}

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
    git_bytes(root, args).map(|b| String::from_utf8_lossy(&b).to_string())
}

/// What git prints, as bytes: `None` on a failure to run it or a non-zero
/// exit.
fn git_bytes(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

/// Every path the index holds, read once per repository for the process:
/// what `Source::Index` answers `exists` and `is_dir` from, so a run that
/// asks of many paths (the origin of each blame file, the removed topic
/// files at a commit) asks git once.
fn index_files(root: &Path) -> std::sync::Arc<std::collections::HashSet<String>> {
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex, OnceLock};
    static INDEX: OnceLock<Mutex<HashMap<std::path::PathBuf, Arc<HashSet<String>>>>> =
        OnceLock::new();
    let map = INDEX.get_or_init(Default::default);
    if let Some(found) = map.lock().unwrap_or_else(|e| e.into_inner()).get(root) {
        return found.clone();
    }
    let Some(paths) = git_paths(root, &["ls-files"]) else {
        // Not kept: a failure to run git is asked again.
        return Arc::new(HashSet::new());
    };
    let set: Arc<HashSet<String>> = Arc::new(paths.into_iter().collect());
    map.lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(root.to_path_buf(), set.clone());
    set
}

/// Every path the index holds with mode 120000 (a symbolic link), read once
/// per repository for the process, the same way as `index_files`.
fn index_links(root: &Path) -> std::sync::Arc<std::collections::HashSet<String>> {
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex, OnceLock};
    static LINKS: OnceLock<Mutex<HashMap<std::path::PathBuf, Arc<HashSet<String>>>>> =
        OnceLock::new();
    let map = LINKS.get_or_init(Default::default);
    if let Some(found) = map.lock().unwrap_or_else(|e| e.into_inner()).get(root) {
        return found.clone();
    }
    let Some(listing) = git_bytes(root, &["ls-files", "-s", "-z"]) else {
        // Not kept: a failure to run git is asked again.
        return Arc::new(HashSet::new());
    };
    let set: Arc<HashSet<String>> = Arc::new(index_link_paths(&listing));
    map.lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(root.to_path_buf(), set.clone());
    set
}

/// The paths with mode 120000 in `git ls-files -s -z` output `listing`.
fn index_link_paths(listing: &[u8]) -> std::collections::HashSet<String> {
    String::from_utf8_lossy(listing)
        .split('\0')
        .filter_map(|entry| {
            let (meta, name) = entry.split_once('\t')?;
            meta.starts_with("120000 ").then(|| name.to_string())
        })
        .collect()
}

/// The paths staged for the next commit, one entry per path, renames shown
/// as a delete and an add; those deleted in the index are left out unless
/// `with_deleted`.
pub fn staged_paths(root: &Path, with_deleted: bool) -> Option<Vec<String>> {
    // No env is cleared: git honours an inherited GIT_INDEX_FILE, as the
    // hooks run it.
    let mut args = vec!["diff", "--cached", "--name-only", "--no-renames"];
    if !with_deleted {
        args.push("--diff-filter=d");
    }
    git_paths(root, &args)
}

/// The paths a git listing command prints (`ls-files`, `diff --name-only`),
/// one entry per path. Runs the command with `-z`: git then ends each path
/// with a NUL and prints it as it is, where the plain form wraps a path
/// holding a byte outside ASCII in quotes and escapes it, a name no later
/// read finds (per "What ties a test to a decision?": "every tracked file
/// outside adrdom"). `None` on any failure to run git.
pub fn git_paths(root: &Path, args: &[&str]) -> Option<Vec<String>> {
    let (cmd, rest) = args.split_first()?;
    let mut full: Vec<&str> = vec![cmd, "-z"];
    full.extend_from_slice(rest);
    git(root, &full).map(|s| {
        s.split('\0')
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string())
            .collect()
    })
}

/// The git blob sha `git hash-object --stdin` gives `content`, without
/// needing a real file on disk: lets a caller hash content it already
/// holds as a string -- read through `Source::Index` at a commit, or
/// `Source::WorkingTree` off disk -- the same way `check` reads every
/// other file it judges (per "Does check at a commit read the staged
/// files?"), rather than only ever hashing whatever sits in the working
/// tree right now. `None` on any failure to run git.
pub fn git_hash_object_stdin(root: &Path, content: &str) -> Option<String> {
    git_hash_object_with(root, content, &["--stdin"])
}

/// Like `git_hash_object_stdin`, but the way git stores the file at `path`
/// from the working tree: the repository's filters for that path (line
/// ending conversion) are applied to `content` first.
pub fn git_hash_object_filtered(root: &Path, content: &str, path: &str) -> Option<String> {
    git_hash_object_with(root, content, &["--stdin", "--path", path])
}

fn git_hash_object_with(root: &Path, content: &str, extra: &[&str]) -> Option<String> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("git")
        .current_dir(root)
        .arg("hash-object")
        .args(extra)
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

/// Whether git would run the hook at `path` at all: git skips a hook file
/// that is not executable, silently, so a hook whose content is current
/// but whose mode is not is no hook. On unix this is the owner execute
/// bit of the working-tree file (the file git runs, whatever `check` is
/// reading content from); elsewhere there is no such bit and every hook
/// counts as executable.
pub fn hook_is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o100 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        true
    }
}

/// The naming half of the file name rule (per "What names a topic file?": a
/// topic file is named by its topic in lowercase words joined by hyphens,
/// with no number or prefix):
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

/// The other half of the file name rule (per "What names a topic file?",
/// "is named by it"): the file
/// stem a topic file with h1 text `topic` must carry -- the text lowercased,
/// every run of characters that are not ASCII letters or digits turned into
/// one hyphen, leading and trailing hyphens dropped. "Ban" gives "ban",
/// "Two Words" gives "two-words", "Rule 2" gives "rule-2". A digit stays:
/// `valid_topic_file_name` accepts none, so a topic whose stem holds one
/// has no name any file can carry, and check refuses the topic text itself
/// (`msg::topic_unnameable`) rather than inventing a name the decision's
/// "named by it" would not give. Check compares the stem to the file's own
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
        assert_eq!(expected_topic_file_stem(""), "");
    }

    // A digit stays in the derived stem, and that stem is then one no
    // file name can carry (`valid_topic_file_name`, the file name rule, accepts no
    // digit): check refuses such a topic outright instead of deriving a
    // name the topic text does not give.
    #[test]
    fn expected_stem_keeps_digits_and_is_then_unnameable() {
        use super::valid_topic_file_name;

        assert_eq!(expected_topic_file_stem("Rule 2"), "rule-2");
        assert_eq!(expected_topic_file_stem("OAuth2"), "oauth2");
        for topic in ["Rule 2", "OAuth2"] {
            assert!(
                !valid_topic_file_name(&format!("{}.md", expected_topic_file_stem(topic))),
                "{}",
                topic
            );
        }
    }
}

#[cfg(test)]
mod git_hash_object_stdin_tests {
    use super::git_hash_object_stdin;
    use std::path::Path;

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
            // The staged bytes as text, refused as `WorkingTree` refuses
            // them when they are not UTF-8: never a lossy copy hashed or
            // judged as if it were the file.
            Source::Index => git_bytes(root, &["show", &format!(":{}", rel)])
                .and_then(|b| String::from_utf8(b).ok()),
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
            Source::Index => index_files(root).contains(rel) || self.is_dir(root, rel),
        }
    }

    /// Whether `rel` (a path relative to `root`) is a directory holding at
    /// least one file in this source.
    pub fn is_dir(&self, root: &Path, rel: &str) -> bool {
        match self {
            Source::WorkingTree => root.join(rel).is_dir(),
            // `git ls-files -- <rel>` also lists `rel` itself when it is a
            // tracked file, so only a path below `rel` makes it a directory.
            Source::Index => {
                let prefix = format!("{}/", rel.trim_end_matches('/'));
                index_files(root).iter().any(|p| p.starts_with(&prefix))
            }
        }
    }

    /// Whether `rel` (a path relative to `root`), or any directory on the
    /// way to it, is a symbolic link. The working tree answers from the
    /// file system, the index from the mode of its entry (120000).
    pub fn is_symlink(&self, root: &Path, rel: &str) -> bool {
        let mut prefix = String::new();
        for part in rel.split('/').filter(|p| !p.is_empty()) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            let link = match self {
                Source::WorkingTree => root
                    .join(&prefix)
                    .symlink_metadata()
                    .is_ok_and(|m| m.file_type().is_symlink()),
                Source::Index => index_links(root).contains(&prefix),
            };
            if link {
                return true;
            }
        }
        false
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
            Source::Index => git_paths(root, &["ls-files", "--", rel_dir]).unwrap_or_default(),
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
