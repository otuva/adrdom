// An answer removes the questions whose answer set lacks it, and writes one
// `dropped` row for each; `check` verifies each row from the record.
//
// adr: 462e22c6-8b23-4061-9a2a-bbf8691fce60 "Does answer remove a question
// whose answer set lacks the answer?"
// adr: f5c242b6-e2f6-45a3-8c61-c743f3ad794f "What does answer write for a
// removed question?"

mod common;

use common::*;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/gate.md";
const ONE: &str = "Is the widget gate open?";
const TWO: &str = "Is the widget gate close?";
const THREE: &str = "Is the widget gate lock once it closes?";
const FOUR: &str = "Is the widget gate sealed?";

const BODY: &str = "\nProblem: the widget breaks.\n\nStandard: plain words.\n\n\
- A) Yes the widget holds.\n  Why: reasons.\n  Costs: costs.\n\
- B) No the widget does not hold.\n  Why: other reasons.\n  Costs: other costs.\n";

fn q(heading: &str, depends: &str) -> String {
    format!("### {}\ndepends: {}\n{}\n", heading, depends, BODY)
}

fn id_of(path: &Path, heading: &str) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    let at = text.find(&format!("### {}\n", heading)).unwrap();
    extract_ids(&text[at..])[0].clone()
}

fn ledger_rows(dir: &Path, event: &str) -> Vec<Vec<String>> {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv"))
        .unwrap()
        .lines()
        .map(|l| l.split('\t').map(|s| s.to_string()).collect::<Vec<_>>())
        .filter(|f| f[1] == event)
        .collect()
}

