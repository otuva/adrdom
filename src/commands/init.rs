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

    println!("adrdom initialized at {}", root.display());
    println!();
    println!("Add this to your Claude Code settings.json permissions.deny list:");
    println!("{}", permissions_snippet());
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

fn permissions_snippet() -> &'static str {
    r#"{
  "permissions": {
    "deny": [
      "Bash(adr lock*)",
      "Bash(* adr lock*)",
      "Bash(adr override*)",
      "Bash(* adr override*)",
      "Edit(adrdom/state/**)",
      "Write(adrdom/state/**)"
    ]
  }
}"#
}
