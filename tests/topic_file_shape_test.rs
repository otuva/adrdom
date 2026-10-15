// The shape of a topic file: one first-level heading, optional prose, then
// the questions as third-level headings ending in ?, nothing else. Text
// under a `##` line between questions and a `###` line with no ? are
// reported by plan, approve and check alike.
mod common;

use common::*;

const REL: &str = "adrdom/decisions/gate.md";

const OPTS: &str = "- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";

fn q(heading: &str) -> String {
    format!("### {}\ndepends: widget\n\nProblem: the widget breaks.\n\nStandard: plain words.\n\n{}", heading, OPTS)
}

fn repo() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget"]);
    dir
}

/// Runs plan, approve and check on `content` (a draft) and asserts each
/// prints `line`, then check on the same file approved fails with it too.
fn reported_by_all(content: &str, line: &str) {
    let dir = repo();
    let record = write_record(&dir, REL, content);
    let path = record.to_str().unwrap();

    let plan = adr(&dir, &["plan", path], &[], "");
    assert!(!plan.status.success(), "{}", stdout(&plan));
    assert!(stderr(&plan).contains(line), "plan: {}", stderr(&plan));

    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default();
    let approve = adr_owner(&dir, &["approve", path], "");
    assert!(!approve.status.success(), "{}", stdout(&approve));
    assert!(stderr(&approve).contains(line), "approve: {}", stderr(&approve));
    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default();
    assert_eq!(ledger_before, ledger_after, "a refused approve writes no row");

    // A draft: a note, the commit goes through.
    let check = adr(&dir, &["check"], &[], "");
    assert!(check.status.success(), "{}", stdout(&check));
    assert!(stdout(&check).contains(line), "check: {}", stdout(&check));

    // Approved: a failure.
    let dir = repo();
    approve_via_ledger(&dir, REL, content);
    let check = adr(&dir, &["check"], &[], "");
    assert!(!check.status.success(), "{}", stdout(&check));
    assert!(stdout(&check).contains(line), "check approved: {}", stdout(&check));
}

#[test]
fn text_under_a_tier_heading_between_questions_is_reported_by_plan_approve_and_check() {
    // adr: 9913568f-6b85-4d26-946b-c6e108f9aa51
    let content = format!("# Gate\n\n{}\n## Tier two\n\nText that sits in no question.\n\n{}", q("Does the widget hold?"), q("Does the widget stay?"));

    let n = content.lines().position(|l| l == "Text that sits in no question.").unwrap() + 1;
    reported_by_all(&content, &format!("{}: stray line {}; agent rewrites the file", REL, n));
}


#[test]
fn a_heading_line_without_a_question_mark_is_reported_by_plan_approve_and_check() {
    // adr: 9913568f-6b85-4d26-946b-c6e108f9aa51
    let content = format!("# Gate\n\n{}\n### Not a question\n", q("Does the widget hold?"));
    let n = content.lines().position(|l| l == "### Not a question").unwrap() + 1;
    reported_by_all(
        &content,
        &format!("{}: line {}: a heading is not a question ending in ?; agent rewrites the line", REL, n),
    );
}