fn commit(dir: &Path, message: &str) {
    run_git(dir, &["add", "-A"]);
    let out = run_git(dir, &["commit", "-q", "-m", message]);
    assert!(
        out.status.success(),
        "commit failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

struct Gate {
    dir: PathBuf,
    path: PathBuf,
    one: String,
    two: String,
    three: String,
}

/// One answers B (root); Two stands on One=A; Three stands on Two=*; Four
/// stands on One=*. Approved, every question asked in one run and
/// committed: what an answer removes was asked.
fn gate() -> Gate {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let plan = format!(
        "# Gate\n\n{}{}{}{}",
        q(ONE, "widget"),
        q(TWO, &format!("widget, {}=A", ONE)),
        q(THREE, &format!("widget, {}=*", TWO)),
        q(FOUR, &format!("widget, {}=*", ONE))
    );
    let path = write_record(&dir, REL, &plan);
    approve_record(&dir, REL);
    let g = Gate {
        one: id_of(&path, ONE),
        two: id_of(&path, TWO),
        three: id_of(&path, THREE),
        dir,
        path,
    };
    commit(&g.dir, "feat: ask the gate");
    g
}

fn check_text(dir: &Path) -> (bool, String) {
    let out = adr(dir, &["check"], &[], "");
    (out.status.success(), stdout(&out) + &stderr(&out))
}

fn ledger_path(dir: &Path) -> PathBuf {
    dir.join("adrdom/state/ledger.tsv")
}

#[test]
fn answer_removes_the_question_and_its_dependents_and_the_file_stays_approved() {
    // adr: 462e22c6-8b23-4061-9a2a-bbf8691fce60
    let g = gate();
    let out = adr_answer(&g.dir, &g.one, "B");
    assert!(out.status.success(), "{}", stderr(&out));

    let text = std::fs::read_to_string(&g.path).unwrap();
    assert!(text.contains(&format!("### {}\n", ONE)), "{}", text);
    assert!(!text.contains(&format!("### {}\n", TWO)), "{}", text);
    assert!(!text.contains(&format!("### {}\n", THREE)), "{}", text);
    // An entry with `=*` is never dropped.
    assert!(text.contains(&format!("### {}\n", FOUR)), "{}", text);

    // No new approve: check passes and does not count the file unapproved.
    let (ok, report) = check_text(&g.dir);
    assert!(ok, "{}", report);
    assert!(!report.contains("awaiting approval"), "{}", report);

    // Four was asked with the others in the first approve; nothing is left to ask.
    let asked_before = asked_row_count(&g.dir);
    let asked = adr_owner(&g.dir, &["approve", g.path.to_str().unwrap()], "");
    assert!(asked.status.success(), "{}", stderr(&asked));
    assert!(!stdout(&asked).contains("asked:"), "{}", stdout(&asked));
    assert_eq!(asked_row_count(&g.dir), asked_before);
}

// adr: 462e22c6-8b23-4061-9a2a-bbf8691fce60
// adr: f5c242b6-e2f6-45a3-8c61-c743f3ad794f
//
// What an answer removes was asked: each removed question has an `asked`
// row and, after the answer, a `dropped` row that ends it. `check` passes
// with the removed ids missing from every file, a later approve of the
// file writes no `withdrawn` row for them, and the commit passes its hook.
#[test]
fn an_asked_question_is_removed_and_check_and_approve_leave_it_gone() {
    let g = gate();
    let asked_ids: Vec<String> = ledger_rows(&g.dir, "asked")
        .into_iter()
        .map(|r| r[2].clone())
        .collect();
    for id in [&g.two, &g.three] {
        assert!(asked_ids.contains(id), "{} was asked: {:?}", id, asked_ids);
    }
    assert!(adr_answer(&g.dir, &g.one, "B").status.success());

    let text = std::fs::read_to_string(&g.path).unwrap();
    assert!(
        !text.contains(&format!("### {}\n", TWO)) && !text.contains(&format!("### {}\n", THREE)),
        "{}",
        text
    );
    let dropped = ledger_rows(&g.dir, "dropped");
    assert_eq!(dropped.len(), 2, "{:?}", dropped);
    let (ok, report) = check_text(&g.dir);
    assert!(ok, "{}", report);

    approve_record(&g.dir, REL);
    assert!(
        ledger_rows(&g.dir, "withdrawn").is_empty(),
        "a dropped id is not withdrawn"
    );
    commit(&g.dir, "feat: answer the gate");
    let (ok, report) = check_text(&g.dir);
    assert!(ok, "{}", report);
}

#[test]
fn a_refusal_removes_nothing() {
    // adr: 462e22c6-8b23-4061-9a2a-bbf8691fce60
    let g = gate();
    let out = adr_answer(&g.dir, &g.one, "refuse");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = std::fs::read_to_string(&g.path).unwrap();
    assert!(text.contains(&format!("### {}\n", TWO)), "{}", text);
    assert!(ledger_rows(&g.dir, "dropped").is_empty());
}

#[test]
fn a_letter_the_set_holds_removes_nothing() {
    // adr: 462e22c6-8b23-4061-9a2a-bbf8691fce60
    let g = gate();
    let out = adr_answer(&g.dir, &g.one, "A");
    assert!(out.status.success(), "{}", stderr(&out));
    let text = std::fs::read_to_string(&g.path).unwrap();
    assert!(text.contains(&format!("### {}\n", TWO)), "{}", text);
    assert!(ledger_rows(&g.dir, "dropped").is_empty());
}

#[test]
fn answer_writes_a_dropped_row_for_each_removed_question() {
    // adr: f5c242b6-e2f6-45a3-8c61-c743f3ad794f
    let g = gate();
    let approved = ledger_rows(&g.dir, "approved");
    let hash_of = |id: &str| approved.iter().find(|r| r[2] == id).unwrap()[3].clone();
    let (two_hash, three_hash) = (hash_of(&g.two), hash_of(&g.three));
    assert!(adr_answer(&g.dir, &g.one, "B").status.success());

    let dropped = ledger_rows(&g.dir, "dropped");
    assert_eq!(dropped.len(), 2, "{:?}", dropped);
    let row = |id: &str| {
        dropped
            .iter()
            .find(|r| r[2] == id)
            .unwrap_or_else(|| panic!("{:?}", dropped))
            .clone()
    };
    // The question that stood on the answered one: the hash from approval,
    // the answered identifier and the letter.
    assert_eq!(row(&g.two)[3], two_hash);
    assert_eq!(row(&g.two)[4], format!("{} B", g.one));
    // The dependent: the identifier of the question it stood on.
    assert_eq!(row(&g.three)[3], three_hash);
    assert_eq!(row(&g.three)[4], g.two);
}

#[test]
fn check_verifies_each_dropped_row() {
    // adr: f5c242b6-e2f6-45a3-8c61-c743f3ad794f
    let g = gate();
    assert!(adr_answer(&g.dir, &g.one, "B").status.success());

    // At the dropping commit, HEAD still holds the sections: they match.
    let (ok, report) = check_text(&g.dir);
    assert!(ok, "{}", report);
    let good_ledger = std::fs::read_to_string(ledger_path(&g.dir)).unwrap();
    let good_file = std::fs::read_to_string(&g.path).unwrap();
    let dropped = ledger_rows(&g.dir, "dropped");
    let two_row = dropped.iter().find(|r| r[2] == g.two).unwrap().join("\t");
    let three_row = dropped.iter().find(|r| r[2] == g.three).unwrap().join("\t");
    let fail_with = |ledger: String, file: Option<&str>, needle: &str| {
        std::fs::write(ledger_path(&g.dir), ledger).unwrap();
        if let Some(f) = file {
            std::fs::write(&g.path, f).unwrap();
        }
        let (ok, report) = check_text(&g.dir);
        assert!(
            !ok && report.contains(needle),
            "wanted {:?}: {}",
            needle,
            report
        );
        std::fs::write(ledger_path(&g.dir), &good_ledger).unwrap();
        std::fs::write(&g.path, &good_file).unwrap();
    };

    // A letter no answer row carries.
    let wrong_letter = two_row.replace(&format!("{} B", g.one), &format!("{} A", g.one));
    fail_with(
        good_ledger.replace(&two_row, &wrong_letter),
        None,
        "has no cause",
    );
    // HEAD's set holds the letter the row names.
    fail_with(
        good_ledger.replace(&two_row, &wrong_letter),
        None,
        "which the set at HEAD holds",
    );
    // A dependent naming a question with no dropped row.
    let other = "00000000-0000-4000-8000-000000000000";
    let no_cause = three_row.replace(&g.two, other);
    fail_with(
        good_ledger.replace(&three_row, &no_cause),
        None,
        "has no cause",
    );
    // A hash no approval row holds.
    let bad_hash = two_row.replace(
        &two_row.split('\t').nth(3).unwrap().to_string(),
        &"0".repeat(64),
    );
    fail_with(
        good_ledger.replace(&two_row, &bad_hash),
        None,
        "matches no approved row",
    );
    // A detail of no known shape.
    let bad_detail = two_row.replace(&format!("{} B", g.one), "nonsense");
    fail_with(
        good_ledger.replace(&two_row, &bad_detail),
        None,
        "has the detail",
    );
    // An id still in a topic file.
    let head_file =
        String::from_utf8_lossy(&run_git(&g.dir, &["show", &format!("HEAD:{}", REL)]).stdout)
            .to_string();
    fail_with(good_ledger.clone(), Some(&head_file), "is still in");
}

#[test]
fn the_dropping_commit_passes_its_hook_and_afterwards_head_holds_nothing_to_verify() {
    // adr: f5c242b6-e2f6-45a3-8c61-c743f3ad794f
    let g = gate();
    assert!(adr_answer(&g.dir, &g.one, "B").status.success());
    let (ok, report) = check_text(&g.dir);
    assert!(ok, "{}", report);

    // The commit that carries the drop passes the hook, and afterwards
    // HEAD holds no section to verify.
    commit(&g.dir, "feat: answer the gate");
    let (ok, report) = check_text(&g.dir);
    assert!(ok, "{}", report);
}

// adr: 61e73795-63de-4669-b09f-48be96919325
//
// "approve hashes each question on its own": only a question dropped after
// the latest approval of the file is put back into its hash. A question
// dropped before a later approval stays out, so the file does not await
// approval again.
#[test]
fn a_drop_before_the_latest_approval_is_not_put_back() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let q3 = "Is the widget gate shut?";
    let plan = format!(
        "# Gate\n\n{}{}{}",
        q(ONE, "widget"),
        q(TWO, "widget"),
        q(q3, &format!("widget, {}=A", ONE))
    );
    let path = write_record(&dir, REL, &plan);
    approve_record(&dir, REL);
    let one = id_of(&path, ONE);
    let two = id_of(&path, TWO);
    assert!(adr_answer(&dir, &one, "B").status.success());
    assert!(!std::fs::read_to_string(&path).unwrap().contains(q3));

    // Re-approve with a new question that stands on Two=A, then drop it.
    let q4 = "Is the widget gate sealed shut?";
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!(
            "{}\n{}",
            text.trim_end(),
            q(q4, &format!("widget, {}=A", TWO))
        ),
    )
    .unwrap();
    approve_record(&dir, REL);
    assert!(adr_answer(&dir, &two, "B").status.success());
    assert!(!std::fs::read_to_string(&path).unwrap().contains(q4));

    let (ok, report) = check_text(&dir);
    assert!(ok, "{}", report);
    // The file stands at its latest approval: show names it approved, with
    // no owner approve move.
    let shown = stdout(&adr(&dir, &["show", REL], &[], ""));
    assert!(shown.starts_with("[approved] "), "{}", shown);
    assert!(!shown.contains("owner runs: adr approve"), "{}", shown);
}

