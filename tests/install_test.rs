mod common;

use common::*;

const ALL_DENY_ENTRIES: &[&str] = &[
    "Bash(adr answer*)",
    "Bash(* adr answer*)",
    "Bash(adr approve*)",
    "Bash(* adr approve*)",
    "Bash(adr unban*)",
    "Bash(* adr unban*)",
    "Bash(adr serve*)",
    "Bash(* adr serve*)",
    "Bash(adr uninstall*)",
    "Bash(* adr uninstall*)",
    "Edit(adrdom/state/**)",
    "Write(adrdom/state/**)",
];

// The one owner-commands list (`OWNER_COMMANDS` in src/commands/mod.rs)
// that both --claude's deny entries and --codex's rules are derived from.
const OWNER_COMMANDS: &[&str] = &["answer", "approve", "unban", "serve", "uninstall"];

// adr: ae63df95-0643-4108-af65-1e7a0009d631
// With --claude, Claude's own files come from the one owner-commands list:
// CLAUDE.md imports AGENTS.md (created here since it was missing), and
// .claude/settings.json's deny entries forbid every command in the list.
#[test]
fn install_claude_flag_derives_claude_files_from_owner_commands() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());

    let claude_md = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert!(
        claude_md.lines().any(|l| l == "@AGENTS.md"),
        "CLAUDE.md must import AGENTS.md: {}",
        claude_md
    );

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
    for cmd in OWNER_COMMANDS {
        assert!(
            deny.contains(&format!("Bash(adr {}*)", cmd).as_str()),
            "settings.json forbids no adr {}: {:?}",
            cmd,
            deny
        );
        assert!(
            deny.contains(&format!("Bash(* adr {}*)", cmd).as_str()),
            "settings.json forbids no * adr {}: {:?}",
            cmd,
            deny
        );
    }
}

#[test]
fn install_creates_layout_and_is_idempotent() {
    let dir = init_repo();

    let out = adr(&dir, &["install", "--claude"], &[], "");
    assert!(out.status.success(), "install failed: {}", stderr(&out));

    // adr: 16a71892-b348-4b9b-82c5-a9d375187bd3
    // One visible adrdom directory at the root: topic files under
    // adrdom/decisions, ledger and ban list under adrdom/state.
    assert!(dir.join("adrdom/decisions").is_dir());
    assert!(dir.join("adrdom/state/ledger.tsv").is_file());
    assert!(dir.join("adrdom/state/banned.sha256").is_file());
    assert!(
        !dir.join("adrdom/state/unchecked.tsv").exists(),
        "install must not create unchecked.tsv"
    );
    assert!(dir.join(".githooks/pre-commit").is_file());
    assert!(dir.join(".githooks/commit-msg").is_file());
    assert!(dir.join(".claude/settings.json").is_file());
    assert!(dir.join("AGENTS.md").is_file());
    assert_eq!(
        std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap(),
        "<!-- adrdom: the next line imports the agent brief; adr uninstall removes both lines -->\n@AGENTS.md\n",
        "install --claude must create CLAUDE.md holding only the import"
    );

    let hooks_cfg = run_git(&dir, &["config", "core.hooksPath"]);
    assert_eq!(stdout(&hooks_cfg).trim(), ".githooks");

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

    // Mark a sentinel in state so we can prove a second install never
    // overwrites it.
    std::fs::write(dir.join("adrdom/state/ledger.tsv"), "sentinel\n").unwrap();
    let settings_before = std::fs::read(dir.join(".claude/settings.json")).unwrap();
    let agents_before = std::fs::read(dir.join("AGENTS.md")).unwrap();

    let out2 = adr(&dir, &["install", "--claude"], &[], "");
    assert!(
        out2.status.success(),
        "second install failed: {}",
        stderr(&out2)
    );

    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        ledger_after, "sentinel\n",
        "install overwrote existing ledger.tsv"
    );

    let settings_after = std::fs::read(dir.join(".claude/settings.json")).unwrap();
    assert_eq!(
        settings_after, settings_before,
        "second install changed settings.json"
    );

    let agents_after = std::fs::read(dir.join("AGENTS.md")).unwrap();
    assert_eq!(
        agents_after, agents_before,
        "second install changed AGENTS.md"
    );
}

