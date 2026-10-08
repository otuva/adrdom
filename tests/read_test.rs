// `adr read`: every current decision read against its nearest neighbours,
// the rows it leaves in adrdom/state/reader.tsv and what it prints. The
// embedding model and the reader are the fakes: a `[word:<w>]` marker in a
// decision's heading is the word the fake reader gives for every pair whose
// first decision it is.

mod common;

use adr::reader;
use adr::record;
use common::*;
use std::path::{Path, PathBuf};
use std::process::Output;

const REASON: &str = "The fake reader's reason.";

struct Dec {
    heading: String,
    id: String,
    marker: String,
}

fn body(n: &str) -> String {
    format!(
        "\nProblem: problem {n} of the widget.\n\nStandard: standard {n}.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        n = n
    )
}

/// An installed repository with one term and one current decision per entry
/// of `markers` ("" for an unmarked one), three decisions to a topic file.
fn repo_with(markers: &[&str]) -> (PathBuf, Vec<Dec>) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget"]);
    let mut out: Vec<Dec> = Vec::new();
    let mut files = Vec::new();
    for (f, chunk) in markers.chunks(3).enumerate() {
        let rel = format!("adrdom/decisions/topic-{}.md", "abcdefghij".chars().nth(f).unwrap());
        let mut text = format!("# Topic {}\n", "abcdefghij".chars().nth(f).unwrap());
        let mut headings = Vec::new();
        for (i, m) in chunk.iter().enumerate() {
            let n = f * 3 + i;
            let plain = format!("Does the widget n{} glow?", n);
            text.push_str(&format!("\n### {}\ndepends: widget\n{}", plain, body(&format!("n{}", n))));
            headings.push((plain, m.to_string()));
        }
        let path = write_record(&dir, &rel, &text);
        approve_record(&dir, &rel);
        let a = adr(&dir, &["ask", path.to_str().unwrap()], &[], "");
        assert!(a.status.success(), "{}{}", stdout(&a), stderr(&a));
        let ids = extract_ids(&std::fs::read_to_string(&path).unwrap());
        assert_eq!(ids.len(), headings.len());
        for (id, (h, m)) in ids.iter().zip(headings) {
            let r = adr_answer(&dir, id, "A");
            assert!(r.status.success(), "{}{}", stdout(&r), stderr(&r));
            out.push(Dec { heading: h, id: id.clone(), marker: m });
        }
        files.push((path, ()));
    }
    // The markers go into the answered headings afterwards: the heading and
    // plan readers accept only same, depends and unrelated, so a marker such
    // as `inconsistent` would stop planning and approving; and `adr read`
    // does not hold answered text to its approved hash the way `check` does.
    for (path, _) in &files {
        let mut text = std::fs::read_to_string(path).unwrap();
        for d in out.iter_mut().filter(|d| !d.marker.is_empty()) {
            let marked = d.heading.replace(" glow?", &format!(" [word:{}] glow?", d.marker));
            if text.contains(&format!("### {}\n", d.heading)) {
                text = text.replace(&format!("### {}\n", d.heading), &format!("### {}\n", marked));
                d.heading = marked;
            }
        }
        std::fs::write(path, text).unwrap();
    }
    (dir, out)
}

fn read_with(dir: &Path, envs: &[(&str, &str)]) -> Output {
    adr(dir, &["read"], envs, "")
}

fn read_ok(dir: &Path) -> String {
    let out = read_with(dir, &[]);
    assert!(out.status.success(), "stdout={} stderr={}", stdout(&out), stderr(&out));
    stdout(&out)
}

fn rows(dir: &Path) -> Vec<reader::Row> {
    reader::read_file(dir).rows
}

/// The rows `adr read` wrote: those on the set prompt.
fn set_rows(dir: &Path) -> Vec<reader::Row> {
    let hash = reader::Kind::Set.prompt_hash();
    rows(dir).into_iter().filter(|r| r.prompt_hash == hash).collect()
}

fn text_of(d: &Dec) -> String {
    format!("{}\nYes.", d.heading)
}

fn pairs(decs: &[Dec]) -> usize {
    decs.len() * (decs.len() - 1) / 2
}

/// The first and second decision of an unordered pair: the lower identifier
/// is the first.
fn ordered<'a>(a: &'a Dec, b: &'a Dec) -> (&'a Dec, &'a Dec) {
    if a.id < b.id {
        (a, b)
    } else {
        (b, a)
    }
}

