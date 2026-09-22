use crate::commands::install::{
    self, CLAUDE_MD_IMPORT, CLAUDE_MD_MARKER, COMMIT_MSG_HOOK, PRE_COMMIT_HOOK,
};
use crate::commands::DENY_ENTRIES;
use crate::repo;
use std::fs;
use std::path::Path;

/// Owner-only: removes everything `adr install` wrote outside `adrdom/`,
/// leaving `adrdom/` itself untouched.
pub fn run() -> i32 {
    if !repo::owner_authorized() {
        eprintln!(
            "uninstall: refusing: owner authorization required (interactive TTY, no CLAUDECODE)"
        );
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("uninstall: not inside a git repository");
            return 1;
        }
    };

    let mut messages = Vec::new();

    match remove_agents_md_section(&root) {
        Ok(Some(m)) => messages.push(m),
        Ok(None) => {}
        Err(e) => {
            eprintln!("uninstall: failed to update AGENTS.md: {}", e);
            return 1;
        }
    }

    match remove_claude_md_import(&root) {
        Ok(Some(m)) => messages.push(m),
        Ok(None) => {}
        Err(e) => {
            eprintln!("uninstall: failed to update CLAUDE.md: {}", e);
            return 1;
        }
    }

    messages.extend(remove_hooks(&root));

    match remove_settings_deny(&root) {
        Ok(mut msgs) => messages.append(&mut msgs),
        Err(e) => {
            eprintln!("uninstall: failed to update .claude/settings.json: {}", e);
            return 1;
        }
    }

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
        return Ok(Some(
            "uninstall: AGENTS.md: removed (held only the brief)".to_string(),
        ));
    }
    fs::write(&path, &new_content).map_err(|e| e.to_string())?;
    Ok(Some(
        "uninstall: AGENTS.md: removed the brief section".to_string(),
    ))
}

/// Removes the marker comment line and the `@AGENTS.md` line right after
/// it from CLAUDE.md, plus the one blank line install put before it if
/// present. This byte-reverts the file when it already ended with a
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

    fs::write(&path, &new_content).map_err(|e| e.to_string())?;
    Ok(Some(
        "uninstall: CLAUDE.md: removed the AGENTS.md import".to_string(),
    ))
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
            messages.push("uninstall: removed .githooks/pre-commit".to_string());
        }
    }
    if let Ok(content) = fs::read_to_string(&commit_msg) {
        if content == COMMIT_MSG_HOOK && fs::remove_file(&commit_msg).is_ok() {
            messages.push("uninstall: removed .githooks/commit-msg".to_string());
        }
    }
    if let Ok(mut entries) = fs::read_dir(&hooks) {
        if entries.next().is_none() && fs::remove_dir(&hooks).is_ok() {
            messages.push("uninstall: removed empty .githooks".to_string());
        }
    }

    if let Some(val) = repo::git(root, &["config", "core.hooksPath"]) {
        if val.trim() == ".githooks" {
            repo::git(root, &["config", "--unset", "core.hooksPath"]);
            messages.push("uninstall: unset core.hooksPath".to_string());
        }
    }

    messages
}

/// Removes every `DENY_ENTRIES` string from `.claude/settings.json`'s
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
    let mut value: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("{}: {}", path.display(), e))?;

    let mut messages = Vec::new();
    if let Some(deny) = value
        .get_mut("permissions")
        .and_then(|p| p.get_mut("deny"))
        .and_then(|d| d.as_array_mut())
    {
        for entry in DENY_ENTRIES {
            let before = deny.len();
            deny.retain(|v| v.as_str() != Some(*entry));
            if deny.len() != before {
                messages.push(format!(
                    "uninstall: .claude/settings.json: removed deny entry {}",
                    entry
                ));
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
