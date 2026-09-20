use crate::banned;
use crate::ledger;
use crate::record;
use crate::repo;
use std::collections::HashSet;
use std::io::Read;

pub fn run() -> i32 {
    if !repo::owner_authorized() {
        eprintln!(
            "unban: refusing: owner authorization required (interactive TTY, no CLAUDECODE)"
        );
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("unban: not inside a git repository");
            return 1;
        }
    };
    let banned_path = root.join("adrdom/state/banned.sha256");
    let ledger_path = root.join("adrdom/state/ledger.tsv");

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("unban: failed to read stdin");
        return 1;
    }

    let mut to_remove: HashSet<String> = HashSet::new();
    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        for tok in banned::tokenize(line) {
            to_remove.insert(record::sha256_hex(&tok));
        }
    }

    let existing = banned::read_set(&banned_path);
    let mut removed: Vec<String> = existing.intersection(&to_remove).cloned().collect();
    removed.sort();

    if removed.is_empty() {
        println!("unban: nothing to remove");
        return 0;
    }

    let mut remaining: Vec<&String> = existing.iter().filter(|h| !to_remove.contains(*h)).collect();
    remaining.sort();
    let mut content = String::new();
    for h in &remaining {
        content.push_str(h);
        content.push('\n');
    }
    if let Err(e) = std::fs::write(&banned_path, content) {
        eprintln!("unban: failed to update banned list: {}", e);
        return 1;
    }

    let rows: Vec<ledger::Row> = removed
        .iter()
        .map(|h| ledger::Row {
            time: ledger::now_rfc3339(),
            event: "unbanned".to_string(),
            id: "-".to_string(),
            hash: h.clone(),
            detail: String::new(),
        })
        .collect();
    if let Err(e) = ledger::append(&ledger_path, &rows) {
        eprintln!("unban: failed to update ledger: {}", e);
        return 1;
    }

    println!("unban: removed {} entries", removed.len());
    0
}
