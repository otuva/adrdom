// The embedding area of the placement plan: the pool and its ten nearest, the
// ignore file for adrdom/cache, and the model needed by planning only.

mod common;

use common::*;

const IGNORE_PATH: &str = "adrdom/.gitignore";

const RECORD: &str = r#"# Example

### What does the hook run?
depends: reasons

Problem: whether the hook runs them.

Standard: clear yes or no.

- A) The hook runs every test.
  Why: reasons.
  Costs: costs.
- B) No.
  Why: other reasons.
  Costs: other costs.

### What does the ledger hold?
depends: reasons

Problem: whether it holds rows.

Standard: clear yes or no.

- A) The ledger holds one row for each event.
  Why: reasons.
  Costs: costs.
- B) No.
  Why: other reasons.
  Costs: other costs.

### Is this only asked?
depends: reasons

Problem: whether it is only asked.

Standard: clear yes or no.

- A) Yes only asked.
  Why: reasons.
  Costs: costs.
- B) No.
  Why: other reasons.
  Costs: other costs.
"#;

/// A repo with twelve terms (one of them retired), and three questions of
/// which two are answered A and one is only asked.
fn pool_repo() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let words: Vec<String> = std::iter::once("reasons".to_string())
        .chain((0..11).map(|i| format!("widget{}", i)))
        .collect();
    let refs: Vec<&str> = words.iter().map(String::as_str).collect();
    approve_terms_with(&dir, &refs);
    let record = write_record(&dir, "adrdom/decisions/example.md", RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    for id in &ids[..2] {
        let out = adr_answer(&dir, id, "A");
        assert!(out.status.success(), "{}", stderr(&out));
    }
    dir
}

#[cfg(feature = "fake-model")]
mod pool {
    use super::*;
    use adr::{index, ledger};