fn line_of(word: &str, first: &Dec, second: &Dec) -> String {
    let mv = if word == "depends" {
        "not yet, nobody moves on a pair that depends"
    } else {
        "agent writes a question that supersedes one of the pair"
    };
    let protects = if word == "depends" {
        "decisions that stand on each other are named"
    } else {
        "decisions that repeat or contradict are named"
    };
    format!("{}: \"{}\" and \"{}\": {}; {}; {}", word, first.heading, second.heading, REASON, protects, mv)
}

/// The move a same or inconsistent pair calls for, as `adr read` prints it.
const SUPERSEDE_MOVE: &str = "agent writes a question that supersedes one decision of each same or inconsistent pair";

/// The word the fake reader gives a pair: the marker of its first decision.
fn word_of(a: &Dec, b: &Dec) -> String {
    let (first, _) = ordered(a, b);
    if first.marker.is_empty() { "unrelated".into() } else { first.marker.clone() }
}

// adr: 1ab6d6c3-7e44-42de-bc2f-2e5849114aef
#[test]
fn every_pair_of_current_decisions_is_read_once() {
    let (dir, decs) = repo_with(&["", "", "", ""]);
    let before = rows(&dir).len();
    let out = read_ok(&dir);
    let n = pairs(&decs);
    assert_eq!(n, 6);
    // Nothing marked: nothing printed, no count (848e1b76 A).
    assert_eq!(out, "", "a reading that marks nothing prints nothing");
    let all = rows(&dir);
    assert_eq!(all.len(), before + n);
    let set = set_rows(&dir);
    assert_eq!(set.len(), n, "{:?}", all);
    for i in 0..decs.len() {
        for j in i + 1..decs.len() {
            let (f, s) = ordered(&decs[i], &decs[j]);
            let hash = record::sha256_hex(&text_of(f));
            assert_eq!(
                set.iter().filter(|r| r.text_hash == hash && r.neighbour == s.id).count(),
                1,
                "pair {} {}",
                f.heading,
                s.heading
            );
        }
    }
    let len = rows(&dir).len();
    let again = read_ok(&dir);
    assert_eq!(again, "", "{}", again);
    assert_eq!(rows(&dir).len(), len, "a second run adds no row");
}

// adr: 4b407ee1-1e64-4ca1-842b-69773fc0e48a
#[test]
fn a_pair_is_one_row_the_lower_identifier_first() {
    let (dir, decs) = repo_with(&["", ""]);
    let before = rows(&dir).len();
    read_ok(&dir);
    let all = rows(&dir);
    assert_eq!(all.len(), before + 1, "{:?}", all);
    let r = all.last().unwrap();
    let (first, second) = ordered(&decs[0], &decs[1]);
    assert_eq!(r.text_hash, record::sha256_hex(&text_of(first)));
    assert_eq!(r.neighbour, second.id);
    assert_eq!(r.word, "unrelated");
    assert_eq!(r.reason, REASON);
    assert_eq!(r.model, reader::MODEL);
    let prompt = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/reader-set.md")).unwrap();
    assert_eq!(r.prompt_hash, record::sha256_hex(&prompt));
}

// adr: 908bddc6-5d30-4bcc-be8d-4c470c6bff11
#[test]
fn every_word_is_kept_and_a_word_outside_the_set_fails_the_run() {
    let (dir, _) = repo_with(&["consistent", "consistent"]);
    read_ok(&dir);
    assert!(set_rows(&dir).iter().any(|r| r.word == "consistent"));

    let (dir, _) = repo_with(&["inconsistent", "inconsistent"]);
    read_ok(&dir);
    assert!(set_rows(&dir).iter().any(|r| r.word == "inconsistent"));

    let (dir, decs) = repo_with(&["maybe", "maybe"]);
    let before = rows(&dir);
    let out = read_with(&dir, &[]);
    assert!(!out.status.success(), "{}", stdout(&out));
    assert_eq!(rows(&dir), before, "no row of the failed chunk");
    let hash_a = record::sha256_hex(&text_of(&decs[0]));
    let hash_b = record::sha256_hex(&text_of(&decs[1]));
    assert!(!set_rows(&dir).iter().any(|r| r.text_hash == hash_a || r.text_hash == hash_b));
}

// adr: bc4291c8-98f1-499d-8e02-46e371035e70
#[test]
fn a_commit_never_runs_the_reader() {
    let (dir, decs) = repo_with(&["", ""]);
    for d in &decs {
        write_test_comment(&dir, &d.id);
    }
    let _ = std::fs::remove_dir_all(dir.join("adrdom/cache"));
    let rows_before = std::fs::read_to_string(dir.join("adrdom/state/reader.tsv")).unwrap_or_default();
    run_git(&dir, &["add", "-A"]);
    let bin_dir = adr_bin().parent().unwrap().to_path_buf();
    let path = format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap_or_default());
    let out = std::process::Command::new("git")
        .current_dir(&dir)
        .args(["commit", "-q", "-m", "chore: commit with no model and no reader"])
        .env("PATH", path)
        .env("ADR_NO_MODEL", "1")
        .env("ADR_NO_READER", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert_eq!(std::fs::read_to_string(dir.join("adrdom/state/reader.tsv")).unwrap_or_default(), rows_before);
    assert!(!dir.join("adrdom/cache").exists(), "the commit created a cache");
}

