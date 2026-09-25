//! `adr show`: per topic file, each derived tier and its questions,
//! plainly. No HTML, no snapshot — reads the file and ledger directly,
//! the same way the old (pre-tier) `show` did.
use crate::depends;
use crate::ledger::{self, Ledger};
use crate::msg::{self, Actor};
use crate::record::{self, RecordFile, Section};
use crate::repo;
use crate::state;
use crate::terms::{self, Term};
use crate::view::{self, QState};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn run(record_arg: Option<PathBuf>) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("show", Actor::Agent));
            return 1;
        }
    };
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);
    let terms_list: Vec<Term> = state::load_terms(&root).map(|tf| tf.terms).unwrap_or_default();
    // A depends `Id` entry is printed as words -- a heading in this file, a
    // term name, or another topic file's decision heading text, so every
    // heading id in the repo is looked up once, here.
    let all_headings = view::all_heading_texts(&root);
    // Same pre-pass `approve`/`check` run before reading `rf.tiers`
    // (`depends::normalize_cross_file_deps`): a heading-shaped `depends:`
    // entry naming a current decision's heading in another topic file is
    // rewritten in memory to that decision's id and the tiers re-derived,
    // so an unapproved file with such an entry still prints its real
    // tiers instead of nothing (the file-wide derive error a plain,
    // unresolved `Text` heading dep would otherwise earn). The ambiguous
    // case (the same heading text naming a current decision in more than
    // one other file) becomes this file's `derive_error`
    // (`DeriveError::AmbiguousHeading`) instead of tiers: `print_file`
    // already prints nothing but the topic line when `rf.tiers` is `None`,
    // so `show` never guesses which decision an ambiguous entry means --
    // `check` is still where the ambiguity itself is reported.
    let phrase_lookup = terms::phrase_lookup(&terms_list);
    // Every topic file, loaded once: the index needs all of them regardless
    // of which one `record_arg` asks to print, so they're loaded here
    // rather than inside `cross_file_heading_index` (which used to load
    // them all itself, then have the loop below load whichever of them it
    // was about to print all over again).
    let all_records: Vec<RecordFile> = repo::list_decision_files(&root)
        .iter()
        .filter_map(|f| record::load_in(&root, f).ok())
        .collect();
    let cross_file_headings = depends::cross_file_heading_index(&all_records, &led);

    let mut records: Vec<RecordFile> = match &record_arg {
        Some(p) => {
            let rel = state::rel_path(&root, p);
            match all_records.into_iter().find(|rf| rf.rel == rel) {
                Some(rf) => vec![rf],
                // Not one of the repo's topic files (an unusual path
                // outside adrdom/decisions/, say): loaded directly, same as
                // before this pass.
                None => record::load(p).into_iter().collect(),
            }
        }
        None => all_records,
    };

    for rf in &mut records {
        depends::normalize_cross_file_deps(rf, &terms_list, &phrase_lookup, &cross_file_headings, &led);
        print_file(rf, &led, &terms_list, &all_headings);
    }
    0
}

/// `# <topic>` then each derived tier as `tier N` followed by its
/// questions. A file whose tier derivation failed prints only its `#
/// <topic>` line; `adr check`/`adr ask`/`adr approve` are where the
/// derivation error itself is reported.
fn print_file(rf: &RecordFile, led: &Ledger, terms_list: &[Term], all_headings: &HashMap<String, String>) {
    println!("# {}", rf.topic.as_deref().unwrap_or(""));

    let Some(tiers) = &rf.tiers else {
        return;
    };
    let max_tier = tiers.iter().copied().max().unwrap_or(0);
    for t in 1..=max_tier {
        let idxs: Vec<usize> = (0..rf.sections.len()).filter(|&i| tiers[i] == t).collect();
        if idxs.is_empty() {
            continue;
        }
        println!("tier {}", t);
        for &i in &idxs {
            print_question(rf, &rf.sections[i], led, terms_list, all_headings);
        }
    }
}

fn print_question(
    rf: &RecordFile,
    sec: &Section,
    led: &Ledger,
    terms_list: &[Term],
    all_headings: &HashMap<String, String>,
) {
    let state = view::qstate(led, sec);
    println!("  [{}] {}", state_label(state, sec), sec.heading_text);
    if !sec.depends.is_empty() {
        let entries: Vec<String> = sec
            .depends
            .iter()
            .map(|d| view::format_dep(d, rf, terms_list, all_headings))
            .collect();
        println!("    stands on: {}", entries.join(", "));
    }
    if state == QState::Answered {
        if let Some(s) = sec.chosen_sentence() {
            println!("    {}", s);
        }
    }
}

/// The state word `show` prints inside `[...]`: `answered` also carries
/// the chosen letter (`answered A`), every other state is the bare word.
fn state_label(state: QState, sec: &Section) -> String {
    match state {
        QState::Skeleton => "skeleton".to_string(),
        QState::Unasked => "unasked".to_string(),
        QState::Open => "open".to_string(),
        QState::Answered => match sec.answer_letter() {
            Some(c) => format!("answered {}", c),
            None => "answered".to_string(),
        },
        QState::Refused => "refused".to_string(),
        QState::Superseded => "superseded".to_string(),
        QState::Withdrawn => "withdrawn".to_string(),
    }
}
