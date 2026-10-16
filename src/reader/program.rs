//! The programs that read: `claude`, `codex` and the fake reader, how each is
//! found and run, and the private directory a run is made in.
use super::*;

/// The agent program that reads, one of the two `adr install` writes
/// files for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Program {
    Claude,
    Codex,
}

impl Program {
    pub fn name(self) -> &'static str {
        match self {
            Program::Claude => "claude",
            Program::Codex => "codex",
        }
    }

    pub fn model(self) -> &'static str {
        match self {
            Program::Claude => MODEL,
            Program::Codex => CODEX_MODEL,
        }
    }
}

/// The program of the agent installed in `root` ("What does the reader
/// run?", 5418b256 A): `claude` when `CLAUDE.md` has the import line,
/// else `codex` when its rules file is there, else none.
pub fn program(root: &Path) -> Option<Program> {
    let imports = std::fs::read_to_string(root.join("CLAUDE.md"))
        .map(|c| {
            c.lines()
                .any(|l| l == crate::commands::install::CLAUDE_MD_IMPORT)
        })
        .unwrap_or(false);
    if imports {
        Some(Program::Claude)
    } else if root.join(".codex/rules/adrdom.rules").is_file() {
        Some(Program::Codex)
    } else {
        None
    }
}

/// The model the rows of `root` are read with now: the installed
/// program's, and the fixed model when there is none or the fake reads.
pub fn model_of(root: &Path) -> &'static str {
    #[cfg(feature = "fake-model")]
    if std::env::var("ADR_FAKE_MODEL").as_deref() == Ok("1") {
        return MODEL;
    }
    program(root).map(Program::model).unwrap_or(MODEL)
}

/// The file suffixes a program name may carry on this platform.
pub(super) const SUFFIXES: &[&str] = if cfg!(windows) {
    &[".exe", ".cmd"]
} else {
    &[""]
};

/// Whether `f` is a file the system can run: on unix, one with an
/// executable bit; elsewhere any file.
fn is_runnable(f: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(f) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// The runnable file of the program `name` in a directory of `path`: in
/// each directory, `name` with each of `suffixes` in order. On unix a file
/// with no executable bit is skipped and the search goes on.
pub fn find_program(name: &str, path: &std::ffi::OsStr, suffixes: &[&str]) -> Option<PathBuf> {
    std::env::split_paths(path)
        .flat_map(|d| {
            suffixes
                .iter()
                .map(move |s| d.join(format!("{}{}", name, s)))
        })
        .find(|f| is_runnable(f))
}

/// Reads with `claude`, found at `exe`.
pub struct Claude {
    pub exe: PathBuf,
}

/// Reads with `codex`, found at `exe`.
pub struct Codex {
    pub exe: PathBuf,
}

/// The message the reader is given for `pair`. Two decisions are given in
/// the pair's fixed order: the new side, the lower identifier, first. Every
/// other kind gives the record first and the new question second.
fn user_message(pair: &Pair) -> String {
    let (first, second) = pair.kind.labels();
    let (a, b) = match pair.kind {
        Kind::Set => (&pair.new_text, &pair.neighbour_text),
        _ => (&pair.neighbour_text, &pair.new_text),
    };
    format!("{}:\n{}\n\n{}:\n{}", first, a, second, b)
}

/// Runs `cmd` in `cwd` with `input` on its standard input, and returns its
/// standard output. The input is written on its own thread and closed, so
/// a program that answers before it has read everything cannot block this
/// one. A run that outlasts `TIMEOUT` is killed; a nonzero exit is an error
/// with the program's standard error.
fn run(mut cmd: Command, name: &str, input: String, cwd: &Path) -> Result<String, String> {
    let mut child = cmd
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                msg::reader_missing(name)
            } else {
                format!("cannot run {}: {}", name, e)
            }
        })?;
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let mut out = child.stdout.take().ok_or("no stdout")?;
    let mut err = child.stderr.take().ok_or("no stderr")?;
    let t_in = std::thread::spawn(move || {
        use std::io::Write;
        let _ = stdin.write_all(input.as_bytes());
    });
    let t_out = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let t_err = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(s) => break s,
            None if start.elapsed() > TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("no answer in {} seconds", TIMEOUT.as_secs()));
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    };
    let _ = t_in.join();
    let stdout = t_out.join().unwrap_or_default();
    let stderr = t_err.join().unwrap_or_default();
    if !status.success() {
        return Err(format!(
            "{} exited with {}: {}",
            name,
            status,
            stderr.trim()
        ));
    }
    Ok(stdout)
}

