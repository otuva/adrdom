// Shared by every test binary in `tests/`; not every binary uses every
// helper here, which is expected.
#![allow(dead_code)]

use adr::{ledger, record, terms};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// The hook texts install writes, as the hooks on disk hold them.
pub const PRE_COMMIT_TEXT: &str = "#!/bin/sh\n# The repository's own hook goes in .githooks/pre-commit.own and runs once adr check passes.\nadr check --staged || exit $?\nown=\"$(dirname \"$0\")/pre-commit.own\"\n[ -x \"$own\" ] || exit 0\nexec \"$own\" \"$@\"\n";
pub const COMMIT_MSG_TEXT: &str = "#!/bin/sh\n# The repository's own hook goes in .githooks/commit-msg.own and runs once adr check passes.\nadr check --message \"$1\" || exit $?\nown=\"$(dirname \"$0\")/commit-msg.own\"\n[ -x \"$own\" ] || exit 0\nexec \"$own\" \"$@\"\n";
pub const PRE_PUSH_TEXT: &str = "#!/bin/sh\n# The repository's own hook goes in .githooks/pre-push.own and runs once adr check passes.\ninput=$(cat)\n[ -z \"$input\" ] || input=\"$input\n\"\nprintf '%s' \"$input\" | adr check --push || exit $?\nown=\"$(dirname \"$0\")/pre-push.own\"\n[ -x \"$own\" ] || exit 0\nprintf '%s' \"$input\" | \"$own\" \"$@\"\n";

pub fn adr_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_adr"))
}

/// How many `asked` ledger rows exist so far: a count of `\tasked\t` lines
/// in `adrdom/state/ledger.tsv`, since every heading already has an id the
/// moment it is approved, so `extract_ids(...).len()` no longer
/// tracks "how many questions have been asked".
pub fn asked_row_count(dir: &Path) -> usize {
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default();
    ledger.lines().filter(|l| l.contains("\tasked\t")).count()
}

