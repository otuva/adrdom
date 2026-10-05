// The terms brief: the file `approve` writes from the terms file for the
// agent to read, what it holds, how `check` holds it to the terms file, and
// the line of the brief that names it.
mod common;

use common::*;

const BRIEF_PATH: &str = "adrdom/state/terms-brief.md";

const TERMS: &str = "# Terms\n\n\
## keeper\n\
definition: The person the repository belongs to.\n\
aliases: keepers, holder\n\
\n\
## journal\n\
definition: The file with one row per event.\n\
aliases: \n\
\n\
## relic\n\
definition: A thing no longer in use.\n\
aliases: relics\n\
retired: yes\n";

const WANT: &str = "- keeper (keepers, holder): The person the repository belongs to.\n\
- journal: The file with one row per event.\n";

fn approved_repo() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", TERMS);
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    dir
}

fn terms_rows(dir: &std::path::Path) -> usize {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap_or_default()
        .lines()
        .filter(|l| l.split('\t').nth(1) == Some("terms"))
        .count()
}

// adr: ec0b9293-916e-4ced-a3d7-3b862ab7b069
//
// "Does the tool write a terms brief for the agent?" (A): approving the
// terms file leaves a terms brief under adrdom/state, built from it.
#[test]
fn approving_the_terms_file_leaves_a_terms_brief() {
    let dir = approved_repo();
    let brief = std::fs::read_to_string(dir.join(BRIEF_PATH)).expect("no terms brief written");
    assert!(brief.contains("keeper"), "{}", brief);
    assert!(brief.contains("The person the repository belongs to."), "{}", brief);
}

// adr: f1294391-48b0-4ba3-82b4-c1d6b1c2c884
//
// "What does the terms brief hold?" (A): one line per term that is not
// retired, with its name, its aliases and its definition -- no identifier,
// no terms line, no retired term.
#[test]
fn the_terms_brief_holds_one_line_per_term_that_is_not_retired() {
    let dir = approved_repo();
    let brief = std::fs::read_to_string(dir.join(BRIEF_PATH)).unwrap();
    assert_eq!(brief, WANT);
    // The terms file itself did get its identifiers: the brief left them out.
    let terms = std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap();
    assert!(terms.contains("\nid: "), "{}", terms);
}

// adr: fcab4bf9-c02d-4094-b3cb-688c4ec312dc
//
// "Does approve write the terms brief?" (A): on every run, whether or not
// the terms file changed -- a terms file that is approved already gets its
// terms brief back from one run, with no new ledger row.
#[test]
fn approve_writes_the_terms_brief_when_the_terms_file_did_not_change() {
    let dir = approved_repo();
    let rows = terms_rows(&dir);
    std::fs::remove_file(dir.join(BRIEF_PATH)).unwrap();

    let terms = dir.join("adrdom/terms.md");
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("already approved"), "{}", stdout(&out));

    assert_eq!(std::fs::read_to_string(dir.join(BRIEF_PATH)).unwrap(), WANT);
    assert_eq!(terms_rows(&dir), rows);
}

// adr: fcab4bf9-c02d-4094-b3cb-688c4ec312dc
//
// A terms file `approve` refuses leaves the terms brief as it was.
#[test]
fn a_refused_terms_file_leaves_the_terms_brief_alone() {
    let dir = approved_repo();
    let terms = write_record(
        &dir,
        "adrdom/terms.md",
        "# Terms\n\n## loop\ndefinition: A loop that refers to loop.\naliases: \n",
    );
    let out = adr_owner(&dir, &["approve", terms.to_str().unwrap()], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    assert_eq!(std::fs::read_to_string(dir.join(BRIEF_PATH)).unwrap(), WANT);
}

// adr: ec3f875c-a0f8-4085-8da1-fec9dcb14377
//
// "Does check hold the terms brief to the terms file?" (A): check passes
// on the terms brief `approve` wrote, and fails when the file is missing or
// differs, naming the owner's move.
#[test]
fn check_fails_a_terms_brief_that_is_missing_or_differs() {
    let dir = approved_repo();
    let ok = adr(&dir, &["check"], &[], "");
    assert!(ok.status.success(), "{}", stdout(&ok));

    std::fs::write(dir.join(BRIEF_PATH), format!("{}- extra: By hand.\n", WANT)).unwrap();
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("terms: adrdom/state/terms-brief.md differs from adrdom/terms.md"),
        "{}",
        text
    );
    assert!(text.contains("owner runs: adr approve adrdom/terms.md"), "{}", text);

    std::fs::remove_file(dir.join(BRIEF_PATH)).unwrap();
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("terms: adrdom/state/terms-brief.md missing"), "{}", text);
    assert!(text.contains("owner runs: adr approve adrdom/terms.md"), "{}", text);
}

// adr: ec3f875c-a0f8-4085-8da1-fec9dcb14377
//
// At a commit check reads the staged files: a terms brief that sits on
// disk and was never staged fails, and the move is the agent's.
#[test]
fn check_at_a_commit_reads_the_staged_terms_brief() {
    let dir = approved_repo();
    run_git(
        &dir,
        &[
            "add",
            "adrdom/terms.md",
            "adrdom/state/ledger.tsv",
            "adrdom/state/reader.tsv",
            "adrdom/.gitignore",
            "adrdom/.gitattributes",
            "AGENTS.md",
            ".githooks",
        ],
    );
    let out = adr(&dir, &["check", "--staged"], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("terms: adrdom/state/terms-brief.md not staged"), "{}", text);
    assert!(text.contains("agent runs: git add adrdom/state/terms-brief.md"), "{}", text);

    run_git(&dir, &["add", BRIEF_PATH]);
    let out = adr(&dir, &["check", "--staged"], &[], "");
    assert!(out.status.success(), "{}", stdout(&out));
}

// adr: adee66f2-c8ff-477f-8ac4-a497f694c912
//
// "Does the brief name the terms brief?" (A): the brief install writes
// into AGENTS.md holds a line with @ and the path of the terms brief, on
// its own and outside any code block, and a sentence telling the agent to
// read that file.
#[test]
fn the_brief_names_the_terms_brief() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();

    let mut fenced = false;
    let mut found = false;
    for line in agents.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        if line.trim_end() == "@adrdom/state/terms-brief.md" {
            assert!(!fenced, "the @ line sits inside a code block");
            found = true;
        }
    }
    assert!(found, "no @ line for the terms brief:\n{}", agents);
    assert!(agents.contains("Read\nthat file before you write"), "{}", agents);
}
