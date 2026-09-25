use crate::actions;
use crate::msg::{self, Actor};
use crate::repo;

pub fn run(id: String, letter: String) -> i32 {
    if !repo::owner_authorized() {
        eprintln!("{}", msg::owner_refusal("answer", &format!("{} {}", id, letter)));
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("answer", Actor::Owner));
            return 1;
        }
    };

    // `adr answer <id> refuse` (per "Does the owner answer a question with
    // a refusal?") shares this command's owner gate and
    // dispatch with the letter path; only `do_answer`/`do_refuse` in
    // src/actions.rs differ.
    let outcome = if letter == "refuse" {
        actions::do_refuse(&root, &id)
    } else {
        match parse_letter(&letter) {
            Some(c) => actions::do_answer(&root, &id, c),
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
