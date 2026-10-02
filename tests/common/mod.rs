// Shared by every test binary in `tests/`; not every binary uses every
// helper here, which is expected.
#![allow(dead_code)]

use adr::{ledger, record, terms};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

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
/// commands like `answer`/`unban` are refused unless the caller passes
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
        // A child that refuses before reading its input closes the pipe
        // first; that broken pipe is the child's outcome, not a harness
        // failure, so it is not unwrapped.
        if let Err(e) = child.stdin.as_mut().unwrap().write_all(stdin.as_bytes()) {
            assert_eq!(e.kind(), std::io::ErrorKind::BrokenPipe, "{}", e);
        }
    }
    let out = child.wait_with_output().expect("adr should finish");
    // A successful approve of the terms file is the baseline a later
    // one-entry-at-a-time approve (see `adr_owner`) starts from.
    if out.status.success()
        && args.len() == 2
        && args[0] == "approve"
        && Path::new(args[1]).ends_with("adrdom/terms.md")
    {
        if let Ok(now) = std::fs::read_to_string(dir.join("adrdom/terms.md")) {
            let _ = std::fs::write(terms_prior_path(dir), now);
        }
    }
    out
}

pub fn adr_owner(dir: &Path, args: &[&str], stdin: &str) -> Output {
    if args.len() == 2 && args[0] == "approve" && Path::new(args[1]).ends_with("adrdom/terms.md") {
        return approve_terms_one_entry_at_a_time(dir, args[1], stdin);
    }
    adr(dir, args, &[("ADR_OWNER_BYPASS", "1")], stdin)
}

// Per "Does approve accept a terms edit that changes more than one entry?":
// approve takes one entry at a time. A fixture that writes a terms file
// holding several new or changed entries is approved the way the owner
// would: one entry per approve, the last approve on the file as written.
// The terms file as last approved is kept in the repository's git
// directory so the next call knows the baseline.
fn terms_prior_path(dir: &Path) -> PathBuf {
    dir.join(".git").join("terms-prior-approved")
}

fn terms_blocks(tf: &terms::TermsFile) -> (Vec<String>, Vec<(terms::Term, String)>) {
    let starts: Vec<usize> = tf.terms.iter().map(|t| t.start_line - 1).collect();
    let first = starts.first().copied().unwrap_or(tf.lines.len());
    let header = tf.lines[..first].to_vec();
    let mut blocks = Vec::new();
    for (i, t) in tf.terms.iter().enumerate() {
        let end = starts.get(i + 1).copied().unwrap_or(tf.lines.len());
        blocks.push((t.clone(), tf.lines[starts[i]..end].join("\n")));
    }
    (header, blocks)
}

fn same_entry(a: &terms::Term, b: &terms::Term) -> bool {
    match (&a.id, &b.id) {
        (Some(x), Some(y)) => x == y,
        _ => a.name == b.name,
    }
}

enum TermsStep {
    Put(terms::Term, String),
    Drop(terms::Term),
}

fn render_terms(header: &[String], blocks: &[(terms::Term, String)]) -> String {
    let mut text = header.join("\n");
    for (_, b) in blocks {
        text.push('\n');
        text.push_str(b);
    }
    text
}

