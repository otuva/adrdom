use crate::actions;
use crate::msg::{self, Actor};
use crate::repo;

pub fn run(id_arg: String, letter: String) -> i32 {
    if !repo::owner_authorized() {
        eprintln!(
            "{}",
            msg::owner_refusal("answer", &format!("{} {}", id_arg, letter))
        );
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("answer", Actor::Owner));
            return 1;
        }
    };

    // Per "Does an answer carry the version of the question the owner
    // read?": the id argument is now `<id>@<version>` -- a uuid never holds
    // an `@`, so splitting on the first one is unambiguous. A bare id (no
    // `@` at all) is refused before anything else is even looked up.
    let (id, version) = match id_arg.split_once('@') {
        Some(pair) => pair,
        None => {
            eprintln!("answer: {}", msg::answer_missing_version(&id_arg));
            return 1;
        }
    };

    // `adr answer <id>@<version> refuse` (per "Does the owner answer a
    // question with a refusal?") shares this command's owner gate and
    // dispatch with the letter path; only `do_answer`/`do_refuse` in
    // src/actions.rs differ.
    let outcome = if letter == "refuse" {
        actions::do_refuse(&root, id, version)
    } else {
        match actions::parse_letter(&letter) {
            Some(c) => actions::do_answer(&root, id, version, c),
            None => {
                eprintln!("answer: {}", msg::invalid_letter(&letter));
                return 1;
            }
        }
    };

    match outcome {
        Ok(outcome) => {
            println!("{}", outcome.summary);
            0
        }
        Err(e) => {
            for line in e.lines() {
                eprintln!("answer: {}", line);
            }
            1
        }
    }
}
