// Approve of the terms file takes the name of one entry that waits, new,
// changed or removed, in any order; each approve here is the raw command,
// never the entry-by-entry helper of `adr_owner`.
mod common;

use common::*;

fn approve(dir: &std::path::Path, entry: Option<&str>) -> std::process::Output {
    let mut args = vec!["approve", "adrdom/terms.md"];
    args.extend(entry);
    adr(dir, &args, &[("ADR_OWNER_BYPASS", "1")], "")
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

fn three(path: &std::path::Path) {
    std::fs::write(
        path,
        format!("# Terms\n{}{}{}", entry("alpha", "A first thing."), entry("beta item", "A second thing."), entry("gamma", "A third thing.")),
    )
    .unwrap();
}

fn owner_lines(text: &str) -> Vec<String> {
    text.lines().filter(|l| l.starts_with("owner runs: adr approve adrdom/terms.md")).map(String::from).collect()
}

fn write_topic_file(dir: &std::path::Path, name: &str, text: &str) {
    std::fs::create_dir_all(dir.join("adrdom/decisions")).unwrap();
    std::fs::write(dir.join("adrdom/decisions").join(name), text).unwrap();
}

const TOPIC: &str = "# Alpha\n\nWhat alpha does.\n\n### Does alpha name gamma?\n";

// adr: da0dc8b1-58cd-4a16-8e64-234e4bfd5320
#[test]
fn show_prints_one_approve_per_waiting_entry_and_each_named_approve_takes_one() {
    let (dir, path) = repo();
    three(&path);
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert_eq!(
        owner_lines(&shown),
        vec![
            "owner runs: adr approve adrdom/terms.md alpha",
            "owner runs: adr approve adrdom/terms.md 'beta item'",
            "owner runs: adr approve adrdom/terms.md gamma",
        ],
        "{}",
        shown
    );

    // Any order: the last entry first, then the one with a space.
    let out = approve(&dir, Some("gamma"));
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("approved: term \"gamma\""), "{}", stdout(&out));
    assert!(!stdout(&out).contains("approved: adrdom/terms.md"), "the file waits while entries do: {}", stdout(&out));
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert_eq!(owner_lines(&shown).len(), 2, "{}", shown);
    assert!(!shown.contains("adrdom/terms.md gamma"), "{}", shown);

    assert!(approve(&dir, Some("beta item")).status.success());
    let out = approve(&dir, Some("alpha"));
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("approved: adrdom/terms.md"), "the last entry approves the file: {}", stdout(&out));
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert!(owner_lines(&shown).is_empty(), "{}", shown);
    let check = adr(&dir, &["check"], &[], "");
    assert!(check.status.success(), "{}", stdout(&check));
}

// adr: da0dc8b1-58cd-4a16-8e64-234e4bfd5320
#[test]
fn the_gate_names_each_entry_still_waiting_until_the_last_is_approved() {
    let (dir, path) = repo();
    three(&path);
    write_topic_file(&dir, "alpha.md", TOPIC);
    assert!(approve(&dir, Some("alpha")).status.success());
    assert!(approve(&dir, Some("gamma")).status.success());
    let plan = stdout(&adr(&dir, &["plan", "adrdom/decisions/alpha.md"], &[], ""));
    assert!(plan.contains("adrdom/terms.md: \"beta item\" is new"), "{}", plan);
    assert!(plan.contains("owner runs: adr approve adrdom/terms.md 'beta item'"), "{}", plan);
    assert!(!plan.contains("\"alpha\" is new"), "{}", plan);
    assert!(approve(&dir, Some("beta item")).status.success());
    let plan = stdout(&adr(&dir, &["plan", "adrdom/decisions/alpha.md"], &[], ""));
    assert!(!plan.contains("adr approve adrdom/terms.md"), "the terms gate is gone: {}", plan);
}

// adr: da0dc8b1-58cd-4a16-8e64-234e4bfd5320
#[test]
fn approve_with_no_name_refuses_and_names_every_waiting_entry() {
    let (dir, path) = repo();
    three(&path);
    let before = std::fs::read_to_string(&path).unwrap();
    let out = approve(&dir, None);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("\"alpha\", \"beta item\", \"gamma\" wait, and approve names none of them"), "{}", err);
    assert!(err.contains("owner runs: adr approve adrdom/terms.md <name>"), "{}", err);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before, "a refusal writes nothing");
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default();
    assert!(!ledger.contains("\tterm\t"), "{}", ledger);
}

// adr: da0dc8b1-58cd-4a16-8e64-234e4bfd5320
#[test]
fn approve_of_a_name_that_does_not_wait_refuses() {
    let (dir, path) = repo();
    three(&path);
    assert!(approve(&dir, Some("alpha")).status.success());
    for name in ["alpha", "delta"] {
        let out = approve(&dir, Some(name));
        assert!(!out.status.success());
        let err = stderr(&out);
        assert!(err.contains(&format!("\"{}\" does not wait; \"beta item\", \"gamma\" wait", name)), "{}", err);
        assert!(err.contains("owner runs: adr approve adrdom/terms.md <name>"), "{}", err);
    }
    // A name given to approve of another file is refused.
    write_topic_file(&dir, "alpha.md", TOPIC);
    let out = adr(&dir, &["approve", "adrdom/decisions/alpha.md", "alpha"], &[("ADR_OWNER_BYPASS", "1")], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("approve takes an entry name for adrdom/terms.md only"), "{}", stderr(&out));
}

