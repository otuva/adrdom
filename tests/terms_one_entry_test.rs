// A terms edit that adds, changes or removes more than one entry is refused
// by approve; each approve here is the raw command, never the one-entry-at-a-
// time helper of `adr_owner`.
mod common;

use common::*;

fn raw_approve(dir: &std::path::Path) -> std::process::Output {
    adr(dir, &["approve", "adrdom/terms.md"], &[("ADR_OWNER_BYPASS", "1")], "")
}

fn entry(name: &str, def: &str) -> String {
    format!("\n## {}\ndefinition: {}\naliases: \n", name, def)
}

// adr: d16be877-9422-4f2e-a6a1-e099be9e3b2b
#[test]
fn approve_refuses_a_terms_edit_that_changes_more_than_one_entry() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let path = dir.join("adrdom/terms.md");

    // A first terms file of two entries is two edits.
    std::fs::write(&path, format!("# Terms\n{}{}", entry("alpha", "A first thing."), entry("beta", "A second thing."))).unwrap();
    let out = raw_approve(&dir);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("2 entries differ from the last approval, the most is 1"), "{}", err);
    assert!(err.contains("agent keeps one edit in the file"), "{}", err);
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
    assert!(!raw_approve(&dir).status.success());
    let start = two.find("## beta").unwrap();
    std::fs::write(&path, format!("{}", two[..start].replace("A first thing.", "A first item."))).unwrap();
    let out = raw_approve(&dir);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("2 entries differ"), "{}", stderr(&out));

    // One change alone passes.
    std::fs::write(&path, two.replace("A first thing.", "A first item.")).unwrap();
    assert!(raw_approve(&dir).status.success());
}
