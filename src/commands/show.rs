//! `adr show`: per topic file, each derived tier and its questions, plainly
//! (spec §7). No HTML, no snapshot — reads the file and ledger directly,
//! the same way the old (pre-tier) `show` did.
use crate::ledger::{self, Ledger};
use crate::record::{self, RecordFile, Section};
use crate::repo;
use crate::view::{self, QState};
use std::path::PathBuf;

pub fn run(record_arg: Option<PathBuf>) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("show: not inside a git repository");
            return 1;
        }
    };
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);

    let files = match &record_arg {
        Some(p) => vec![p.clone()],
        None => repo::list_decision_files(&root),
    };

    for f in &files {
        let Ok(rf) = record::load(f) else { continue };
        print_file(&rf, &led);
    }
    0
}

/// `# <topic>` then each derived tier as `tier N` followed by its
/// questions. A file whose tier derivation failed prints only its `#
/// <topic>` line; `adr check`/`adr ask`/`adr approve` are where the
/// derivation error itself is reported.
fn print_file(rf: &RecordFile, led: &Ledger) {
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
            print_question(&rf.sections[i], led);
        }
    }
}

fn print_question(sec: &Section, led: &Ledger) {
    let state = view::qstate(led, sec);
    println!("  [{}] {}", state_label(state, sec), sec.heading_text);
    if !sec.depends.is_empty() {
        let entries: Vec<String> = sec.depends.iter().map(view::format_dep).collect();
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
