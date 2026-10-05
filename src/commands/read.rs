//! `adr read`: the agent's command for reading the current decisions
//! against each other. Every current decision is read against its ten
//! nearest other current decisions ("Does the tool read every current
//! decision against its neighbours?", 1ab6d6c3); the reader's word on a
//! pair is one of same, consistent, inconsistent, depends or unrelated,
//! after one sentence of reason ("What does the reader say about two
//! current decisions?", 908bddc6); each pair is read once, in one fixed
//! order, the decision with the lower identifier first, and the reason
//! names which stands on which ("Does the tool read a pair of decisions in
//! both directions?", 4b407ee1); no commit runs it ("Does a commit run the
//! reading of the set?", bc4291c8); every pair marked same, inconsistent or
//! depends is printed ("Does the tool print every pair the reader marks?",
//! 2db33118).
//!
//! The pool is the current decisions only, no term. The reader is asked in
//! chunks as large as it reads at once, so a failure keeps what earlier
//! chunks read and loses at most one wave. The
//! model and the reader are loaded only here and in `adr plan`.
use crate::embed;
use crate::index::{self, Rec};
use crate::ledger;
use crate::msg::{self, Actor};
use crate::reader::{self, Kind, Pair};
use crate::record::{self, sha256_hex};
use crate::repo;
use crate::state;
use std::collections::{BTreeSet, HashSet};

/// How many pairs go to the reader in one call: the reader's concurrency,
/// so a failed call loses at most one wave of reads.
const CHUNK: usize = reader::AT_ONCE;

pub fn run() -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("read", "", Actor::Agent));
            return 1;
        }
    };
    match read_all(&root) {
        Ok(out) => {
            print!("{}", out);
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

/// The plain heading of a decision's text.
fn heading_of(text: &str) -> &str {
    text.split('\n').next().unwrap_or(text)
}

fn row_key(p: &Pair) -> (String, String, String, String) {
    (sha256_hex(&p.new_text), p.neighbour.clone(), p.kind.prompt_hash(), reader::MODEL.to_string())
}

fn read_all(root: &std::path::Path) -> Result<String, String> {
    let led = ledger::read(&root.join(repo::LEDGER_PATH));
    let files = record::load_all(root);
    let terms = state::load_terms(root).map(|t| t.terms).unwrap_or_default();
    let recs = index::pool(&files, &led, &terms);
    let decisions: Vec<&Rec> = recs.iter().filter(|r| r.kind == "decision").collect();
    let mut out = String::new();
    if decisions.len() < 2 {
        out.push_str(&msg::read_summary(0, 0, 0));
        out.push('\n');
        return Ok(out);
    }
    let emb = embed::load(root)?;
    // The whole pool is refreshed, so the cache file holds what `adr plan` keeps.
    let (all_vecs, _) = index::refresh(root, emb.as_ref(), &recs, &[])?;
    let vecs: Vec<Vec<f32>> = recs.iter().zip(all_vecs).filter(|(r, _)| r.kind == "decision").map(|(_, v)| v).collect();
    // Each unordered pair once, as (lower id, higher id) positions.
    let mut set: BTreeSet<(usize, usize)> = BTreeSet::new();
    for i in 0..decisions.len() {
        for (j, _) in index::rank(&vecs[i], &vecs, |k| k != i, index::NEIGHBOURS) {
            set.insert(if decisions[i].id < decisions[j].id { (i, j) } else { (j, i) });
        }
    }
    let mut ordered: Vec<(usize, usize)> = set.into_iter().collect();
    ordered.sort_by(|a, b| (&decisions[a.0].id, &decisions[a.1].id).cmp(&(&decisions[b.0].id, &decisions[b.1].id)));
    let pairs: Vec<Pair> = ordered
        .iter()
        .map(|&(a, b)| Pair {
            kind: Kind::Set,
            new_text: decisions[a].text.clone(),
            neighbour: decisions[b].id.clone(),
            neighbour_text: decisions[b].text.clone(),
        })
        .collect();
    reader::load()?;
    let before = |have: &HashSet<(String, String, String, String)>| pairs.iter().filter(|p| have.contains(&row_key(p))).count();
    let have_keys = || -> HashSet<_> {
        reader::read_file(root).rows.iter().filter(|r| r.model == reader::MODEL).map(|r| {
            (r.text_hash.clone(), r.neighbour.clone(), r.prompt_hash.clone(), r.model.clone())
        }).collect()
    };
    let read_before = before(&have_keys());
    let mut rows = Vec::new();
    for chunk in pairs.chunks(CHUNK) {
        match reader::read(root, chunk) {
            Ok(r) => rows.extend(r),
            Err(e) => {
                let unread = pairs.len() - before(&have_keys());
                return Err(msg::read_stopped(&e, unread));
            }
        }
    }
    // Marked pairs by (group, first id, second id, line).
    let mut marked: Vec<(usize, &str, &str, String)> = Vec::new();
    for (&(a, b), row) in ordered.iter().zip(&rows) {
        let group = match row.word.as_str() {
            "same" => 0,
            "inconsistent" => 1,
            "depends" => 2,
            _ => continue,
        };
        let line = msg::read_pair(&row.word, heading_of(&decisions[a].text), heading_of(&decisions[b].text), &reader::flatten(&row.reason));
        marked.push((group, &decisions[a].id, &decisions[b].id, line));
    }
    marked.sort();
    for m in &marked {
        out.push_str(&m.3);
        out.push('\n');
    }
    if marked.iter().any(|m| m.0 < 2) {
        out.push_str(&msg::read_next());
        out.push('\n');
    }
    out.push_str(&msg::read_summary(pairs.len() - read_before, read_before, marked.len()));
    out.push('\n');
    Ok(out)
}
