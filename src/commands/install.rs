use crate::commands::{deny_entries, OWNER_COMMANDS};
use crate::ledger;
use crate::msg::{self, Actor, Next};
use crate::repo;
use crate::terms;
use std::fs;
use std::path::Path;

/// The agent's next move on any I/O failure while installing: fix the
/// underlying problem, then run install again.
fn retry() -> Next {
    Next::agent("checks the error and retries")
}

/// The agent brief, final at `assets/brief.md`. `adr install` writes it
/// into AGENTS.md between the marker lines below; `adr check`'s `install`
/// check compares AGENTS.md's marked section against this text.
pub const BRIEF: &str = include_str!("../../assets/brief.md");

pub(crate) const BRIEF_BEGIN: &str = "<!-- adrdom brief begin -->";
pub(crate) const BRIEF_END: &str = "<!-- adrdom brief end -->";

pub(crate) const CLAUDE_MD_MARKER: &str =
    "<!-- adrdom: the next line imports the agent brief; adr uninstall removes both lines -->";
pub(crate) const CLAUDE_MD_IMPORT: &str = "@AGENTS.md";

pub(crate) const PRE_COMMIT_HOOK: &str = "#!/bin/sh\nadr check --staged\nexit $?\n";
pub(crate) const COMMIT_MSG_HOOK: &str = "#!/bin/sh\nadr check --message \"$1\"\nexit $?\n";

/// `.codex/rules/adrdom.rules`'s content: one `prefix_rule` block per
/// `OWNER_COMMANDS` entry (`commands::OWNER_COMMANDS`), `decision =
/// "forbidden"`. `check`'s `install`
/// check compares the file on disk against this text (see
/// `commands/check/install.rs`), the same way it compares AGENTS.md against
/// `BRIEF`.
pub(crate) fn codex_rules_content() -> String {
    let mut out = String::new();
    for cmd in OWNER_COMMANDS {
        out.push_str(&format!(
            "prefix_rule(\n    pattern = [\"adr\", \"{}\"],\n    decision = \"forbidden\",\n    justification = \"adrdom: the owner runs this\",\n)\n",
            cmd
        ));
    }
    out
}

pub fn run(claude: bool, codex: bool) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("install", Actor::Agent));
            return 1;
        }
    };

    let decisions = root.join("adrdom/decisions");
    let state = root.join("adrdom/state");
    if let Err(e) = fs::create_dir_all(&decisions) {
        eprintln!(
            "install: {}",
            msg::create_failed(&decisions.display().to_string(), e, retry())
        );
        return 1;
    }
    if let Err(e) = fs::create_dir_all(&state) {
        eprintln!(
            "install: {}",
            msg::create_failed(&state.display().to_string(), e, retry())
        );
        return 1;
    }

    create_if_missing(&state.join("ledger.tsv"), "");
    create_if_missing(&state.join("banned.sha256"), "");

    let mut messages = Vec::new();

    match backfill_terms_snapshot(&root) {
        Ok(Some(m)) => messages.push(m),
        Ok(None) => {}
        Err(e) => {
            eprintln!(
                "install: {}",
                msg::write_failed("adrdom/state/terms.approved.md", e, retry())
            );
            return 1;
        }
    }

    let hooks = root.join(".githooks");
    fs::create_dir_all(&hooks).ok();
    create_hook(&hooks.join("pre-commit"), PRE_COMMIT_HOOK);
    create_hook(&hooks.join("commit-msg"), COMMIT_MSG_HOOK);

    repo::git(&root, &["config", "core.hooksPath", ".githooks"]);

    match install_agents_md(&root) {
        Ok(m) => messages.push(m),
        Err(e) => {
            eprintln!("install: {}", msg::update_failed("AGENTS.md", e, retry()));
            return 1;
        }
    }

    match install_claude_md(&root, claude) {
        Ok(Some(m)) => messages.push(m),
        Ok(None) => {}
        Err(e) => {
            eprintln!("install: {}", msg::update_failed("CLAUDE.md", e, retry()));
            return 1;
        }
    }

    if claude {
        match merge_settings(&root) {
            Ok(mut msgs) => messages.append(&mut msgs),
            Err(e) => {
                eprintln!(
                    "install: {}",
                    msg::update_failed(".claude/settings.json", e, retry())
                );
                return 1;
            }
        }
    }

    if codex {
        match install_codex_rules(&root) {
            Ok(mut msgs) => messages.append(&mut msgs),
            Err(e) => {
                eprintln!(
                    "install: {}",
                    msg::write_failed(".codex/rules/adrdom.rules", e, retry())
                );
                return 1;
            }
        }
    }

    println!("adrdom installed at {}", root.display());
    for m in &messages {
        println!("{}", m);
    }
    0
}

