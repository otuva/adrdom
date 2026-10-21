// At most four questions stand on one question, or on terms alone; the count
// is per question, never per derived tier.
mod common;

use common::*;
use std::path::{Path, PathBuf};

const BODY: &str = "\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";

fn h(n: usize) -> String {
    format!("Is widget option {} chosen?", n)
}

const P: &str = "Is widget root P chosen?";
const Q: &str = "Is widget root Q chosen?";

/// One question; `depends` is the whole depends line text, `sup` an optional
/// supersedes entry.
fn sec(heading: &str, depends: &str, sup: Option<&str>) -> String {
    let mut s = format!("\n### {}\ndepends: {}\n", heading, depends);
    if let Some(x) = sup {
        s.push_str(&format!("supersedes: {}\n", x));
    }
    s.push_str(BODY);
    s
}

/// A question standing on the in-file question `parent` (any answer).
fn on(heading: &str, parent: &str) -> String {
    sec(heading, &format!("{}=*", parent), None)
}

/// A question standing on terms alone.
fn terms(heading: &str) -> String {
    sec(heading, "widget", None)
}

fn rel(name: &str) -> String {
    format!("adrdom/decisions/{}.md", name)
}

fn append(path: &Path, text: &str) {
    let mut c = std::fs::read_to_string(path).unwrap();
    c.push_str(text);
    std::fs::write(path, c).unwrap();
}

fn id_of(path: &Path, heading: &str) -> String {
    let c = std::fs::read_to_string(path).unwrap();
    let mut lines = c.lines().skip_while(|l| *l != format!("### {}", heading));
    assert!(lines.next().is_some(), "no heading {} in {}", heading, c);
    for l in lines {
        if let Some(id) = l.strip_prefix("id: ") {
            return id.trim().to_string();
        }
    }
    panic!("no id under {}: {}", heading, c);
}

fn approve(dir: &Path, name: &str) -> std::process::Output {
    adr_owner(dir, &["approve", dir.join(rel(name)).to_str().unwrap()], "")
}

fn approve_ok(dir: &Path, name: &str) {
    let out = approve(dir, name);
    assert!(
        out.status.success(),
        "approve {} failed: {}",
        name,
        stderr(&out)
    );
}

fn setup() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    dir
}

fn said_cap(out: &std::process::Output) -> String {
    format!("{}{}", stdout(out), stderr(out))
}

fn refused_with(out: &std::process::Output, parts: &[&str]) {
    assert!(!out.status.success(), "should refuse: {}", stdout(out));
    let all = said_cap(out);
    for p in parts {
        assert!(all.contains(p), "missing {:?} in: {}", p, all);
    }
}