impl Reader for Claude {
    fn model(&self) -> &'static str {
        MODEL
    }

    fn ask(&self, pair: &Pair) -> Result<String, String> {
        let user = user_message(pair);
        // A private directory made for this run: another local user cannot
        // plant settings or a CLAUDE.md in it.
        let cwd = PrivateDir::create()?;
        let prompt_file = cwd.0.join("system-prompt.txt");
        std::fs::write(&prompt_file, pair.kind.prompt())
            .map_err(|e| format!("cannot write the reader's prompt: {}", e))?;
        let mut cmd = Command::new(&self.exe);
        cmd.args([
            "-p",
            "--model",
            MODEL,
            "--output-format",
            "json",
            "--no-session-persistence",
            // No MCP server of the user's config loads into a reader call.
            "--strict-mcp-config",
            // Not the user's global settings or CLAUDE.md (cwd is a private temp dir).
            "--setting-sources",
            "project",
            "--tools",
            "",
            // A file, not an argument: a Windows .cmd shim refuses a multi-line one.
            "--system-prompt-file",
        ])
        .arg(&prompt_file);
        let stdout = run(cmd, "claude", user, &cwd.0)?;
        let v: serde_json::Value =
            serde_json::from_str(&stdout).map_err(|e| format!("reply is not JSON: {}", e))?;
        v.get("result")
            .and_then(|r| r.as_str())
            .map(String::from)
            .ok_or_else(|| "reply has no result field".to_string())
    }
}

impl Reader for Codex {
    fn model(&self) -> &'static str {
        CODEX_MODEL
    }

    fn ask(&self, pair: &Pair) -> Result<String, String> {
        // `codex exec` has no system prompt flag: the prompt comes first on
        // standard input, set apart from the message. The user's config
        // (effort, MCP servers) and the rules are not loaded, and the shell
        // tool is off, so text inside a pair cannot make it run a command.
        let input = format!("{}\n\n=====\n\n{}", pair.kind.prompt(), user_message(pair));
        let cwd = PrivateDir::create()?;
        let reply = cwd.0.join("reply.txt");
        let mut cmd = Command::new(&self.exe);
        cmd.args([
            "exec",
            "--skip-git-repo-check",
            "--ephemeral",
            "--ignore-user-config",
            "--ignore-rules",
            "--disable",
            "shell_tool",
            "--sandbox",
            "read-only",
            "--color",
            "never",
            "-m",
            CODEX_MODEL,
            "-C",
        ])
        .arg(&cwd.0)
        .arg("-o")
        .arg(&reply)
        .arg("-");
        run(cmd, "codex", input, &cwd.0)?;
        std::fs::read_to_string(&reply)
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("codex wrote no reply: {}", e))
    }
}

#[cfg(feature = "fake-model")]
pub struct Fake;

/// The word a marker in `text` picks for `neighbour`, if any.
#[cfg(feature = "fake-model")]
fn marked_word(text: &str, neighbour: &str) -> Option<String> {
    let mut general: Option<String> = None;
    let mut rest = text;
    while let Some(i) = rest.find("[word:") {
        rest = &rest[i + "[word:".len()..];
        let Some(end) = rest.find(']') else { break };
        let body = &rest[..end];
        rest = &rest[end..];
        match body.split_once('@') {
            Some((w, key)) if neighbour.contains(key) => return Some(w.to_string()),
            Some(_) => {}
            None if general.is_none() => general = Some(body.to_string()),
            None => {}
        }
    }
    general
}