fn approve_terms_one_entry_at_a_time(dir: &Path, arg: &str, stdin: &str) -> Output {
    let path = dir.join("adrdom/terms.md");
    let run = || adr(dir, &["approve", arg], &[("ADR_OWNER_BYPASS", "1")], stdin);
    let target_text = std::fs::read_to_string(&path).unwrap();
    let target = terms::parse(&record::split_lines(&target_text));
    let prior_text = std::fs::read_to_string(terms_prior_path(dir)).unwrap_or_default();
    let prior = terms::parse(&record::split_lines(&prior_text));
    let (header, target_blocks) = terms_blocks(&target);
    let (_, mut cur) = terms_blocks(&prior);

    // The steps: each target entry that is new or differs, then each prior
    // entry the target no longer holds.
    let mut steps: Vec<TermsStep> = Vec::new();
    for (t, text) in &target_blocks {
        let same = cur.iter().find(|(c, _)| same_entry(c, t));
        if same.map_or(true, |(c, _)| terms::term_hash(c) != terms::term_hash(t)) {
            steps.push(TermsStep::Put(t.clone(), text.clone()));
        }
    }
    for (c, _) in &cur {
        if !target_blocks.iter().any(|(t, _)| same_entry(c, t)) {
            steps.push(TermsStep::Drop(c.clone()));
        }
    }
    if steps.len() <= 1 {
        return run();
    }

    // Every step but the last is approved on its own, on the file as the
    // approve before it wrote it.
    steps.pop();
    for step in steps {
        match step {
            TermsStep::Put(t, text) => match cur.iter().position(|(c, _)| same_entry(c, &t)) {
                Some(i) => cur[i] = (t, text),
                None => cur.push((t, text)),
            },
            TermsStep::Drop(t) => cur.retain(|(c, _)| !same_entry(c, &t)),
        }
        std::fs::write(&path, render_terms(&header, &cur)).unwrap();
        let o = run();
        if !o.status.success() {
            return o;
        }
        let now = std::fs::read_to_string(&path).unwrap();
        cur = terms_blocks(&terms::parse(&record::split_lines(&now))).1;
    }

    // The last approve holds the target's order and its last step's text;
    // every other entry keeps what the approve before wrote.
    let fin: Vec<(terms::Term, String)> = target_blocks
        .iter()
        .map(|(t, text)| match cur.iter().find(|(c, _)| same_entry(c, t)) {
            Some((c, b)) if terms::term_hash(c) == terms::term_hash(t) => (c.clone(), b.clone()),
            _ => (t.clone(), text.clone()),
        })
        .collect();
    std::fs::write(&path, render_terms(&header, &fin)).unwrap();
    run()
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
        .last()
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
            rest.starts_with(what) || (rest.starts_with(" (\"") && rest.contains(&format!("\"){}", what)))
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
aliases: \n\
\n\
## gone\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A retired fixture term.\n\
aliases: \n\
retired: yes\n";

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

/// A repo with `pass`/`gone` (retired) terms approved, and `BLAME_TOPIC`
/// approved with both headings minted an id: "Is this pass?" answered A
/// (current), "Is this also pass?" only asked (not current). Returns
/// (dir, id of "Is this pass?", id of "Is this also pass?").
pub fn blame_setup() -> (PathBuf, String, String) {
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
    let (id1, id2) = (ids[0].clone(), ids[1].clone());
    let out = adr_answer(&dir, &id1, "A");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    // The fixture's own current decision needs a tie for `check`'s
    // `tested` category, same as any other answered question in a repo
    // `adr check` judges.
    write_test_comment(&dir, &id1);
    (dir, id1, id2)
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

// --- Plan/context approval, without depending on `adr approve` -----------
//
// `approve_via_ledger` below writes `content` to `rel` and loads it back
// through `record::load` so the `approved`/`context` ledger rows it writes
// carry the real `record::plan_hash`/`record::context_hash` of that file
// (see src/record.rs), rather than a hand-rolled mirror of that logic.

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
    let path = dir.join(rel);
    let out = adr_owner(dir, &["approve", path.to_str().unwrap()], "");
    assert!(
        out.status.success(),
        "approve_record: adr approve {} failed: {}",
        rel,
        stderr(&out)
    );
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
pub fn append_ledger_row_at(dir: &Path, event: &str, id: &str, hash: &str, detail: &str, time: &str) {
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
    let _ = std::fs::write(terms_prior_path(dir), content);
    append_ledger_row(dir, "terms", "-", &hash, "adrdom/terms.md");
    std::fs::create_dir_all(dir.join("adrdom/state")).unwrap();
    std::fs::write(dir.join("adrdom/state/terms-brief.md"), terms::brief_text(&tf)).unwrap();
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
    let sec = sections
        .first()
        .unwrap_or_else(|| panic!("mirror_section_hash: no section parsed from: {}", section_text));
    record::section_hash(&lines, sec)
}