/// Two questions in a file: Two stands on One=A. Both asked and committed.
fn pair() -> (PathBuf, PathBuf, String, String) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let plan = format!(
        "# Gate\n\n{}{}",
        q(ONE, "widget"),
        q(TWO, &format!("widget, {}=A", ONE))
    );
    let path = write_record(&dir, REL, &plan);
    approve_record(&dir, REL);
    let (one, two) = (id_of(&path, ONE), id_of(&path, TWO));
    commit(&dir, "feat: ask the pair");
    (dir, path, one, two)
}

fn row_hash(dir: &Path, event: &str, id: &str) -> String {
    ledger_rows(dir, event)
        .into_iter()
        .find(|r| r[2] == id)
        .unwrap()[3]
        .clone()
}

// adr: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d
//
// A second standing answer for One (a merge of two branches) is asked
// again; answering it with a letter outside Two's answer set would remove
// Two, which holds an answer: the answer refuses and writes nothing.
#[test]
fn an_answer_never_removes_an_answered_question() {
    let (dir, path, one, two) = pair();
    assert!(adr_answer(&dir, &one, "A").status.success());
    assert!(adr_answer(&dir, &two, "A").status.success());
    let h1 = row_hash(&dir, "answered", &one);
    append_ledger_row_at(&dir, "answered", &one, &h1, "B", "2099-01-01T00:00:00Z");
    let asked = adr_owner(&dir, &["approve", path.to_str().unwrap()], "");
    assert!(asked.status.success(), "{}", stderr(&asked));

    let before = std::fs::read_to_string(&path).unwrap();
    let ledger_before = std::fs::read_to_string(ledger_path(&dir)).unwrap();
    let out = adr_answer(&dir, &one, "B");
    assert!(!out.status.success(), "{}{}", stdout(&out), stderr(&out));
    let err = stderr(&out);
    assert!(err.contains(TWO) && err.contains("answered"), "{}", err);
    assert!(err.contains("agent writes the question"), "{}", err);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    assert_eq!(
        std::fs::read_to_string(ledger_path(&dir)).unwrap(),
        ledger_before
    );
}

