// What `adr show --ids` prints: each decision's `id:` line under its heading,
// in the listing and under a file, and nothing of it without the flag. The
// id is the one a test tie names, so a builder reads it here and not in the
// topic file.
//
// adr: cdc905a2-5b58-4c38-beff-4a8bd3b6f42e

mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/gate.md";
const ONE: &str = "Is the widget gate open?";

const BODY: &str = "\nProblem: the widget breaks.\n\nStandard: plain words.\n\n\
- A) Yes the widget holds.\n  Why: reasons.\n  Costs: costs.\n\
- B) No the widget does not hold.\n  Why: other reasons.\n  Costs: other costs.\n";

fn show(dir: &Path, args: &[&str]) -> String {
    let mut all = vec!["show"];
    all.extend_from_slice(args);
    let out = adr(dir, &all, &[], "");
    assert!(out.status.success(), "{}", stderr(&out));
    stdout(&out)
}

/// A repository with one question of `REL` approved, asked and answered A.
fn answered() -> (PathBuf, String) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let path = write_record(
        &dir,
        REL,
        &format!("# Gate\n\n### {}\ndepends: widget\n{}\n", ONE, BODY),
    );
    approve_record(&dir, REL);
    let asked = adr(&dir, &["ask", path.to_str().unwrap()], &[], "");
    assert!(asked.status.success(), "{}", stderr(&asked));
    let id = extract_ids(&std::fs::read_to_string(&path).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    (dir, id)
}

#[test]
fn the_listing_prints_each_decisions_id_under_its_heading_only_under_the_flag() {
    let (dir, id) = answered();
    let plain = show(&dir, &[]);
    assert!(!plain.contains("id: "), "{}", plain);
    let with = show(&dir, &["--ids"]);
    let want = format!("  {}\n    id: {}\n    Yes the widget holds.\n", ONE, id);
    assert!(with.contains(&want), "{}", with);
}

#[test]
fn a_file_prints_each_decisions_id_under_its_heading_only_under_the_flag() {
    let (dir, id) = answered();
    let plain = show(&dir, &[REL]);
    assert!(!plain.contains("id: "), "{}", plain);
    let with = show(&dir, &[REL, "--ids"]);
    let want = format!(
        "[answered A] {}\n  id: {}\n  Yes the widget holds.\n",
        ONE, id
    );
    assert!(with.contains(&want), "{}", with);
}