/// Creates a fresh temp dir, `git init -q`s it, configures a test user,
/// and makes an initial commit so `git show HEAD:...` has something to
/// compare against later.
pub fn init_repo() -> PathBuf {
    SWEEP.call_once(sweep_dead_test_repos);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "adr-test-{}-{}-{}",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();

    run_git(&dir, &["init", "-q"]);
    run_git(&dir, &["config", "user.email", "test@example.com"]);
    run_git(&dir, &["config", "user.name", "Test User"]);
    run_git(&dir, &["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.join("README.md"), "test repo\n").unwrap();
    run_git(&dir, &["add", "README.md"]);
    run_git(&dir, &["commit", "-q", "-m", "initial"]);

    dir
}

/// A directory no git repository holds, removed when dropped: the first of
/// the temp dir, /tmp and /var/tmp with no `.git` above it. A temp dir set
/// inside a working tree (TMPDIR under a repository) would make a command run
/// there act on that repository, so it is passed over.
pub struct OutsideDir(pub PathBuf);

impl OutsideDir {
    pub fn new(tag: &str) -> OutsideDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        for base in [
            std::env::temp_dir(),
            PathBuf::from("/tmp"),
            PathBuf::from("/var/tmp"),
        ] {
            if base.ancestors().any(|a| a.join(".git").exists()) {
                continue;
            }
            let dir = base.join(format!("adr-{}-{}-{}", tag, std::process::id(), n));
            if std::fs::create_dir_all(&dir).is_ok() {
                return OutsideDir(dir);
            }
        }
        panic!("no directory outside a git repository to run in");
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for OutsideDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

static SWEEP: std::sync::Once = std::sync::Once::new();

/// Removes the `adr-test-<pid>-*` repos `init_repo` made in runs whose
/// process is gone. A test binary has no hook at exit to remove its own
/// repos, so each run clears the ones its predecessors left; a run still
/// going keeps its repos, since its pid is still in /proc.
fn sweep_dead_test_repos() {
    if !Path::new("/proc/self").exists() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(rest) = name.to_str().and_then(|n| n.strip_prefix("adr-test-")) else {
            continue;
        };
        // The bare remotes and shallow clones some tests make carry a word
        // before the pid.
        let rest = rest
            .strip_prefix("bare-")
            .or_else(|| rest.strip_prefix("shallow-"))
            .unwrap_or(rest);
        let Some(pid) = rest.split('-').next().filter(|p| p.parse::<u32>().is_ok()) else {
            continue;
        };
        // A run in another pid namespace that shares the temp dir has no
        // entry in this /proc: a repo is taken for dead only when its pid is
        // gone here and nothing in it changed for an hour.
        let idle = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > std::time::Duration::from_secs(3600));
        if idle && !Path::new("/proc").join(pid).exists() {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Runs git with the directory holding our freshly built `adr` binary
/// prepended to PATH, so that `.githooks/*` scripts (which invoke a bare
/// `adr`) find this build rather than some other `adr` that might already
/// be on the machine's PATH.
pub fn run_git(dir: &Path, args: &[&str]) -> Output {
    run_git_as(dir, args, None)
}

/// `run_git` in the session `session` of an agent program: the session
/// variable is set when `Some`, and never inherited from the shell that runs
/// the tests.
pub fn run_git_as(dir: &Path, args: &[&str], session: Option<&str>) -> Output {
    let bin_dir = adr_bin().parent().unwrap().to_path_buf();
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", bin_dir.display(), old_path);
    let mut cmd = scrubbed(Command::new("git"));
    cmd.current_dir(dir).args(args).env("PATH", new_path);
    if let Some(id) = session {
        cmd.env(adr::session::SESSION_VARS[0], id);
    }
    cmd.output().expect("git should run")
}

/// `cmd` without the variables an agent program sets for its session: a
/// test run inside such a session must not give the child its identifier.
#[allow(dead_code)]
pub fn scrubbed(mut cmd: Command) -> Command {
    for name in adr::session::SESSION_VARS {
        cmd.env_remove(name);
    }
    cmd
}

/// Runs `adr <args>` in `dir` with the given extra env vars and stdin.
/// Always runs non-interactively (stdin is piped, never inherited), so
/// commands like `answer`/`unban` are refused unless the caller passes
/// ADR_OWNER_BYPASS=1 (and the binary was built with the owner-bypass
/// feature, which `cargo test --features owner-bypass` provides).
pub fn adr(dir: &Path, args: &[&str], envs: &[(&str, &str)], stdin: &str) -> Output {
    adr_as(dir, args, envs, None, stdin)
}

/// `adr` in the session `session` of an agent program: the session variable
/// is set when `Some`, and never inherited from the shell that runs the tests.
pub fn adr_as(
    dir: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
    session: Option<&str>,
    stdin: &str,
) -> Output {
    let mut cmd = scrubbed(Command::new(adr_bin()));
    cmd.current_dir(dir)
        .args(args)
        .env_remove("CLAUDECODE")
        // An owner bypass exported in the shell that runs the tests must not
        // turn a test of an agent's run into the owner's: only `adr_owner`
        // (through `envs`) sets it.
        .env_remove("ADR_OWNER_BYPASS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Tests never download the embedding model: the fake one answers.
        // It has an effect only in a build with the `fake-model` feature.
        .env("ADR_FAKE_MODEL", "1");
    if let Some(id) = session {
        cmd.env(adr::session::SESSION_VARS[0], id);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("adr should spawn");
    {
        use std::io::Write;
        // A child that refuses before reading its input closes the pipe
        // first; that broken pipe is the child's outcome, not a harness
        // failure, so it is not unwrapped.
        if let Err(e) = child.stdin.as_mut().unwrap().write_all(stdin.as_bytes()) {
            assert_eq!(e.kind(), std::io::ErrorKind::BrokenPipe, "{}", e);
        }
    }
    child.wait_with_output().expect("adr should finish")
}

pub fn adr_owner(dir: &Path, args: &[&str], stdin: &str) -> Output {
    if args.len() == 2 && args[0] == "approve" && Path::new(args[1]).ends_with("adrdom/terms.md") {
        return approve_terms_by_entry(dir, args[1], stdin);
    }
    // Approve of a topic file needs the planning records first: the agent's
    // moves of `adr plan`, done by `plan_record`.
    if args.len() == 2
        && args[0] == "approve"
        && args[1].ends_with(".md")
        && !args[1].ends_with("terms.md")
        && !args[1].contains("adrdom/blame/")
    {
        let p = Path::new(args[1]);
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            dir.join(p)
        };
        if let Ok(rel) = abs.strip_prefix(dir) {
            plan_record(dir, &rel.to_string_lossy());
        }
    }
    adr(dir, args, &[("ADR_OWNER_BYPASS", "1")], stdin)
}

// Per "What does approve take for a terms file holding several entries that
// wait?" (da0dc8b1 A): approve takes the name of one entry that waits. A
// fixture that writes a terms file holding several new, changed or removed
// entries is approved the way the owner would: one named approve per entry
// that waits, then, with none waiting, the file itself when it still
// differs. The output is the last approve's.
fn approve_terms_by_entry(dir: &Path, arg: &str, stdin: &str) -> Output {
    let run = |entry: Option<&str>| {
        let mut args = vec!["approve", arg];
        args.extend(entry);
        adr(dir, &args, &[("ADR_OWNER_BYPASS", "1")], stdin)
    };
    let mut last = None;
    for _ in 0..1000 {
        let led = ledger::read(&dir.join("adrdom/state/ledger.tsv"));
        let Ok(tf) = terms::load(&dir.join("adrdom/terms.md")) else {
            return run(None);
        };
        let waiting = adr::state::waiting_terms(&led, &tf);
        let Some((name, _)) = waiting.first() else {
            return last.unwrap_or_else(|| run(None));
        };
        let o = run(Some(name));
        if !o.status.success() {
            return o;
        }
        last = Some(o);
    }
    panic!("approve_terms_by_entry: entries still wait after 1000 approves");
}

/// The version the owner would read off `adr show` for `id` right now (per
/// "Does an answer carry the version of the question the owner read?"):
/// the first 8 hex characters of `id`'s latest `asked` row's hash in the
/// ledger (the same prefix `record::version_of` computes from the topic
/// file directly -- reading it from the ledger instead lets a test ask for
/// the version without re-deriving `record::hash_range`/`section_hash`
/// itself). Panics if `id` was never asked: every caller uses this right
/// after asking (or re-asking) the question it names.
pub fn current_version(dir: &Path, id: &str) -> String {
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default();
    let hash = ledger
        .lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() >= 4 && cols[1] == "asked" && cols[2] == id {
                Some(cols[3].to_string())
            } else {
                None
            }
        })
        .next_back()
        .unwrap_or_else(|| panic!("current_version: no asked row for id {}", id));
    hash[..8].to_string()
}

/// `<id>@<version>`, the argument shape `adr answer` now takes (per "Does
/// an answer carry the version of the question the owner read?").
pub fn id_at_version(dir: &Path, id: &str) -> String {
    format!("{}@{}", id, current_version(dir, id))
}

/// Runs `adr answer <id>@<version> <letter>` as the owner, reading the
/// current version off the ledger first (`current_version`): what every
/// caller that used to run `adr answer <id> <letter>` directly now needs,
/// since a bare id is refused (per "Does an answer carry the version of
/// the question the owner read?").
pub fn adr_answer(dir: &Path, id: &str, letter: &str) -> Output {
    adr_owner(dir, &["answer", &id_at_version(dir, id), letter], "")
}

pub fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

pub fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

/// True when one line of `text` opens its report on `id` with `what`: the
/// identifier, then the name the tool puts beside it when it holds one
/// (`<id> ("<name>")`), then `what` (`": untested"`, `": stale ..."`).
pub fn line_with(text: &str, id: &str, what: &str) -> bool {
    text.lines().any(|l| match l.find(id) {
        Some(at) => {
            let rest = &l[at + id.len()..];
            rest.starts_with(what)
                || (rest.starts_with(" (\"") && rest.contains(&format!("\"){}", what)))
        }
        None => false,
    })
}

pub const EXAMPLE_RECORD: &str = r#"# Example

### 1.1 What is the unit of authority called?
depends: example

Problem: names leaked into decisions.

Standard: plain words.

- A) The authoritative sentence is a decision, and the question that produced it is a question.
  Why: plainest word.
  Costs: two nouns.
- B) The whole unit is a block.
  Why: matches predecessors.
  Costs: names a region.
"#;

pub fn write_record(dir: &Path, rel: &str, content: &str) -> PathBuf {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
    path
}

/// The two options every gate fixture question carries.
pub const GATE_OPTIONS: &str =
    "- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";

/// A question that stands on the term `widget`, with `problem` as its
/// Problem line: what a case puts a loaded word into.
pub fn gate_question(heading: &str, problem: &str) -> String {
    format!(
        "### {}\ndepends: widget\n\nProblem: {}\n\nStandard: plain words.\n\n{}",
        heading, problem, GATE_OPTIONS
    )
}

/// An installed repository with the term `widget` approved.
pub fn widget_repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget"]);
    dir
}

