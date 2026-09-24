use crate::commands::install::{
    self, CLAUDE_MD_IMPORT, CLAUDE_MD_MARKER, COMMIT_MSG_HOOK, PRE_COMMIT_HOOK,
};
use crate::commands::deny_entries;
use crate::msg::{self, Actor, Next};
use crate::repo;
use std::fs;
use std::path::Path;

/// The owner's next move on any I/O failure while uninstalling: fix the
/// underlying problem, then run uninstall again.
fn retry() -> Next {
    Next::owner_runs("adr uninstall again")
}

/// Owner-only: removes everything `adr install` wrote outside `adrdom/`,
/// leaving `adrdom/` itself untouched.
pub fn run() -> i32 {
    if !repo::owner_authorized() {
        eprintln!("{}", msg::owner_refusal("uninstall", ""));
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("uninstall", Actor::Owner));
            return 1;
        }
    };

    let mut messages = Vec::new();

    match remove_agents_md_section(&root) {
        Ok(Some(m)) => messages.push(m),
        Ok(None) => {}
        Err(e) => {
            eprintln!("uninstall: {}", msg::update_failed("AGENTS.md", e, retry()));
            return 1;
        }
    }

    match remove_claude_md_import(&root) {
        Ok(Some(m)) => messages.push(m),
        Ok(None) => {}
        Err(e) => {
            eprintln!("uninstall: {}", msg::update_failed("CLAUDE.md", e, retry()));
            return 1;
        }
    }

    messages.extend(remove_hooks(&root));

    match remove_settings_deny(&root) {
        Ok(mut msgs) => messages.append(&mut msgs),
        Err(e) => {
            eprintln!(
                "uninstall: {}",
                msg::update_failed(".claude/settings.json", e, retry())
            );
            return 1;
        }
    }

    messages.extend(remove_codex_rules(&root));

    println!("adrdom uninstalled at {}", root.display());
    for m in &messages {
        println!("{}", m);
    }
    0
}

/// Removes the marked brief section from AGENTS.md, plus the one blank
/// line right before it if there is one. Deletes the file entirely when
/// nothing but whitespace remains.
fn remove_agents_md_section(root: &Path) -> Result<Option<String>, String> {
    let path = root.join("AGENTS.md");
    let existing = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    let parts: Vec<&str> = existing.split('\n').collect();
    let Some((begin, end)) = install::find_markers(&parts) else {
        return Ok(None);
    };

    let remove_from = if begin > 0 && parts[begin - 1].is_empty() {
        begin - 1
    } else {
        begin
    };

    let mut new_parts: Vec<&str> = Vec::new();
    new_parts.extend_from_slice(&parts[..remove_from]);
    new_parts.extend_from_slice(&parts[end + 1..]);
    let new_content = new_parts.join("\n");

    if new_content.trim().is_empty() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
        return Ok(Some(msg::agents_md_removed_whole()));
    }
    fs::write(&path, &new_content).map_err(|e| e.to_string())?;
    Ok(Some(msg::agents_md_section_removed()))
}

/// Removes the marker comment line and the `@AGENTS.md` line right after
/// it from CLAUDE.md, plus the one blank line install put before it if
/// present, and deletes the file when nothing else is in it (the case
/// where `adr install --claude` created it). This byte-reverts the file when it already ended with a
/// single trailing newline (the common case); a file install found with
/// no trailing newline, or with more than one blank line at the end,
/// canonicalizes to a single trailing newline instead of restoring
/// exactly, because install's blank-line insertion collapses those cases
/// to the same on-disk text. A bare `@AGENTS.md` line with no marker
/// above it is the owner's own and is left alone.
fn remove_claude_md_import(root: &Path) -> Result<Option<String>, String> {
    let path = root.join("CLAUDE.md");
    let existing = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    let parts: Vec<&str> = existing.split('\n').collect();
    let Some(marker_idx) = parts.iter().position(|l| *l == CLAUDE_MD_MARKER) else {
        return Ok(None);
    };
    if parts.get(marker_idx + 1).copied() != Some(CLAUDE_MD_IMPORT) {
        return Ok(None);
    }

    let remove_from = if marker_idx > 0 && parts[marker_idx - 1].is_empty() {
        marker_idx - 1
    } else {
        marker_idx
    };

    let mut new_parts: Vec<&str> = Vec::new();
    new_parts.extend_from_slice(&parts[..remove_from]);
    new_parts.extend_from_slice(&parts[marker_idx + 2..]);
    let new_content = new_parts.join("\n");

    if new_content.trim().is_empty() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
        return Ok(Some(msg::claude_md_deleted()));
    }

    fs::write(&path, &new_content).map_err(|e| e.to_string())?;
    Ok(Some(msg::claude_md_import_removed()))
}

