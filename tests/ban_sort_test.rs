mod common;

use adr::{banned, record};
use common::*;
use std::path::Path;

fn banned_lines(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("adrdom/state/banned.sha256"))
        .unwrap()
        .lines()
        .map(|s| s.to_string())
        .collect()
}

fn assert_sorted(lines: &[String]) {
    let mut sorted = lines.to_vec();
    sorted.sort();
    assert_eq!(
        lines,
        sorted.as_slice(),
        "banned.sha256 is not sorted by hash: {:?}",
        lines
    );
}

// "Does the tool write the ban list in one fixed order?": ban writes the
// whole list sorted by hash, not an append at the end.
#[test]
fn ban_writes_the_file_sorted_by_hash_after_each_call() {
    // adr: bf27b020-4e60-48e0-8d3b-5f818e6bf6d8
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    // Chosen so their sha256 hashes (checked offline with sha256sum) sort
    // in a different order than they are banned in: "wordthree"'s hash is
    // lowest of the three, so banning "wordone", "wordtwo", "wordthree" in
    // that order means the last ban has to land its hash ahead of the two
    // already in the file, not just append after them.
    let words = ["wordone", "wordtwo", "wordthree"];
    for w in words {
        let out = adr(&dir, &["ban"], &[], &format!("{}\n", w));
        assert!(out.status.success(), "ban {} failed: {}", w, stderr(&out));
        assert_sorted(&banned_lines(&dir));
    }

    let lines = banned_lines(&dir);
    assert_eq!(lines.len(), 3, "{:?}", lines);
    // No duplicate hash lines.
    let mut hashes: Vec<&str> = lines.iter().map(|l| l.split(' ').next().unwrap()).collect();
    hashes.sort();
    hashes.dedup();
    assert_eq!(hashes.len(), 3, "{:?}", lines);
}

// The same rule holds after `unban` removes an entry: the file it rewrites
// is still sorted, since both commands now share one writer.
#[test]
fn file_stays_sorted_after_an_unban() {
    // adr: bf27b020-4e60-48e0-8d3b-5f818e6bf6d8
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    let out = adr(&dir, &["ban"], &[], "wordone\nwordtwo\nwordthree\n");
    assert!(out.status.success(), "ban failed: {}", stderr(&out));
    assert_sorted(&banned_lines(&dir));

    // "wordone" sorts in the middle of the three (see the sibling test's
    // comment); removing it forces a real re-sort of what remains, not
    // just dropping the last line.
    let unban_out = adr_owner(&dir, &["unban"], "wordone\n");
    assert!(
        unban_out.status.success(),
        "unban failed: {}",
        stderr(&unban_out)
    );
    let lines = banned_lines(&dir);
    assert_eq!(lines.len(), 2, "{:?}", lines);
    assert_sorted(&lines);
}

// --- (c): two branches, each banning a different word, merge cleanly on
// banned.sha256 ------------------------------------------------------------

