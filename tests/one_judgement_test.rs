// One state of the records, one judgement: what `approve`, `ask` or
// `answer` refuse on, `check` reports, and `show` and `plan` print with the
// same move (per "Does a command refuse what check passes?", fe14991d A,
// and "Does a command compute a move of its own?", aac5c313 A). Each case
// is a state a review found the commands disagreeing on.
mod common;

use common::*;
use common::{approve_unplanned as approve, gate_question as q, widget_repo as repo};
use std::path::Path;

const REL: &str = "adrdom/decisions/gate.md";

fn all_of(dir: &Path, args: &[&str]) -> String {
    let out = adr(dir, args, &[], "");
    format!("{}{}", stdout(&out), stderr(&out))
}

/// A gate file of one question, approved and asked.
fn asked_gate(dir: &Path) -> std::path::PathBuf {
    let rec = write_record(dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks.")));
    approve_record(dir, REL);
    assert!(adr(dir, &["ask", rec.to_str().unwrap()], &[], "").status.success());
    rec
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// A word taken from the records runs as the word it is: a blame file named
// with a shell substitution is named quoted in the owner's approve move,
// and an `id:` line holding no identifier the tool assigns is a finding
// every command names.
#[test]
fn a_path_or_an_id_from_the_records_never_runs_as_shell() {
    let dir = repo();
    asked_gate(&dir);
    let blob = write_origin(&dir, "docs/old.md", "one\ntwo\n");
    write_blame_file(&dir, "adrdom/blame/a$(touch PWNED).md", "docs/old.md", &blob, "1-2\tdetail\n");
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    let mv = shown.lines().find(|l| l.contains("adr approve adrdom/blame/") || l.contains("adr approve 'adrdom/blame/"));
    assert_eq!(mv, Some("owner runs: adr approve 'adrdom/blame/a$(touch PWNED).md'"), "{}", shown);

    let draft = "adrdom/decisions/forged.md";
    write_record(
        &dir,
        draft,
        &format!("# Forged\n\n### Does the widget fit?\ndepends: widget\n\nid: x$(touch PWNED2)\n\nProblem: p.\n\nStandard: s.\n\n{}", GATE_OPTIONS),
    );
    let finding = "id line holds \"x$(touch PWNED2)\", which is no identifier the tool assigns";
    for args in [vec!["show", draft], vec!["plan", draft], vec!["check"]] {
        let text = all_of(&dir, &args);
        assert!(text.contains(finding), "{:?} lacks the finding:\n{}", args, text);
        assert!(!text.contains("adr answer x$("), "{:?}:\n{}", args, text);
    }
    let out = approve(&dir, &dir.join(draft));
    assert!(!out.status.success());
    assert!(stderr(&out).contains(finding), "{}", stderr(&out));
    assert!(!dir.join("PWNED").exists() && !dir.join("PWNED2").exists());
}

// adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
//
// Two depends entries that name one term (its name, written twice in two
// cases) are a finding before approval, in every command, and approve
// refuses rather than write one identifier twice.
#[test]
fn two_entries_naming_one_target_are_found_before_approval() {
    let dir = repo();
    let rec = write_record(
        &dir,
        REL,
        &format!("# Gate\n\n### Does the widget hold?\ndepends: widget, Widget\n\nProblem: p.\n\nStandard: s.\n\n{}", GATE_OPTIONS),
    );
    plan_record(&dir, REL);
    let finding = "depends entry \"Widget\" names what an earlier entry of the line names";
    for args in [vec!["show", REL], vec!["plan", REL], vec!["check"]] {
        let text = all_of(&dir, &args);
        assert!(text.contains(finding), "{:?} lacks the finding:\n{}", args, text);
    }
    let out = approve(&dir, &rec);
    assert!(!out.status.success(), "approve takes it: {}", stdout(&out));
    assert!(stderr(&out).contains(finding), "{}", stderr(&out));
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
//
// An answered question deleted from its file: approve refuses any file on
// it, so show and plan print its finding and name no owner approve.
#[test]
fn an_answered_question_no_file_holds_is_named_and_approve_is_not() {
    let dir = repo();
    let rec = asked_gate(&dir);
    let id = extract_ids(&std::fs::read_to_string(&rec).unwrap()).remove(0);
    assert!(adr_answer(&dir, &id, "A").status.success());
    let text = std::fs::read_to_string(&rec).unwrap();
    let cut = text.find("### Does the widget hold?").unwrap();
    std::fs::write(&rec, format!("{}### Does the widget stay?\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n{}", &text[..cut], GATE_OPTIONS)).unwrap();
    plan_record(&dir, REL);
    let checked = stdout(&adr(&dir, &["check"], &[], ""));
    let missing = checked.lines().find(|l| l.contains("is answered and current but missing from every topic file") && l.contains("Does the widget hold?")).unwrap_or_else(|| panic!("{}", checked)).to_string();
    for args in [vec!["show", REL], vec!["plan", REL]] {
        let text = all_of(&dir, &args);
        assert!(text.contains(&missing), "{:?} lacks the finding:\n{}", args, text);
        assert!(!text.contains("owner runs: adr approve adrdom/decisions/gate.md"), "{:?}:\n{}", args, text);
    }
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
//
// Two standing answers for one question: no decision stands to build, so
// no command names the build; each names the conflict with its move.
#[test]
fn two_standing_answers_name_the_conflict_and_no_build() {
    let dir = repo();
    let rec = asked_gate(&dir);
    let id = extract_ids(&std::fs::read_to_string(&rec).unwrap()).remove(0);
    assert!(adr_answer(&dir, &id, "A").status.success());
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let row = ledger.lines().rfind(|l| l.contains("\tanswered\t")).unwrap();
    let hash = row.split('\t').nth(3).unwrap().to_string();
    append_ledger_row_at(&dir, "answered", &id, &hash, "B", "2099-01-01T00:00:00Z");
    for args in [vec!["show"], vec!["show", REL], vec!["plan", REL], vec!["check"]] {
        let text = all_of(&dir, &args);
        assert!(text.contains("two standing answers"), "{:?} lacks the conflict:\n{}", args, text);
        assert!(!text.contains("agent builds"), "{:?} names a build:\n{}", args, text);
    }
}

// adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
//
// A term entry waiting for its approval: the topic file using its word is
// judged with it (no "no term" finding), the terms file's approve comes
// first in show, and no command names a topic-file move approve, ask or
// answer would refuse on the terms file.
#[test]
fn a_term_awaiting_approval_holds_the_topic_files_and_is_named_first() {
    let dir = repo();
    let terms = dir.join("adrdom/terms.md");
    let mut t = std::fs::read_to_string(&terms).unwrap();
    t.push_str("\n## gizmo\ndefinition: A fixture term for tests.\naliases: \n");
    std::fs::write(&terms, &t).unwrap();
    write_record(&dir, REL, &format!("# Gate\n\n### Does the gizmo hold?\ndepends: gizmo\n\nProblem: the gizmo breaks.\n\nStandard: s.\n\n{}", GATE_OPTIONS));
    plan_record(&dir, REL);
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    let first_move = shown.lines().find(|l| l.contains(" runs: ")).unwrap_or_default();
    assert_eq!(first_move, "owner runs: adr approve adrdom/terms.md", "{}", shown);
    for args in [vec!["show"], vec!["show", REL], vec!["plan", REL], vec!["check"]] {
        let text = all_of(&dir, &args);
        assert!(!text.contains("which is no term") && !text.contains("uses no term"), "{:?}:\n{}", args, text);
        assert!(!text.contains("owner runs: adr approve adrdom/decisions/gate.md"), "{:?}:\n{}", args, text);
    }
    let plan = all_of(&dir, &["plan", REL]);
    assert!(plan.contains("owner runs: adr approve adrdom/terms.md"), "{}", plan);
}

// adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
//
// A banned word in a term's definition: approve refuses the terms file on
// it, so check names no owner approve of the terms file beside it, and the
// hit names the line the file holds whatever entries wait for an id.
#[test]
fn a_banned_word_in_the_terms_file_holds_its_approve_and_names_its_line() {
    let dir = repo();
    let out = adr(&dir, &["ban"], &[], "zorblax\n");
    assert!(out.status.success(), "{}", stderr(&out));
    run_git(&dir, &["add", "-A"]);
    assert!(run_git(&dir, &["commit", "-q", "-m", "base"]).status.success());
    let terms = dir.join("adrdom/terms.md");
    let mut t = std::fs::read_to_string(&terms).unwrap();
    // An entry with no id yet ahead of the hit: minting it would move the
    // lines below it.
    t.push_str("\n## gizmo\ndefinition: A fixture term for tests.\naliases: \n");
    t.push_str("\n## sprocket\ndefinition: A zorblax part.\naliases: \n");
    std::fs::write(&terms, &t).unwrap();
    let line = t.lines().position(|l| l.contains("zorblax")).unwrap() + 1;

    let checked = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(checked.contains(&format!("adrdom/terms.md:{}", line)), "{}", checked);
    assert!(!checked.contains("owner runs: adr approve adrdom/terms.md"), "{}", checked);
    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert!(!shown.contains("owner runs: adr approve adrdom/terms.md"), "{}", shown);
    let out = adr(&dir, &["approve", terms.to_str().unwrap()], &[("ADR_OWNER_BYPASS", "1")], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains(&format!("adrdom/terms.md:{}:", line)), "{}", stderr(&out));
}

// adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
//
// A hand-written answer line under an approved question never asked: check
// fails it, so ask refuses it with the same finding.
#[test]
fn ask_refuses_a_hand_written_answer_line_check_fails() {
    let dir = repo();
    let rec = write_record(&dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks.")));
    approve_record(&dir, REL);
    let text = std::fs::read_to_string(&rec).unwrap();
    std::fs::write(&rec, format!("{}\n**Answer:** A\n", text.trim_end())).unwrap();
    let checked = stdout(&adr(&dir, &["check"], &[], ""));
    let finding = checked.lines().find(|l| l.contains("answer line without ledger row")).unwrap_or_else(|| panic!("{}", checked));
    let out = adr(&dir, &["ask", rec.to_str().unwrap()], &[], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    let message = finding;
    assert!(stderr(&out).contains(message), "{}", stderr(&out));
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// approve takes the files show takes: a path outside adrdom/decisions,
// adrdom/terms.md or adrdom/blame is refused, nothing written; the same
// topic file named `./…` or absolute is approved as its root-relative path.
#[test]
fn approve_takes_only_the_files_show_takes_and_reads_any_spelling_of_one() {
    let dir = repo();
    let before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    write_record(&dir, "docs/readme.md", "# Readme\n");
    let out = adr(&dir, &["approve", "docs/readme.md"], &[("ADR_OWNER_BYPASS", "1")], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("docs/readme.md is not a file approve takes"), "{}", stderr(&out));
    assert_eq!(std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap(), before);

    write_record(&dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks.")));
    plan_record(&dir, REL);
    let out = adr(&dir, &["approve", &format!("./{}", REL)], &[("ADR_OWNER_BYPASS", "1")], "");
    assert!(out.status.success(), "{}", stderr(&out));
    let shown = stdout(&adr(&dir, &["show", REL], &[], ""));
    assert!(shown.starts_with("[approved] "), "{}", shown);
    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(!ledger.contains("./adrdom"), "{}", ledger);
}

// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// An answer the ledger cannot take leaves the topic file as it was, so the
// command the refusal names runs as printed and clears it.
#[cfg(unix)]
#[test]
fn an_answer_the_ledger_refuses_is_not_half_written() {
    use std::os::unix::fs::PermissionsExt;
    let dir = repo();
    let rec = asked_gate(&dir);
    let id = extract_ids(&std::fs::read_to_string(&rec).unwrap()).remove(0);
    let ledger = dir.join("adrdom/state/ledger.tsv");
    std::fs::set_permissions(&ledger, std::fs::Permissions::from_mode(0o444)).unwrap();
    if std::fs::OpenOptions::new().append(true).open(&ledger).is_ok() {
        // Run as a user permissions do not hold (root): nothing to show.
        std::fs::set_permissions(&ledger, std::fs::Permissions::from_mode(0o644)).unwrap();
        return;
    }
    let before = std::fs::read_to_string(&rec).unwrap();
    let id_arg = id_at_version(&dir, &id);
    let out = adr_owner(&dir, &["answer", &id_arg, "A"], "");
    std::fs::set_permissions(&ledger, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(&rec).unwrap(), before, "the topic file is put back");
    let err = stderr(&out);
    let command = format!("owner runs: adr answer {} A", id_arg);
    assert!(err.trim_end().ends_with(&command), "{}", err);
    let again = adr_owner(&dir, &["answer", &id_arg, "A"], "");
    assert!(again.status.success(), "the named command clears it: {}", stderr(&again));
}

// adr: fe14991d-9715-4ae0-9a23-d314a950d3b9
//
// At a commit, the planning records judged are the ones the commit
// carries: a draft staged without its reader rows is told it has none,
// whatever the working tree holds.
#[test]
fn check_at_a_commit_reads_the_staged_reader_rows() {
    let dir = repo();
    run_git(&dir, &["add", "-A"]);
    assert!(run_git(&dir, &["commit", "-q", "-m", "base"]).status.success());
    write_record(&dir, REL, &format!("# Gate\n\n{}", q("Does the widget hold?", "the widget breaks.")));
    plan_record(&dir, REL);
    run_git(&dir, &["add", REL]);
    let staged = stdout(&adr(&dir, &["check", "--staged"], &[], ""));
    assert!(staged.contains("has no reader rows"), "{}", staged);
    let plain = stdout(&adr(&dir, &["check"], &[], ""));
    assert!(!plain.contains("has no reader rows"), "{}", plain);
}
