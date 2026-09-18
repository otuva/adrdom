use crate::banned;
use crate::ledger;
use crate::record;
use crate::repo;
use std::collections::HashSet;
use std::io::Read;

pub fn run() -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("ban: not inside a git repository");
            return 1;
        }
    };
    let banned_path = root.join("adrdom/state/banned.sha256");
    let ledger_path = root.join("adrdom/state/ledger.tsv");

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("ban: failed to read stdin");
        return 1;
    }

    let mut existing = banned::read_set(&banned_path);
    let mut new_hashes: Vec<String> = Vec::new();
    let mut seen_this_run: HashSet<String> = HashSet::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        for tok in banned::tokenize(line) {
            let h = record::sha256_hex(&tok);
            if !existing.contains(&h) && seen_this_run.insert(h.clone()) {
                new_hashes.push(h.clone());
                existing.insert(h);
            }
        }
    }

    if new_hashes.is_empty() {
        println!("ban: no new entries");
        return 0;
    }

    if let Err(e) = banned::append(&banned_path, &new_hashes) {
        eprintln!("ban: failed to update banned list: {}", e);
        return 1;
    }

    let rows: Vec<ledger::Row> = new_hashes
        .iter()
        .map(|h| ledger::Row {
            time: ledger::now_rfc3339(),
            event: "banned".to_string(),
            id: "-".to_string(),
            hash: h.clone(),
            detail: String::new(),
        })
        .collect();
    if let Err(e) = ledger::append(&ledger_path, &rows) {
        eprintln!("ban: failed to update ledger: {}", e);
        return 1;
    }

    println!("ban: added {} entries", new_hashes.len());
    0
}