/// `adrdom/state/terms.approved.md` (per items 1+2 of the bootstrap-gaps
/// review, "Does check fail a terms file that differs from its latest
/// approval?"): a repo that approved `adrdom/terms.md` before this snapshot
/// mechanism existed carries a `terms` ledger row but never got the
/// snapshot file itself written. Restoring it here, once, is what lets
/// `check` tell a real edit apart from the mere absence of a file this
/// build introduced -- without it, `check` fails safe (`terms_snapshot_missing`)
/// on the very next `adrdom/terms.md` edit, no matter how small, since it
/// has nothing to diff against.
///
/// Only backfilled when the snapshot is missing AND the file's current
/// parsed hash equals the latest `terms` ledger row's hash for
/// `adrdom/terms.md` -- so the current file IS what was approved. When
/// there is no terms file, no `terms` ledger row, or the file has already
/// moved since that row (an edit sitting on top of the last approval),
/// nothing is written: a snapshot restored from the wrong text would be
/// worse than none, since `check` would then trust it as ground truth.
fn backfill_terms_snapshot(root: &Path) -> Result<Option<String>, String> {
    let snapshot_path = root.join("adrdom/state/terms.approved.md");
    if snapshot_path.exists() {
        return Ok(None);
    }
    let terms_path = root.join("adrdom/terms.md");
    if !terms_path.exists() {
        return Ok(None);
    }
    let tf = match terms::load(&terms_path) {
        Ok(tf) => tf,
        Err(_) => return Ok(None),
    };
    let file_hash = terms::file_hash(&tf);

    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);
    let approved = led
        .latest_for_path("terms", "adrdom/terms.md")
        .map(|r| r.hash == file_hash)
        .unwrap_or(false);
    if !approved {
        return Ok(None);
    }

    fs::write(&snapshot_path, tf.lines.join("\n")).map_err(|e| e.to_string())?;
    Ok(Some(msg::terms_snapshot_backfilled()))
}

fn create_if_missing(path: &Path, content: &str) {
    if !path.exists() {
        if let Err(e) = fs::write(path, content) {
            eprintln!("install: {}", msg::write_failed(&path.display().to_string(), e, retry()));
        }
    }
}

fn create_hook(path: &Path, content: &str) {
    if path.exists() {
        return;
    }
    if let Err(e) = fs::write(path, content) {
        eprintln!("install: {}", msg::write_failed(&path.display().to_string(), e, retry()));
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).ok();
        }
    }
}

/// Locates the marker lines in `lines` (as produced by `content.split('\n')`,
/// so indices line up with a raw reconstruction). Both markers must match
/// exactly (no trimming), and the end marker must come after the begin
/// marker.
pub(crate) fn find_markers(lines: &[&str]) -> Option<(usize, usize)> {
    let begin = lines.iter().position(|l| *l == BRIEF_BEGIN)?;
    let end = lines[begin + 1..]
        .iter()
        .position(|l| *l == BRIEF_END)
        .map(|i| i + begin + 1)?;
    Some((begin, end))
}

/// The marker lines plus the brief's own lines, in file order, as they
/// should appear in AGENTS.md (begin marker, then every brief line, then
/// end marker).
fn section_lines() -> Vec<&'static str> {
    let mut v = vec![BRIEF_BEGIN];
    v.extend(BRIEF.lines());
    v.push(BRIEF_END);
    v
}

/// Writes/refreshes the marked brief section in AGENTS.md at the repo
/// root: creates the file holding only the section when it's missing,
/// appends (after a blank line) when the file exists without markers, or
/// replaces the marked span in place when it already has one. Never
/// touches text outside the markers, and a no-op leaves the file
/// byte-identical.
fn install_agents_md(root: &Path) -> Result<String, String> {
    let path = root.join("AGENTS.md");

    if !path.exists() {
        let content = section_lines().join("\n") + "\n";
        fs::write(&path, &content).map_err(|e| e.to_string())?;
        return Ok(msg::agents_md_created());
    }

    let existing = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let parts: Vec<&str> = existing.split('\n').collect();

    match find_markers(&parts) {
        Some((begin, end)) => {
            let mut new_parts: Vec<&str> = Vec::new();
            new_parts.extend_from_slice(&parts[..begin]);
            new_parts.extend(section_lines());
            new_parts.extend_from_slice(&parts[end + 1..]);
            let new_content = new_parts.join("\n");

            if new_content == existing {
                return Ok(msg::agents_md_up_to_date());
            }
            fs::write(&path, &new_content).map_err(|e| e.to_string())?;
            Ok(msg::agents_md_replaced())
        }
        None => {
            let mut new_content = existing;
            if !new_content.is_empty() {
                if !new_content.ends_with('\n') {
                    new_content.push('\n');
                }
                if !new_content.ends_with("\n\n") {
                    new_content.push('\n');
                }
            }
            new_content.push_str(&section_lines().join("\n"));
            new_content.push('\n');
            fs::write(&path, &new_content).map_err(|e| e.to_string())?;
            Ok(msg::agents_md_appended())
        }
    }
}

