mod common;

use common::*;

const ALL_DENY_ENTRIES: &[&str] = &[
    "Bash(adr lock*)",
    "Bash(* adr lock*)",
    "Bash(adr approve*)",
    "Bash(* adr approve*)",
    "Bash(adr unban*)",
    "Bash(* adr unban*)",
    "Bash(adr serve*)",
    "Bash(* adr serve*)",
    "Edit(adrdom/state/**)",
    "Write(adrdom/state/**)",
    "Edit(adrdom/rules.md)",
    "Write(adrdom/rules.md)",
];

#[test]
fn init_creates_layout_and_is_idempotent() {
    let dir = init_repo();

    let out = adr(&dir, &["init"], &[], "");
    assert!(out.status.success(), "init failed: {}", stderr(&out));

    assert!(dir.join("adrdom/decisions").is_dir());
    assert!(dir.join("adrdom/state/ledger.tsv").is_file());
    assert!(dir.join("adrdom/state/banned.sha256").is_file());
    assert!(dir.join("adrdom/state/unchecked.tsv").is_file());
    assert!(dir.join(".githooks/pre-commit").is_file());
    assert!(dir.join(".githooks/commit-msg").is_file());
    assert!(dir.join(".editorconfig").is_file());
    assert!(dir.join("adrdom/rules.md").is_file());
    assert!(dir.join("CLAUDE.md").is_file());
    assert!(dir.join(".claude/settings.json").is_file());

    let hooks_cfg = run_git(&dir, &["config", "core.hooksPath"]);
    assert_eq!(stdout(&hooks_cfg).trim(), ".githooks");

    let editorconfig = std::fs::read_to_string(dir.join(".editorconfig")).unwrap();
    assert!(editorconfig.contains("[adrdom/**]"));
    assert!(editorconfig.contains("trim_trailing_whitespace = false"));
    assert!(editorconfig.contains("insert_final_newline = false"));

    let rules = std::fs::read_to_string(dir.join("adrdom/rules.md")).unwrap();
    assert!(rules.contains("# adrdom rules"));

    let claude_md = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert!(claude_md.contains("@adrdom/rules.md"));

    let settings: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap(),
    )
    .unwrap();
    let deny: Vec<&str> = settings["permissions"]["deny"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for entry in ALL_DENY_ENTRIES {
        assert!(
            deny.contains(entry),
            "missing deny entry {}: {:?}",
            entry,
            deny
        );
    }

    // Mark a sentinel in state so we can prove a second init never overwrites it.
    std::fs::write(dir.join("adrdom/state/ledger.tsv"), "sentinel\n").unwrap();
    let editorconfig_before = std::fs::read_to_string(dir.join(".editorconfig")).unwrap();
    let rules_before = std::fs::read(dir.join("adrdom/rules.md")).unwrap();
    let claude_md_before = std::fs::read(dir.join("CLAUDE.md")).unwrap();
    let settings_before = std::fs::read(dir.join(".claude/settings.json")).unwrap();

    let out2 = adr(&dir, &["init"], &[], "");
    assert!(
        out2.status.success(),
        "second init failed: {}",
        stderr(&out2)
    );

    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger_after, "sentinel\n",
        "init overwrote existing ledger.tsv"
    );

    let editorconfig_after = std::fs::read_to_string(dir.join(".editorconfig")).unwrap();
    assert_eq!(
        editorconfig_after, editorconfig_before,
        "second init duplicated the [adrdom/**] section"
    );

    let rules_after = std::fs::read(dir.join("adrdom/rules.md")).unwrap();
    assert_eq!(rules_after, rules_before, "second init changed rules.md");

    let claude_md_after = std::fs::read(dir.join("CLAUDE.md")).unwrap();
    assert_eq!(
        claude_md_after, claude_md_before,
        "second init changed CLAUDE.md"
    );

    let settings_after = std::fs::read(dir.join(".claude/settings.json")).unwrap();
    assert_eq!(
        settings_after, settings_before,
        "second init changed settings.json"
    );
}

#[test]
fn init_preserves_existing_settings_and_claude_md_content() {
    let dir = init_repo();

    std::fs::create_dir_all(dir.join(".claude")).unwrap();
    std::fs::write(
        dir.join(".claude/settings.json"),
        r#"{
  "otherKey": "value",
  "permissions": {
    "deny": [
      "Bash(rm -rf*)"
    ]
  }
}
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("CLAUDE.md"),
        "# Project notes\n\nExisting content.\n",
    )
    .unwrap();

    let out = adr(&dir, &["init"], &[], "");
    assert!(out.status.success(), "init failed: {}", stderr(&out));

    let settings: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(settings["otherKey"], "value");
    let deny: Vec<&str> = settings["permissions"]["deny"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(deny.contains(&"Bash(rm -rf*)"), "{:?}", deny);
    for entry in ALL_DENY_ENTRIES {
        assert!(deny.contains(entry), "missing {}: {:?}", entry, deny);
    }

    let claude_md = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert!(claude_md.contains("Existing content."));
    assert!(claude_md.contains("@adrdom/rules.md"));
    assert_eq!(
        claude_md.matches("@adrdom/rules.md").count(),
        1,
        "line duplicated: {}",
        claude_md
    );
}
