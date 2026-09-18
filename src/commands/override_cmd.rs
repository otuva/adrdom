use crate::ledger;
use crate::repo;

pub fn run(check: String, id: String, reason: String) -> i32 {
    if !repo::owner_authorized() {
        eprintln!(
            "override: refusing: owner authorization required (interactive TTY, no CLAUDECODE)"
        );
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("override: not inside a git repository");
            return 1;
        }
    };
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let row = ledger::Row {
        time: ledger::now_rfc3339(),
        event: "overridden".to_string(),
        id: id.clone(),
        hash: "-".to_string(),
        detail: format!("{}: {}", check, reason),
    };
    if let Err(e) = ledger::append(&ledger_path, &[row]) {
        eprintln!("override: failed to update ledger: {}", e);
        return 1;
    }
    println!("overridden: {} for {} ({})", check, id, reason);
    0
}
