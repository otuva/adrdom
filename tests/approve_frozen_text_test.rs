mod common;

use common::*;
use std::path::{Path, PathBuf};

fn body(n: &str) -> String {
    format!(
        "Problem: p{n}.\n\nStandard: s{n}.\n\n- A) x, a decision.\n  Why: w.\n  Costs: c.\n- B) y, a decision.\n  Why: w.\n  Costs: c.\n",
        n = n
    )
}

fn setup() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "other"]);
    dir
}

fn approved_rows(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap()
        .lines()
        .filter(|l| l.contains("\tapproved\t"))
        .count()
}

/// Approves, asks and answers A on every question of `rel`, tier by tier.
fn answer_all(dir: &Path, rel: &str, ids: &[String]) {
    let path = dir.join(rel);
    for id in ids {
        let o = adr(dir, &["ask", path.to_str().unwrap()], &[], "");
        assert!(o.status.success(), "ask failed: {}", stderr(&o));
        let o = adr_answer(dir, id, "A");
        assert!(o.status.success(), "answer failed: {}", stderr(&o));
    }
}

fn assert_refused(dir: &Path, rel: &str, before: usize, heading: &str) {
    let path = dir.join(rel);
    let out = adr_owner(dir, &["approve", path.to_str().unwrap()], "");
    assert!(!out.status.success(), "approve should refuse: {}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains(heading), "stderr should name the section: {}", err);
    assert!(err.contains("text changed since being answered"), "{}", err);
    assert_eq!(approved_rows(dir), before, "no approved row on refusal");
}

// A frozen section whose depends entry is swapped for another resolvable
// identifier is refused.
#[test]
fn approve_refuses_a_swapped_uuid_entry_in_an_answered_section() {
    let dir = setup();
    let rel = "adrdom/decisions/topic-a.md";
    let content = format!("# Topic A\n\n### Is A do a thing?\ndepends: decision\n\n{}", body("1"));
    let path = write_record(&dir, rel, &content);
    approve_record(&dir, rel);
    let ids = extract_ids(&std::fs::read_to_string(&path).unwrap());
    answer_all(&dir, rel, &ids);

    let cur = std::fs::read_to_string(&path).unwrap();
    let swapped = cur.replacen(
        "depends: 99999990-9999-9999-9999-999999999999",
        "depends: 99999991-9999-9999-9999-999999999999",
        1,
    );
    assert_ne!(cur, swapped, "fixture: the term id should be in the file: {}", cur);
    std::fs::write(&path, swapped).unwrap();
    let before = approved_rows(&dir);
    assert_refused(&dir, rel, before, "Is A do a thing?");
    let chk = adr(&dir, &["check"], &[], "");
    assert!(!chk.status.success(), "check agrees it is edited");
    let both = format!("{}{}", stdout(&chk), stderr(&chk));
    assert_eq!(
        both.matches("text changed since being answered").count(),
        1,
        "check prints the line once: {}",
        both
    );
}

// A frozen section that loses one of its entries is refused.
#[test]
fn approve_refuses_a_deleted_entry_in_an_answered_section() {
    let dir = setup();
    let rel = "adrdom/decisions/topic-a.md";
    let content = format!("# Topic A\n\n### Is A do a thing?\ndepends: decision, other\n\n{}", body("1"));
    let path = write_record(&dir, rel, &content);
    approve_record(&dir, rel);
    let ids = extract_ids(&std::fs::read_to_string(&path).unwrap());
    answer_all(&dir, rel, &ids);

    let cur = std::fs::read_to_string(&path).unwrap();
    let cut = cur
        .lines()
        .map(|l| {
            if l.starts_with("depends: ") {
                "depends: 99999990-9999-9999-9999-999999999999".to_string()
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_ne!(cur, cut);
    std::fs::write(&path, cut).unwrap();
    let before = approved_rows(&dir);
    assert_refused(&dir, rel, before, "Is A do a thing?");
}

// An untouched frozen section beside a new heading is no false positive.
#[test]
fn approve_accepts_an_untouched_answered_section_with_a_new_heading() {
    let dir = setup();
    let rel = "adrdom/decisions/topic-a.md";
    let content = format!("# Topic A\n\n### Is A do a thing?\ndepends: decision, other\n\n{}", body("1"));
    let path = write_record(&dir, rel, &content);
    approve_record(&dir, rel);
    let ids = extract_ids(&std::fs::read_to_string(&path).unwrap());
    answer_all(&dir, rel, &ids);

    let mut cur = std::fs::read_to_string(&path).unwrap();
    cur.push_str(&format!("\n### Is B do a thing?\ndepends: decision\n\n{}", body("2")));
    std::fs::write(&path, cur).unwrap();
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(out.status.success(), "approve should accept: {}", stderr(&out));
}

// Only the answer set on an answered section changes: check fails it as
// edited (the depends line is inside its hashed text), so approve refuses.
#[test]
fn approve_refuses_a_changed_answer_set_on_an_answered_section() {
    let dir = setup();
    let rel = "adrdom/decisions/topic-a.md";
    let content = format!(
        "# Topic A\n\n### Is A do a thing?\ndepends: decision\n\n{}\n### Is B stand on A?\ndepends: Is A do a thing?=A\n\n{}",
        body("1"),
        body("2")
    );
    let path = write_record(&dir, rel, &content);
    approve_record(&dir, rel);
    let ids = extract_ids(&std::fs::read_to_string(&path).unwrap());
    answer_all(&dir, rel, &ids);

    let cur = std::fs::read_to_string(&path).unwrap();
    assert!(cur.contains("=A\n"), "fixture: {}", cur);
    std::fs::write(&path, cur.replacen("=A\n", "=*\n", 1)).unwrap();
    let before = approved_rows(&dir);
    assert_refused(&dir, rel, before, "Is B stand on A?");
    let chk = adr(&dir, &["check"], &[], "");
    assert!(!chk.status.success(), "check also fails it");
}