/// The owner's approve with nothing planned on the way: `adr_owner` plans
/// a topic file first, which would write the very lines a case leaves out.
pub fn approve_unplanned(dir: &Path, path: &Path) -> Output {
    adr(
        dir,
        &["approve", path.to_str().unwrap()],
        &[("ADR_OWNER_BYPASS", "1")],
        "",
    )
}

/// The refusal lines a command printed, each with the command's own prefix
/// taken off: the words every other command names the same finding with.
pub fn refusal_lines(out: &Output, prefix: &str) -> Vec<String> {
    stderr(out)
        .lines()
        .map(|l| l.strip_prefix(prefix).unwrap_or(l).to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// The finding and move lines `adr show <rel>` prints for the file, each
/// finding whole with its move: the one computation every command that names
/// a move for the file reads (per "Does a command compute a move of its
/// own?", aac5c313 A). Every line at the left edge that is no status line,
/// marked or not.
pub fn file_moves(dir: &Path, rel: &str) -> Vec<String> {
    let out = adr(dir, &["show", rel], &[], "");
    stdout(&out)
        .lines()
        .filter(|l| !l.starts_with([' ', '[', '#', '+', '-']))
        .map(str::to_string)
        .collect()
}

/// Returns every `id: <value>` found in a record, in file order.
pub fn extract_ids(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|l| l.strip_prefix("id: "))
        .map(|s| s.trim().to_string())
        .collect()
}

/// Records a genuine test reference for `id`: appends `adr: <id>` to a
/// tracked, non-`adrdom/` file and `git add`s it, the same shape `check`'s
/// tested scan looks for (a full uuid right after `adr: ` in a tracked
/// file).
pub fn write_test_comment(dir: &Path, id: &str) {
    let path = dir.join("tested.txt");
    let mut content = std::fs::read_to_string(&path).unwrap_or_default();
    content.push_str(&format!("adr: {}\n", id));
    std::fs::write(&path, content).unwrap();
    run_git(dir, &["add", "tested.txt"]);
}

// --- blame fixtures, shared by every blame_*_test.rs ----------------------

pub const BLAME_TERMS: &str = "# Terms\n\
\n\
## pass\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: Whether something is accepted.\n\
aliases: \n";

pub const BLAME_TOPIC: &str = r#"# Example

### Is this pass?
depends: pass

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.

### Is this also pass?
depends: pass

Problem: whether it also passes.

Standard: clear yes or no.

- A) Yes it passes too.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.
"#;

