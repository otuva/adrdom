//! `adr read`: the agent's command for reading the current decisions
//! against each other. Every current decision is read against its ten
//! nearest other current decisions ("Does the tool read each current
//! decision against its neighbours?", 48c47c42); the reader's word on a
//! pair is one of same, consistent, inconsistent or unrelated, after one
//! sentence of reason ("What word does the reader give two current
//! decisions?", a116a1c5); each pair is read once, in one fixed order, the
//! decision with the lower identifier first ("Does the tool read a pair of
//! decisions in each direction?", ed372cc6); no commit runs it ("Does a
//! commit run the reading of the whole set?", 8cba4463); each pair marked
//! same or inconsistent is printed, and the move is a question that
//! supersedes one of the pair ("Does the tool print each pair the reader
//! marks?", 27bd0855). Per "What does a command print when it writes
//! nothing?" (1af76f91 A), those pairs and the move they call for are all
//! it prints: a reading that marks nothing prints nothing.
//!
//! The pool is the current decisions only, no term. The reader is asked in
//! chunks as large as it reads at once, so a failure keeps what earlier
//! chunks read and loses at most one wave. The
//! model and the reader are loaded only here and in `adr plan`.
use crate::rules::Command;
use crate::embed;
use crate::index::{self, Rec};
use crate::ledger;
use crate::msg::{self, Actor};
use crate::out::{self, Line};
use crate::reader::{self, Kind, Pair};
use crate::record;
use crate::repo;
use crate::rules::{Class, Finding};
use crate::state;
use std::collections::BTreeSet;

/// How many pairs go to the reader in one call: the reader's concurrency,
/// so a failed call loses at most one wave of reads.
const CHUNK: usize = reader::AT_ONCE;

pub fn run() -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            out::eprint(&[out::refusal(Command::Read, &msg::not_a_repo("read", &[], Actor::Agent))]);
            return 1;
        }
    };
    match read_all(&root) {
        Ok(lines) => {
            out::print(&lines);
            0
        }
        Err(e) => {
            out::eprint_stop(Command::Read, &e);
            1
        }
    }
}

/// The plain heading of a decision's text.
fn heading_of(text: &str) -> &str {
    text.split('\n').next().unwrap_or(text)
}

/// Every marked pair as a finding, sorted same, then inconsistent, and
/// the move that follows; or the stop that ended it.
fn read_all(root: &std::path::Path) -> Result<Vec<Line>, String> {
    let led = ledger::read(&root.join(repo::LEDGER_PATH));
    let files = record::load_all(root);
    let terms = state::load_terms(root).map(|t| t.terms).unwrap_or_default();
    let recs = index::pool(&files, &led, &terms);
    let decisions: Vec<&Rec> = recs.iter().filter(|r| r.kind == "decision").collect();
    if decisions.len() < 2 {
        return Ok(Vec::new());
    }
    reader::load()?;
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
            letters: Vec::new(),
        })
        .collect();
    let mut rows = Vec::new();
    // Read once for the run, not once per chunk.
    let live = reader::Live::of(root);
    for chunk in pairs.chunks(CHUNK) {
        match reader::read_live(root, chunk, &live) {
            Ok(r) => rows.extend(r),
            // The rows of earlier chunks are kept; the error names the move.
            Err(e) => return Err(e),
        }
    }
    // Marked pairs by (group, first id, second id, line).
    let mut marked: Vec<(usize, &str, &str, Finding)> = Vec::new();
    for (&(a, b), row) in ordered.iter().zip(&rows) {
        let group = match row.word.as_str() {
            "same" => 0,
            "inconsistent" => 1,
            _ => continue,
        };
        let next = msg::Next::agent(msg::Act::Writes("a question that supersedes one of the pair".to_string()));
        let said = msg::read_pair(
            &row.word,
            heading_of(&decisions[a].text),
            heading_of(&decisions[b].text),
            &reader::flatten(&row.reason),
            next,
        );
        marked.push((group, &decisions[a].id, &decisions[b].id, Finding::said(Class::Reader, said)));
    }
    marked.sort_by(|x, y| (x.0, x.1, x.2).cmp(&(y.0, y.1, y.2)));
    let mut out: Vec<Line> = marked.iter().map(|m| Line::Finding(m.3.clone())).collect();
    if !marked.is_empty() {
        out.push(Line::Move(crate::moves::of_marked_pairs()));
    }
    Ok(out)
}