    // adr: de8ef568-3a8d-42ba-bef7-d7fd467b27be
    #[test]
    fn the_pool_is_current_decisions_and_live_terms_and_the_nearest_ten_come_back() {
        std::env::set_var("ADR_FAKE_MODEL", "1");
        let dir = pool_repo();
        let led = ledger::read(&dir.join("adrdom/state/ledger.tsv"));
        let q = vec![
            "does the hook run every test".to_string(),
            "widget3: a fixture term".to_string(),
        ];
        let out = index::nearest(&dir, &led, &q, index::NEIGHBOURS).unwrap();
        assert_eq!(index::NEIGHBOURS, 10);
        assert_eq!(out.len(), 2);
        for n in &out {
            // 12 terms and 2 current decisions are in the pool: ten come back.
            assert_eq!(n.len(), 10);
            assert!(n.windows(2).all(|w| w[0].score >= w[1].score));
            // The question that is only asked is not in the pool.
            assert!(n.iter().all(|x| !x.text.contains("only asked")));
        }
        // The nearest of a decision's own words is that decision, with its
        // heading and picked option as the embedded text.
        assert_eq!(out[0][0].kind, "decision");
        assert_eq!(
            out[0][0].text,
            "What does the hook run?\nThe hook runs every test."
        );
        // A term is embedded as `name: definition`.
        assert_eq!(out[1][0].kind, "term");
        assert_eq!(out[1][0].text, "widget3: A fixture term for tests.");

        // The index is written under the cache, with its header and one row
        // for each record of the pool.
        let idx = std::fs::read_to_string(dir.join(index::INDEX_PATH)).unwrap();
        assert!(idx.starts_with("# model fake-bag-of-words dim 256\n"));
        assert_eq!(idx.lines().count(), 1 + 12 + 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // adr: de8ef568-3a8d-42ba-bef7-d7fd467b27be
    #[test]
    fn a_retired_term_leaves_the_pool() {
        std::env::set_var("ADR_FAKE_MODEL", "1");
        let dir = pool_repo();
        let terms = std::fs::read_to_string(dir.join("adrdom/terms.md")).unwrap();
        let led = ledger::read(&dir.join("adrdom/state/ledger.tsv"));
        let before = index::nearest(&dir, &led, &["widget4".to_string()], 100).unwrap();
        assert_eq!(before[0].len(), 14);
        // Retire widget4 in the terms file, as an approved edit would leave it.
        let edited = terms.replacen(
            "## widget4\nid: 99999995-9999-9999-9999-999999999999\n",
            "## widget4\nid: 99999995-9999-9999-9999-999999999999\nretired: yes\n",
            1,
        );
        assert_ne!(edited, terms);
        std::fs::write(dir.join("adrdom/terms.md"), edited).unwrap();
        let after = index::nearest(&dir, &led, &["widget4".to_string()], 100).unwrap();
        assert_eq!(after[0].len(), 13);
        assert!(after[0].iter().all(|x| !x.text.starts_with("widget4:")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // adr: de8ef568-3a8d-42ba-bef7-d7fd467b27be
    #[test]
    fn check_runs_in_a_pool_repo_with_the_model_absent() {
        // The model is absent only in a child process: the variable is not set here.
        let dir = pool_repo();
        let out = adr(&dir, &["check"], &[("ADR_NO_MODEL", "1")], "");
        assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

// adr: 0d977e39-a657-4596-8eca-8ceb335f3081
#[test]
fn install_writes_the_ignore_file_and_check_holds_it() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert_eq!(std::fs::read_to_string(dir.join(IGNORE_PATH)).unwrap(), "cache/\nstate/roles.tsv\n");
    // A second install leaves it as it is.
    let again = adr(&dir, &["install"], &[], "");
    assert!(!stdout(&again).contains("adrdom/.gitignore"), "{}", stdout(&again));
    assert!(adr(&dir, &["check"], &[], "").status.success());

    // Missing fails, and install clears it.
    std::fs::remove_file(dir.join(IGNORE_PATH)).unwrap();
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("adrdom/.gitignore is missing; the adrdom gitignore equals what install writes; agent runs: adr install"),
        "{}",
        stdout(&out)
    );
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert!(adr(&dir, &["check"], &[], "").status.success());

    // Different fails, and install writes it back.
    std::fs::write(dir.join(IGNORE_PATH), "cache/\nother/\n").unwrap();
    let out = adr(&dir, &["check"], &[], "");
    assert!(!out.status.success());
    assert!(
        stdout(&out).contains("adrdom/.gitignore differs from what install writes; the adrdom gitignore equals what install writes; agent runs: adr install"),
        "{}",
        stdout(&out)
    );
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert_eq!(std::fs::read_to_string(dir.join(IGNORE_PATH)).unwrap(), "cache/\nstate/roles.tsv\n");
    assert!(adr(&dir, &["check"], &[], "").status.success());

    // Git ignores what the tool writes under the cache.
    std::fs::create_dir_all(dir.join("adrdom/cache")).unwrap();
    std::fs::write(dir.join("adrdom/cache/index.tsv"), "x").unwrap();
    let st = stdout(&run_git(&dir, &["status", "--porcelain", "--untracked-files=all"]));
    assert!(!st.contains("adrdom/cache"), "{}", st);
    std::fs::remove_dir_all(&dir).unwrap();
}

// adr: 21cdad59-4f0c-42a5-b4f3-6af946637df3
#[cfg(feature = "fake-model")]
#[test]
fn every_command_but_planning_runs_without_the_model() {
    let dir = init_repo();
    let no_model = [("ADR_NO_MODEL", "1")];
    assert!(adr(&dir, &["install"], &no_model, "").status.success());
    approve_minimal_terms(&dir, "reasons");
    let record = write_record(&dir, "adrdom/decisions/example.md", RECORD);
    let out = adr_owner(&dir, &["approve", record.to_str().unwrap()], "");
    assert!(out.status.success(), "{}", stderr(&out));
    // Planning made the cache with the fake model; the calls below run with
    // the model absent, and none of them may make it again.
    std::fs::remove_dir_all(dir.join("adrdom/cache")).unwrap();
    // `adr_owner` has no env of its own beyond the bypass, so every call below
    // is run with the model absent by hand.
    let ask = adr(&dir, &["ask", record.to_str().unwrap()], &no_model, "");
    assert!(ask.status.success(), "{}", stderr(&ask));
    let show = adr(&dir, &["show"], &no_model, "");
    assert!(show.status.success(), "{}", stderr(&show));
    let check = adr(&dir, &["check"], &no_model, "");
    assert!(check.status.success(), "{}{}", stdout(&check), stderr(&check));
    // No cache directory was made: nothing but planning touches the model.
    assert!(!dir.join("adrdom/cache").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

// adr: 21cdad59-4f0c-42a5-b4f3-6af946637df3
#[cfg(feature = "fake-model")]
#[test]
fn the_loader_refuses_when_the_model_is_missing() {
    let dir = init_repo();
    std::env::set_var("ADR_NO_MODEL", "1");
    let r = adr::embed::load(&dir);
    std::env::remove_var("ADR_NO_MODEL");
    let e = r.err().expect("the model is absent");
    assert!(e.contains("the embedding model is missing"), "{}", e);
    assert!(e.contains("agent fetches it"), "{}", e);
}

// adr: de8ef568-3a8d-42ba-bef7-d7fd467b27be
// Needs the network the first time (about 100 MB into adrdom/cache/model).
#[test]
#[ignore]
#[cfg(feature = "model")]
fn the_real_model_puts_a_related_text_nearer_than_an_unrelated_one() {
    let dir = init_repo();
    std::env::remove_var("ADR_FAKE_MODEL");
    std::env::remove_var("ADR_NO_MODEL");
    let emb = adr::embed::load(&dir).expect("model loads");
    let v = emb
        .embed(&[
            "Does the pre-commit hook run the tests?".to_string(),
            "The git hook runs every test before a commit.".to_string(),
            "Bananas ripen faster in a paper bag.".to_string(),
        ])
        .unwrap();
    assert!(adr::index::dot(&v[0], &v[1]) > adr::index::dot(&v[0], &v[2]));
}
