mod common;

use common::*;

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

    let hooks_cfg = run_git(&dir, &["config", "core.hooksPath"]);
    assert_eq!(stdout(&hooks_cfg).trim(), ".githooks");

    let editorconfig = std::fs::read_to_string(dir.join(".editorconfig")).unwrap();
    assert!(editorconfig.contains("[adrdom/**]"));
    assert!(editorconfig.contains("trim_trailing_whitespace = false"));
    assert!(editorconfig.contains("insert_final_newline = false"));

    let snippet = stdout(&out);
    assert!(snippet.contains("Bash(adr lock*)"));
    assert!(snippet.contains("Bash(* adr lock*)"));
    assert!(snippet.contains("Bash(adr override*)"));
    assert!(snippet.contains("Bash(* adr override*)"));
    assert!(snippet.contains("Edit(adrdom/state/**)"));
    assert!(snippet.contains("Write(adrdom/state/**)"));

    // Mark a sentinel in state so we can prove a second init never overwrites it.
    std::fs::write(dir.join("adrdom/state/ledger.tsv"), "sentinel\n").unwrap();
    let editorconfig_before = std::fs::read_to_string(dir.join(".editorconfig")).unwrap();

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
}