/// A repo with the term `pass` approved, and `BLAME_TOPIC` approved with both
/// headings minted an id: "Is this pass?" answered A (current), "Is this also
/// pass?" only asked (not current). Returns the repository.
pub fn blame_setup() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_via_ledger(&dir, BLAME_TERMS);
    let record = write_record(&dir, "adrdom/decisions/example.md", BLAME_TOPIC);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    assert_eq!(ids.len(), 2, "{}", content);
    let id1 = ids[0].clone();
    let out = adr_answer(&dir, &id1, "A");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    // The fixture's own current decision needs a tie for `check`'s
    // `tested` category, same as any other answered question in a repo
    // `adr check` judges.
    write_test_comment(&dir, &id1);
    dir
}

/// Writes `content` to `rel` under `dir` and returns the git blob sha
/// `git hash-object -w` gives it, without staging or committing it.
pub fn write_origin(dir: &Path, rel: &str, content: &str) -> String {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
    let out = run_git(dir, &["hash-object", "-w", rel]);
    assert!(out.status.success(), "hash-object failed");
    stdout(&out).trim().to_string()
}

pub fn blame_row_count(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap_or_default()
        .lines()
        .filter(|l| l.contains("\tblame\t"))
        .count()
}

pub fn write_blame_file(dir: &Path, rel: &str, origin: &str, blob: &str, rows: &str) -> PathBuf {
    let content = format!("origin: {}\nblob: {}\n\n{}", origin, blob, rows);
    write_record(dir, rel, &content)
}

/// Writes the origin `origin_rel` and a blame file `adrdom/blame/<name>.md`
/// over it with `rows`; the blame file is not approved.
pub fn blame_file(
    dir: &Path,
    name: &str,
    origin_rel: &str,
    origin_text: &str,
    rows: &str,
) -> PathBuf {
    let blob = write_origin(dir, origin_rel, origin_text);
    write_blame_file(
        dir,
        &format!("adrdom/blame/{}.md", name),
        origin_rel,
        &blob,
        rows,
    )
}