// adr: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d
//
// A `dropped` row of a question with an earlier `answered` row fails check.
#[test]
fn check_fails_a_dropped_row_after_an_answered_row_of_the_same_question() {
    let (dir, _path, one, two) = pair();
    let h2 = row_hash(&dir, "asked", &two);
    assert!(adr_answer(&dir, &one, "B").status.success());
    assert_eq!(ledger_rows(&dir, "dropped").len(), 1);
    let (ok, report) = check_text(&dir);
    assert!(ok, "{}", report);
    append_ledger_row_at(&dir, "answered", &two, &h2, "A", "2020-01-01T00:00:00Z");
    let (ok, report) = check_text(&dir);
    assert!(!ok, "{}", report);
    assert!(
        report.contains(&format!("dropped row for {} follows its answered row", two)),
        "{}",
        report
    );
}

// adr: ce19c883-c61d-477b-a6d4-fee5134c6e9a
//
// A `dropped` row counts among the state rows that compete: one more state
// row of the same id at the dropped row's time fails check.
#[test]
fn check_fails_a_dropped_row_and_another_state_row_at_one_time() {
    let (dir, _path, one, two) = pair();
    assert!(adr_answer(&dir, &one, "B").status.success());
    let dropped = ledger_rows(&dir, "dropped")
        .into_iter()
        .find(|r| r[2] == two)
        .unwrap();
    append_ledger_row_at(&dir, "withdrawn", &two, &dropped[3], "", &dropped[0]);
    let (ok, report) = check_text(&dir);
    assert!(!ok, "{}", report);
    assert!(report.contains("compete for"), "{}", report);
}

