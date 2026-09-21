use crate::ledger;
use crate::record;
use crate::repo;
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
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            let is_current = matches!(led.latest_state(id), Some(row) if row.event == "answered");
            if !is_current {
                continue;
            }
            let Some(sentence) = sec.chosen_sentence() else {
                continue;
            };
            println!("{}", sec.heading_text);
            println!("  {}", sentence);
        }
    }
    0
}