// adr: 2db33118-82b4-4edf-a9cb-e92ac62738b9
#[test]
fn marked_pairs_print_their_lines_in_the_order_of_the_words() {
    // Two decisions to each word: that pair is marked by both.
    let (dir, decs) = repo_with(&[
        "same", "same", "inconsistent", "inconsistent", "depends", "depends", "consistent", "consistent",
    ]);
    let out = read_ok(&dir);
    let lines: Vec<&str> = out.lines().collect();

    let mut expected: Vec<(String, String)> = Vec::new();
    let mut marked = 0;
    for i in 0..decs.len() {
        for j in i + 1..decs.len() {
            let w = word_of(&decs[i], &decs[j]);
            let (f, s) = ordered(&decs[i], &decs[j]);
            if ["same", "inconsistent", "depends"].contains(&w.as_str()) {
                marked += 1;
                expected.push((w.clone(), line_of(&w, f, s)));
            } else {
                assert!(
                    !out.contains(&format!("\"{}\" and \"{}\"", f.heading, s.heading)),
                    "{} pair printed: {}",
                    w,
                    out
                );
            }
        }
    }
    // The pairs both of whose decisions carry one marker are always there.
    for (a, b, w) in [(0, 1, "same"), (2, 3, "inconsistent"), (4, 5, "depends")] {
        let (f, s) = ordered(&decs[a], &decs[b]);
        assert!(lines.contains(&line_of(w, f, s).as_str()), "{}", out);
    }
    // The consistent pair prints nothing.
    let (f, s) = ordered(&decs[6], &decs[7]);
    assert!(!out.contains(&format!("\"{}\" and \"{}\"", f.heading, s.heading)), "{}", out);

    // Groups in the order same, inconsistent, depends; each line exactly.
    let mut want: Vec<String> = Vec::new();
    for w in ["same", "inconsistent", "depends"] {
        let mut g: Vec<String> = expected.iter().filter(|(x, _)| x == w).map(|(_, l)| l.clone()).collect();
        g.sort();
        let mut got: Vec<String> = lines.iter().filter(|l| l.starts_with(&format!("{}: ", w))).map(|l| l.to_string()).collect();
        got.sort();
        assert_eq!(got, g, "{}", out);
        want.extend(g);
    }
    let printed: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| ["same: ", "inconsistent: ", "depends: "].iter().any(|p| l.starts_with(p)))
        .collect();
    assert_eq!(printed.len(), want.len());
    let rank = |l: &str| ["same", "inconsistent", "depends"].iter().position(|w| l.starts_with(&format!("{}: ", w))).unwrap();
    assert!(printed.windows(2).all(|w| rank(w[0]) <= rank(w[1])), "{}", out);

    // The pairs, then the move they call for, and nothing else: no count
    // (848e1b76 A).
    assert_eq!(lines.last().unwrap(), &SUPERSEDE_MOVE, "{}", out);
    assert_eq!(lines.len(), marked + 1, "{}", out);
    // Every pair of a decision with every other is read and kept as a row.
    assert_eq!(set_rows(&dir).len(), pairs(&decs));
}

// adr: 2db33118-82b4-4edf-a9cb-e92ac62738b9
#[test]
fn depends_alone_prints_no_next_move() {
    let (dir, decs) = repo_with(&["depends", "depends"]);
    let out = read_ok(&dir);
    let (f, s) = ordered(&decs[0], &decs[1]);
    assert_eq!(out, format!("{}\n", line_of("depends", f, s)), "the pair and nothing else");
    assert!(!out.contains(SUPERSEDE_MOVE), "{}", out);
    assert!(!out.contains("agent ") && !out.contains("owner runs:"), "no move is asked of anyone: {}", out);
}

#[test]
fn reading_stops_without_the_model() {
    let (dir, _) = repo_with(&["", ""]);
    let before = rows(&dir);
    let out = read_with(&dir, &[("ADR_NO_MODEL", "1")]);
    assert!(!out.status.success(), "{}", stdout(&out));
    assert!(stderr(&out).contains("the embedding model is missing"), "{}", stderr(&out));
    assert_eq!(rows(&dir), before);
}

