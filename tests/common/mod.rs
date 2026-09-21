// Shared by every test binary in `tests/`; not every binary uses every
// helper here, which is expected.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn adr_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_adr"))
}

/// Creates a fresh temp dir, `git init -q`s it, configures a test user,
/// and makes an initial commit so `git show HEAD:...` has something to
/// compare against later.
pub fn init_repo() -> PathBuf {
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

/// Runs git with the directory holding our freshly built `adr` binary
/// prepended to PATH, so that `.githooks/*` scripts (which invoke a bare
/// `adr`) find this build rather than some other `adr` that might already
/// be on the machine's PATH.
pub fn run_git(dir: &Path, args: &[&str]) -> Output {
    let bin_dir = adr_bin().parent().unwrap().to_path_buf();
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", bin_dir.display(), old_path);
    Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("PATH", new_path)
        .output()
        .expect("git should run")
}

/// Runs `adr <args>` in `dir` with the given extra env vars and stdin.
/// Always runs non-interactively (stdin is piped, never inherited), so
/// commands like `lock`/`unban` are refused unless the caller passes
/// ADR_OWNER_BYPASS=1 (and the binary was built with the owner-bypass
/// feature, which `cargo test --features owner-bypass` provides).
pub fn adr(dir: &Path, args: &[&str], envs: &[(&str, &str)], stdin: &str) -> Output {
    let mut cmd = Command::new(adr_bin());
    cmd.current_dir(dir)
        .args(args)
        .env_remove("CLAUDECODE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("adr should spawn");
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
    }
    child.wait_with_output().expect("adr should finish")
}

pub fn adr_owner(dir: &Path, args: &[&str], stdin: &str) -> Output {
    adr(dir, args, &[("ADR_OWNER_BYPASS", "1")], stdin)
}

pub fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

pub fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

pub const EXAMPLE_RECORD: &str = r#"# Example

## Batch 1

### 1.1 What is the unit of authority called?

Problem: names leaked into decisions.

Standard: plain words.

- A) The authoritative sentence is a decision, and the question that produced it is a question.
  Why: plainest word.
  Costs: two nouns.
- B) The whole unit is a block.
  Why: matches predecessors.
  Costs: names a region.

**P1. A decision is one commitment in one sentence.**

Problem: two commitments could not change separately.

Standard: one commitment per decision.

**Answer:** yes
"#;

pub fn write_record(dir: &Path, rel: &str, content: &str) -> PathBuf {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
    path
}

/// Returns every `id: <value>` found in a record, in file order.
pub fn extract_ids(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|l| l.strip_prefix("id: "))
        .map(|s| s.trim().to_string())
        .collect()
}

/// Records a genuine tie for `id`: appends `adr: <id>` to a tracked,
/// non-`adrdom/` file and `git add`s it, the same shape `check`'s tie scan
/// looks for (a full uuid right after `adr: ` in a tracked file).
pub fn write_tie(dir: &Path, id: &str) {
    let path = dir.join("tie.txt");
    let mut content = std::fs::read_to_string(&path).unwrap_or_default();
    content.push_str(&format!("adr: {}\n", id));
    std::fs::write(&path, content).unwrap();
    run_git(dir, &["add", "tie.txt"]);
}

// --- Plan/context approval, without depending on `adr approve` -----------
//
// The crate has no lib target, so this test binary can't call
// `record::plan_hash`/`context_hash` directly; these mirror that logic
// (see src/record.rs) closely enough to produce the same hashes for
// well-formed fixture content, so tests can write `approved`/`context`
// ledger rows straight into adrdom/state/ledger.tsv without needing the
// `approve` command (being built concurrently in a separate slice).

fn sha256_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

fn is_principle_heading(line: &str) -> bool {
    if !line.starts_with("**P") {
        return false;
    }
    let rest = &line[3..];
    let digits_len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits_len == 0 {
        return false;
    }
    let after_digits = &rest[digits_len..];
    if !after_digits.starts_with(". ") {
        return false;
    }
    let after_dot = &after_digits[2..];
    if after_dot.len() < 2 || !after_dot.ends_with("**") {
        return false;
    }
    !after_dot[..after_dot.len() - 2].is_empty()
}

/// Mirrors `record::plan_hash`: sha256 of the trimmed h2/h3/principle
/// heading lines, in file order, joined with `\n`.
pub fn plan_hash(content: &str) -> String {
    let mut heads: Vec<&str> = Vec::new();
    for raw in content.split('\n') {
        let line = raw.trim_end();
        if line.starts_with("## ") {
            heads.push(line);
        } else if line.starts_with("### ") && line.ends_with('?') {
            heads.push(line);
        } else if is_principle_heading(line) {
            heads.push(line);
        }
    }
    sha256_hex(&heads.join("\n"))
}

/// Mirrors `record::context_hash`: sha256 of the context prose lines
/// (between the h1 and the first h2) with leading/trailing blank lines
/// trimmed, joined with `\n`; the empty string hashes when there is none.
pub fn context_hash(content: &str) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    let h1_idx = lines.iter().position(|l| l.starts_with("# "));
    let h2_idx = lines.iter().position(|l| l.starts_with("## "));
    let start = h1_idx.map(|i| i + 1).unwrap_or(0);
    let end = h2_idx.unwrap_or(lines.len());
    let start = start.min(end);
    let slice = &lines[start..end];

    let mut from = 0;
    while from < slice.len() && slice[from].trim().is_empty() {
        from += 1;
    }
    let mut to = slice.len();
    while to > from && slice[to - 1].trim().is_empty() {
        to -= 1;
    }
    sha256_hex(&slice[from..to].join("\n"))
}

/// Writes `approved`/`context` ledger rows for `content` as if `adr
/// approve` had just run on `rel` (path relative to `dir`, forward
/// slashes). Call this with the record's as-written content before `ask`
/// or `lock` touch it: neither changes headings or context prose, so the
/// hashes stay valid afterward.
pub fn approve_via_ledger(dir: &Path, rel: &str, content: &str) {
    let plan = plan_hash(content);
    let ctx = context_hash(content);
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let mut existing = std::fs::read_to_string(&ledger_path).unwrap_or_default();
    existing.push_str(&format!("2024-01-01T00:00:00Z\tapproved\t-\t{}\t{}\n", plan, rel));
    existing.push_str(&format!("2024-01-01T00:00:00Z\tcontext\t-\t{}\t{}\n", ctx, rel));
    std::fs::write(&ledger_path, existing).unwrap();
}

/// Appends a hand-crafted ledger row (bypassing every command): `event`,
/// `id`, `hash`, `detail`, tab-separated with a fixed timestamp.
pub fn append_ledger_row(dir: &Path, event: &str, id: &str, hash: &str, detail: &str) {
    let ledger_path = dir.join("adrdom/state/ledger.tsv");
    let mut existing = std::fs::read_to_string(&ledger_path).unwrap_or_default();
    existing.push_str(&format!(
        "2024-01-01T00:00:00Z\t{}\t{}\t{}\t{}\n",
        event, id, hash, detail
    ));
    std::fs::write(&ledger_path, existing).unwrap();
}
