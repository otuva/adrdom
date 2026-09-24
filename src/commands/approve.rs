use crate::actions;
use crate::repo;
use std::path::PathBuf;

pub fn run(file: PathBuf) -> i32 {
    if !repo::owner_authorized() {
        eprintln!(
            "approve: refusing: this is the owner's command; owner runs: adr approve {}",
            file.display()
        );
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("approve: not inside a git repository");
            return 1;
        }
    };

    match actions::do_approve(&root, &file) {
        Ok(outcome) => {
            println!("{}", outcome.summary);
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}
