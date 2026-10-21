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

/// Appends `sec` to `rel` (creating it with `title`), approves (which asks), answers
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
    let ids = extract_ids(&std::fs::read_to_string(&path).unwrap());
    let id = ids.last().unwrap().clone();
    let out = adr_answer(dir, &id, "A");
    assert!(out.status.success(), "answer failed: {}", stderr(&out));
    id
}

/// a.md alpha answered; b.md beta stands on alpha, answered. Returns
/// (dir, alpha id). Beta is answered while it sits in a.md, standing on
/// alpha there, and then moves to b.md with its text unchanged: the record
/// of a question that names a decision of another file, which a question
/// written now cannot be.
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
    let beta = add_answered(
        &dir,
        "adrdom/decisions/a.md",
        "A",
        &section("Does beta hold authority?", &alpha, ""),
    );
    let a = std::fs::read_to_string(dir.join("adrdom/decisions/a.md")).unwrap();
    let at = a.find("### Does beta hold authority?").unwrap();
    let (kept, moved) = a.split_at(at);
    write_record(&dir, "adrdom/decisions/a.md", kept.trim_end_matches('\n'));
    write_record(&dir, "adrdom/decisions/b.md", &format!("# B\n\n{}", moved));
    let _ = beta;
    for rel in ["adrdom/decisions/a.md", "adrdom/decisions/b.md"] {
        let out = adr_owner(&dir, &["approve", dir.join(rel).to_str().unwrap()], "");
        assert!(
            out.status.success(),
            "approve {}: {}{}",
            rel,
            stdout(&out),
            stderr(&out)
        );
    }
    (dir, alpha)
}

// adr: 9afe335c-2a7a-4625-b115-f74d3f091cac
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
    assert!(
        !text.contains("which names no term and no question of this file"),
        "{}",
        text
    );
    assert!(
        !text.contains("tiers: adrdom/decisions/b.md: \n"),
        "{}",
        text
    );
    assert!(
        !text.contains("tiers: adrdom/decisions/b.md:\n"),
        "{}",
        text
    );
    for line in text.lines() {
        assert!(
            !line.starts_with("plan:"),
            "unexpected plan refusal: {}\n{}",
            line,
            text
        );
    }
}

// adr: 9afe335c-2a7a-4625-b115-f74d3f091cac
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
            && text.contains("which names no term and no question of this file"),
        "{}",
        text
    );
}

/// No line reports the depends entry that names alpha: the tie reminders
/// name alpha by id too, so only lines about a depends entry are read.
fn assert_no_dep_line(text: &str, alpha: &str) {
    for line in text.lines() {
        assert!(
            !(line.contains(alpha) && line.contains("depends"))
                && !line.contains("decision of another file"),
            "old entry reported: {}\n{}",
            line,
            text
        );
    }
}

// adr: 9afe335c-2a7a-4625-b115-f74d3f091cac
#[test]
fn old_cross_file_entry_on_a_closed_question_does_not_stop_a_draft_file() {
    let (dir, alpha) = alpha_and_beta();
    let rel = "adrdom/decisions/b.md";
    let path = dir.join(rel);
    let mut content = std::fs::read_to_string(&path).unwrap();
    content.push_str(&section("Does delta hold authority?", "authority", ""));
    write_record(&dir, rel, &content);

    let out = adr(&dir, &["check"], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert_no_dep_line(&text, &alpha);

    plan_record(&dir, rel);
    let out = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(out.status.success(), "approve failed: {}", text);
    assert_no_dep_line(&text, &alpha);
}
