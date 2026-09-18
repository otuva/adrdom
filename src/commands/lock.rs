use crate::ledger;
use crate::record;
use crate::repo;

pub fn run(id: String, letter: String) -> i32 {
    if !repo::owner_authorized() {
        eprintln!("lock: refusing: owner authorization required (interactive TTY, no CLAUDECODE)");
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("lock: not inside a git repository");
            return 1;
        }
    };
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);

    let letter_c = match parse_letter(&letter) {
        Some(c) => c,
        None => {
            eprintln!("lock: invalid letter {}", letter);
            return 1;
        }
    };

    let files = repo::list_decision_files(&root);
    let mut found = None;
    for f in &files {
        if let Ok(rf) = record::load(f) {
            if let Some(idx) = rf
                .sections
                .iter()
                .position(|s| s.id.as_deref() == Some(id.as_str()))
            {
                found = Some((rf, idx));
                break;
            }
        }
    }
    let Some((mut rf, idx)) = found else {
        eprintln!("lock: id {} not found in any record", id);
        return 1;
    };

    let cur_hash;
    let supersedes_val;
    {
        let sec = &rf.sections[idx];
        if sec.answer_line.is_some() {
            eprintln!("lock: \"{}\" is already answered", sec.heading_text);
            return 1;
        }
        if !sec.options.iter().any(|o| o.letter == letter_c) {
            eprintln!(
                "lock: letter {} does not name an option in \"{}\"",
                letter_c, sec.heading_text
            );
            return 1;
        }
        cur_hash = record::section_hash(&rf.lines, sec);
        match led.latest_state(&id) {
            Some(row) if row.event == "asked" && row.hash == cur_hash => {}
            _ => {
                eprintln!(
                    "lock: \"{}\" is not currently in the 'asked' state at its current hash",
                    sec.heading_text
                );
                return 1;
            }
        }
        supersedes_val = sec.supersedes.clone();
        if let Some(old) = &supersedes_val {
            match led.latest_state(old) {
                Some(row) if row.event == "answered" => {}
                _ => {
                    eprintln!("lock: supersedes target {} is not currently in force", old);
                    return 1;
                }
            }
        }
    }

    let heading_for_print;
    let sentence_for_print;
    {
        let sec = &rf.sections[idx];
        let (_, hash_end) = record::hash_range(&rf.lines, sec);
        heading_for_print = sec.heading_text.clone();
        sentence_for_print = sec
            .options
            .iter()
            .find(|o| o.letter == letter_c)
            .and_then(|o| o.sentence.clone())
            .unwrap_or_default();
        rf.lines.splice(
            hash_end..hash_end,
            [String::new(), format!("**Answer:** {}", letter_c)],
        );
    }

    if let Err(e) = record::save(&rf) {
        eprintln!("lock: failed to write record: {}", e);
        return 1;
    }

    let mut rows = vec![ledger::Row {
        time: ledger::now_rfc3339(),
        event: "answered".to_string(),
        id: id.clone(),
        hash: cur_hash,
        detail: letter_c.to_string(),
    }];
    if let Some(old) = &supersedes_val {
        rows.push(ledger::Row {
            time: ledger::now_rfc3339(),
            event: "superseded".to_string(),
            id: old.clone(),
            hash: "-".to_string(),
            detail: id.clone(),
        });
    }
    if let Err(e) = ledger::append(&ledger_path, &rows) {
        eprintln!("lock: failed to update ledger: {}", e);
        return 1;
    }

    println!("locked: {}", heading_for_print);
    println!("  {}", sentence_for_print);
    0
}

fn parse_letter(s: &str) -> Option<char> {
    let mut chars = s.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if c.is_ascii_uppercase() {
        Some(c)
    } else {
        None
    }
}
