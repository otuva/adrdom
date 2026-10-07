use crate::actions;
use crate::msg::{self, Actor};
use crate::out;
use crate::repo;
use std::io::Read;

pub fn run() -> i32 {
    if !repo::owner_authorized() {
        out::eprint_stop("unban", &msg::owner_refusal("unban", &[]));
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            out::eprint_stop("unban", &msg::not_a_repo("unban", &[], Actor::Owner));
            return 1;
        }
    };

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        out::eprint_stop("unban", &msg::stdin_read_failed(msg::after_fix_owner_runs(crate::moves::Cmd::fixed("adr unban"))));
        return 1;
    }

    // One line per `unbanned` row, and on standard error the finding on
    // the words given that were not banned (per "What does a command that
    // writes print?", 069a6b4a A).
    match actions::do_unban(&root, &input) {
        Ok(outcome) => {
            out::print_events(&outcome.events);
            out::eprint(&out::stop("unban", &outcome.input_findings.join("\n")));
            0
        }
        Err(e) => {
            out::eprint(&out::stop("unban", &e));
            1
        }
    }
}