#[test]
fn install_preserves_existing_settings_content() {
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

    let out = adr(&dir, &["install", "--claude"], &[], "");
    assert!(out.status.success(), "install failed: {}", stderr(&out));

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
}

#[test]
fn install_without_claude_flag_leaves_no_claude_directory() {
    let dir = init_repo();

    let out = adr(&dir, &["install"], &[], "");
    assert!(out.status.success(), "install failed: {}", stderr(&out));

    assert!(dir.join("adrdom/decisions").is_dir());
    assert!(dir.join(".githooks/pre-commit").is_file());
    assert!(
        !dir.join(".claude").exists(),
        "plain install must not create .claude"
    );
}

// adr: 4f925b13-ad16-48b8-acd1-3e30d6e308b6
// Plain install (no flags) writes only its git hooks (tracked .githooks,
// hooksPath pointed at it) and the brief as a marked section of AGENTS.md;
// nothing for one agent -- no CLAUDE.md, no .claude, no .codex -- without a
// flag naming that agent.
#[test]
fn install_without_flags_writes_only_githooks_and_agents_md_brief() {
    let dir = init_repo();

    let out = adr(&dir, &["install"], &[], "");
    assert!(out.status.success(), "install failed: {}", stderr(&out));

    assert!(dir.join(".githooks/pre-commit").is_file());
    assert!(dir.join(".githooks/commit-msg").is_file());
    let hooks_cfg = run_git(&dir, &["config", "core.hooksPath"]);
    assert_eq!(stdout(&hooks_cfg).trim(), ".githooks");

    let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(agents.contains("<!-- adrdom brief begin -->"));
    assert!(agents.contains("This repository is governed by `adr`."));

    assert!(
        !dir.join("CLAUDE.md").exists(),
        "plain install must not create CLAUDE.md"
    );
    assert!(
        !dir.join(".claude").exists(),
        "plain install must not create .claude"
    );
    assert!(
        !dir.join(".codex").exists(),
        "plain install must not create .codex"
    );
}

// --- git hooks -----------------------------------------------------------

// adr: 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
// install refreshes an existing hook whose content differs from what it
// writes, and leaves an already-current hook untouched (byte-identical
// second run) -- the same missing/stale/current shape as AGENTS.md.
#[test]
fn install_rewrites_a_stale_hook_and_leaves_a_current_one_untouched() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let commit_msg = std::fs::read_to_string(dir.join(".githooks/commit-msg")).unwrap();
    assert_eq!(commit_msg, "#!/bin/sh\nadr check --message \"$1\"\nexit $?\n");
    let first = std::fs::read(dir.join(".githooks/pre-commit")).unwrap();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let second = std::fs::read(dir.join(".githooks/pre-commit")).unwrap();
    assert_eq!(
        first, second,
        "a second install must leave an already-current hook byte-identical"
    );

    std::fs::write(dir.join(".githooks/pre-commit"), "#!/bin/sh\necho edited\n").unwrap();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert_eq!(
        std::fs::read_to_string(dir.join(".githooks/pre-commit")).unwrap(),
        "#!/bin/sh\nadr check --staged\nexit $?\n",
        "install must rewrite a hook whose content differs from what it writes"
    );
}

// install's "already up to date" and check's "hook differs" both compare a
// hook's on-disk content to what install writes; a hook that differs only
// by trailing whitespace on one line must read the same way on both sides,
// or install would rewrite a hook check calls current (or the reverse).
#[test]
fn install_and_check_agree_a_hook_with_trailing_whitespace_is_current() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    std::fs::write(
        dir.join(".githooks/pre-commit"),
        "#!/bin/sh \nadr check --staged\nexit $?\n",
    )
    .unwrap();

    let out = adr(&dir, &["install"], &[], "");
    assert!(out.status.success());
    assert!(
        stdout(&out).contains("install: .githooks/pre-commit: already up to date"),
        "install must not rewrite a hook that differs only by trailing whitespace: {}",
        stdout(&out)
    );

    assert!(
        adr(&dir, &["check"], &[], "").status.success(),
        "check must agree the same hook is current"
    );
}

