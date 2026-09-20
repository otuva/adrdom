use crate::commands::DENY_ENTRIES;
use crate::repo;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn run() -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("init: not inside a git repository");
            return 1;
        }
    };

    let decisions = root.join("adrdom/decisions");
    let state = root.join("adrdom/state");
    if let Err(e) = fs::create_dir_all(&decisions) {
        eprintln!("init: failed to create {}: {}", decisions.display(), e);
        return 1;
    }
    if let Err(e) = fs::create_dir_all(&state) {
        eprintln!("init: failed to create {}: {}", state.display(), e);
        return 1;
    }

    create_if_missing(&state.join("ledger.tsv"), "");
    create_if_missing(&state.join("banned.sha256"), "");
    create_if_missing(&state.join("unchecked.tsv"), "");

    let hooks = root.join(".githooks");
    fs::create_dir_all(&hooks).ok();
    create_hook(
        &hooks.join("pre-commit"),
        "#!/bin/sh\nadr check --staged\nexit $?\n",
    );
    create_hook(
        &hooks.join("commit-msg"),
        "#!/bin/sh\nadr check --message \"$1\"\nexit $?\n",
    );

    repo::git(&root, &["config", "core.hooksPath", ".githooks"]);

    update_editorconfig(&root.join(".editorconfig"));

    let mut messages = Vec::new();
    match merge_settings(&root) {
        Ok(mut msgs) => messages.append(&mut msgs),
        Err(e) => {
            eprintln!("init: failed to update .claude/settings.json: {}", e);
            return 1;
        }
    }

    println!("adrdom initialized at {}", root.display());
    for m in &messages {
        println!("{}", m);
    }
    0
}

fn create_if_missing(path: &Path, content: &str) {
    if !path.exists() {
        if let Err(e) = fs::write(path, content) {
            eprintln!("init: failed to write {}: {}", path.display(), e);
        }
    }
}

fn create_hook(path: &Path, content: &str) {
    if path.exists() {
        return;
    }
    if let Err(e) = fs::write(path, content) {
        eprintln!("init: failed to write {}: {}", path.display(), e);
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

fn update_editorconfig(path: &Path) {
    let section = "\n[adrdom/**]\ntrim_trailing_whitespace = false\ninsert_final_newline = false\n";
    if path.exists() {
        let content = fs::read_to_string(path).unwrap_or_default();
        if content.contains("[adrdom/**]") {
            return;
        }
        if let Ok(mut f) = fs::OpenOptions::new().append(true).open(path) {
            f.write_all(section.as_bytes()).ok();
        }
    } else {
        fs::write(path, section.trim_start()).ok();
    }
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
                "init: .claude/settings.json: deny entry {} already present",
                entry
            ));
        } else {
            deny_arr.push(serde_json::Value::String(entry.to_string()));
            messages.push(format!(
                "init: .claude/settings.json: added deny entry {}",
                entry
            ));
        }
    }

    let mut pretty = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    pretty.push('\n');
    fs::write(&path, pretty).map_err(|e| e.to_string())?;

    Ok(messages)
}
