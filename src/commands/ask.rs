use crate::ledger;
use crate::record;
use crate::repo;
use std::path::PathBuf;
use uuid::Uuid;

pub fn run(record_path: PathBuf) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("ask: not inside a git repository");
            return 1;
        }
    };
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);

    let mut rf = match record::load(&record_path) {
        Ok(rf) => rf,
        Err(e) => {
            eprintln!("ask: cannot read {}: {}", record_path.display(), e);
            return 1;
        }
    };

    // Refuse (and change nothing) if any answered section was edited since
    // it was answered.
    for sec in &rf.sections {
        if let Some(id) = &sec.id {
            if let Some(row) = led.latest_state(id) {
                if row.event == "answered" {
                    let cur = record::section_hash(&rf.lines, sec);
                    if cur != row.hash {
                        eprintln!(
                            "ask: {}: \"{}\" was edited after being answered; refusing",
                            record_path.display(),
                            sec.heading_text
                        );
                        return 1;
                    }
                }
            }
        }
    }

    // Insert missing ids, bottom-to-top so earlier insertions don't shift
    // the line numbers sections below still need to be found at.
    let needs_id: Vec<bool> = rf.sections.iter().map(|s| s.id.is_none()).collect();
    let mut inserted_any = false;
    for i in (0..rf.sections.len()).rev() {
        if needs_id[i] {
            let start = rf.sections[i].start;
            let new_id = Uuid::new_v4().to_string();
            rf.lines.splice(
                start + 1..start + 1,
                [String::new(), format!("id: {}", new_id), String::new()],
            );
            inserted_any = true;
        }
    }
    if inserted_any {
        rf.sections = record::parse_sections(&rf.lines);
    }

    let mut new_rows = Vec::new();
    let mut summary = Vec::new();
    for sec in &rf.sections {
        let Some(id) = &sec.id else { continue };
        let cur = record::section_hash(&rf.lines, sec);
        let label = match led.latest_state(id) {
            None => Some("asked"),
            Some(row) if row.event == "asked" && row.hash != cur => Some("re-asked"),
            _ => None,
        };
        if let Some(label) = label {
            new_rows.push(ledger::Row {
                time: ledger::now_rfc3339(),
                event: "asked".to_string(),
                id: id.clone(),
                hash: cur,
                detail: sec.heading_text.clone(),
            });
            summary.push(format!("{}: {}", label, sec.heading_text));
        }
    }

    if new_rows.is_empty() {
        println!("ask: nothing to do in {}", record_path.display());
        return 0;
    }

    if let Err(e) = record::save(&rf) {
        eprintln!("ask: failed to write {}: {}", record_path.display(), e);
        return 1;
    }
    if let Err(e) = ledger::append(&ledger_path, &new_rows) {
        eprintln!("ask: failed to update ledger: {}", e);
        return 1;
    }

    for s in &summary {
        println!("{}", s);
    }
    0
}