// --- AGENTS.md ---------------------------------------------------------

#[test]
fn install_creates_agents_md_with_the_brief_and_is_idempotent() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let content = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(content.starts_with("<!-- adrdom brief begin -->\n"));
    assert!(content.trim_end().ends_with("<!-- adrdom brief end -->"));
    assert!(content.contains("This repository is governed by `adr`."));

    let first = std::fs::read(dir.join("AGENTS.md")).unwrap();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let second = std::fs::read(dir.join("AGENTS.md")).unwrap();
    assert_eq!(
        first, second,
        "second install must leave AGENTS.md byte-identical"
    );
}

#[test]
fn install_appends_to_an_existing_agents_md_without_touching_owner_text() {
    let dir = init_repo();
    std::fs::write(
        dir.join("AGENTS.md"),
        "# My repo\n\nSome owner-written notes.\n",
    )
    .unwrap();

    assert!(adr(&dir, &["install"], &[], "").status.success());

    let content = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(content.starts_with("# My repo\n\nSome owner-written notes.\n\n"));
    assert!(content.contains("<!-- adrdom brief begin -->"));
    assert!(content.contains("<!-- adrdom brief end -->"));
    assert!(content.contains("This repository is governed by `adr`."));
}

#[test]
fn install_appends_a_single_blank_line_when_owner_file_already_ends_with_one() {
    let dir = init_repo();
    // Owner-written AGENTS.md that already ends with a blank line: install
    // must not add a second one on top of it.
    std::fs::write(
        dir.join("AGENTS.md"),
        "# My repo\n\nSome owner-written notes.\n\n",
    )
    .unwrap();

    assert!(adr(&dir, &["install"], &[], "").status.success());

    let content = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(
        content.starts_with("# My repo\n\nSome owner-written notes.\n\n<!-- adrdom brief begin -->"),
        "expected exactly one blank line before the brief, got: {:?}",
        &content[..content.len().min(120)]
    );
}

#[test]
fn install_replaces_a_stale_brief_section_in_place() {
    let dir = init_repo();
    std::fs::write(
        dir.join("AGENTS.md"),
        "# My repo\n\nBefore.\n\n<!-- adrdom brief begin -->\nstale text\n<!-- adrdom brief end -->\n\nAfter.\n",
    )
    .unwrap();

    assert!(adr(&dir, &["install"], &[], "").status.success());

    let content = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(content.starts_with("# My repo\n\nBefore.\n\n<!-- adrdom brief begin -->\n"));
    assert!(content.ends_with("<!-- adrdom brief end -->\n\nAfter.\n"));
    assert!(!content.contains("stale text"));
    assert!(content.contains("This repository is governed by `adr`."));
}

// --- CLAUDE.md -----------------------------------------------------------

// adr: 4f925b13-ad16-48b8-acd1-3e30d6e308b6
// Plain install (no --claude) is a no-op on an existing CLAUDE.md, even one
// that doesn't import AGENTS.md yet: CLAUDE.md is Claude-specific, so it
// only changes behind the --claude flag.
#[test]
fn install_without_claude_flag_leaves_an_existing_claude_md_untouched() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert!(
        !dir.join("CLAUDE.md").exists(),
        "install without --claude must not create CLAUDE.md"
    );

    std::fs::write(dir.join("CLAUDE.md"), "# Notes\n\nSome text.\n").unwrap();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    assert_eq!(
        std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap(),
        "# Notes\n\nSome text.\n",
        "install without --claude must not add the import to an existing CLAUDE.md"
    );
}

