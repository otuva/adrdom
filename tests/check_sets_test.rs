// The frozen-set check on `assumes` rows (a depends line's answer set must
// not silently change after the target was asked, but a dependent heading
// that vanished entirely is fine).
// Most fixtures are hand-written (record content plus
// `append_ledger_row`/`assumes_hash`) rather than through a real `adr ask`,
// though some tests below do run `adr approve`/`adr ask` for real.

mod common;

use common::*;

const TARGET_ID: &str = "44444444-4444-4444-4444-444444444444";
// an `assumes` row keys on the dependent's own heading id,
// not its heading text -- so a fixture that hand-writes such a row (rather
// than going through real `ask`) needs the dependent heading to carry a
// fixed id of its own, used as that key.
const DEPENDENT_ID: &str = "55555555-5555-5555-5555-555555555555";
const REMOVED_DEPENDENT_ID: &str = "66666666-6666-6666-6666-666666666666";

fn target_section(answer: Option<&str>) -> String {
    let mut s = format!(
        "### 1.1 Which color?\n\nid: {}\n\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) Red.\n  Why: w.\n  Costs: c.\n- B) Blue.\n  Why: w.\n  Costs: c.",
        TARGET_ID
    );
    if let Some(a) = answer {
        s.push_str(&format!("\n\n**Answer:** {}", a));
    }
    s
}

#[test]
fn check_fails_when_a_dependents_answer_set_changed_after_ask() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = format!(
        "# Colors\n\n{}\n\n### 1.2 Depends on color?\n\nid: {}\n\ndepends: 1.1 Which color?=A\n",
        target_section(None),
        DEPENDENT_ID
    );
    write_record(&dir, "adrdom/decisions/colors.md", &content);
    append_ledger_row(&dir, "asked", TARGET_ID, "anyhash", "1.1 Which color?");
    // Recorded at ask time with set B; the file's current depends line
    // reads =A: the set moved after asking.
    let stale_hash = assumes_hash(DEPENDENT_ID, "B");
    append_ledger_row(&dir, "assumes", TARGET_ID, &stale_hash, DEPENDENT_ID);

    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains(
            "adrdom/decisions/colors.md: 1.2 Depends on color? answer set on 1.1 Which color? changed after ask; a depends entry's answer set stays as asked; agent restores the answer set from git"
        ),
        "{}",
        text
    );
}

#[test]
fn check_passes_when_a_dependents_answer_set_is_unchanged() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = format!(
        "# Colors\n\n{}\n\n### 1.2 Depends on color?\n\nid: {}\n\ndepends: 1.1 Which color?=A\n",
        target_section(None),
        DEPENDENT_ID
    );
    write_record(&dir, "adrdom/decisions/colors.md", &content);
    append_ledger_row(&dir, "asked", TARGET_ID, "anyhash", "1.1 Which color?");
    let matching_hash = assumes_hash(DEPENDENT_ID, "A");
    append_ledger_row(&dir, "assumes", TARGET_ID, &matching_hash, DEPENDENT_ID);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("changed after ask"), "{}", text);
}

#[test]
fn check_passes_when_the_dependent_heading_vanished() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    // No dependent section any more: the depends line (and the heading it
    // was on) is gone.
    let content = format!("# Colors\n\n{}\n", target_section(None));
    write_record(&dir, "adrdom/decisions/colors.md", &content);
    append_ledger_row(&dir, "asked", TARGET_ID, "anyhash", "1.1 Which color?");
    let old_hash = assumes_hash(REMOVED_DEPENDENT_ID, "A");
    append_ledger_row(
        &dir,
        "assumes",
        TARGET_ID,
        &old_hash,
        REMOVED_DEPENDENT_ID,
    );

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.lines().any(|l| l.starts_with("answers:") && l.contains("answer set")),
        "a vanished dependent heading must not be flagged: {}",
        text
    );
}

#[test]
fn check_passes_when_the_dependents_heading_has_extra_leading_whitespace() {
    // record.rs's `heading_text` used to be trimmed only at the end, so a
    // heading written with extra space right after `### ` kept that space
    // in the string `ask` hashed into the `assumes` row's detail/hash, while
    // `check`'s frozen-set comparison always trimmed both ends before
    // recomputing the hash. The two disagreed and `check` wrongly reported
    // the set as "changed after ask". Run through the real `ask`/`check`
    // pair (not hand-written ledger rows) so the fix is verified at the
    // point the mismatch used to happen.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    // "Extra space?" stands on "Which color?" and is a dependent of it for
    // the assumes row this ask writes.
    // the term-pairing scan now leaves out `depends:` lines
    // (they hold references, not prose), so "widget" has to appear in the
    // heading/Problem/Standard/option text itself for this to pair the term.
    let content = "# Colors\n\n\
### 1.1 Which color is the widget?\n\n\
depends: widget\n\n\
Problem: p of the widget.\n\n\
Standard: s.\n\n\
- A) Red.\n  Why: w.\n  Costs: c.\n\
- B) Blue.\n  Why: w.\n  Costs: c.\n\n\
###   Extra space?\n\
depends: 1.1 Which color is the widget?=A\n\n\
Problem: p of the widget.\n\n\
Standard: s.\n\n\
- A) Yes.\n  Why: w.\n  Costs: c.\n\
- B) No.\n  Why: w.\n  Costs: c.\n";
    let record = write_record(&dir, "adrdom/decisions/colors.md", content);
    approve_record(&dir, "adrdom/decisions/colors.md");

    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(
        ask_out.status.success(),
        "stdout: {} stderr: {}",
        stdout(&ask_out),
        stderr(&ask_out)
    );

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(
        !text.contains("changed after ask"),
        "extra whitespace after ### must not desync ask's hash from check's: {}",
        text
    );
}

#[test]
fn check_does_not_note_a_dependent_whose_set_still_includes_the_answer() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let content = format!(
        "# Colors\n\n{}\n\n### 1.2 Depends on color?\ndepends: 1.1 Which color?=A|B\n",
        target_section(Some("B"))
    );
    write_record(&dir, "adrdom/decisions/colors.md", &content);

    let out = adr(&dir, &["check"], &[], "");
    let text = stdout(&out);
    assert!(!text.contains("no case left:"), "{}", text);
}
