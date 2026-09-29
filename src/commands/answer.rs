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
            // The question the bare id names, when it names one: the
            // refusal then prints the command whole, version included.
            let found = crate::record::load_all(&root).into_iter().find_map(|rf| {
                rf.sections
                    .iter()
                    .find(|s| s.id.as_deref() == Some(id_arg.as_str()) && !s.is_skeleton)
                    .map(|s| (s.heading_text.clone(), crate::record::version_of(&rf.lines, s)))
            });
            let asked = found.as_ref().map(|(h, v)| (h.as_str(), v.as_str()));
            eprintln!("answer: {}", msg::answer_missing_version(&id_arg, asked));
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
                eprintln!("answer: {}", msg::invalid_letter(&letter, &id_arg));
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
