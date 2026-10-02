// A terms edit that adds, changes or removes more than one entry is refused
// by approve and reported by check; each approve here is the raw command,
// never the one-entry-at-a-time helper of `adr_owner`.
mod common;

use common::*;

fn raw_approve(dir: &std::path::Path) -> std::process::Output {
    adr(dir, &["approve", "adrdom/terms.md"], &[("ADR_OWNER_BYPASS", "1")], "")
}

fn entry(name: &str, def: &str) -> String {
    format!("\n## {}\ndefinition: {}\naliases: \n", name, def)
}

fn repo() -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let path = dir.join("adrdom/terms.md");
    (dir, path)
}

// adr: d16be877-9422-4f2e-a6a1-e099be9e3b2b
#[test]
fn approve_refuses_a_terms_edit_that_changes_more_than_one_entry() {
    let (dir, path) = repo();

    // A first terms file of two entries is two edits.
    std::fs::write(&path, format!("# Terms\n{}{}", entry("alpha", "A first thing."), entry("beta", "A second thing."))).unwrap();
    let out = raw_approve(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("2 entries are new, and the file has no approval yet: \"alpha\", \"beta\""), "{}", err);
    assert!(err.contains("agent keeps one of these edits in the file"), "{}", err);
    assert_eq!(err.matches("owner runs").count(), 0, "{}", err);
    assert_eq!(std::fs::read_to_string(&path).unwrap().matches("\nid: ").count(), 0, "a refusal writes nothing");

    // One entry passes, then the second as its own approve.
    std::fs::write(&path, format!("# Terms\n{}", entry("alpha", "A first thing."))).unwrap();
    assert!(raw_approve(&dir).status.success());
    let one = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{}{}", one, entry("beta", "A second thing."))).unwrap();
    assert!(raw_approve(&dir).status.success());

    // Changing one definition and adding an entry is two edits; removing
    // one and changing another is two edits.
    let two = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{}{}", two.replace("A first thing.", "A first item."), entry("gamma", "A third thing."))).unwrap();
    let out = raw_approve(&dir);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("2 entries differ from the last approval: \"alpha\", \"gamma\""), "{}", stderr(&out));
    let start = two.find("## beta").unwrap();
    std::fs::write(&path, two[..start].replace("A first thing.", "A first item.")).unwrap();
    let out = raw_approve(&dir);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("2 entries differ"), "{}", stderr(&out));

    // One change alone passes.
    std::fs::write(&path, two.replace("A first thing.", "A first item.")).unwrap();
    assert!(raw_approve(&dir).status.success());
}

// adr: d16be877-9422-4f2e-a6a1-e099be9e3b2b
#[test]
fn an_approved_removal_does_not_count_against_a_later_edit() {
    let (dir, path) = repo();
    std::fs::write(&path, format!("# Terms\n{}", entry("alpha", "A first thing."))).unwrap();
    assert!(raw_approve(&dir).status.success());
    let one = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{}{}", one, entry("beta", "A second thing."))).unwrap();
    assert!(raw_approve(&dir).status.success());
    // Remove beta: one edit, approved.
    std::fs::write(&path, &one).unwrap();
    let out = raw_approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
    // Add gamma alone: one edit, so it passes.
    std::fs::write(&path, format!("{}{}", one, entry("gamma", "A third thing."))).unwrap();
    let out = raw_approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
}

// adr: d16be877-9422-4f2e-a6a1-e099be9e3b2b
#[test]
fn a_ledger_with_terms_rows_and_no_term_rows_approves_the_unchanged_file() {
    let (dir, path) = repo();
    approve_terms_with(&dir, &["alpha", "beta"]);
    let out = raw_approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(dir.join("adrdom/state/terms-brief.md").exists());
    // The catch-up approve is also what takes one edit to either entry.
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("A fixture term for tests.", "A changed term.", 1)).unwrap();
    let out = raw_approve(&dir);
    assert!(out.status.success(), "{}", stderr(&out));
}

// adr: d16be877-9422-4f2e-a6a1-e099be9e3b2b
#[test]
fn check_reports_a_two_entry_edit_with_the_agent_move() {
    let (dir, path) = repo();
    std::fs::write(&path, format!("# Terms\n{}", entry("alpha", "A first thing."))).unwrap();
    assert!(raw_approve(&dir).status.success());
    let one = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{}{}{}", one, entry("beta", "A second thing."), entry("gamma", "A third thing."))).unwrap();
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("2 entries differ from the last approval: \"beta\", \"gamma\""), "{}", text);
    assert!(text.contains("agent keeps one of these edits in the file"), "{}", text);
    assert!(!text.contains("owner runs: adr approve adrdom/terms.md"), "{}", text);
}