#[cfg(feature = "fake-model")]
impl Reader for Fake {
    fn ask(&self, pair: &Pair) -> Result<String, String> {
        let word =
            marked_word(&pair.new_text, &pair.neighbour).unwrap_or_else(|| "unrelated".to_string());
        Ok(format!("The fake reader's reason.\n{}", word))
    }

    fn model(&self) -> &'static str {
        MODEL
    }
}

/// A fresh directory under the absolute temp dir, named at random and open
/// to its owner only; removed when dropped. `create_dir` fails when the name
/// exists. The temp dir is made absolute, because the program runs with this
/// directory as its working directory and is given paths into it.
struct PrivateDir(PathBuf);

impl PrivateDir {
    fn create() -> Result<PrivateDir, String> {
        PrivateDir::create_in(&std::env::temp_dir())
    }

    fn create_in(base: &Path) -> Result<PrivateDir, String> {
        let base = std::fs::canonicalize(base).map_err(|e| {
            format!(
                "cannot make a directory for the reader: {}: {}",
                base.display(),
                e
            )
        })?;
        let path = base.join(format!("adr-reader-{}", uuid::Uuid::new_v4()));
        #[allow(unused_mut)]
        let mut dir = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            dir.mode(0o700);
        }
        dir.create(&path)
            .map_err(|e| format!("cannot make a directory for the reader: {}", e))?;
        Ok(PrivateDir(path))
    }
}

impl Drop for PrivateDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: ed372cc6-c161-4b90-85bd-c73a6e788c5e
    #[test]
    fn a_set_pair_is_given_the_lower_identifier_first() {
        let mut p = Pair {
            kind: Kind::Set,
            new_text: "NEW".into(),
            neighbour: "n".into(),
            neighbour_text: "NEIGHBOUR".into(),
            letters: Vec::new(),
        };
        assert_eq!(
            user_message(&p),
            "First decision:\nNEW\n\nSecond decision:\nNEIGHBOUR"
        );
        p.kind = Kind::Body;
        assert!(user_message(&p).starts_with("Existing record:\nNEIGHBOUR"));
    }

    #[test]
    fn a_run_directory_is_private_fresh_and_removed_after() {
        let a = PrivateDir::create().unwrap();
        let b = PrivateDir::create().unwrap();
        assert_ne!(a.0, b.0);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&a.0).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        let path = a.0.clone();
        drop(a);
        assert!(!path.exists());
    }

    #[test]
    fn a_run_directory_is_made_under_the_canonical_base() {
        let base = PrivateDir::create().unwrap();
        std::fs::create_dir(base.0.join("x")).unwrap();
        let d = PrivateDir::create_in(&base.0.join("x").join("..")).unwrap();
        assert!(d.0.is_absolute());
        assert!(
            !d.0.components()
                .any(|c| c == std::path::Component::ParentDir),
            "{:?}",
            d.0
        );
        assert_eq!(d.0.parent().unwrap(), base.0.canonicalize().unwrap());
        let e = PrivateDir::create_in(&base.0.join("missing"))
            .err()
            .expect("refuses");
        assert!(
            e.starts_with("cannot make a directory for the reader"),
            "{}",
            e
        );
    }

    #[cfg(feature = "fake-model")]
    #[test]
    fn the_fake_reader_follows_the_marker_for_the_neighbour() {
        let p = |text: &str, n: &str| Pair {
            kind: Kind::Body,
            new_text: text.to_string(),
            neighbour: n.to_string(),
            neighbour_text: String::new(),
            letters: vec!['A', 'B'],
        };
        let word = |p: &Pair| {
            parse_reply(p.kind, &p.letters, &Fake.ask(p).unwrap())
                .unwrap()
                .1
        };
        assert_eq!(word(&p("Does X? [word:same]", "id-1")), "same");
        assert_eq!(word(&p("Does X?", "id-1")), "unrelated");
        let t = "Does X? [word:same] [word:closes B@id-2]";
        assert_eq!(word(&p(t, "id-1")), "same");
        assert_eq!(word(&p(t, "id-2")), "closes B");
    }
}
