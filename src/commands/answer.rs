use crate::rules::Command;
use crate::actions;
use crate::msg::{self, Actor};
use crate::out;
use crate::repo;

pub fn run(id_arg: String, letter: String) -> i32 {
    if !repo::owner_authorized() {
        out::eprint_stop(Command::Answer, &msg::owner_refusal("answer", &[id_arg.as_str(), letter.as_str()]));
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            out::eprint_stop(Command::Answer, &msg::not_a_repo("answer", &[id_arg.as_str(), letter.as_str()], Actor::Owner));
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
            let led = crate::ledger::read(&root.join(repo::LEDGER_PATH));
            let found = crate::record::load_all(&root).into_iter().find_map(|rf| {
                rf.sections
                    .iter()
                    .find(|s| s.id.as_deref() == Some(id_arg.as_str()))
                    .filter(|s| {
                        let hash = crate::record::section_hash(&rf.lines, s);
                        s.answer_line.is_none() && crate::state::asked_text_stands(&led, &id_arg, &hash)
                    })
                    .map(|s| (s.heading_text.clone(), crate::record::version_of(&rf.lines, s)))
            });
            let asked = found.as_ref().map(|(h, v)| (h.as_str(), v.as_str()));
            out::eprint_stop(Command::Answer, &msg::answer_missing_version(&id_arg, asked));
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
                out::eprint_stop(Command::Answer, &msg::invalid_letter(&letter, &id_arg));
                return 1;
            }
        }
    };

    // One line per ledger row written, or the findings that stopped it (per
    // "What does a command that writes print?", 069a6b4a A).
    match outcome {
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