// adr: da0dc8b1-58cd-4a16-8e64-234e4bfd5320
#[test]
fn a_change_and_a_removal_wait_as_entries_and_are_approved_by_name() {
    let (dir, path) = repo();
    three(&path);
    for name in ["alpha", "beta item", "gamma"] {
        assert!(approve(&dir, Some(name)).status.success());
    }
    let approved = std::fs::read_to_string(&path).unwrap();
    let start = approved.find("## gamma").unwrap();
    std::fs::write(&path, approved[..start].replace("A first thing.", "A first item.")).unwrap();

    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert_eq!(owner_lines(&shown).len(), 2, "{}", shown);
    let check = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(check.contains("adrdom/terms.md: \"alpha\" is changed"), "{}", check);
    assert!(check.contains("adrdom/terms.md: \"gamma\" is removed"), "{}", check);

    let out = approve(&dir, Some("gamma"));
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("removed: term \"gamma\""), "{}", stdout(&out));
    let out = approve(&dir, Some("alpha"));
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(adr(&dir, &["check"], &[], "").status.success());
}

// adr: da0dc8b1-58cd-4a16-8e64-234e4bfd5320
#[test]
fn a_file_that_differs_with_no_entry_waiting_is_approved_with_no_name() {
    let (dir, path) = repo();
    three(&path);
    for name in ["alpha", "beta item", "gamma"] {
        assert!(approve(&dir, Some(name)).status.success());
    }
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{}\n\n", text)).unwrap();
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert_eq!(owner_lines(&shown), vec!["owner runs: adr approve adrdom/terms.md"], "{}", shown);
    let out = approve(&dir, Some("alpha"));
    assert!(!out.status.success());
    assert!(stderr(&out).contains("\"alpha\" does not wait, and no entry does"), "{}", stderr(&out));
    assert!(approve(&dir, None).status.success());
}

// adr: da0dc8b1-58cd-4a16-8e64-234e4bfd5320
#[test]
fn plan_check_and_show_name_the_commit_that_waits_while_the_terms_file_waits() {
    let (dir, path) = repo();
    three(&path);
    write_topic_file(&dir, "alpha.md", TOPIC);
    let line = "not yet: commit, after the owner approves adrdom/terms.md";
    let plan = stdout(&adr(&dir, &["plan", "adrdom/decisions/alpha.md"], &[], ""));
    assert_eq!(plan.lines().filter(|l| *l == line).count(), 1, "{}", plan);
    let check = stdout(&adr(&dir, &["check"], &[], ""));
    assert_eq!(check.lines().filter(|l| *l == line).count(), 1, "{}", check);
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert_eq!(shown.lines().filter(|l| *l == line).count(), 1, "{}", shown);

    for name in ["alpha", "beta item", "gamma"] {
        assert!(approve(&dir, Some(name)).status.success());
    }
    assert!(!stdout(&adr(&dir, &["plan", "adrdom/decisions/alpha.md"], &[], "")).contains(line));
    assert!(!stdout(&adr(&dir, &["check"], &[], "")).contains(line));
    assert!(!stdout(&adr(&dir, &["show"], &[], "")).contains(line));
}

// adr: fcab4bf9-c02d-4094-b3cb-688c4ec312dc
// The terms brief holds approved definitions only: a named approve that
// leaves entries waiting writes no brief, the one that settles the file does.
#[test]
fn the_terms_brief_is_written_once_the_file_settles_and_holds_no_waiting_entry() {
    let (dir, path) = repo();
    let brief = dir.join("adrdom/state/terms-brief.md");
    three(&path);
    assert!(approve(&dir, Some("alpha")).status.success());
    assert!(!brief.exists(), "a brief written while beta and gamma wait");
    assert!(approve(&dir, Some("beta item")).status.success());
    assert!(!brief.exists(), "a brief written while gamma waits");
    let out = approve(&dir, Some("gamma"));
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = std::fs::read_to_string(&brief).unwrap();
    assert!(stdout(&out).contains("adrdom/state/terms-brief.md"), "{}", stdout(&out));
    for name in ["alpha", "beta item", "gamma"] {
        assert!(text.contains(name), "{}", text);
    }
    // A changed entry leaves the brief at the last approval until approved.
    let file = std::fs::read_to_string(&path).unwrap().replace("A second thing.", "A changed thing.");
    std::fs::write(&path, file).unwrap();
    let before = std::fs::read_to_string(&brief).unwrap();
    assert_eq!(before, text);
    assert!(approve(&dir, Some("beta item")).status.success());
    assert!(std::fs::read_to_string(&brief).unwrap().contains("A changed thing."));
}