#[test]
fn install_claude_appends_import_to_an_existing_claude_md_and_is_idempotent() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert!(
        !dir.join("CLAUDE.md").exists(),
        "install without --claude must not create CLAUDE.md"
    );

    std::fs::write(dir.join("CLAUDE.md"), "# Notes\n\nSome text.\n").unwrap();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());

    let content = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert!(content.starts_with("# Notes\n\nSome text.\n"));
    assert!(content.lines().any(|l| l == "@AGENTS.md"));
    assert!(content.contains("adrdom: the next line imports the agent brief"));

    let first = std::fs::read(dir.join("CLAUDE.md")).unwrap();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());
    let second = std::fs::read(dir.join("CLAUDE.md")).unwrap();
    assert_eq!(
        first, second,
        "second install --claude must leave CLAUDE.md unchanged"
    );
}

#[test]
fn install_does_nothing_when_claude_md_already_imports_agents_md() {
    let dir = init_repo();
    std::fs::write(dir.join("CLAUDE.md"), "# Notes\n\n@AGENTS.md\n").unwrap();

    assert!(adr(&dir, &["install"], &[], "").status.success());

    let content = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert_eq!(content, "# Notes\n\n@AGENTS.md\n");
}

// adr: c304b7da-1843-43ad-930c-b01b3310483d
// A file (CLAUDE.md) that holds nothing but what install wrote is deleted
// on uninstall, not just emptied.
#[test]
fn install_claude_creates_claude_md_and_uninstall_deletes_it() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());
    let created = std::fs::read(dir.join("CLAUDE.md")).unwrap();

    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());
    assert_eq!(
        std::fs::read(dir.join("CLAUDE.md")).unwrap(),
        created,
        "second install --claude must leave CLAUDE.md unchanged"
    );
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert_eq!(
        std::fs::read(dir.join("CLAUDE.md")).unwrap(),
        created,
        "plain install must leave the created CLAUDE.md unchanged"
    );

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "uninstall failed: {}", stderr(&out));
    assert!(
        !dir.join("CLAUDE.md").exists(),
        "CLAUDE.md should be deleted once it holds only the import"
    );
}

// --- --codex -----------------------------------------------------------

#[test]
fn install_without_codex_flag_leaves_no_codex_directory() {
    let dir = init_repo();

    let out = adr(&dir, &["install"], &[], "");
    assert!(out.status.success(), "install failed: {}", stderr(&out));

    assert!(
        !dir.join(".codex").exists(),
        "plain install must not create .codex"
    );
}

// adr: ae63df95-0643-4108-af65-1e7a0009d631
// With --codex, Codex's permission file (.codex/rules/adrdom.rules)
// forbids every command in the same owner-commands list --claude's deny
// entries are derived from.
#[test]
fn install_codex_writes_rules_file_and_is_idempotent() {
    let dir = init_repo();

    let out = adr(&dir, &["install", "--codex"], &[], "");
    assert!(out.status.success(), "install failed: {}", stderr(&out));
    assert!(
        stdout(&out).contains("Codex enforces project rules only for a project marked trusted"),
        "{}",
        stdout(&out)
    );

    let rules = std::fs::read_to_string(dir.join(".codex/rules/adrdom.rules")).unwrap();
    for cmd in OWNER_COMMANDS {
        assert!(
            rules.contains(&format!("pattern = [\"adr\", \"{}\"]", cmd)),
            "missing rule for {}: {}",
            cmd,
            rules
        );
        assert!(rules.contains("decision = \"forbidden\""), "{}", rules);
    }

    let out2 = adr(&dir, &["install", "--codex"], &[], "");
    assert!(out2.status.success(), "second install failed: {}", stderr(&out2));
    let rules_after = std::fs::read_to_string(dir.join(".codex/rules/adrdom.rules")).unwrap();
    assert_eq!(rules, rules_after, "second install --codex must not change the file");
}

#[test]
fn install_claude_and_codex_flags_combine() {
    let dir = init_repo();
    let out = adr(&dir, &["install", "--claude", "--codex"], &[], "");
    assert!(out.status.success(), "install failed: {}", stderr(&out));

    assert!(dir.join("CLAUDE.md").is_file());
    assert!(dir.join(".codex/rules/adrdom.rules").is_file());
}