/// `blame_file`, approved by the owner.
pub fn approved_blame_file(
    dir: &Path,
    name: &str,
    origin_rel: &str,
    origin_text: &str,
    rows: &str,
) -> PathBuf {
    let path = blame_file(dir, name, origin_rel, origin_text, rows);
    let out = adr_owner(dir, &["approve", path.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve of the blame file {} failed: {}",
        name,
        stderr(&out)
    );
    path
}

/// The identifier approve wrote on line 3 of a blame file.
pub fn id_line_of(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    let id = text
        .lines()
        .nth(2)
        .and_then(|l| l.strip_prefix("id: "))
        .unwrap_or_else(|| panic!("line 3 is no id line:\n{}", text));
    id.trim().to_string()
}

/// The topic file of one question that stands on the term `widget`, with
/// `lines` (each ending in a newline) directly under its depends line: where
/// a case puts a `blame:` line.
pub fn gate_topic(heading: &str, lines: &str) -> String {
    let question = gate_question(heading, "the widget breaks.").replacen(
        "depends: widget\n",
        &format!("depends: widget\n{}", lines),
        1,
    );
    format!("# Gate\n\nThe gate.\n\n{}", question)
}

// --- Plan/context approval, without depending on `adr approve` -----------
//
// `approve_via_ledger` below writes `content` to `rel` and loads it back
// through `record::load` so the `approved`/`context` ledger rows it writes
// carry the real `record::plan_hash`/`record::context_hash` of that file
// (see src/record), rather than a hand-rolled mirror of that logic.

/// Writes `approved`/`context` ledger rows for `content` as if `adr
/// approve` had just run on `rel` (path relative to `dir`, forward
/// slashes) -- without minting a heading id or rewriting a single depends
/// line to one. Call this only for a
/// file that a test wants to look "plan/context approved" to `check`
/// without ever really being approvable (e.g. deliberately missing a
/// depends line, or naming a term that doesn't exist) -- `check`'s own
/// independent validation is what such a test means to exercise. Anything
/// that goes on to run `adr ask`/`adr answer` for real needs `approve_record`
/// instead: `ask` now refuses outright on a section with no id, and only
/// real `approve` gives one.
pub fn approve_via_ledger(dir: &Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    // Guarantee the file on disk is exactly `content`, regardless of what
    // the caller already wrote, so the real loader below parses the same
    // text the caller means to have "approved".
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
    let rf = record::load(&path)
        .unwrap_or_else(|e| panic!("approve_via_ledger: failed to load {}: {}", rel, e));
    let plan = record::plan_hash(&rf);
    let ctx = record::context_hash(&rf);
    append_ledger_row(dir, "approved", "-", &plan, rel);
    append_ledger_row(dir, "context", "-", &ctx, rel);
}

/// Runs the real, owner-gated `adr approve <rel>` (`approve` mints
/// every heading's id and rewrites its depends entries to ids). Use this,
/// not `approve_via_ledger`, for any fixture that a test then hands to a
/// real `adr ask`/`adr answer` call, since those now require a section to
/// already have an id -- which only a real approve gives it. Panics (with
/// the command's stderr) if approve refuses, since a fixture meant to reach
/// `ask` must be one real `approve` actually accepts.
pub fn approve_record(dir: &Path, rel: &str) {
    plan_record(dir, rel);
    let path = dir.join(rel);
    let out = adr_owner(dir, &["approve", path.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve_record: adr approve {} failed: {}",
        rel,
        stderr(&out)
    );
}

/// Does the agent's moves of `adr plan` on `rel` with the fake model and the
/// fake reader, until the tool says the file is ready: every neighbour the
/// tool reports on no line gets an entry on the `unrelated:` line under its
/// heading. It switches no rule off; it is the plumbing a fixture needs
/// before the owner's approve. A plan that stops for another reason (a body
/// missing, a shape error) is left as it is, so approve reports that.
pub fn plan_record(dir: &Path, rel: &str) {
    let path = dir.join(rel);
    for _ in 0..6 {
        let out = adr(dir, &["plan", path.to_str().unwrap()], &[], "");
        if !out.status.success() {
            return;
        }
        let wanted = lines_wanted(&stdout(&out));
        if wanted.is_empty() {
            return;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, add_unrelated(&text, &wanted)).unwrap();
    }
}

/// The (heading, name) of each finding `adr plan` printed of a neighbour on
/// no line: `<rel>: the <kind> "<name>" is a neighbour of
/// "<heading>" and is on no line; ...`.
pub fn lines_wanted(plan_stdout: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for l in plan_stdout.lines() {
        let Some((left, right)) = l.split_once("\" is a neighbour of \"") else {
            continue;
        };
        let Some((_, name)) = left.rsplit_once(" \"") else {
            continue;
        };
        let Some((heading, _)) = right.split_once("\" and is on no line") else {
            continue;
        };
        let pair = (heading.to_string(), name.to_string());
        if !out.contains(&pair) {
            out.push(pair);
        }
    }
    out
}

/// `text` with each name added to the `unrelated:` line under its heading,
/// a new line directly under the `depends:` line (and under a `supersedes:`
/// line that follows it) when there is none.
pub fn add_unrelated(text: &str, wanted: &[(String, String)]) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    for (heading, name) in wanted {
        let Some(h) = lines
            .iter()
            .position(|l| l.strip_prefix("### ").map(str::trim) == Some(heading.as_str()))
        else {
            continue;
        };
        let end = (h + 1..lines.len())
            .find(|&j| lines[j].starts_with("### "))
            .unwrap_or(lines.len());
        if let Some(j) = (h + 1..end).find(|&j| lines[j].starts_with("unrelated: ")) {
            lines[j].push_str(&format!(", {}", name));
            continue;
        }
        let mut at = (h + 1..end)
            .find(|&j| lines[j].starts_with("depends: "))
            .unwrap_or(h);
        while at + 1 < end && lines[at + 1].starts_with("supersedes: ") {
            at += 1;
        }
        lines.insert(at + 1, format!("unrelated: {}", name));
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Appends a hand-crafted ledger row (bypassing every command): `event`,
/// `id`, `hash`, `detail`, with a fixed timestamp and the real binary's
/// version, formatted through `ledger::row_line` so it has the same six
/// tab-separated fields a real command would write.
pub fn append_ledger_row(dir: &Path, event: &str, id: &str, hash: &str, detail: &str) {
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let mut existing = std::fs::read_to_string(&ledger_path).unwrap_or_default();
    let row = ledger::Row {
        time: "2024-01-01T00:00:00Z".to_string(),
        event: event.to_string(),
        id: id.to_string(),
        hash: hash.to_string(),
        detail: detail.to_string(),
        version: adr::version::VERSION.to_string(),
    };
    existing.push_str(&ledger::row_line(&row));
    existing.push('\n');
    std::fs::write(&ledger_path, existing).unwrap();
}

/// Same as `append_ledger_row`, but with an explicit `time` instead of the
/// fixed one every call to that helper shares. Per "Does the tool take the
/// order of ledger rows from their time?": two rows about the same
/// subject that carry that same fixed time are ordered by row text, not by
/// which one a fixture wrote first, so a fixture whose own point is which
/// row is "the latest" (e.g. an `asked` row and the `answered` row that
/// follows it) needs each row to carry a time later than the one before
/// it, not a shared one.
pub fn append_ledger_row_at(
    dir: &Path,
    event: &str,
    id: &str,
    hash: &str,
    detail: &str,
    time: &str,
) {
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let mut existing = std::fs::read_to_string(&ledger_path).unwrap_or_default();
    let row = ledger::Row {
        time: time.to_string(),
        event: event.to_string(),
        id: id.to_string(),
        hash: hash.to_string(),
        detail: detail.to_string(),
        version: adr::version::VERSION.to_string(),
    };
    existing.push_str(&ledger::row_line(&row));
    existing.push('\n');
    std::fs::write(&ledger_path, existing).unwrap();
}

/// Writes and approves a term-less `adrdom/terms.md`. Since `check` fails
/// once any record exists and `adrdom/terms.md` is missing (an unapproved
/// file alone is only a note now, never a failure), tests that write a
/// record but don't care about terms at all call this to satisfy that gate
/// without adding any actual term.
pub fn approve_empty_terms(dir: &Path) {
    let content = "# Terms\n";
    std::fs::write(dir.join("adrdom/terms.md"), content).unwrap();
    approve_terms_via_ledger(dir, content);
}

/// Writes `adrdom/terms.md` with `content` and hand-writes a `terms`
/// ledger row as if `adr approve adrdom/terms.md` had just run on exactly
/// that content: writes `content` to disk (so it matches whatever the
/// caller already wrote there) and hashes it back with the real
/// `terms::file_hash`, through `terms::load`. Writes the terms brief too,
/// as `approve` does.
pub fn approve_terms_via_ledger(dir: &Path, content: &str) {
    let path = dir.join("adrdom/terms.md");
    std::fs::write(&path, content).unwrap();
    let tf = terms::load(&path)
        .unwrap_or_else(|e| panic!("approve_terms_via_ledger: failed to load terms.md: {}", e));
    let hash = terms::file_hash(&tf);
    append_ledger_row(dir, "terms", "-", &hash, "adrdom/terms.md");
    std::fs::create_dir_all(dir.join("adrdom/state")).unwrap();
    std::fs::write(
        dir.join("adrdom/state/terms-brief.md"),
        terms::brief_text(&tf),
    )
    .unwrap();
}

// --- Terms approval, without depending on `adr approve` -------------------
//
// `ask` now refuses a candidate section unless adrdom/terms.md exists, is
// approved, and the section's text pairs at least one term. Most tests
// that only care about plan/context/answer behavior need a minimal, already
// approved terms.md with a single term whose name appears somewhere in
// the fixture text; `approve_terms_via_ledger` above (real `terms::file_hash`)
// gets there without routing through the `approve` command.
pub fn approve_minimal_terms(dir: &Path, word: &str) {
    let content = format!(
        "# Terms\n\n## {}\nid: 99999999-9999-9999-9999-999999999999\ndefinition: A fixture term for tests.\naliases: \n",
        word
    );
    std::fs::write(dir.join("adrdom/terms.md"), &content).unwrap();
    approve_terms_via_ledger(dir, &content);
}

/// Like `approve_minimal_terms`, for more than one word at once (e.g.
/// `EXAMPLE_RECORD`, which carries a `depends: example` line alongside its
/// `decision` mention): one fixture term per word in `words`, each with its
/// own uuid.
pub fn approve_terms_with(dir: &Path, words: &[&str]) {
    let mut content = String::from("# Terms\n");
    for (i, word) in words.iter().enumerate() {
        content.push_str(&format!(
            "\n## {}\nid: 9999999{}-9999-9999-9999-999999999999\ndefinition: A fixture term for tests.\naliases: \n",
            word, i
        ));
    }
    std::fs::write(dir.join("adrdom/terms.md"), &content).unwrap();
    approve_terms_via_ledger(dir, &content);
}

/// An `assumes` row's hash (see src/commands/check/answers.rs):
/// `record::assumes_hash` of `dependent_key` against the answer set `set`
/// spells out in the shape `check` renders it and this helper's callers
/// already pass: a single letter, letters joined by `|`, or `*` for "any".
/// Lets a test hand-write an `assumes` row that either matches or
/// deliberately mismatches a depends line's current set, without depending
/// on `adr ask`.
pub fn assumes_hash(dependent_key: &str, set: &str) -> String {
    let parsed = if set == "*" {
        record::AnswerSet::Any
    } else {
        record::AnswerSet::Letters(set.chars().filter(|c| *c != '|').collect())
    };
    record::assumes_hash(dependent_key, &parsed)
}

/// `record::section_hash` for a caller that already has the exact block of
/// lines making up one section (from its heading through its last content
/// line, no trailing blank, no `**Answer:**`): wraps `section_text` as the
/// sole question of a throwaway topic file, loads it back through
/// `record::load`, and hashes its one section for real. Lets a test
/// hand-craft an `asked`/`refused`/`answered` ledger row matching a record
/// it edited directly (e.g. after stripping a `terms:` line `ask` wrote).
pub fn mirror_section_hash(section_text: &str) -> String {
    let content = format!("# Topic\n\n{}\n", section_text.trim_end_matches('\n'));
    let lines = record::split_lines(&content);
    let sections = record::parse_sections(&lines);
    let sec = sections.first().unwrap_or_else(|| {
        panic!(
            "mirror_section_hash: no section parsed from: {}",
            section_text
        )
    });
    record::section_hash(&lines, sec)
}

/// The lines `adr check` prints that fail it, as check words them: the
/// report of the working tree read through the library, its failures alone.
/// The printed report mixes failures and notes, each a finding with its
/// move, and only the exit code tells them apart.
#[allow(dead_code)]
pub fn check_failures(dir: &Path) -> String {
    use adr::commands::check;
    let tests = check::test_index(dir, &adr::repo::Source::WorkingTree);
    check::collect_with_tests(dir, &tests)
        .failures
        .into_iter()
        .map(|f| adr::out::text(&adr::out::Line::Finding(f)))
        .collect::<Vec<_>>()
        .join("\n")
}

// --- the source of the tool, for the tests that guard its shape ----------

/// Every `.rs` file under `src/` of this repository, with its path from the repository root.
pub fn source_files() -> Vec<(String, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    let mut dirs = vec![root.join("src")];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

/// The lines of `text` that are code: comment lines left out, and the unit
/// tests at the end of a file (`#[cfg(test)]` onward) left out.
pub fn code_lines(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    // Inside a `#[cfg(test)]` item: the depth of its braces and whether its
    // opening brace has come.
    let mut skipping: Option<(i64, bool)> = None;
    for (i, l) in text.lines().enumerate() {
        if let Some((depth, opened)) = skipping {
            let opens = l.matches('{').count() as i64;
            let depth = depth + opens - l.matches('}').count() as i64;
            let opened = opened || opens > 0;
            // An item with no braces (a `use` or a `fn` on one line) ends at
            // its `;`.
            let ended = (opened && depth <= 0) || (!opened && l.trim_end().ends_with(';'));
            skipping = if ended { None } else { Some((depth, opened)) };
            continue;
        }
        if l.trim() == "#[cfg(test)]" {
            skipping = Some((0, false));
            continue;
        }
        if !l.trim_start().starts_with("//") {
            out.push((i + 1, l));
        }
    }
    out
}

/// The text inside each double-quoted string literal on `line`, escapes
/// kept as written.
pub fn string_literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Option<String> = None;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match (&mut cur, c) {
            (None, '"') => cur = Some(String::new()),
            (None, '\'') => {
                // A char literal such as '"': skip it whole.
                let rest: String = chars.clone().take(3).collect();
                if rest.starts_with("\"'") || rest.starts_with("\\\"'") {
                    chars.nth(if rest.starts_with("\"'") { 1 } else { 2 });
                }
            }
            (Some(s), '\\') => {
                s.push('\\');
                if let Some(n) = chars.next() {
                    s.push(n);
                }
            }
            (Some(_), '"') => out.push(cur.take().unwrap()),
            (Some(s), c) => s.push(c),
            (None, _) => {}
        }
    }
    out
}

// --- the lines of a page --------------------------------------------------

/// The lines a page shows as text, each as the words of the line a command
/// prints: a `<p>` of the class record, status, finding, move or event, and a
/// status shown as badges (`<span class="st">`) as `[name] subject`, the
/// version of an asked question left off. Without the indent. The page's own
/// words (headings, counts, links) are not lines.
pub fn page_lines(body: &str) -> Vec<String> {
    fn unhtml(s: &str) -> String {
        let mut text = String::new();
        let mut in_tag = false;
        for c in s.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => text.push(c),
                _ => {}
            }
        }
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&")
    }
    let mut out = Vec::new();
    let mut rest = body;
    loop {
        let p = rest.find("<p class=\"");
        let b = rest.find("<span class=\"st\">");
        match (p, b) {
            (None, None) => break,
            (Some(i), j) if j.is_none_or(|j| i < j) => {
                let after = &rest[i..];
                let class = after["<p class=\"".len()..].split('"').next().unwrap_or("");
                let end = after.find("</p>").unwrap_or(after.len());
                if ["record", "status", "finding", "move", "event"]
                    .contains(&class.split(' ').next().unwrap_or(""))
                {
                    out.push(unhtml(&after[..end]).trim_start().to_string());
                }
                rest = &after[end..];
            }
            (_, Some(j)) => {
                let after = &rest[j..];
                let end = after
                    .find("</span></span>")
                    .map(|e| e + "</span></span>".len())
                    .unwrap_or(after.len());
                let block = &after[..end];
                let name_start = block.find("<span class=\"badge").unwrap();
                let name_html = &block[name_start..];
                let name = unhtml(&name_html[..name_html.find("</span>").unwrap()]);
                let subject_at = block.find("<span class=\"subject\">").unwrap();
                let subject = unhtml(&block[subject_at..]);
                out.push(format!("[{}] {}", name, subject));
                rest = &after[end..];
            }
            _ => unreachable!(),
        }
    }
    out
}