#[test]
fn reading_stops_without_the_reader() {
    let (dir, _) = repo_with(&["", ""]);
    let before = rows(&dir);
    let out = read_with(&dir, &[("ADR_NO_READER", "1")]);
    assert!(!out.status.success(), "{}", stdout(&out));
    assert_eq!(rows(&dir), before);
}

#[test]
fn a_planning_run_on_an_unrelated_file_repeats_no_set_row() {
    let (dir, decs) = repo_with(&["", ""]);
    read_ok(&dir);
    let rel = "adrdom/decisions/other.md";
    write_record(&dir, rel, &format!("# Other\n\n### Does the widget other glow?\ndepends: widget\n{}", body("other")));
    let path = dir.join(rel);
    let first = adr(&dir, &["plan", path.to_str().unwrap()], &[], "");
    assert!(first.status.success(), "{}{}", stdout(&first), stderr(&first));
    let n = set_rows(&dir).len();
    assert_eq!(n, 1, "{}", decs.len());
    // Planning again reads nothing new, and never touches the set rows.
    let len = rows(&dir).len();
    let second = adr(&dir, &["plan", path.to_str().unwrap()], &[], "");
    assert!(second.status.success());
    assert_eq!(rows(&dir).len(), len);
    assert_eq!(set_rows(&dir).len(), n);
}

// adr: 1ab6d6c3-7e44-42de-bc2f-2e5849114aef
#[test]
fn a_superseded_decision_drops_out_of_the_reading() {
    let (dir, decs) = repo_with(&["same", "same", "same"]);
    let first = read_ok(&dir);
    assert!(first.contains(&decs[0].heading), "{}", first);
    assert_eq!(set_rows(&dir).len(), 3);
    assert_eq!(first.lines().filter(|l| l.starts_with("same: ")).count(), 3, "{}", first);

    // A new answered question supersedes the first decision.
    let rel = "adrdom/decisions/topic-d.md";
    let h = "Does the widget n9 glow?";
    let path = write_record(
        &dir,
        rel,
        &format!("# Topic d\n\n### {}\ndepends: widget\nsupersedes: {}\n{}", h, decs[0].id, body("n9")),
    );
    approve_record(&dir, rel);
    let a = adr(&dir, &["ask", path.to_str().unwrap()], &[], "");
    assert!(a.status.success(), "{}{}", stdout(&a), stderr(&a));
    let id = extract_ids(&std::fs::read_to_string(&path).unwrap())[0].clone();
    let r = adr_answer(&dir, &id, "A");
    assert!(r.status.success(), "{}{}", stdout(&r), stderr(&r));

    let before = set_rows(&dir).len();
    let out = read_ok(&dir);
    assert!(!out.contains(&decs[0].heading), "the superseded decision is printed: {}", out);
    // The new decision is paired with the two decisions that remain; the
    // pair of those two was read before and is not read again.
    assert_eq!(set_rows(&dir).len(), before + 2);
    assert!(set_rows(&dir).iter().any(|r| r.neighbour == id || r.text_hash == record::sha256_hex(&format!("{}\nYes.", h))));
}

// adr: 1ab6d6c3-7e44-42de-bc2f-2e5849114aef
#[test]
fn a_decision_is_read_against_ten_neighbours_not_every_other() {
    let (dir, decs) = repo_with(&[""; 15]);
    read_ok(&dir);
    let rs = set_rows(&dir);
    assert!(rs.len() < pairs(&decs), "{} rows for {} pairs", rs.len(), pairs(&decs));
    for d in &decs {
        let h = record::sha256_hex(&text_of(d));
        let n = rs.iter().filter(|r| r.text_hash == h || r.neighbour == d.id).count();
        assert!(n >= 10, "{} is in {} rows", d.id, n);
    }
}

// adr: 848e1b76-9cfe-483b-94dd-094eb9b2f995
#[test]
fn a_stopped_reading_prints_the_stop_and_its_move_and_no_count() {
    let (dir, decs) = repo_with(&["maybe", "maybe", "maybe", "", "", ""]);
    let out = read_with(&dir, &[]);
    assert!(!out.status.success(), "{}", stdout(&out));
    assert_eq!(stdout(&out), "", "nothing on standard output");
    let err = stderr(&out);
    assert_eq!(err.matches("agent ").count(), 1, "one move: {}", err);
    assert!(err.lines().all(|l| l.starts_with("read: ")), "{}", err);
    assert!(!err.contains(" unread"), "no count of unread pairs: {}", err);
    // The rows of the chunks read before the stop are kept, and the next run
    // reads only what is left. Six decisions each read against five others:
    // every pair is a neighbour pair.
    assert!(set_rows(&dir).len() < pairs(&decs), "{}", err);
}