fn chain(refuse_first: bool) -> (PathBuf, String, String, String) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let plan = format!(
        "# Gate\n\n{}{}{}",
        q(ONE, "widget"),
        q(TWO, &format!("widget, {}=*", ONE)),
        q(THREE, &format!("widget, {}=*", TWO))
    );
    let path = write_record(&dir, REL, &plan);
    approve_record(&dir, REL);
    let ids = (id_of(&path, ONE), id_of(&path, TWO), id_of(&path, THREE));
    if refuse_first {
        assert!(adr_answer(&dir, &ids.0, "refuse").status.success());
    }
    (dir, ids.0, ids.1, ids.2)
}

// adr: da80931c-68cf-4c51-acce-060307bbd973
//
// Answering Three, which stands on Two, which stands on One, names the
// owner's command for One: the first question `answer` takes.
#[test]
fn a_waiting_chain_names_the_first_question_answer_takes() {
    let (dir, one, two, three) = chain(false);
    let out = adr_answer(&dir, &three, "A");
    assert!(!out.status.success());
    let err = stderr(&out);
    let want = format!(
        "owner runs: adr answer {} <letter>",
        id_at_version(&dir, &one)
    );
    assert!(err.contains(&want), "{}", err);
    assert!(
        !err.contains(&format!("adr answer {}", id_at_version(&dir, &two))),
        "{}",
        err
    );
}

// adr: da80931c-68cf-4c51-acce-060307bbd973
//
// When the chain ends on a refused question, the move is the plan change.
#[test]
fn a_waiting_chain_ending_on_a_refused_question_names_the_plan_change() {
    let (dir, _one, _two, three) = chain(true);
    let out = adr_answer(&dir, &three, "A");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains(&format!("\"{}\"", ONE)) && err.contains("refused"),
        "{}",
        err
    );
    assert!(
        err.contains("agent rewrites the plan") && err.contains("adr approve"),
        "{}",
        err
    );
    assert!(!err.contains("adr answer"), "{}", err);
    // The move names the plan change once, though Three both waits on the
    // refused question through Two and sits in a tier above it.
    assert!(
        err.contains("agent rewrites the plan, owner runs"),
        "{}",
        err
    );
}