/// A line a command prints as the page words it: the mark of a change and the
/// version of an asked question left off a status line.
pub fn page_words(line: &str) -> String {
    let bare = line
        .strip_prefix("+ ")
        .or_else(|| line.strip_prefix("- "))
        .unwrap_or(line);
    if !bare.starts_with('[') {
        return line.to_string();
    }
    match (bare.find(" @"), bare.find(']')) {
        (Some(a), Some(b)) if a < b => format!("{}{}", &bare[..a], &bare[b..]),
        _ => bare.to_string(),
    }
}

/// Whether a line a command prints is the owner's move, which the page shows
/// as a form or a link.
pub fn is_owner_move(line: &str) -> bool {
    line.starts_with("owner runs: ")
}

/// The owner's move `line` is on the body as what makes it: the link to the
/// answer page of the question (or the answer form carrying its identifier
/// and version), or the approve form naming the file.
pub fn assert_owner_move_shown(line: &str, body: &str) {
    if let Some(rest) = line.strip_prefix("owner runs: adr answer ") {
        let (iv, _) = rest.split_once(' ').unwrap_or((rest, ""));
        let (id, version) = iv.split_once('@').expect("id@version");
        assert!(
            body.contains(&format!("href=\"/next?show={}\"", id))
                || (body.contains(&format!("name=\"id\" value=\"{}\"", id))
                    && body.contains(&format!("name=\"v\" value=\"{}\"", version))),
            "no link or form for {:?}: {}",
            line,
            body
        );
    } else if let Some(path) = line.strip_prefix("owner runs: adr approve ") {
        // `adrdom/terms.md <name>` is one entry's approve: the form carries
        // the file and the entry; the name is shell-quoted when it holds a space.
        let (file, entry) = match path.split_once(' ') {
            Some((f, e)) => (f, Some(e.trim_matches('\''))),
            None => (path, None),
        };
        assert!(
            body.contains(&format!("name=\"file\" value=\"{}\"", file)),
            "no approve form for {:?}: {}",
            line,
            body
        );
        if let Some(e) = entry {
            assert!(
                body.contains(&format!("name=\"entry\" value=\"{}\"", e)),
                "no entry approve form for {:?}: {}",
                line,
                body
            );
        }
    } else {
        panic!("an owner move this test does not know: {:?}", line);
    }
}