// adr: c304b7da-1843-43ad-930c-b01b3310483d
// A directory install created (.codex/rules, then .codex) is removed too,
// once uninstall has emptied it.
#[test]
fn uninstall_removes_codex_rules_and_empty_directories() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--codex"], &[], "").status.success());
    assert!(dir.join(".codex/rules/adrdom.rules").is_file());

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "uninstall failed: {}", stderr(&out));

    assert!(!dir.join(".codex/rules/adrdom.rules").exists());
    assert!(
        !dir.join(".codex").exists(),
        "empty .codex should be removed"
    );
}

// adr: 292359a4-0512-465b-b6a7-79838e66e5a5
// uninstall removes a piece only while it still matches what install
// wrote; a hand-edited file stays.
#[test]
fn uninstall_leaves_an_owner_edited_codex_rules_file_alone() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--codex"], &[], "").status.success());
    std::fs::write(dir.join(".codex/rules/adrdom.rules"), "owner edited\n").unwrap();

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "uninstall failed: {}", stderr(&out));

    assert_eq!(
        std::fs::read_to_string(dir.join(".codex/rules/adrdom.rules")).unwrap(),
        "owner edited\n",
        "uninstall must not remove a rules file it didn't write"
    );
}

// --- uninstall -------------------------------------------------------------

// adr: 292359a4-0512-465b-b6a7-79838e66e5a5
// uninstall is an owner command: an agent (no owner authorization) is
// refused and nothing it would have removed is touched.
#[test]
fn uninstall_refuses_without_owner_authorization() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());

    let out = adr(&dir, &["uninstall"], &[], "");
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(
        dir.join("AGENTS.md").exists(),
        "uninstall must not have run"
    );
    assert!(dir.join(".githooks/pre-commit").exists());
}

// adr: 292359a4-0512-465b-b6a7-79838e66e5a5
// uninstall removes everything install wrote outside adrdom (hooks,
// hooksPath, the AGENTS.md/CLAUDE.md pieces, the deny entries) and leaves
// adrdom itself in place.
#[test]
fn uninstall_removes_everything_install_wrote_outside_adrdom() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());

    // Layer install onto owner-written AGENTS.md/CLAUDE.md content, the
    // same way a real repo would look after the owner added their own
    // text and re-ran install.
    std::fs::write(
        dir.join("AGENTS.md"),
        "# My repo\n\nOwner notes.\n\n<!-- adrdom brief begin -->\nstale\n<!-- adrdom brief end -->\n",
    )
    .unwrap();
    std::fs::write(dir.join("CLAUDE.md"), "# Notes\n\nOwner text.\n").unwrap();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    assert!(dir.join(".githooks/pre-commit").is_file());
    let hooks_cfg = run_git(&dir, &["config", "core.hooksPath"]);
    assert_eq!(stdout(&hooks_cfg).trim(), ".githooks");

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "uninstall failed: {}", stderr(&out));

    let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert_eq!(agents, "# My repo\n\nOwner notes.\n");

    // uninstall removes the marker comment line, the `@AGENTS.md` line,
    // and the blank line install put before them, so the file byte-reverts.
    let claude = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert_eq!(claude, "# Notes\n\nOwner text.\n");

    assert!(!dir.join(".githooks/pre-commit").exists());
    assert!(!dir.join(".githooks/commit-msg").exists());
    assert!(
        !dir.join(".githooks").exists(),
        "empty .githooks should be removed"
    );

    let hooks_cfg_after = run_git(&dir, &["config", "core.hooksPath"]);
    assert!(
        !hooks_cfg_after.status.success(),
        "core.hooksPath should be unset"
    );

    // This settings.json held nothing but the deny entries install --claude
    // wrote, so uninstall deletes the file (and the .claude it created)
    // rather than leaving it behind with an empty deny array.
    assert!(
        !dir.join(".claude/settings.json").exists(),
        "settings.json held only install's deny entries and should be deleted"
    );
    assert!(
        !dir.join(".claude").exists(),
        "empty .claude should be removed"
    );

    assert!(dir.join("adrdom/decisions").is_dir());
    assert!(dir.join("adrdom/state/ledger.tsv").is_file());
    assert!(dir.join("adrdom/state/banned.sha256").is_file());
}

