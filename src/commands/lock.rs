use crate::actions;
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

    let letter_c = match parse_letter(&letter) {
        Some(c) => c,
        None => {
            eprintln!("lock: invalid letter {}", letter);
            return 1;
        }
    };

    match actions::do_lock(&root, &id, letter_c) {
        Ok(outcome) => {
            println!("{}", outcome.summary);
            0
        }
        Err(e) => {
            eprintln!("lock: {}", e);
            1
        }
    }
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