const SATELLITE_QUESTION: &str = "# Example\n\n### 1.1 What is the unit of authority called?\ndepends: authority\n\nProblem: names leaked into decisions.\n\nStandard: plain words.\n\n- A) The authoritative sentence is a decision.\n  Why: plainest word.\n  Costs: two nouns.\n- B) The whole unit is a block.\n  Why: matches predecessors.\n  Costs: names a region.\n";

/// The path from `from` to `to` as a satellite's row and `adrdom/primary`
/// hold it, both repositories being temp directories.
pub fn relative_between(from: &Path, to: &Path) -> String {
    adr::satellite::relative_path(
        &std::fs::canonicalize(from).unwrap(),
        &std::fs::canonicalize(to).unwrap(),
    )
}

/// A primary with a term and one answered decision, a second repository
/// made its satellite by `adr install --satellite`, both committed. Returns
/// (primary, satellite, the decision's id).
pub fn satellite_pair() -> (PathBuf, PathBuf, String) {
    let primary = init_repo();
    assert!(adr(&primary, &["install"], &[], "").status.success());
    approve_minimal_terms(&primary, "authority");
    let record = write_record(&primary, "adrdom/decisions/example.md", SATELLITE_QUESTION);
    approve_record(&primary, "adrdom/decisions/example.md");
    assert!(adr(&primary, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&primary, &id, "A").status.success());

    let satellite = init_repo();
    let rel = relative_between(&primary, &satellite);
    let out = adr(&primary, &["install", "--satellite", &rel], &[], "");
    assert!(out.status.success(), "{}", stderr(&out));

    for dir in [&primary, &satellite] {
        assert!(run_git(dir, &["add", "-A"]).status.success());
    }
    // The primary's commit runs its hooks.
    let c = run_git(&primary, &["commit", "-q", "-m", "chore: record"]);
    assert!(c.status.success(), "{}", stdout(&c));
    let c = run_git(&satellite, &["commit", "-q", "-m", "chore: install"]);
    assert!(c.status.success(), "{}", stderr(&c));
    (primary, satellite, id)
}