/// Deletes the two hooks only when their content still matches exactly
/// what `adr install` writes, removes `.githooks` once empty, and unsets
/// `core.hooksPath` only when it's still `.githooks`.
fn remove_hooks(root: &Path) -> Vec<String> {
    let mut messages = Vec::new();
    let hooks = root.join(".githooks");
    let pre_commit = hooks.join("pre-commit");
    let commit_msg = hooks.join("commit-msg");

    if let Ok(content) = fs::read_to_string(&pre_commit) {
        if content == PRE_COMMIT_HOOK && fs::remove_file(&pre_commit).is_ok() {
            messages.push(msg::hook_removed("pre-commit"));
        }
    }
    if let Ok(content) = fs::read_to_string(&commit_msg) {
        if content == COMMIT_MSG_HOOK && fs::remove_file(&commit_msg).is_ok() {
            messages.push(msg::hook_removed("commit-msg"));
        }
    }
    if let Ok(mut entries) = fs::read_dir(&hooks) {
        if entries.next().is_none() && fs::remove_dir(&hooks).is_ok() {
            messages.push(msg::githooks_dir_removed());
        }
    }

    if let Some(val) = repo::git(root, &["config", "core.hooksPath"]) {
        if val.trim() == ".githooks" {
            repo::git(root, &["config", "--unset", "core.hooksPath"]);
            messages.push(msg::hooks_path_unset());
        }
    }

    messages
}

/// Removes `.codex/rules/adrdom.rules` only when its content still matches
/// exactly what `adr install --codex` writes, then removes `.codex/rules`
/// once empty, and `.codex` once that is empty too. Leaves an
/// owner-edited or foreign file alone (mirrors `remove_hooks`).
fn remove_codex_rules(root: &Path) -> Vec<String> {
    let mut messages = Vec::new();
    let rules_dir = root.join(".codex/rules");
    let rules_path = rules_dir.join("adrdom.rules");
    let codex_dir = root.join(".codex");

    if let Ok(content) = fs::read_to_string(&rules_path) {
        if content == install::codex_rules_content() && fs::remove_file(&rules_path).is_ok() {
            messages.push(msg::codex_rules_removed());
        }
    }
    if let Ok(mut entries) = fs::read_dir(&rules_dir) {
        if entries.next().is_none() && fs::remove_dir(&rules_dir).is_ok() {
            messages.push(msg::codex_rules_dir_removed());
        }
    }
    if let Ok(mut entries) = fs::read_dir(&codex_dir) {
        if entries.next().is_none() && fs::remove_dir(&codex_dir).is_ok() {
            messages.push(msg::codex_dir_removed());
        }
    }

    messages
}

/// Removes every `deny_entries()` string from `.claude/settings.json`'s
/// `permissions.deny`, leaving every other key and entry untouched. Does
/// nothing when the file doesn't exist.
fn remove_settings_deny(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join(".claude/settings.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| msg::parse_failed(&path.display().to_string(), e, retry()))?;

    let mut messages = Vec::new();
    if let Some(deny) = value
        .get_mut("permissions")
        .and_then(|p| p.get_mut("deny"))
        .and_then(|d| d.as_array_mut())
    {
        for entry in deny_entries() {
            let before = deny.len();
            deny.retain(|v| v.as_str() != Some(entry.as_str()));
            if deny.len() != before {
                messages.push(msg::deny_entry_removed(&entry));
            }
        }
    }

    if messages.is_empty() {
        return Ok(messages);
    }

    let mut pretty = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    pretty.push('\n');
    fs::write(&path, pretty).map_err(|e| e.to_string())?;

    Ok(messages)
}
