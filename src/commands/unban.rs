use crate::actions;
use crate::repo;
use std::io::Read;

pub fn run() -> i32 {
    if !repo::owner_authorized() {
        eprintln!("{}", repo::owner_refusal("unban", ""));
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("unban: not inside a git repository");
            return 1;
        }
    };

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("unban: failed to read stdin");
        return 1;
    }

    match actions::do_unban(&root, &input) {
        Ok(outcome) => {
            println!("unban: {}", outcome.summary);
            0
        }
        Err(e) => {
            eprintln!("unban: {}", e);
            1
        }
    }
}
