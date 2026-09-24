use crate::banned;
use crate::commands::check::collect_targets;
use crate::ledger;
use crate::record;
use crate::repo;
use crate::state;
use std::collections::HashSet;
use std::io::Read;

/// Resolves `--term <name>` to that term's uuid, matched case-insensitively
/// by name or alias. Only `state::load_terms` plus the term's `id`/`name`/
/// `aliases`/`retired` are used here, so this keeps working whatever else
/// changes in the terms file's shape. A retired term is never matched
/// (9.3 B): this pairing sees current terms only, same as `terms::resolve`.
fn resolve_term_uuid(root: &std::path::Path, name: &str) -> Option<String> {
    let tf = state::load_terms(root)?;
    let needle = name.trim().to_lowercase();
    for t in &tf.terms {
        if t.retired {
            continue;
        }
        if t.name.trim().to_lowercase() == needle {
            return t.id.clone();
        }
        if t.aliases.iter().any(|a| a.trim().to_lowercase() == needle) {
            return t.id.clone();
        }
    }
    None
}

pub fn run(term: Option<String>) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("ban: not inside a git repository");
            return 1;
        }
    };
    let banned_path = root.join("adrdom/state/banned.sha256");
    let ledger_path = root.join("adrdom/state/ledger.tsv");

    let uuid: Option<String> = match &term {
        Some(name) => match resolve_term_uuid(&root, name) {
            Some(u) => Some(u),
            None => {
                eprintln!("ban: no term named \"{}\" in adrdom/terms.md", name);
                return 1;
            }
        },
        None => None,
    };

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("ban: failed to read stdin");
        return 1;
    }

    let existing_hashes: HashSet<String> = banned::read_set(&banned_path);
    let mut new_hashes: Vec<String> = Vec::new();
    let mut seen_this_run: HashSet<String> = HashSet::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        for tok in banned::tokenize(line) {
            let h = record::sha256_hex(&tok);
            if !existing_hashes.contains(&h) && seen_this_run.insert(h.clone()) {
                new_hashes.push(h);
            }
        }
    }

    if new_hashes.is_empty() {
        println!("ban: no new entries");
        return 0;
    }

    // Refuse before writing anything if any new word is already in use
    // anywhere the non-staged `check` scan looks.
    let new_hash_set: HashSet<String> = new_hashes.iter().cloned().collect();
    let mut hits: Vec<String> = Vec::new();
    for (name, content) in collect_targets(&root, false, None) {
        for (i, line) in content.lines().enumerate() {
            let mut seen_words: HashSet<String> = HashSet::new();
            for tok in banned::tokenize(line) {
                if new_hash_set.contains(&record::sha256_hex(&tok)) && seen_words.insert(tok.clone())
                {
                    hits.push(format!("{}:{}: \"{}\"", name, i + 1, tok));
                }
            }
        }
    }

    if !hits.is_empty() {
        eprintln!("ban: refusing: already in use");
        for h in &hits {
            eprintln!("{}", h);
        }
        return 1;
    }

    let entries: Vec<(String, Option<String>)> = new_hashes
        .iter()
        .map(|h| (h.clone(), uuid.clone()))
        .collect();

    if let Err(e) = banned::append(&banned_path, &entries) {
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
            detail: uuid.clone().unwrap_or_default(),
            version: crate::version::VERSION.to_string(),
        })
        .collect();
    if let Err(e) = ledger::append(&ledger_path, &rows) {
        eprintln!("ban: failed to update ledger: {}", e);
        return 1;
    }

    println!("ban: added {} entries", new_hashes.len());
    0
}
