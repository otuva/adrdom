// A dead section (refused, superseded or withdrawn) is history: its frozen
// `depends:` line is no longer resolved against the current decisions, and
// `check` prints no empty `tiers:` note for a file with no live question.
// A live section still refuses on a dependency that is not current.

mod common;

use common::*;
use std::path::Path;

fn section(heading: &str, depends: &str, supersedes: &str) -> String {
    let sup = if supersedes.is_empty() {
        String::new()
    } else {
        format!("supersedes: {}\n", supersedes)
    };
    format!(
        "\n### {}\ndepends: {}\n{}\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        heading, depends, sup
    )
}

/// Appends `sec` to `rel` (creating it with `title`), approves, asks, answers
/// A, and returns the id of the new section (the last id in the file).
fn add_answered(dir: &Path, rel: &str, title: &str, sec: &str) -> String {
    let path = dir.join(rel);
    let mut content = if path.exists() {
        std::fs::read_to_string(&path).unwrap()
    } else {
        format!("# {}\n", title)
    };
    content.push_str(sec);
    write_record(dir, rel, &content);
    approve_record(dir, rel);
    assert!(adr(dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    let ids = extract_ids(&std::fs::read_to_string(&path).unwrap());
    let id = ids.last().unwrap().clone();
    let out = adr_answer(dir, &id, "A");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    id
}

/// a.md alpha answered; b.md beta stands on alpha, answered. Returns
/// (dir, alpha id).
fn alpha_and_beta() -> (std::path::PathBuf, String) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let alpha = add_answered(
        &dir,
        "adrdom/decisions/a.md",
        "A",
        &section("Does alpha hold authority?", "authority", ""),
    );
    add_answered(
        &dir,
        "adrdom/decisions/b.md",
        "B",
        &section("Does beta hold authority?", &alpha, ""),
    );
    (dir, alpha)
}

#[test]
fn check_ignores_the_depends_line_of_a_superseded_section() {
    let (dir, _alpha) = alpha_and_beta();
    add_answered(
        &dir,
        "adrdom/decisions/a.md",
        "A",
        &section(
            "Does gamma hold authority?",
            "authority",
            "Does alpha hold authority?, Does beta hold authority?",
        ),
    );

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("which is not a current decision"), "{}", text);
    assert!(!text.contains("tiers: adrdom/decisions/b.md: \n"), "{}", text);
    assert!(!text.contains("tiers: adrdom/decisions/b.md:\n"), "{}", text);
    for line in text.lines() {
        assert!(
            !line.starts_with("plan:"),
            "unexpected plan refusal: {}\n{}",
            line,
            text
        );
    }
}

#[test]
fn check_still_refuses_a_live_section_standing_on_a_superseded_decision() {
    let (dir, alpha) = alpha_and_beta();
    // The rows stand for a merge of two branches: one superseded alpha while
    // the other answered beta standing on it.
    append_ledger_row_at(
        &dir,
        "superseded",
        &alpha,
        "-",
        "11111111-2222-3333-4444-555555555555",
        "2099-01-01T00:00:00Z",
    );
    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!out.status.success(), "{}", text);
    assert!(
        text.contains(&format!("depends on {} (", alpha))
            && text.contains("which is not a current decision"),
        "{}",
        text
    );
}