/// Adds the `@AGENTS.md` import to CLAUDE.md at the repo root, but only
/// with `--claude`: without the flag, CLAUDE.md is untouched whether it's
/// missing or already exists (4f925b13-ad16-48b8-acd1-3e30d6e308b6,
/// answer A -- nothing for one agent without a flag naming it). With
/// `--claude`, a missing CLAUDE.md is created holding only the marker and
/// the import; an existing one gets the import appended unless it already
/// has a line equal to `@AGENTS.md`.
fn install_claude_md(root: &Path, create: bool) -> Result<Option<String>, String> {
    if !create {
        return Ok(None);
    }
    let path = root.join("CLAUDE.md");
    let created = !path.exists();

    let existing = if created {
        String::new()
    } else {
        fs::read_to_string(&path).map_err(|e| e.to_string())?
    };
    if existing.lines().any(|l| l == CLAUDE_MD_IMPORT) {
        return Ok(None);
    }

    let mut new_content = existing;
    if !new_content.is_empty() {
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        if !new_content.ends_with("\n\n") {
            new_content.push('\n');
        }
    }
    new_content.push_str(CLAUDE_MD_MARKER);
    new_content.push('\n');
    new_content.push_str(CLAUDE_MD_IMPORT);
    new_content.push('\n');

    fs::write(&path, &new_content).map_err(|e| e.to_string())?;
    Ok(Some(if created {
        msg::claude_md_created()
    } else {
        msg::claude_md_import_added()
    }))
}

/// Merges `deny_entries()` into `.claude/settings.json`'s
/// `permissions.deny` array, preserving every other key and array entry.
/// Creates the directory/file if missing. Returns one message per entry.
fn merge_settings(root: &Path) -> Result<Vec<String>, String> {
    let dir = root.join(".claude");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("settings.json");

    let mut value: serde_json::Value = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if content.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&content)
                .map_err(|e| msg::parse_failed(&path.display().to_string(), e, retry()))?
        }
    } else {
        serde_json::json!({})
    };

    if !value.is_object() {
        return Err(msg::settings_not_json_object(&path.display().to_string(), retry()));
    }
    let obj = value.as_object_mut().unwrap();

    let permissions = obj
        .entry("permissions")
        .or_insert_with(|| serde_json::json!({}));
    if !permissions.is_object() {
        return Err(msg::settings_permissions_not_object(
            &path.display().to_string(),
            retry(),
        ));
    }
    let permissions_obj = permissions.as_object_mut().unwrap();

    let deny = permissions_obj
        .entry("deny")
        .or_insert_with(|| serde_json::json!([]));
    if !deny.is_array() {
        return Err(msg::settings_deny_not_array(&path.display().to_string(), retry()));
    }
    let deny_arr = deny.as_array_mut().unwrap();

    let mut messages = Vec::new();
    for entry in deny_entries() {
        let already = deny_arr.iter().any(|v| v.as_str() == Some(entry.as_str()));
        if already {
            messages.push(msg::deny_entry_present(&entry));
        } else {
            deny_arr.push(serde_json::Value::String(entry.clone()));
            messages.push(msg::deny_entry_added(&entry));
        }
    }

    let mut pretty = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    pretty.push('\n');
    fs::write(&path, pretty).map_err(|e| e.to_string())?;

    Ok(messages)
}

/// Writes `.codex/rules/adrdom.rules` (creating `.codex/rules/`), then adds
/// the note that Codex only enforces project rules for a project the owner
/// has marked trusted in their own Codex config (tested fact; adrdom never
/// touches that config, which lives outside the repo, under the owner's
/// home directory).
fn install_codex_rules(root: &Path) -> Result<Vec<String>, String> {
    let dir = root.join(".codex/rules");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("adrdom.rules");
    let content = codex_rules_content();

    let mut messages = Vec::new();
    let already = fs::read_to_string(&path).map(|c| c == content).unwrap_or(false);
    if already {
        messages.push(msg::codex_rules_up_to_date());
    } else {
        fs::write(&path, &content).map_err(|e| e.to_string())?;
        messages.push(msg::codex_rules_written());
    }
    messages.push(msg::codex_trust_note());
    Ok(messages)
}