// adr: c304b7da-1843-43ad-930c-b01b3310483d
// A file (AGENTS.md) that holds nothing but what install wrote is deleted
// on uninstall, not left behind empty.
#[test]
fn uninstall_deletes_agents_md_that_held_only_the_brief() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert!(dir.join("AGENTS.md").exists());

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "uninstall failed: {}", stderr(&out));

    assert!(
        !dir.join("AGENTS.md").exists(),
        "AGENTS.md should be deleted once it holds only the brief"
    );
}

// adr: c304b7da-1843-43ad-930c-b01b3310483d
// A file with more in it than what install wrote (CLAUDE.md with the
// owner's own text around the marker/import pair) keeps the rest: only
// install's own lines come out, the file itself stays.
#[test]
fn uninstall_removes_claude_md_pair_but_keeps_a_bare_agents_import() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    std::fs::write(dir.join("CLAUDE.md"), "# Notes\n\nSome text.\n").unwrap();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());
    let with_pair = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert!(with_pair.lines().any(|l| l == "@AGENTS.md"));

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "uninstall failed: {}", stderr(&out));
    // uninstall byte-reverts CLAUDE.md: marker line, import line, and the
    // blank line install put before them are all removed.
    let after = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert_eq!(after, "# Notes\n\nSome text.\n");

    // A bare `@AGENTS.md` the owner wrote themselves (no marker comment
    // above it) is theirs and stays, even after another uninstall.
    std::fs::write(dir.join("CLAUDE.md"), "# Notes\n\n@AGENTS.md\n").unwrap();
    let out2 = adr_owner(&dir, &["uninstall"], "");
    assert!(
        out2.status.success(),
        "second uninstall failed: {}",
        stderr(&out2)
    );
    let after2 = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert_eq!(after2, "# Notes\n\n@AGENTS.md\n");
}

// adr: c304b7da-1843-43ad-930c-b01b3310483d
// A file (.claude/settings.json) that holds nothing but what install
// wrote is deleted on uninstall, and the directory install created goes
// too once it's empty.
#[test]
fn uninstall_deletes_settings_json_that_held_only_the_deny_entries() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());
    assert!(dir.join(".claude/settings.json").is_file());

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "uninstall failed: {}", stderr(&out));

    assert!(
        !dir.join(".claude/settings.json").exists(),
        "settings.json should be deleted once it holds only the deny entries"
    );
    assert!(
        !dir.join(".claude").exists(),
        "empty .claude should be removed"
    );
}

// adr: c304b7da-1843-43ad-930c-b01b3310483d
// A settings.json with anything else in it (another top-level key, or an
// extra deny entry) keeps the rest: only install's own deny entries come
// out, the file and directory stay.
#[test]
fn uninstall_keeps_settings_json_with_other_content() {
    let dir = init_repo();
    std::fs::create_dir_all(dir.join(".claude")).unwrap();
    std::fs::write(
        dir.join(".claude/settings.json"),
        r#"{
  "model": "opus",
  "permissions": {
    "deny": [
      "Bash(rm -rf*)"
    ]
  }
}
"#,
    )
    .unwrap();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());

    let out = adr_owner(&dir, &["uninstall"], "");
    assert!(out.status.success(), "uninstall failed: {}", stderr(&out));

    assert!(
        dir.join(".claude/settings.json").is_file(),
        "settings.json with other content must not be deleted"
    );
    let settings: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(settings["model"], "opus");
    let deny: Vec<&str> = settings["permissions"]["deny"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        deny,
        vec!["Bash(rm -rf*)"],
        "the owner's own deny entry must stay, install's must go: {:?}",
        deny
    );
}

#[test]
fn uninstall_is_idempotent() {
    let dir = init_repo();
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());

    let first = adr_owner(&dir, &["uninstall"], "");
    assert!(first.status.success(), "uninstall failed: {}", stderr(&first));

    let second = adr_owner(&dir, &["uninstall"], "");
    assert!(
        second.status.success(),
        "second uninstall failed: {}",
        stderr(&second)
    );

    assert!(dir.join("adrdom/decisions").is_dir());
    assert!(dir.join("adrdom/state/ledger.tsv").is_file());
}
