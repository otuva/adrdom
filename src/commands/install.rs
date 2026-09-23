use crate::commands::DENY_ENTRIES;
use crate::repo;
use std::fs;
use std::path::Path;

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

pub fn run(claude: bool) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("install: not inside a git repository");
            return 1;
        }
    };

    let decisions = root.join("adrdom/decisions");
    let state = root.join("adrdom/state");
    if let Err(e) = fs::create_dir_all(&decisions) {
        eprintln!("install: failed to create {}: {}", decisions.display(), e);
        return 1;
    }
    if let Err(e) = fs::create_dir_all(&state) {
        eprintln!("install: failed to create {}: {}", state.display(), e);
        return 1;
    }

    create_if_missing(&state.join("ledger.tsv"), "");
    create_if_missing(&state.join("banned.sha256"), "");

    let hooks = root.join(".githooks");
    fs::create_dir_all(&hooks).ok();
    create_hook(&hooks.join("pre-commit"), PRE_COMMIT_HOOK);
    create_hook(&hooks.join("commit-msg"), COMMIT_MSG_HOOK);

    repo::git(&root, &["config", "core.hooksPath", ".githooks"]);

    let mut messages = Vec::new();

    match install_agents_md(&root) {
        Ok(m) => messages.push(m),
        Err(e) => {
            eprintln!("install: failed to update AGENTS.md: {}", e);
            return 1;
        }
    }

    match install_claude_md(&root, claude) {
        Ok(Some(m)) => messages.push(m),
        Ok(None) => {}
        Err(e) => {
            eprintln!("install: failed to update CLAUDE.md: {}", e);
            return 1;
        }
    }

    if claude {
        match merge_settings(&root) {
            Ok(mut msgs) => messages.append(&mut msgs),
            Err(e) => {
                eprintln!("install: failed to update .claude/settings.json: {}", e);
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

fn create_if_missing(path: &Path, content: &str) {
    if !path.exists() {
        if let Err(e) = fs::write(path, content) {
            eprintln!("install: failed to write {}: {}", path.display(), e);
        }
    }
}

fn create_hook(path: &Path, content: &str) {
    if path.exists() {
        return;
    }
    if let Err(e) = fs::write(path, content) {
        eprintln!("install: failed to write {}: {}", path.display(), e);
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
        return Ok("install: AGENTS.md: created with the brief".to_string());
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
                return Ok("install: AGENTS.md: brief already up to date".to_string());
            }
            fs::write(&path, &new_content).map_err(|e| e.to_string())?;
            Ok("install: AGENTS.md: replaced the stale brief section".to_string())
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
            Ok("install: AGENTS.md: appended the brief".to_string())
        }
    }
}

/// Adds the `@AGENTS.md` import to CLAUDE.md at the repo root when that
/// file exists and doesn't already have a line equal to `@AGENTS.md`.
/// With `--claude` a missing CLAUDE.md is created holding only the marker
/// and the import; without the flag a missing file is left missing.
fn install_claude_md(root: &Path, create: bool) -> Result<Option<String>, String> {
    let path = root.join("CLAUDE.md");
    let created = !path.exists();
    if created && !create {
        return Ok(None);
    }

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
        "install: CLAUDE.md: created with the AGENTS.md import".to_string()
    } else {
        "install: CLAUDE.md: added the AGENTS.md import".to_string()
    }))
}

/// Merges `DENY_ENTRIES` into `.claude/settings.json`'s
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
                .map_err(|e| format!("{}: {}", path.display(), e))?
        }
    } else {
        serde_json::json!({})
    };

    if !value.is_object() {
        return Err(format!("{} is not a JSON object", path.display()));
    }
    let obj = value.as_object_mut().unwrap();

    let permissions = obj
        .entry("permissions")
        .or_insert_with(|| serde_json::json!({}));
    if !permissions.is_object() {
        return Err(format!("{}: permissions is not an object", path.display()));
    }
    let permissions_obj = permissions.as_object_mut().unwrap();

    let deny = permissions_obj
        .entry("deny")
        .or_insert_with(|| serde_json::json!([]));
    if !deny.is_array() {
        return Err(format!(
            "{}: permissions.deny is not an array",
            path.display()
        ));
    }
    let deny_arr = deny.as_array_mut().unwrap();

    let mut messages = Vec::new();
    for entry in DENY_ENTRIES {
        let already = deny_arr.iter().any(|v| v.as_str() == Some(*entry));
        if already {
            messages.push(format!(
                "install: .claude/settings.json: deny entry {} already present",
                entry
            ));
        } else {
            deny_arr.push(serde_json::Value::String(entry.to_string()));
            messages.push(format!(
                "install: .claude/settings.json: added deny entry {}",
                entry
            ));
        }
    }

    let mut pretty = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    pretty.push('\n');
    fs::write(&path, pretty).map_err(|e| e.to_string())?;

    Ok(messages)
}