/// Strips conflict markers, keeping the "ours" block then the "theirs"
/// block for every conflicted hunk -- the same de-marking policy
/// merge_lab_ban_test.rs's `resolve_text_ours_then_theirs` uses, copied
/// here rather than shared across test binaries (integration tests each
/// compile as their own crate).
fn resolve_ours_then_theirs(content: &str) -> String {
    let mut out = String::new();
    let mut in_ours = false;
    let mut in_theirs = false;
    let mut ours_buf: Vec<&str> = Vec::new();
    let mut theirs_buf: Vec<&str> = Vec::new();
    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_ours = true;
            in_theirs = false;
            ours_buf.clear();
            theirs_buf.clear();
            continue;
        }
        if line.starts_with("=======") && in_ours {
            in_ours = false;
            in_theirs = true;
            continue;
        }
        if line.starts_with(">>>>>>>") && in_theirs {
            in_theirs = false;
            for l in &ours_buf {
                out.push_str(l);
                out.push('\n');
            }
            for l in &theirs_buf {
                out.push_str(l);
                out.push('\n');
            }
            continue;
        }
        if in_ours {
            ours_buf.push(line);
            continue;
        }
        if in_theirs {
            theirs_buf.push(line);
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn current_branch(dir: &Path) -> String {
    stdout(&run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"]))
        .trim()
        .to_string()
}

#[test]
fn ban_on_two_branches_merges_banned_sha256_with_no_conflict() {
    // adr: bf27b020-4e60-48e0-8d3b-5f818e6bf6d8
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());

    // Ten base words, all banned in the base commit. Branch "a" then bans
    // "mergeeight" and branch "b" bans "mergefour": their sha256 hashes
    // (checked offline with sha256sum) sort into the ten-word file at
    // different points -- "mergeeight" between "basewordfour" and
    // "basewordeight" (after the file's 4th sorted line), "mergefour"
    // between "basewordsix" and "basewordone" (after the 7th) -- three
    // base lines apart, so each branch's one-line insertion sits in a
    // different, non-adjacent part of the sorted file and git's merge
    // never has to reconcile the same region twice.
    let base_words = "basewordone\nbasewordtwo\nbasewordthree\nbasewordfour\n\
                       basewordfive\nbasewordsix\nbasewordseven\nbasewordeight\n\
                       basewordnine\nbasewordten\n";
    let base_out = adr(&dir, &["ban"], &[], base_words);
    assert!(
        base_out.status.success(),
        "base ban failed: {}",
        stderr(&base_out)
    );
    run_git(&dir, &["add", "-A"]);
    let commit_out = run_git(&dir, &["commit", "-q", "-m", "base: ban ten words"]);
    assert!(
        commit_out.status.success(),
        "base commit failed: {}",
        stderr(&commit_out)
    );
    let base_branch = current_branch(&dir);

    run_git(&dir, &["checkout", "-b", "a"]);
    let a_out = adr(&dir, &["ban"], &[], "mergeeight\n");
    assert!(a_out.status.success(), "a ban failed: {}", stderr(&a_out));
    run_git(&dir, &["add", "-A"]);
    let a_commit = run_git(&dir, &["commit", "-q", "-m", "a: ban a new word"]);
    assert!(
        a_commit.status.success(),
        "a commit failed: {}",
        stderr(&a_commit)
    );

    run_git(&dir, &["checkout", &base_branch]);
    run_git(&dir, &["checkout", "-b", "b"]);
    let b_out = adr(&dir, &["ban"], &[], "mergefour\n");
    assert!(b_out.status.success(), "b ban failed: {}", stderr(&b_out));
    run_git(&dir, &["add", "-A"]);
    let b_commit = run_git(&dir, &["commit", "-q", "-m", "b: ban another new word"]);
    assert!(
        b_commit.status.success(),
        "b commit failed: {}",
        stderr(&b_commit)
    );

    run_git(&dir, &["checkout", "a"]);
    let merge_out = run_git(&dir, &["merge", "--no-ff", "--no-edit", "b"]);

    let conflicted = stdout(&run_git(&dir, &["diff", "--name-only", "--diff-filter=U"]));
    assert!(
        !conflicted
            .lines()
            .any(|l| l == "adrdom/state/banned.sha256"),
        "banned.sha256 must merge with no conflict: conflicted files = {:?}, merge stderr = {}",
        conflicted,
        stderr(&merge_out)
    );

    // Whether the merge conflicts on anything else (e.g. ledger.tsv, whose
    // rows this test does not care about) is not what this test is
    // about -- `adrdom/.gitattributes` (`merge=union`, committed as part of
    // the shared base before "a" and "b" diverge) may auto-resolve it
    // cleanly. Accept either outcome: if the merge already succeeded,
    // there is nothing left to commit; if it left a conflict, resolve it
    // by keeping both sides' rows, then finish the merge commit.
    if !merge_out.status.success() {
        let ledger_path = dir.join("adrdom/state/ledger.tsv");
        let content = std::fs::read_to_string(&ledger_path).unwrap();
        if content.contains("<<<<<<<") {
            let resolved = resolve_ours_then_theirs(&content);
            std::fs::write(&ledger_path, resolved).unwrap();
            run_git(&dir, &["add", "adrdom/state/ledger.tsv"]);
        }
        let commit_out = run_git(&dir, &["commit", "--no-edit"]);
        assert!(
            commit_out.status.success(),
            "merge commit failed: {}",
            stderr(&commit_out)
        );
    }

    let lines = banned_lines(&dir);
    assert_eq!(
        lines.len(),
        12,
        "expected 10 base + 2 merged words: {:?}",
        lines
    );
    assert_sorted(&lines);

    let hash_a = record::sha256_hex("mergeeight");
    let hash_b = record::sha256_hex("mergefour");
    assert!(
        lines.iter().any(|l| l.starts_with(&hash_a)),
        "missing mergeeight's hash: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|l| l.starts_with(&hash_b)),
        "missing mergefour's hash: {:?}",
        lines
    );
}

// Two entries that share one hash but carry different term uuids (the shape
// a merge of two `--term` bans of the same word can produce): whichever
// order they arrive in, `write_sorted` keeps the same one.
#[test]
fn same_hash_two_uuids_keeps_the_same_one_regardless_of_input_order() {
    // adr: bf27b020-4e60-48e0-8d3b-5f818e6bf6d8
    let dir = init_repo();
    let same_hash = record::sha256_hex("sharedword");
    let entry_none: (String, Option<String>) = (same_hash.clone(), None);
    let entry_uuid: (String, Option<String>) = (
        same_hash.clone(),
        Some("11111111-1111-1111-1111-111111111111".to_string()),
    );

    let path_a = dir.join("order-a.sha256");
    banned::write_sorted(&path_a, vec![entry_none.clone(), entry_uuid.clone()]).unwrap();
    let content_a = std::fs::read_to_string(&path_a).unwrap();

    let path_b = dir.join("order-b.sha256");
    banned::write_sorted(&path_b, vec![entry_uuid, entry_none]).unwrap();
    let content_b = std::fs::read_to_string(&path_b).unwrap();

    assert_eq!(
        content_a, content_b,
        "write_sorted's result depends on input order"
    );
    // Pinned to the actual rule (uuid absent sorts first, so the no-uuid
    // entry survives the dedup): if this ever needs to change, the test
    // should change with it, not just the equality check above.
    assert_eq!(content_a, format!("{}\n", same_hash));
}
