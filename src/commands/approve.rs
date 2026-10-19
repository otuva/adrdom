use crate::actions;
use crate::msg::{self, Actor};
use crate::out;
use crate::repo;
use crate::rules::Command;
use std::path::PathBuf;

pub fn run(file: PathBuf, entry: Option<String>) -> i32 {
    if !repo::owner_authorized() {
        out::eprint_stop(
            Command::Approve,
            &msg::owner_refusal("approve", &[&file.display().to_string()]),
        );
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            out::eprint_stop(
                Command::Approve,
                &msg::not_a_repo("approve", &[&file.display().to_string()], Actor::Owner),
            );
            return 1;
        }
    };

    let given = file.display().to_string();
    let mut args = vec![given.as_str()];
    args.extend(entry.as_deref());
    if super::refused_in_satellite(&root, Command::Approve, &args, Actor::Owner) {
        return 1;
    }

    // One line per ledger row written, or the findings that stopped it (per
    // "What does a command that writes print?", 069a6b4a A).
    match actions::do_approve(&root, &file, entry.as_deref()) {
        Ok(outcome) => {
            out::print_events(&outcome.events);
            0
        }
        Err(e) => {
            out::eprint(&e);
            1
        }
    }
}
