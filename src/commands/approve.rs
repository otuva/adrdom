use crate::actions;
use crate::msg::{self, Actor};
use crate::repo;
use std::path::PathBuf;

pub fn run(file: PathBuf) -> i32 {
    if !repo::owner_authorized() {
        eprintln!("{}", msg::owner_refusal("approve", &file.display().to_string()));
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("approve", &file.display().to_string(), Actor::Owner));
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