/// P and `n` questions standing on it.
fn under_p(n: usize) -> String {
    let mut c = format!("# A\n{}", terms(P));
    for i in 1..=n {
        c.push_str(&on(&h(i), P));
    }
    c
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn approve_refuses_five_questions_standing_on_one_question() {
    let dir = setup();
    write_record(&dir, &rel("a"), &under_p(5));
    let out = approve(&dir, "a");
    let first = format!("\"{}\"", h(1));
    let fifth = format!("\"{}\"", h(5));
    refused_with(
        &out,
        &[
            &format!("5 questions stand on \"{}\", the most is 4", P),
            &first,
            &fifth,
        ],
    );
    assert!(!said_cap(&out).contains("answered"), "{}", said_cap(&out));
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn approve_passes_four_questions_standing_on_one_question() {
    let dir = setup();
    write_record(&dir, &rel("a"), &under_p(4));
    approve_ok(&dir, "a");
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn approve_refuses_five_questions_standing_on_terms_alone() {
    let dir = setup();
    let mut c = String::from("# A\n");
    for i in 1..=5 {
        c.push_str(&terms(&h(i)));
    }
    write_record(&dir, &rel("a"), &c);
    let out = approve(&dir, "a");
    refused_with(&out, &["5 questions stand on terms alone, the most is 4"]);
    assert!(
        said_cap(&out).contains("agent writes the question"),
        "{}",
        said_cap(&out)
    );
}

/// a.md: P and four questions on it (or four on terms alone when `on_p` is
/// false), approved, asked and answered, plus a fifth open question that
/// replaces the first. b.md holds an open question replacing the first of
/// a.md, so a.md's count is four; when b.md's question is refused the first
/// is back and a.md holds five, which approve stops.
fn approve_side(on_p: bool) -> std::process::Output {
    let dir = setup();
    let mut c = String::from("# A\n");
    if on_p {
        c.push_str(&terms(P));
    }
    let kid = |i: usize| if on_p { on(&h(i), P) } else { terms(&h(i)) };
    for i in 1..=4 {
        c.push_str(&kid(i));
    }
    let a = write_record(&dir, &rel("a"), &c);
    approve_ok(&dir, "a");
    if on_p {
        let out = adr_answer(&dir, &id_of(&a, P), "A");
        assert!(out.status.success(), "answer failed: {}", stderr(&out));
    }
    for i in 1..=4 {
        let out = adr_answer(&dir, &id_of(&a, &h(i)), "A");
        assert!(out.status.success(), "answer failed: {}", stderr(&out));
    }
    let b = write_record(
        &dir,
        &rel("b"),
        &format!("# B\n{}", sec(&h(9), "widget", Some(&h(1)))),
    );
    approve_ok(&dir, "b");
    if on_p {
        // P is answered now: its entry carries no answer set.
        append(&a, &sec(&h(5), &id_of(&a, P), None));
    } else {
        append(&a, &kid(5));
    }
    approve_ok(&dir, "a");
    let out = adr_answer(&dir, &id_of(&b, &h(9)), "refuse");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));
    approve(&dir, "a")
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn approve_stops_a_fifth_question_standing_on_terms_alone() {
    let out = approve_side(false);
    refused_with(
        &out,
        &["5 questions stand on terms alone, 4 answered, the most is 4"],
    );
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn approve_stops_a_fifth_question_standing_on_one_question() {
    let out = approve_side(true);
    refused_with(
        &out,
        &[&format!(
            "5 questions stand on \"{}\", 4 answered, the most is 4",
            P
        )],
    );
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn five_questions_at_one_derived_tier_over_two_parents_pass_approve() {
    let dir = setup();
    let mut c = format!("# A\n{}{}", terms(P), terms(Q));
    for i in 1..=3 {
        c.push_str(&on(&h(i), P));
    }
    for i in 4..=5 {
        c.push_str(&on(&h(i), Q));
    }
    let a = write_record(&dir, &rel("a"), &c);
    approve_ok(&dir, "a");
    let rf = adr::record::load(&a).unwrap();
    let tiers: Vec<usize> = (0..rf.sections.len())
        .map(|i| rf.tier_of(i).unwrap())
        .collect();
    assert_eq!(tiers, vec![1, 1, 2, 2, 2, 2, 2]);
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn a_question_naming_two_questions_counts_under_each() {
    let dir = setup();
    let mut c = format!("# A\n{}{}", terms(P), terms(Q));
    for i in 1..=4 {
        c.push_str(&on(&h(i), P));
    }
    c.push_str(&sec(&h(5), &format!("{}=*, {}=*", P, Q), None));
    write_record(&dir, &rel("a"), &c);
    let out = approve(&dir, "a");
    refused_with(
        &out,
        &[&format!("5 questions stand on \"{}\", the most is 4", P)],
    );
    assert!(
        !said_cap(&out).contains(&format!("stand on \"{}\"", Q)),
        "{}",
        said_cap(&out)
    );
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn a_question_naming_two_questions_counts_under_the_second_too() {
    let dir = setup();
    let mut c = format!("# A\n{}{}", terms(P), terms(Q));
    for i in 1..=4 {
        c.push_str(&on(&h(i), Q));
    }
    c.push_str(&sec(&h(5), &format!("{}=*, {}=*", P, Q), None));
    write_record(&dir, &rel("a"), &c);
    let out = approve(&dir, "a");
    refused_with(
        &out,
        &[&format!("5 questions stand on \"{}\", the most is 4", Q)],
    );
    assert!(
        !said_cap(&out).contains(&format!("stand on \"{}\"", P)),
        "{}",
        said_cap(&out)
    );
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn a_child_of_a_refused_question_counts_under_it() {
    let dir = setup();
    let a = write_record(&dir, &rel("a"), &under_p(4));
    approve_ok(&dir, "a");
    let out = adr_answer(&dir, &id_of(&a, P), "refuse");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));
    append(&a, &sec(&h(5), &id_of(&a, P), None));
    let out = approve(&dir, "a");
    refused_with(
        &out,
        &[&format!("5 questions stand on \"{}\", the most is 4", P)],
    );
}

// adr: 9c1b476c-9296-444b-a51c-e2957ad40ff5
#[test]
fn an_open_replacement_frees_a_place_under_a_question() {
    let dir = setup();
    let a = write_record(&dir, &rel("a"), &under_p(4));
    approve_ok(&dir, "a");
    let out = adr_answer(&dir, &id_of(&a, P), "A");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    for i in 1..=4 {
        let out = adr_answer(&dir, &id_of(&a, &h(i)), "A");
        assert!(out.status.success(), "answer failed: {}", stderr(&out));
    }
    append(&a, &sec(&h(5), &id_of(&a, P), Some(&h(1))));
    approve_ok(&dir, "a");
}

// adr: 4555583f-3499-4963-993b-34556fe519f7
#[test]
fn approve_passes_a_group_whose_questions_are_all_answered() {
    let dir = setup();
    let a = five_answered(&dir);
    approve_ok(&dir, "a");
    let first = id_of(&a, &h(1));
    append(&a, &sec("Is widget extra chosen?", &first, None));
    approve_ok(&dir, "a");
    let out = approve(&dir, "a");
    assert!(out.status.success(), "approve failed: {}", said_cap(&out));
    assert!(
        !said_cap(&out).contains("the most is 4"),
        "{}",
        said_cap(&out)
    );
}

/// a.md with five questions on terms alone, all answered, written the way a
/// plan approved before the cap would stand (ids, answers and ledger rows by
/// hand: approve would stop a fifth).
fn five_answered(dir: &Path) -> PathBuf {
    let terms_c = std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap();
    let term_id = terms_c
        .lines()
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap_or_else(|| panic!("no term id: {}", terms_c))
        .trim()
        .to_string();
    let mut c = String::from("# A\n");
    let mut rows = Vec::new();
    for i in 1..=5 {
        let id = format!("0000000{}-aaaa-4aaa-8aaa-aaaaaaaaaaaa", i);
        let section = format!(
            "### {}\ndepends: {}\n\nid: {}\n{}\n**Answer:** A\n",
            h(i),
            term_id,
            id,
            BODY
        );
        rows.push((id, mirror_section_hash(&section)));
        c.push_str(&section);
        c.push('\n');
    }
    let p = write_record(dir, &rel("a"), &c);
    for (id, hash) in rows {
        append_ledger_row(dir, "answered", &id, &hash, "A");
    }
    p
}

// adr: 4555583f-3499-4963-993b-34556fe519f7
#[test]
fn a_group_whose_questions_are_all_answered_is_not_judged() {
    let dir = setup();
    let a = five_answered(&dir);
    approve_ok(&dir, "a");
    let chk = adr(&dir, &["check"], &[], "");
    assert!(
        !said_cap(&chk).contains("the most is 4"),
        "check printed a cap refusal: {}",
        said_cap(&chk)
    );
    // A question standing on one of the answered five opens a group of its
    // own; the answered group is still all answered.
    let first = id_of(&a, &h(1));
    append(&a, &sec("Is widget extra chosen?", &first, None));
    approve_ok(&dir, "a");
}

// adr: 4555583f-3499-4963-993b-34556fe519f7
#[test]
fn an_open_question_joining_an_answered_group_is_judged_and_named_alone() {
    let dir = setup();
    let a = five_answered(&dir);
    approve_ok(&dir, "a");
    append(&a, &terms("Is widget sixth chosen?"));
    let out = approve(&dir, "a");
    refused_with(
        &out,
        &[
            "6 questions stand on terms alone, 5 answered, the most is 4: \"Is widget sixth chosen?\"",
        ],
    );
    assert!(
        !said_cap(&out).contains(&format!("\"{}\"", h(1))),
        "{}",
        said_cap(&out)
    );
}

// adr: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
#[test]
fn a_refused_question_is_not_counted() {
    let dir = setup();
    let mut c = String::from("# A\n");
    for i in 1..=4 {
        c.push_str(&terms(&h(i)));
    }
    let a = write_record(&dir, &rel("a"), &c);
    approve_ok(&dir, "a");
    let out = adr_answer(&dir, &id_of(&a, &h(1)), "refuse");
    assert!(out.status.success(), "refuse failed: {}", stderr(&out));
    append(&a, &terms(&h(5)));
    approve_ok(&dir, "a");
}
