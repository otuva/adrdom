//! Every user-facing message the tool prints lives here: one place for
//! wording, and a type (`Next`) that makes a refusal without a next move a
//! compile error. A note (`tested: 3`, `tiers: ...`) carries no `Next` and
//! is a plain function; a refusal or failure always takes one, either
//! built by the caller (when the next move differs by call site) or fixed
//! inside the function (when every caller wants the same one).
//!
//! Shape: `<what happened>; <agent ...>` or `<what happened>; owner runs:
//! <command>`, matching the wording the rest of the tool already used.
//! Some functions bake in their own `<cmd>: ` prefix (a single call site);
//! most don't, since the same text is reused under `approve: `, `check`'s
//! per-category prefix, and so on, exactly as before this module existed.

use crate::moves::MoveKind;
use std::fmt::Display;

#[derive(Clone, Copy)]
pub enum Actor {
    Agent,
    Owner,
}

/// One or more next-move clauses, joined with `, `. `Next::agent(..)`
/// reads as `agent <action>`; `Next::owner_runs(..)` as `owner runs:
/// <command>` -- the owner's clause always names a command to run, never a
/// bare instruction, so a refusal never asks the owner to do something the
/// tool cannot also point at directly.
pub struct Next(Vec<(Actor, String)>);

impl Next {
    pub fn agent(action: impl Into<String>) -> Next {
        Next(vec![(Actor::Agent, action.into())])
    }

    pub fn owner_runs(command: impl Into<String>) -> Next {
        Next(vec![(Actor::Owner, format!("runs: {}", command.into()))])
    }

    /// The clauses of every `Next` in `parts`, in order, as one.
    pub fn all(parts: Vec<Next>) -> Next {
        Next(parts.into_iter().flat_map(|n| n.0).collect())
    }

    pub fn and_owner_runs(mut self, command: impl Into<String>) -> Next {
        self.0.push((Actor::Owner, format!("runs: {}", command.into())));
        self
    }

    pub fn render(&self) -> String {
        self.0
            .iter()
            .map(|(actor, action)| {
                let who = match actor {
                    Actor::Agent => "agent",
                    Actor::Owner => "owner",
                };
                format!("{} {}", who, action)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn refuse(what: String, next: Next) -> String {
    format!("{}; {}", what, next.render())
}

/// Prefixes `s` with `approve: `, the command prefix `do_approve_record`
/// puts on every refusal and failure it returns (the same shape `check`'s
/// own per-category prefix follows, but fixed to one command here).
pub fn approve_prefixed(s: impl Display) -> String {
    format!("approve: {}", s)
}

// --- generic I/O failures --------------------------------------------------
// Shared by actions.rs, ask.rs, install.rs, uninstall.rs, ban.rs: the
// `Next` is always supplied by the caller, since the same shape of failure
// wants an agent fix in an agent command and an owner retry in an owner
// command.

pub fn write_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("failed to write {}: {}", target, e), next)
}

pub fn read_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("cannot read {}: {}", target, e), next)
}

pub fn update_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("failed to update {}: {}", target, e), next)
}

pub fn create_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("failed to create {}: {}", target, e), next)
}

pub fn remove_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("failed to remove {}: {}", target, e), next)
}

/// `git config` did not take the change to `core.hooksPath`. `doing` is
/// "set" or "unset".
pub fn hooks_path_failed(doing: &str, next: Next) -> String {
    refuse(format!("git config failed to {} core.hooksPath", doing), next)
}

pub fn parse_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("{}: {}", target, e), next)
}

pub fn stdin_read_failed(next: Next) -> String {
    refuse("failed to read stdin".to_string(), next)
}

/// Standard output refused a line for a cause other than its reader going
/// away (a full disk, a closed descriptor): what was printed is incomplete.
pub fn stdout_write_failed(e: impl Display) -> String {
    refuse(
        format!("output: standard output cannot be written: {}", e),
        Next::agent("frees the place standard output goes to and runs the command again"),
    )
}

// --- repo.rs ----------------------------------------------------------------

/// `<cmd>: not inside a git repository; ...` -- printed by every command
/// before it does anything else. `actor` is who was already confirmed to
/// be running `cmd` (the owner, for the five owner-only commands, since
/// `owner_authorized` already gated by the time this fires; the agent for
/// every other command).
/// `args` is what the command was given, so the owner's line is the whole
/// command.
pub fn not_a_repo(cmd: &str, args: &str, actor: Actor) -> String {
    let command = if args.is_empty() {
        format!("adr {}", cmd)
    } else {
        format!("adr {} {}", cmd, args)
    };
    // What clears it is where the command runs, so the move names that
    // first: the command itself is run again from inside a repository.
    let step = format!("changes into a git repository and runs: {}", command);
    let next = match actor {
        Actor::Agent => Next::agent(step),
        Actor::Owner => Next(vec![(Actor::Owner, step)]),
    };
    refuse(format!("{}: not inside a git repository", cmd), next)
}

/// The refusal every owner-only command prints when stdin isn't an
/// interactive terminal: `<cmd>: refusing: this is the owner's command;
/// owner runs: adr <cmd> <args>`.
pub fn owner_refusal(cmd: &str, args: &str) -> String {
    let command = if args.is_empty() {
        format!("adr {}", cmd)
    } else {
        format!("adr {} {}", cmd, args)
    };
    refuse(
        format!("{}: refusing: this is the owner's command", cmd),
        Next::owner_runs(command),
    )
}

// --- adrdom/terms.md: shared by actions.rs, ask.rs, check/terms.rs --------

pub fn terms_missing() -> String {
    refuse(
        "adrdom/terms.md missing".to_string(),
        Next::agent("writes it").and_owner_runs("adr approve adrdom/terms.md"),
    )
}

/// A read/parse failure on `adrdom/terms.md` itself (an I/O error, not one
/// of its shape rules) -- rare enough that the caller's own context decides
/// who fixes it.
pub fn terms_load_error(e: &str, next: Next) -> String {
    refuse(format!("adrdom/terms.md: {}", e), next)
}

/// One of `adrdom/terms.md`'s shape errors (from `terms::parse`): always
/// the agent's fix, regardless of which command found it.
pub fn terms_shape_error(e: &str) -> String {
    refuse(
        format!("adrdom/terms.md: {}", e),
        Next::agent("fixes adrdom/terms.md"),
    )
}

/// `adrdom/terms.md` exists and parses, but isn't approved at its current
/// hash -- named from a dependent topic file's own path (`rel`), the way
/// `ask`/`answer`/`approve` all report it.
pub fn terms_not_approved(rel: &str, status: FileStatus) -> String {
    refuse(
        format!("{}: adrdom/terms.md is {}", rel, status.name()),
        Next::owner_runs("adr approve adrdom/terms.md"),
    )
}

/// `adrdom/terms.md` exists and parses, but isn't approved at its current
/// hash (per "Does the tool stop while the terms file waits for approval?",
/// aa475aa3 A): `check`'s own report of the same fact `terms_not_approved`
/// names from a dependent topic file -- this one names the file itself,
/// since `check` judges it on its own, not on behalf of one dependent.
pub fn terms_file_not_approved(status: FileStatus) -> String {
    refuse(
        format!("adrdom/terms.md is {}", status.name()),
        Next::owner_runs("adr approve adrdom/terms.md"),
    )
}

/// `check`: no terms brief where the terms file is approved (per "Does
/// check hold the terms brief to the terms file?", ec3f875c A). `approve`
/// is the one writer (per "Does approve write the terms brief?", fcab4bf9
/// A), so the move is the owner's.
pub fn terms_brief_missing() -> String {
    refuse(
        format!("{} missing", crate::repo::TERMS_BRIEF_PATH),
        Next::owner_runs("adr approve adrdom/terms.md"),
    )
}

/// `check`: the terms brief does not hold what the terms file gives.
pub fn terms_brief_differs() -> String {
    refuse(
        format!("{} differs from adrdom/terms.md", crate::repo::TERMS_BRIEF_PATH),
        Next::owner_runs("adr approve adrdom/terms.md"),
    )
}

/// `check` at a commit: the working tree holds the right terms brief and
/// the staged files do not.
pub fn terms_brief_not_staged() -> String {
    refuse(
        format!("{} not staged", crate::repo::TERMS_BRIEF_PATH),
        Next::agent(format!("runs: git add {}", crate::repo::TERMS_BRIEF_PATH)),
    )
}

/// A cycle's names, in traversal order, closed by repeating the first at
/// the end: "a -> b -> a" -- what `circular_definition` names, and what the
/// page's own display of the same cycle names too.
pub fn cycle_chain(names: &[String]) -> String {
    let mut chain = names.to_vec();
    if let Some(first) = names.first() {
        chain.push(first.clone());
    }
    chain.join(" -> ")
}

pub fn circular_definition(chain: &str) -> String {
    refuse(
        format!("adrdom/terms.md: circular definition: {}", chain),
        Next::agent("defines one of them from plainer words"),
    )
}

/// A dependent (a decision's or a term's own `terms:` line) still names a
/// uuid that no longer exists in `adrdom/terms.md` at all.
/// `word` is the word `source` names the term by, when it names it by a
/// word (its terms line); a depends line names it by the identifier alone.
/// `decision` is the dependent's id and heading when it is a current
/// decision. Marking the term retired is not offered: a retire is held while
/// the dependent stands (per "What holds a retire of a term?"), so the move
/// is restoring the term as it was, then superseding the decision, named the
/// way `term_retired_decision_next` names its move.
pub fn term_removed_but_carried(
    uuid: &str,
    word: Option<&str>,
    source: &str,
    decision: Option<(&str, &str)>,
) -> String {
    let next = match decision {
        Some((id, heading)) => Next::agent(format!(
            "restores the term in adrdom/terms.md, then supersedes {} (\"{}\") with a new question",
            id, heading
        )),
        None => Next::agent("restores the term in adrdom/terms.md").and_owner_runs("adr approve adrdom/terms.md"),
    };
    refuse(
        format!(
            "adrdom/terms.md: term {} removed but used by {}",
            named(uuid, word),
            source
        ),
        next,
    )
}

/// A dependent still names a term that is now retired. `next` differs by
/// dependent kind: a decision supersedes itself with a new question
/// (built by the caller, which knows the decision's id); a term's own
/// definition is reworded by the agent and re-approved by the owner --
/// terms are never superseded (per "What holds a retire of a term?"; the fix for a term-dependent is
/// always this same pair, so it's fixed here).
pub fn term_retired_but_carried(name: &str, source: &str, next: Next) -> String {
    refuse(
        format!(
            "adrdom/terms.md: term \"{}\" retired but used by {}",
            name, source
        ),
        next,
    )
}

/// The term-dependent `Next` for `term_retired_but_carried`: terms are
/// never superseded, so the move is the agent rewording the definition
/// that still carries the retired word, then the owner re-approving.
pub fn term_reword_and_approve() -> Next {
    Next::agent("rewords the definition to drop the retired word")
        .and_owner_runs("adr approve adrdom/terms.md")
}

/// A dependent's pairing (`word=uuid`) no longer resolves that word to
/// that term at all -- the wording moved, without the term being retired
/// or removed. `next` mirrors `term_retired_but_carried`'s split.
pub fn word_moved_off_term(word: &str, term_name: &str, source: &str, next: Next) -> String {
    refuse(
        format!(
            "adrdom/terms.md: word \"{}\" no longer names term \"{}\", used by {}",
            word, term_name, source
        ),
        next,
    )
}

/// A term's definition runs past two sentences (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76:
/// a definition says what a thing is "in one or two sentences").
pub fn definition_too_long(name: &str, count: usize) -> String {
    refuse(
        format!(
            "adrdom/terms.md: term \"{}\" definition has {} sentences, more than two",
            name, count
        ),
        Next::agent("shortens the definition to one or two sentences"),
    )
}

pub fn terms_banned_word(line: usize, suffix: &str) -> String {
    refuse(
        format!("adrdom/terms.md:{}: banned word{}", line, suffix),
        Next::agent("edits adrdom/terms.md to remove it"),
    )
}

// --- a topic file's own shape/tells/depends/tier refusals ------------------
// Shared by `rules::record_refusals` (approve + check) and, for the
// derive-error variant, by `ask` as well.

pub fn shape_error(rel: &str, e: &str) -> String {
    refuse(
        format!("{}: {}", rel, e),
        Next::agent("fixes the topic file"),
    )
}

/// A `### ` line that does not end in `?` (per "What does a topic file
/// hold?": a heading is a question).
pub fn heading_not_a_question(rel: &str, line: usize) -> String {
    refuse(
        format!("{}: line {}: a heading is a question ending in ?", rel, line),
        Next::agent("rewrites the line"),
    )
}

pub fn section_shape_error(rel: &str, heading: &str, e: &str) -> String {
    refuse(
        format!("{}: \"{}\": {}", rel, heading, e),
        Next::agent("fixes the topic file"),
    )
}

/// One provenance/code-state/date/loaded-heading tell (per "Does the tool
/// stop loaded phrases in a question?").
pub fn tell(rel: &str, line: usize, label: &str, phrase: &str, what: &str) -> String {
    refuse(
        format!("{}:{}: \"{}\": \"{}\" {}", rel, line + 1, label, phrase, what),
        Next::agent("rewrites the line"),
    )
}

/// An option, Why line or Costs line over the length rule (per "Does ask
/// refuse an option over a word limit?").
pub fn too_long(rel: &str, line: usize, label: &str, kind: &str, found: &str) -> String {
    refuse(
        format!("{}:{}: \"{}\": the {} has {}", rel, line + 1, label, kind, found),
        if found.ends_with(crate::tells::ONE_WORD_RULE) {
            Next::agent("writes one word")
        } else {
            Next::agent("shortens the line")
        },
    )
}

pub fn no_depends_line(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: \"{}\" has no depends line", rel, heading),
        Next::agent("writes what the question stands on"),
    )
}

/// Same as `no_depends_line`, without naming the file: `approve`'s own
/// defensive fallback in `insert_missing_ids`, unreachable in practice
/// since `record_refusals` already refuses on this first.
pub fn no_depends_line_bare(heading: &str) -> String {
    refuse(
        format!("\"{}\" has no depends line", heading),
        Next::agent("writes what the question stands on"),
    )
}

pub fn frozen_depends_edited(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: \"{}\": answered", rel, heading),
        Next::agent("removes the edit"),
    )
}

pub fn dep_not_current_decision(rel: &str, heading: &str, id: &str, name: Option<&str>) -> String {
    refuse(
        format!(
            "{}: \"{}\" depends on {} which is not a current decision",
            rel,
            heading,
            named(id, name)
        ),
        Next::agent("names a current decision or removes the line"),
    )
}

/// A live section's depends line names, by identifier, a term that is
/// retired.
pub fn dep_retired_term_id(rel: &str, heading: &str, id: &str, name: Option<&str>) -> String {
    refuse(
        format!(
            "{}: \"{}\" depends on the term {}, which is retired",
            rel,
            heading,
            name.map_or_else(|| id.to_string(), |n| format!("\"{}\"", n))
        ),
        Next::agent("names a term that is not retired or removes the entry"),
    )
}

pub fn dep_no_term(rel: &str, heading: &str, text: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" depends on \"{}\" which is no term",
            rel, heading, text
        ),
        Next::agent("adds the term or names another"),
    )
}

/// `show <file>` on a file that is not one of the topic files.
/// `show` names a path approve does not take (per "Does show print what a
/// file holds beyond its latest approval?": show takes every file approve
/// takes).
/// `plan` names a path that is no topic file: it plans topic files alone.
pub fn plan_not_a_topic_file(rel: &str) -> String {
    refuse(
        format!("{} is not a topic file", rel),
        Next::agent("names a topic file under adrdom/decisions"),
    )
}

pub fn show_not_a_file_approve_takes(rel: &str) -> String {
    refuse(
        format!("{} is not a file approve takes", rel),
        Next::agent("names a topic file under adrdom/decisions, adrdom/terms.md or a blame file under adrdom/blame"),
    )
}

pub fn dep_retired_term(rel: &str, heading: &str, text: &str) -> String {
    refuse(
        format!("{}: \"{}\" names the retired term \"{}\"", rel, heading, text),
        Next::agent("names another term"),
    )
}

/// A heading-shaped `depends:` entry naming no heading in its own file, but
/// a current decision's heading text more than once elsewhere (the
/// cross-file gap "Does a reference in a topic file carry the identifier
/// of what it names?" left open): which one it means is ambiguous, so this
/// refuses instead of guessing. `files` is the (deduped, sorted) list of
/// files those decisions live in -- one entry when the same topic file
/// happens to carry the heading text on two different current decisions
/// (`check`'s cross-file duplicate-heading check only flags that between
/// files, not within one), more than one when the decisions are spread
/// across files; the wording says which, rather than always calling it
/// "more than one file" when it may be only one file holding more than one
/// decision. Reported as a file's one `derive_error` (`derive_what`'s
/// `DeriveError::AmbiguousHeading` arm, from
/// `depends::normalize_cross_file_deps`), never per entry:
/// `depends::dep_refusal_text` never reaches this case (see its own doc
/// comment).
fn ambiguous_heading_what(rel: &str, heading: &str, text: &str, files: &[String]) -> String {
    let detail = match files {
        [one] => format!("names more than one current decision in {}", one),
        many => format!("is a current decision in more than one file: {}", many.join(", ")),
    };
    format!("{}: \"{}\" depends on \"{}\" which {}", rel, heading, text, detail)
}

fn ambiguous_heading_next() -> Next {
    Next::agent("names it by id instead")
}

/// A heading `depends:` entry's `=A|B` set names a letter its target
/// doesn't have.
pub fn bad_option_letter(rel: &str, dependent: &str, target: &str, letter: char) -> String {
    refuse(
        format!(
            "{}: \"{}\" depends on \"{}\" names option {} which \"{}\" does not have",
            rel, dependent, target, letter, target
        ),
        Next::agent("fixes the answer set to only name options the target has"),
    )
}

/// `open` are the questions of the tier that are not answered yet, in file
/// order; `answered` counts the ones the owner answered, which hold their
/// place and are never moved.
pub fn tier_cap_exceeded(rel: &str, open: &[String], answered: usize, cap: usize) -> String {
    let names = open
        .iter()
        .map(|h| format!("\"{}\"", h))
        .collect::<Vec<_>>()
        .join(", ");
    if answered == 0 {
        return refuse(
            format!(
                "{}: {} questions would be put to the owner together, the most is {}: {}",
                rel,
                open.len(),
                cap,
                names
            ),
            Next::agent(
                "finds a question among these that assumes the answer of another one and names that one on its depends line, or splits the file when none does",
            ),
        );
    }
    refuse(
        format!(
            "{}: {} questions stand together, the most is {}: {} answered, and {}",
            rel,
            open.len() + answered,
            cap,
            answered,
            names
        ),
        Next::agent(
            "finds a question among the ones named that assumes the answer of another one and names that one on its depends line, or moves it to another file when none does",
        ),
    )
}

// `derive_tiers`' three failure shapes (record::DeriveError), each with two
// wordings: the plain one (`approve`/`check`'s `plan` category) and `ask`'s,
// which always points at re-approving afterward.

use crate::record::DeriveError;

fn derive_what(rel: &str, e: &DeriveError) -> String {
    match e {
        DeriveError::Dangling { section, entry } => format!(
            "{}: \"{}\" depends on \"{}\" which is no heading in this file and no current decision",
            rel, section, entry
        ),
        DeriveError::Cycle(names) => {
            let mut chain: Vec<String> = names.clone();
            if let Some(first) = names.first() {
                chain.push(first.clone());
            }
            format!(
                "{}: cycle: {}",
                rel,
                chain
                    .iter()
                    .map(|n| format!("\"{}\"", n))
                    .collect::<Vec<_>>()
                    .join(" -> ")
            )
        }
        DeriveError::SelfDep(h) => format!("{}: \"{}\" depends on itself", rel, h),
        DeriveError::AmbiguousHeading { section, entry, files } => {
            ambiguous_heading_what(rel, section, entry, files)
        }
    }
}

fn derive_next(e: &DeriveError) -> Next {
    match e {
        DeriveError::Dangling { .. } => {
            Next::agent("names a heading of this file or a current decision")
        }
        DeriveError::Cycle(_) => Next::agent("breaks it"),
        DeriveError::SelfDep(_) => Next::agent("removes the line or names something else"),
        DeriveError::AmbiguousHeading { .. } => ambiguous_heading_next(),
    }
}


/// A `depends:` entry names a question of another topic file that is not
/// answered yet.
pub fn dep_waits_on_other_file(rel: &str, section: &str, entry: &str, other_rel: &str) -> String {
    format!("{}{}", dep_waits_on_other_file_what(rel, section, entry, other_rel), crate::moves::WAIT_TAIL)
}

/// `dep_waits_on_other_file` without its waiting tail: what the `not yet`
/// move names.
pub fn dep_waits_on_other_file_what(rel: &str, section: &str, entry: &str, other_rel: &str) -> String {
    format!("{}: \"{}\" depends on \"{}\" ({}) which is not answered yet", rel, section, entry, other_rel)
}

/// A `depends:` entry names a question of another topic file that is
/// refused.
pub fn dep_on_refused_in_other_file(rel: &str, section: &str, entry: &str, other_rel: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" depends on \"{}\" ({}) which is refused",
            rel, section, entry, other_rel
        ),
        Next::agent("names another or drops the entry"),
    )
}

pub fn derive_error(rel: &str, e: &DeriveError) -> String {
    refuse(derive_what(rel, e), derive_next(e))
}

pub fn answered_id_missing(id: &str, heading: &str) -> String {
    refuse(
        format!(
            "id {} (\"{}\") is answered and current but missing from every topic file",
            id, heading
        ),
        Next::agent("supersedes it or restores the heading"),
    )
}

// --- actions.rs: answer/refuse --------------------------------------------

pub fn id_not_found(id: &str) -> String {
    refuse(
        format!(
            "id {} not found in any topic file; show prints the id of every question that waits for an answer",
            id
        ),
        Next::owner_runs("adr show"),
    )
}

/// The id was in no topic file that could be read, and `unread` (paths,
/// never empty) could not be read at all: the question may be in one of
/// them.
pub fn id_not_found_some_unread(id: &str, unread: &[String]) -> String {
    refuse(
        format!(
            "id {} not found in the topic files that could be read; cannot read {}",
            id,
            unread.join(", ")
        ),
        Next::agent("fixes each file so it can be read as text (e.g. its encoding)"),
    )
}

pub fn already_refused(heading: &str) -> String {
    refuse(
        format!(
            "\"{}\" is already refused; show prints every question that waits for an answer",
            heading
        ),
        Next::owner_runs("adr show"),
    )
}

pub fn already_answered(heading: &str) -> String {
    refuse(
        format!(
            "\"{}\" is already answered; show prints every question that waits for an answer",
            heading
        ),
        Next::owner_runs("adr show"),
    )
}

/// `letters` are the letters of the question's options; `id` and `version`
/// are the ones the owner gave, both good when this is printed.
pub fn letter_not_an_option(letter: char, heading: &str, letters: &[char], id: &str, version: &str) -> String {
    let has: Vec<String> = letters.iter().map(|c| c.to_string()).collect();
    refuse(
        format!(
            "letter {} does not name an option in \"{}\"; its options are {}",
            letter,
            heading,
            has.join(", ")
        ),
        crate::moves::clause(MoveKind::Answer { id: id.to_string(), version: version.to_string(), heading: heading.to_string() }),
    )
}

/// `report`: what the rules find in the question's file and its moves
/// (`moves::of_unasked_question_in`).
pub fn not_in_asked_state(heading: &str, report: &crate::moves::Report) -> String {
    format!("\"{}\" is edited since it was asked; {}", heading, report.tail())
}

/// The question's latest ledger state is `withdrawn`: it was struck from
/// the plan, and `ask` never picks it up.
pub fn already_withdrawn(heading: &str) -> String {
    refuse(
        format!(
            "\"{}\" was withdrawn from the plan; show prints every question that waits for an answer",
            heading
        ),
        Next::owner_runs("adr show"),
    )
}

/// The question's latest ledger state is `superseded`: a newer decision
/// replaced it.
pub fn already_superseded(heading: &str) -> String {
    refuse(
        format!(
            "\"{}\" is already superseded; show prints every question that waits for an answer",
            heading
        ),
        Next::owner_runs("adr show"),
    )
}

/// The question's latest ledger state is `dropped`: an answer whose set
/// lacked its letter removed it, and its identifier never returns.
pub fn already_dropped(heading: &str) -> String {
    refuse(
        format!(
            "\"{}\" is already dropped; show prints every question that waits for an answer",
            heading
        ),
        Next::owner_runs("adr show"),
    )
}

/// A question with a body that no `ask` has put to the owner.
/// `report`: what the rules find in the question's file and its moves
/// (`moves::of_unasked_question_in`).
pub fn not_asked_yet(heading: &str, report: &crate::moves::Report) -> String {
    format!("\"{}\" is {}; {}", heading, crate::status::QuestionStatus::NotAsked.name(), report.tail())
}

/// An answer that would remove a question with no identifier.
pub fn drop_without_id(heading: &str) -> String {
    refuse(
        format!("answering would remove \"{}\", which has no identifier", heading),
        Next::agent("says so in chat and stops"),
    )
}

/// An answer that would remove a question that holds a decision or a
/// refusal (per "Does a decision change after it is answered?").
pub fn drop_of_closed(heading: &str, refused: bool) -> String {
    refuse(
        format!(
            "answering would remove \"{}\", which is {}",
            heading,
            if refused { "refused" } else { "answered" }
        ),
        Next::agent("says so in chat and stops"),
    )
}

/// An answer that would remove a question whose text matches no approval row.
pub fn drop_not_approved(heading: &str, rel: &str) -> String {
    refuse(
        format!(
            "answering would remove \"{}\", whose text matches no approved row of {}",
            heading, rel
        ),
        Next::agent("says so in chat and stops").and_owner_runs(format!("adr approve {}", rel)),
    )
}

/// A question with no body yet: nothing to ask or answer.
pub fn not_written(heading: &str, report: &crate::moves::Report) -> String {
    format!("\"{}\" is [{}]; {}", heading, crate::status::QuestionStatus::NoBody.name(), report.tail())
}

/// `adr answer <id> <letter>` with no `@<version>` (per "Does an answer
/// carry the version of the question the owner read?"): a bare id is
/// refused before it is ever looked up, since there is nothing yet to
/// check it against.
/// `asked` is the heading and the version of the question `id` names, when
/// it names one that is asked: the command is then printed whole.
pub fn answer_missing_version(id: &str, asked: Option<(&str, &str)>) -> String {
    match asked {
        Some((heading, version)) => refuse(
            format!("{}: no version given", named(id, Some(heading))),
            crate::moves::clause(MoveKind::Answer { id: id.to_string(), version: version.to_string(), heading: heading.to_string() }),
        ),
        None => refuse(
            format!("{}: no version given", id),
            Next::owner_runs("adr show"),
        ),
    }
}

/// The version on an answer (from the terminal's `<id>@<version>` or the
/// page's hidden field) no longer matches the question's current text (per
/// "Does an answer carry the version of the question the owner read?"):
/// the question moved -- usually a re-ask after an edit -- since that
/// version was read.
///
/// The refusal never prints the current version: the version on an answer
/// is the owner's receipt of having read the text, so the owner is sent to
/// read the file as it is now.
pub fn stale_version(heading: &str, rel: &str) -> String {
    refuse(
        format!(
            "\"{}\" has moved since that version was read; show prints it as it is now",
            heading
        ),
        Next::owner_runs(format!("adr show {}", rel)),
    )
}

/// The version on `<id>@<version>` is not exactly `record::VERSION_LEN`
/// hex characters once lowercased (per "Does an answer carry the version
/// of the question the owner read?"): a malformed argument, never a
/// question that moved after being read, so it gets its own message
/// instead of `stale_version`.
pub fn invalid_version_format(version: &str) -> String {
    refuse(
        format!(
            "\"{}\" is not a valid version; show prints the id and version of every question that waits for an answer",
            version
        ),
        Next::owner_runs("adr show"),
    )
}

pub fn no_terms_line(id: &str, heading: &str) -> String {
    refuse(
        format!("{}: no terms line", named(id, Some(heading))),
        Next::agent("re-asks"),
    )
}

pub fn malformed_terms_line(id: &str, heading: &str) -> String {
    refuse(
        format!("{}: malformed terms line", named(id, Some(heading))),
        Next::agent("re-asks"),
    )
}

/// `word` is the word of the question the gone term was named by.
pub fn terms_line_names_gone_term(id: &str, heading: &str, uuid: &str, word: &str) -> String {
    refuse(
        format!(
            "{}: terms line names term {} that no longer exists",
            named(id, Some(heading)),
            named(uuid, Some(word))
        ),
        Next::agent("re-asks"),
    )
}

pub fn terms_line_names_retired_term(id: &str, heading: &str, name: &str) -> String {
    refuse(
        format!(
            "{}: terms line names retired term \"{}\"",
            named(id, Some(heading)),
            name
        ),
        Next::agent("re-asks"),
    )
}

/// Printed after `adr answer <id> refuse` succeeds: the heading is closed,
/// the plan around it likely needs another pass.
pub fn refused_summary(heading: &str, rel: &str) -> String {
    refuse(
        format!("refused \"{}\"", heading),
        Next::agent("re-plans").and_owner_runs(format!("adr approve {}", rel)),
    )
}

/// An identifier with the name the tool can put to it (a heading, a term)
/// right beside it, `<id> ("<name>")`, or the identifier alone when the
/// tool holds no name for it.
pub fn named(id: &str, name: Option<&str>) -> String {
    match name {
        Some(n) => format!("{} (\"{}\")", id, n),
        None => id.to_string(),
    }
}

// --- ask.rs -----------------------------------------------------------------

pub fn edited_after_answered(path: &str, heading: &str) -> String {
    refuse(
        format!("{}: \"{}\" was edited after being answered", path, heading),
        Next::agent("reverts the edit or writes a new question that supersedes it"),
    )
}

pub fn edited_after_refused(path: &str, heading: &str) -> String {
    refuse(
        format!("{}: \"{}\" was edited after being refused", path, heading),
        Next::agent("reverts the edit or writes a new question"),
    )
}

/// A plan that differs from its latest approval, with the moves of its file
/// (`moves::of_topic_file_in`): the same moves `show` names for the file, so
/// the refusal never names the owner's approve while approve would refuse
/// (per "Does a command compute a move of its own?", aac5c313 A).
pub fn plan_not_approved(rel: &str, status: FileStatus, report: &crate::moves::Report) -> String {
    format!("{}: plan is {}; {}", rel, status.name(), report.tail())
}

/// `plan_not_approved` for the prose above the first question.
pub fn context_not_approved(rel: &str, status: FileStatus, report: &crate::moves::Report) -> String {
    format!("{}: prose is {}; {}", rel, status.name(), report.tail())
}

pub fn missing_answer_set(rel: &str, dependent: &str, target: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" depends on \"{}\" without an answer set",
            rel, dependent, target
        ),
        Next::agent("writes =A, =A|B or =* on that depends line")
            .and_owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn set_on_term(rel: &str, dependent: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" depends on a term under an answer set, and a term has no options",
            rel, dependent
        ),
        Next::agent("removes the answer set from that depends entry")
            .and_owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn set_on_answered_question(rel: &str, dependent: &str, target: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" depends on \"{}\", which is answered, under an answer set",
            rel, dependent, target
        ),
        Next::agent("removes the answer set from that depends entry")
            .and_owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn question_without_body(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: \"{}\" is [{}]", rel, heading, crate::status::QuestionStatus::NoBody.name()),
        crate::moves::clause(MoveKind::WriteBodies { headings: vec![heading.to_string()] }),
    )
}

/// An `unrelated:` entry names no current decision, no term, no other
/// question of the file and no dropped question.
pub fn unrelated_names_nothing(rel: &str, heading: &str, entry: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\": the unrelated entry \"{}\" names no current decision, term or question of this file",
            rel, heading, entry
        ),
        Next::agent("writes what it names, or removes the entry"),
    )
}

/// An `unrelated:` entry is the heading of current decisions in several files.
pub fn unrelated_ambiguous(rel: &str, heading: &str, entry: &str, files: &[String]) -> String {
    refuse(
        format!(
            "{}: \"{}\": the unrelated entry \"{}\" is the heading of current decisions in {}",
            rel,
            heading,
            entry,
            files.join(", ")
        ),
        Next::agent("writes the identifier of the one it means"),
    )
}

/// A question not yet asked has no rows of the reader at one of its hashes
/// (`what` is `heading` or `heading with its body`).
pub fn plan_rows_missing(rel: &str, heading: &str, what: &str) -> String {
    refuse(
        format!("{}: \"{}\" has no reader rows for its {}", rel, heading, what),
        crate::moves::clause(MoveKind::Plan { rel: rel.to_string() }),
    )
}

/// A neighbour in the reader's rows is on no depends, supersedes or
/// unrelated line of the question.
pub fn plan_line_missing(rel: &str, heading: &str, kind: &str, name: &str) -> String {
    refuse(
        format!("{}: the {} \"{}\" is a neighbour of \"{}\" and is on no line", rel, kind, name, heading),
        crate::moves::clause(MoveKind::WriteLines { headings: vec![heading.to_string()] }),
    )
}

/// One question `adr plan` read the neighbours of, and the step it read them
/// at: its heading, or its heading with its body.
pub fn plan_question_read(heading: &str, what: &str) -> String {
    format!("\"{}\": {}", heading, what)
}

/// One neighbour of a question as every command prints it (`adr plan` for
/// what it read, `adr show --neighbours`): its kind and name, the step whose
/// text the reader read and the reader's word, the line the agent put it
/// on, and the reader's sentence of reason.
pub fn neighbour(kind: &str, name: &str, step: &str, word: &str, line: Option<&str>, reason: &str) -> String {
    let on = match line {
        Some(l) => format!("on the {} line", l),
        None => "no line".to_string(),
    };
    let text = format!("{} \"{}\": {} {}; {}", kind, name, step, word, on);
    if reason.is_empty() {
        text
    } else {
        format!("{}; {}", text, reason)
    }
}

/// One pair of current decisions the reader marked, as `adr read` prints it.
pub fn read_pair(word: &str, first: &str, second: &str, reason: &str) -> String {
    format!("{}: \"{}\" and \"{}\": {}", word, first, second, reason)
}

pub fn ask_supersedes_not_current(rel: &str, heading: &str, old: &str, name: Option<&str>) -> String {
    let target = if crate::record::is_uuid(old) {
        named(old, name)
    } else {
        format!("\"{}\"", old)
    };
    refuse(
        format!(
            "{}: \"{}\": supersedes {}, which is not a current decision",
            rel, heading, target
        ),
        Next::agent("removes it from the supersedes line or names a current decision"),
    )
}

/// The moment a refusal about the supersedes line is printed: it decides
/// whether the agent's move ends in asking again.
#[derive(Clone, Copy)]
pub enum SupersedesMoment {
    /// `approve`: nothing of the question has been asked yet.
    Approve,
    /// `ask` or `answer`: the question was, or is about to be, asked.
    Asked,
}

/// The supersedes line of `heading` leaves out current decisions standing
/// on what it replaces. `missing` are `(file, heading, id)`; a heading of
/// another file is printed with its file.
pub fn supersedes_leaves_out(
    rel: &str,
    heading: &str,
    missing: &[(String, String, String)],
    moment: SupersedesMoment,
) -> String {
    let names = missing
        .iter()
        .map(|(file, h, _)| {
            if file == rel {
                format!("\"{}\"", h)
            } else {
                format!("{}: \"{}\"", file, h)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    refuse(
        format!(
            "{}: \"{}\": supersedes a decision that current decisions stand on, and the line leaves out {}",
            rel, heading, names
        ),
        Next::agent(match moment {
            SupersedesMoment::Approve => "adds them to the supersedes line",
            SupersedesMoment::Asked => "adds them to the supersedes line and asks again",
        }),
    )
}

/// A question whose depends line names a decision that its own supersedes
/// line replaces.
pub fn stands_on_what_it_supersedes(rel: &str, heading: &str, target: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" stands on \"{}\", which it supersedes",
            rel, heading, target
        ),
        Next::agent("removes that entry from its depends line"),
    )
}

/// A question of the plan stands on a decision that another question of the
/// same plan supersedes.
pub fn stands_on_what_the_plan_supersedes(rel: &str, heading: &str, target: &str, replacer: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" stands on \"{}\", which \"{}\" supersedes",
            rel, heading, target, replacer
        ),
        Next::agent(format!(
            "makes it stand on \"{}\" or removes the entry from its depends line",
            replacer
        )),
    )
}

/// A `supersedes:` entry written as a heading that more than one current
/// decision carries.
pub fn ask_supersedes_ambiguous(rel: &str, heading: &str, entry: &str, files: &[String]) -> String {
    refuse(
        format!(
            "{}: \"{}\": supersedes \"{}\", which is a current decision in more than one place: {}",
            rel,
            heading,
            entry,
            files.join(", ")
        ),
        ambiguous_heading_next(),
    )
}

pub fn banned_word_in_candidate(rel: &str, heading: &str, suffix: &str) -> String {
    refuse(
        format!("{}: \"{}\": banned word{}", rel, heading, suffix),
        Next::agent("rewrites the line without the banned word"),
    )
}

pub fn pairs_no_term(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: \"{}\" uses no term", rel, heading),
        Next::agent("adds the term to adrdom/terms.md or rewrites the question"),
    )
}

pub fn pairs_no_term_at_write(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: \"{}\": uses no term", rel, heading),
        Next::agent("runs: adr ask"),
    )
}

pub fn no_id_line(rel: &str, heading: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\": no id line found to write the terms line after",
            rel, heading
        ),
        Next::owner_runs(format!("adr approve {}", rel)),
    )
}

// --- what is next in a topic file (`view::next_moves`) ---------------------
// One line per move, each opening with who makes it, in the words a
// refusal uses. `show` prints them under the file, `ask` when it has
// nothing to ask.


// --- show: the lines of its listing ----------------------------------------
// Labels and layout, no refusal among them: a note carries no `Next`.

use crate::status::FileStatus;

pub fn show_topic(topic: &str, rel: &str) -> String {
    if topic.is_empty() {
        return format!("# ({})", rel);
    }
    format!("# {} ({})", topic, rel)
}

/// A question of another topic file, as `show --dependents` names it.
pub fn show_question_of(heading: &str, rel: &str) -> String {
    format!("{} ({})", heading, rel)
}


/// Under a question not yet asked: the reader has recorded nothing for it.
pub fn show_no_records(rel: &str) -> String {
    format!("    neighbours: no reader records yet; {}", crate::moves::clause(MoveKind::Plan { rel: rel.to_string() }).render())
}

// --- ask: the lines of its summary ------------------------------------------

// --- adr --version -----------------------------------------------------------

/// What `adr --version` prints after the name. A build with the test-only
/// `owner-bypass` feature says so ahead of the number, so it is never taken
/// for the binary the owner uses. The ledger's version column keeps the
/// bare number (`version::VERSION`).
pub fn version_shown(version: &str, takes_owner_bypass: bool) -> String {
    if takes_owner_bypass {
        format!("test build with the owner bypass, {}", version)
    } else {
        version.to_string()
    }
}

// --- install / uninstall: the closing line ---------------------------------

// --- answer.rs / ban.rs / unban.rs / install.rs / uninstall.rs -------------

/// `id_arg` is the `<id>@<version>` the owner gave.
pub fn invalid_letter(letter: &str, id_arg: &str) -> String {
    refuse(
        format!(
            "invalid letter {}; an answer is one uppercase letter, or the word refuse",
            letter
        ),
        Next::owner_runs(format!("adr answer {} <letter>", id_arg)),
    )
}

pub fn no_such_term(name: &str) -> String {
    refuse(
        format!("no term named \"{}\" in adrdom/terms.md", name),
        Next::agent("names a term that exists in adrdom/terms.md"),
    )
}

/// `ban --term` with no terms file to look the name up in.
pub fn no_terms_file_for_term(name: &str) -> String {
    refuse(
        format!("cannot look up \"{}\": adrdom/terms.md is missing or cannot be read", name),
        Next::agent("writes adrdom/terms.md, or fixes it so it can be read as text"),
    )
}

pub fn already_in_use() -> String {
    refuse(
        "refusing: already in use".to_string(),
        Next::agent("removes the word from the repository first, or bans a different word"),
    )
}

// --- settings.json shape (install --claude / check / uninstall) -----------

/// The move on a failure of an owner command the owner cannot clear by
/// running it again (a file it cannot read or write): the agent fixes what
/// the error names, then the owner runs `command`.
pub fn after_fix_owner_runs(command: impl Into<String>) -> Next {
    Next::agent("fixes what the error names").and_owner_runs(command)
}

/// The agent's move after a command of its own failed to read or write what
/// the error names: fix that, then run `command` again.
pub fn after_fix_agent_runs(command: impl Into<String>) -> Next {
    Next::agent(format!("fixes what the error names, then runs: {}", command.into()))
}

/// The agent's move on a topic file `cmd` could not read at `rel`: a path
/// that does not exist is named again, any other failure is fixed first.
pub fn topic_file_unread(cmd: &str, rel: &str, e: &std::io::Error) -> Next {
    if e.kind() == std::io::ErrorKind::NotFound {
        Next::agent(format!("names a topic file under adrdom/decisions that exists, then runs: adr {} with it", cmd))
    } else {
        after_fix_agent_runs(format!("adr {} {}", cmd, rel))
    }
}

/// The move of a failure inside a step several commands share (the
/// embedding cache, the reader's file, the terms file as loaded): fix `what`,
/// then run the command that stopped, named at the head of its line, again.
pub fn fix_then_same_command(what: &str) -> Next {
    Next::agent(format!("fixes {}, then runs the same adr command again", what))
}

/// The agent's move on a file a command cannot take as it stands: fix the
/// file at `path` so `so` holds, then run `command` again.
pub fn fix_then(path: &str, so: &str, command: &str) -> Next {
    Next::agent(format!("fixes {} so {}, then runs: {}", path, so, command))
}

pub fn settings_not_json_object(path: &str, next: Next) -> String {
    refuse(format!("{} is not a JSON object", path), next)
}

pub fn settings_permissions_not_object(path: &str, next: Next) -> String {
    refuse(format!("{}: permissions is not an object", path), next)
}

pub fn settings_deny_not_array(path: &str, next: Next) -> String {
    refuse(format!("{}: permissions.deny is not an array", path), next)
}

// --- check/*.rs --------------------------------------------------------

pub fn check_parse_error(rel: &str, heading: &str, e: &str) -> String {
    refuse(
        format!("{}: {}: {}", rel, heading, e),
        Next::agent("fixes the topic file"),
    )
}

pub fn duplicate_id(id: &str, prev: &str, rel: &str, heading: &str) -> String {
    refuse(
        format!("duplicate id {} in {} and {}:{}", id, prev, rel, heading),
        Next::agent("changes one heading's id to a fresh uuid"),
    )
}

/// An asked/refused id missing from every current topic file, struck from
/// the plan of `rel`, the file awaiting approval it was approved in: the
/// owner's approve of that file writes its withdrawn row.
pub fn removed_from_plan(id: &str, heading: &str, event: &str, rel: &str) -> String {
    refuse(
        format!("{} (\"{}\", {}) was removed from the plan", id, heading, event),
        Next::owner_runs(format!("adr approve {}", rel)),
    )
}

/// An asked/refused id whose file is removed, its removal awaiting approval:
/// `next` is the move `removal_move` names for that file.
pub fn removed_with_file(id: &str, heading: &str, event: &str, next: Next) -> String {
    refuse(format!("{} (\"{}\", {}) was removed from the plan", id, heading, event), next)
}

/// `heading` is the heading the ledger holds for the question, from the
/// row written when it was asked.
pub fn ledger_id_not_found(id: &str, heading: Option<&str>) -> String {
    refuse(
        format!("ledger id {} not found in any topic file", named(id, heading)),
        Next::agent("checks why this id's heading is not in any topic file"),
    )
}

pub fn withdrawn_id_present(id: &str, heading: &str, rel: &str) -> String {
    refuse(
        format!("withdrawn id {} still present in {}", named(id, Some(heading)), rel),
        Next::agent("removes the heading, or leaves it when it is to come back")
            .and_owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn refusal_without_row(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: {}: refusal line without ledger row", rel, heading),
        Next::agent("removes the hand-written answer line; only adr answer writes one backed by the ledger"),
    )
}

/// A hand-written `**Answer:** <letter>` line with no genuine `answered`
/// ledger row behind it at all -- distinct from `answered_text_changed`,
/// whose answer line *is* backed by a real row at that same letter, but the
/// question's own text moved since. Removing the line is only the right fix
/// here, where nothing genuine is being thrown away.
pub fn answer_without_row(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: {}: answer line without ledger row", rel, heading),
        Next::agent("removes the hand-written answer line; only adr answer writes one backed by the ledger"),
    )
}

/// An answered section (a genuine `answered` row exists for its id, at the
/// letter its `**Answer:**` line names) whose current text no longer
/// hashes to that row: the answer is genuine, the question's text moved
/// since (per "Does check fail an answered question whose text changed?",
/// cd83e277 A). Never reported as `answer_without_row` -- that message's
/// fix, removing the answer line, would throw away a real answer over a
/// text edit; an answered question can't be re-asked either (per "Does a
/// decision change after it is answered?"), so the fix is to restore the
/// text as it was answered, from git, or to write a new question that
/// supersedes it when the change is meant.
pub fn answered_text_changed(rel: &str, heading: &str, id: &str) -> String {
    refuse(
        format!("{}: {}: text changed since being answered", rel, heading),
        Next::agent(format!(
            "restores it from git (e.g. git checkout -- {}), or writes a new question that supersedes {}",
            rel, id
        )),
    )
}

pub fn answered_row_without_line(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: {}: answered row without answer line", rel, heading),
        Next::agent("restores the **Answer:** line the ledger already recorded"),
    )
}

pub fn refused_row_without_line(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: {}: refused row without answer line", rel, heading),
        Next::agent("restores the **Answer:** refused line the ledger already recorded"),
    )
}

/// An asked question whose text no longer hashes as asked. `report`: the
/// findings and moves of the question's file (`moves::of_unasked_question_in`), the ones
/// `show` names for it (per "Does a command compute a move of its own?",
/// aac5c313 A).
pub fn edited_after_asking(rel: &str, heading: &str, report: &crate::moves::Report) -> String {
    format!("{}: {}: edited after asking; {}", rel, heading, report.tail())
}

pub fn answer_set_changed(rel: &str, dep_heading: &str, heading: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" answer set on \"{}\" changed after ask",
            rel, dep_heading, heading
        ),
        Next::agent("restores it, a change is a plan change"),
    )
}

/// A question whose standing ledger rulings name two or more different
/// options (per "Does check fail a question answered with two different
/// options?", e3200081-9995-4bbe-983f-2f430dc341e7): two branches each
/// answered (or one answered and one refused) the same question, and a
/// merge kept both rows. The topic file's own `**Answer:**` line only shows
/// one side, so this is caught from the ledger directly, never from the
/// file text. The move is always the same: the agent re-asks the question,
/// and the owner answers it again.
pub fn answer_conflict(rel: &str, heading: &str, id: &str, a: &str, b: &str) -> String {
    refuse(
        format!("{}: {}: two standing answers, {} and {}", rel, heading, a, b),
        Next::agent(format!("runs: adr ask {}", rel))
            .and_owner_runs(format!("adr answer {}@<version> <letter>", id)),
    )
}

/// The `"path:line"` location a failure names, one-based.
pub fn location(name: &str, line: usize) -> String {
    format!("{}:{}", name, line)
}

/// The `" (term {})"` suffix a banned-word hit carries when the word was
/// banned with `--term`; empty when it wasn't.
pub fn term_suffix(term: Option<&str>) -> String {
    term.map(|n| format!(" (term {})", n)).unwrap_or_default()
}

pub fn banned_word_at(location: &str, suffix: &str) -> String {
    refuse(
        format!("{}: banned word{}", location, suffix),
        Next::agent("rewrites the line without the banned word"),
    )
}

pub fn banned_missing_from_file(hash_prefix: &str) -> String {
    refuse(
        format!(
            "{} in ledger, missing from adrdom/state/banned.sha256",
            hash_prefix
        ),
        Next::agent("runs: adr ban --rebuild"),
    )
}

pub fn banned_missing_from_ledger(hash_prefix: &str) -> String {
    refuse(
        format!("{} in file, no ledger row", hash_prefix),
        Next::agent("runs: adr ban --rebuild"),
    )
}

/// A hash that stands in the file, was banned at some time, but the
/// ledger's latest row for it is `unbanned` -- the file still carries a
/// line the ledger already voided (per "Does check hold the ban list
/// equal to the ledger?"). `adr ban --rebuild` clears this: it rewrites the
/// file to exactly what the ledger currently says is banned, whatever the
/// file held before.
pub fn banned_unbanned_but_in_file(hash_prefix: &str) -> String {
    refuse(
        format!("{} in file, ledger says unbanned", hash_prefix),
        Next::agent("runs: adr ban --rebuild"),
    )
}

/// An asked/answered section that carries no `terms:` line at all. `ask`
/// always writes one, so an answered section can only lose it through an
/// edit made after the answer -- and an answered question can never be
/// re-asked (per "Does a decision change after it is answered?"), so
/// `answered` picks the legal move: re-asking while the question is still
/// only asked (`ask` will write the line the next time it runs), or
/// restoring the edit from git / superseding once it's answered (per "Does
/// check fail an answered question whose text changed?").
pub fn section_no_terms_line(rel: &str, heading: &str, id: &str, answered: bool) -> String {
    let next = if answered {
        Next::agent(format!(
            "restores the terms line from git (e.g. git checkout -- {}), or writes a question that supersedes {}",
            rel, id
        ))
    } else {
        Next::agent("re-asks so the terms line is written")
    };
    refuse(format!("{}: {}: no terms line", rel, heading), next)
}

/// approve of the terms file found an asked question whose `terms:` line
/// cannot be read, so it cannot tell which terms the question names.
pub fn asked_terms_line_unreadable(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: {}: terms line cannot be read", rel, heading),
        Next::agent(format!("restores {} from git (e.g. git checkout -- {})", rel, rel)),
    )
}

/// A section's `terms:` line is present but doesn't parse (per
/// `terms::validate_pairs`'s `Malformed`). Same answered/asked split as
/// `section_no_terms_line`.
pub fn section_terms_line_malformed(rel: &str, heading: &str, id: &str, answered: bool) -> String {
    let next = if answered {
        Next::agent(format!(
            "restores the terms line from git (e.g. git checkout -- {}), or writes a question that supersedes {}",
            rel, id
        ))
    } else {
        Next::agent("re-asks so the terms line is written correctly")
    };
    refuse(format!("{}: {}: malformed terms line", rel, heading), next)
}

/// A section's `terms:` line names a uuid that isn't in `adrdom/terms.md`
/// at all. Same split as `section_no_terms_line`: an asked section can
/// still be re-asked; an answered one is frozen, so the fix moves to
/// restoring the term in `adrdom/terms.md` (mirrors `term_removed_but_carried`)
/// or superseding the decision.
pub fn section_term_not_found(
    rel: &str,
    heading: &str,
    uuid: &str,
    word: &str,
    id: &str,
    answered: bool,
) -> String {
    let next = if answered {
        Next::agent(format!(
            "restores the term in adrdom/terms.md, or writes a question that supersedes {}",
            id
        ))
    } else {
        Next::agent("re-asks so the terms line matches current terms")
    };
    refuse(
        format!("{}: {}: term {} not found", rel, heading, named(uuid, Some(word))),
        next,
    )
}

/// A section's `terms:` line names a term that is now retired. Same split
/// as `section_no_terms_line`/`section_term_not_found`; the answered fix
/// mirrors `term_retired_but_carried`'s decision-dependent branch.
pub fn section_term_retired(rel: &str, heading: &str, name: &str, id: &str, answered: bool) -> String {
    let next = if answered {
        Next::agent(format!(
            "un-retires the term in adrdom/terms.md, or writes a question that supersedes {}",
            id
        ))
    } else {
        Next::agent("re-asks so the terms line drops the retired term")
    };
    refuse(
        format!("{}: {}: uses the retired term \"{}\"", rel, heading, name),
        next,
    )
}

/// A term's own `terms:` line (its definition's pairings) names a uuid
/// that no longer exists.
pub fn term_definition_term_not_found(name: &str, uuid: &str, word: &str) -> String {
    refuse(
        format!(
            "adrdom/terms.md: \"{}\": term {} not found",
            name,
            named(uuid, Some(word))
        ),
        Next::agent("rewords the definition so it uses a term that exists"),
    )
}

pub fn term_definition_term_retired(name: &str, retired_name: &str) -> String {
    refuse(
        format!(
            "adrdom/terms.md: \"{}\": uses the retired term \"{}\"",
            name, retired_name
        ),
        Next::agent("rewords the definition so it no longer uses the retired term"),
    )
}

/// A term's own `terms:` line (machine-written by `approve` from its
/// definition) doesn't parse at all (per `terms::validate_pairs`'s
/// `Malformed`) -- never an agent's line to hand-fix, since only `approve`
/// ever writes it; the move is the same re-approve that regenerates it.
pub fn term_definition_terms_line_malformed(name: &str) -> String {
    refuse(
        format!("adrdom/terms.md: \"{}\": malformed terms line", name),
        Next::owner_runs("adr approve adrdom/terms.md"),
    )
}

pub fn agents_md_stale() -> String {
    refuse(
        "AGENTS.md brief differs from this binary's".to_string(),
        Next::agent("runs: adr install"),
    )
}

pub fn agents_md_missing_brief() -> String {
    refuse(
        "AGENTS.md lacks the adrdom brief".to_string(),
        Next::agent("runs: adr install"),
    )
}

pub fn claude_md_missing_import() -> String {
    refuse(
        "CLAUDE.md does not import AGENTS.md".to_string(),
        Next::agent("runs: adr install --claude"),
    )
}

pub fn missing_deny_entry(entry: &str) -> String {
    refuse(
        format!(".claude/settings.json lacks deny entry {}", entry),
        Next::agent("runs: adr install --claude"),
    )
}

pub fn settings_unparseable(path: &str) -> String {
    refuse(format!("{} cannot be parsed", path), fix_then(".claude/settings.json", "it parses as JSON", "adr install --claude"))
}

/// `check`'s `install` check on a git hook (`.githooks/pre-commit` or
/// `.githooks/commit-msg`, per `path`): the file exists but its content
/// differs from what `adr install` writes.
pub fn hook_differs(path: &str) -> String {
    refuse(
        format!("{} differs from what install writes", path),
        Next::agent("runs: adr install"),
    )
}

/// Same check as `hook_differs`, but the hook's content is current and the
/// file is not executable, so git would skip it without a word; install
/// repairs the mode.
pub fn hook_not_executable(path: &str) -> String {
    refuse(
        format!("{} is not executable, git skips it", path),
        Next::agent("runs: adr install"),
    )
}

/// Same check as `hook_differs`, but the hook file is missing outright.
pub fn hook_missing(path: &str) -> String {
    refuse(
        format!("{} is missing", path),
        Next::agent("runs: adr install"),
    )
}

pub fn codex_rules_stale() -> String {
    refuse(
        ".codex/rules/adrdom.rules differs from this binary's".to_string(),
        Next::agent("runs: adr install --codex"),
    )
}

pub fn codex_rules_missing() -> String {
    refuse(
        ".codex/rules/adrdom.rules missing".to_string(),
        Next::agent("runs: adr install --codex"),
    )
}

/// `check`'s `install` check on `adrdom/.gitattributes`: the file exists
/// but its content differs from what `adr install` writes.
pub fn gitattributes_differs() -> String {
    refuse(
        "adrdom/.gitattributes differs from what install writes".to_string(),
        Next::agent("runs: adr install"),
    )
}

/// Same check as `gitattributes_differs`, but the file is missing
/// outright.
pub fn gitattributes_missing() -> String {
    refuse(
        "adrdom/.gitattributes is missing".to_string(),
        Next::agent("runs: adr install"),
    )
}

pub fn malformed_ledger_row(lineno: usize, raw: &str) -> String {
    refuse(
        format!(
            "line {}: malformed row: {}; the owner decides which rows are real before anything is committed",
            lineno, raw
        ),
        Next::owner_runs("git diff adrdom/state/ledger.tsv"),
    )
}

/// Per "Does check fail a ledger whose rows changed place?"
/// (c10c3a49-e993-4c9f-a46b-4a1ba3646dcf A): a HEAD row is missing or
/// changed -- a row's place in the file is free, so this never fires for
/// one that only moved.
pub fn ledger_not_append_only() -> String {
    refuse(
        "a row from HEAD's ledger is missing or changed in the working copy; a row may move, none may be dropped or edited, and what is missing or changed is restored by hand".to_string(),
        Next::owner_runs("git diff adrdom/state/ledger.tsv"),
    )
}

/// A merge (pending, via `MERGE_HEAD`, or already committed, via `HEAD`'s
/// second parent) is missing a ledger row one of the two branches wrote
/// (per "Does check at a merge look for the ledger rows of both
/// branches?"): the merge itself dropped a row instead of carrying it
/// forward, the one thing an append-only ledger can never do.
pub fn ledger_merge_drops_rows() -> String {
    refuse(
        "merge drops a branch's ledger rows".to_string(),
        Next::owner_runs(
            "the rows of the merged branch are restored from that branch, by hand, only by adding them back",
        ),
    )
}

/// HEAD is a merge commit, but at least one of its parents is not in this
/// repository at all -- a shallow clone, where the parent's own commit
/// object was never fetched. Distinct from a parent that is fully present
/// and simply holds no ledger file at that path (nothing to compare, no
/// failure): here there is nothing to read, so `ledger_merge_drops_rows`'s
/// own test -- every row either branch wrote survives into the merge --
/// cannot run at all, and this refuses instead of a check that never
/// looked passing anyway.
pub fn ledger_merge_parents_unreadable() -> String {
    refuse(
        "this merge commit's parents cannot be read; the clone is too shallow".to_string(),
        Next::agent("runs: git fetch --deepen=1"),
    )
}

pub fn ledger_answered_without_asked(id: &str, heading: Option<&str>) -> String {
    refuse(
        format!(
            "answered row for {} matches no asked row; an answered row needs the asked row it answers",
            named(id, heading)
        ),
        Next::owner_runs("git diff adrdom/state/ledger.tsv"),
    )
}

/// A `dropped` row whose detail is neither `<id> <letter>` nor `<id>`.
pub fn ledger_dropped_malformed(id: &str, detail: &str) -> String {
    refuse(
        format!("dropped row for {} has the detail \"{}\"; it holds an identifier and a letter, or an identifier", id, detail),
        Next::owner_runs("git diff adrdom/state/ledger.tsv"),
    )
}

/// A `dropped` row with no answer row (direct) or no dropped row (dependent) behind it.
pub fn ledger_dropped_without_cause(id: &str, cause: &str) -> String {
    refuse(
        format!("dropped row for {} has no cause: {}", id, cause),
        Next::owner_runs("git diff adrdom/state/ledger.tsv"),
    )
}

/// A `dropped` row whose id and hash match no per-question approved row.
pub fn ledger_dropped_not_approved(id: &str) -> String {
    refuse(
        format!("dropped row for {} matches no approved row of that question", id),
        Next::owner_runs("git diff adrdom/state/ledger.tsv"),
    )
}

/// A `dropped` row of a question with an earlier `answered` or `refused` row.
pub fn ledger_dropped_closed(id: &str, event: &str) -> String {
    refuse(
        format!("dropped row for {} follows its {} row", id, event),
        Next::owner_runs("git diff adrdom/state/ledger.tsv"),
    )
}

/// A dropped question that is still in a topic file.
pub fn ledger_dropped_still_in_file(id: &str, rel: &str) -> String {
    refuse(
        format!("{} has a dropped row but is still in {}", id, rel),
        Next::agent("removes the question from the file, or the owner's answer is run again"),
    )
}

/// At the commit that removes a dropped question: HEAD's text hashes to something else.
pub fn ledger_dropped_text_differs(id: &str) -> String {
    refuse(
        format!("dropped row for {} holds a hash that is not the hash of the text HEAD holds", id),
        Next::owner_runs("git diff adrdom/state/ledger.tsv"),
    )
}

/// At the commit that removes a dropped question: HEAD's set already held the letter.
pub fn ledger_dropped_set_holds_letter(id: &str, letter: &str) -> String {
    refuse(
        format!("dropped row for {} names the answer {}, which the question's set at HEAD holds", id, letter),
        Next::owner_runs("git diff adrdom/state/ledger.tsv"),
    )
}

/// At the commit that removes a dropped dependent: HEAD's text does not stand on the named question.
pub fn ledger_dropped_not_standing_on(id: &str, on: &str) -> String {
    refuse(
        format!("dropped row for {} says it stood on {}, which the question at HEAD does not", id, on),
        Next::owner_runs("git diff adrdom/state/ledger.tsv"),
    )
}

/// `adr check --push`, one line pre-push hands it: `local_ref`'s ledger, at
/// the sha about to be pushed, is missing a row `remote_ref` already holds
/// at its own sha (per "Does check look at the ledger before a push?",
/// 19e1c023-a678-4842-a414-bb0e45c41601 P). Same remedy as
/// `ledger_merge_drops_rows` -- the missing rows are restored, only by
/// adding them back -- since a push that drops a row is the same kind of
/// loss, just found before it reaches the remote instead of after a merge.
pub fn ledger_push_drops_rows(local_ref: &str, remote_ref: &str) -> String {
    refuse(
        format!(
            "push of {} would drop ledger rows the remote's {} already holds",
            local_ref, remote_ref
        ),
        Next::owner_runs("the rows already on the remote branch are restored, by hand, only by adding them back, then the push runs again"),
    )
}

/// `adr check --push`: the remote sha a pre-push line names is not a
/// commit this repository has -- a shallow clone, or a remote branch never
/// fetched -- so there is nothing local to compare it against, the same
/// blind spot `ledger_merge_parents_unreadable` refuses on for a shallow
/// merge.
pub fn ledger_push_remote_unreadable(remote_ref: &str) -> String {
    refuse(
        format!("{}'s remote commit cannot be read here", remote_ref),
        Next::agent("fetches the remote branch first"),
    )
}

/// `adr check --since <commit>` (per "Does check in CI compare the ledger
/// with the commit before the push?", a26876ba-5789-4d05-965e-853d00d9edd2
/// C): the ledger being judged is missing a row `<commit>`'s own ledger
/// already holds. Same remedy as `ledger_push_drops_rows`.
pub fn ledger_since_drops_rows(commit: &str) -> String {
    refuse(
        format!("ledger drops a row {} already holds", commit),
        Next::owner_runs("the rows already at that commit are restored, by hand, only by adding them back"),
    )
}

/// `adr check --since <commit>`: `<commit>` itself is not in this
/// repository (too shallow a fetch), so its ledger cannot be read at all.
pub fn ledger_since_unreadable(commit: &str) -> String {
    refuse(
        format!("{} cannot be read here", commit),
        Next::agent("fetches that commit first"),
    )
}

/// Per "Does check fail two ledger rows on one subject with the same
/// time?" (ce19c883-c61d-477b-a6d4-fee5134c6e9a T): two rows that are not
/// byte-identical, carry the same time, and compete for one subject --
/// `subject` already names which kind it is and the value (`"id <uuid>"`,
/// `"hash <prefix>"`, or `"file <path>"`). The remedy is always the same
/// command run again, since that writes a later row, and a later row
/// settles which one stands.
pub fn ledger_same_time_competition(time: &str, subject: &str) -> String {
    refuse(
        format!("two ledger rows at {} compete for {}", time, subject),
        Next::agent("runs the command again, so it writes a later row that stands"),
    )
}

/// `locations` are the `path:line` places the tag was found at.
pub fn stale_test(id: &str, heading: Option<&str>, locations: &[String]) -> String {
    let at = if locations.is_empty() {
        String::new()
    } else {
        format!(" at {}", locations.join(", "))
    };
    refuse(
        format!("{}: tied to a test and not a current decision{}", named(id, heading), at),
        Next::agent(format!("removes the comment \"adr: {}\" from the test", id)),
    )
}

pub fn duplicate_heading_in_file(heading: &str, rel: &str) -> String {
    refuse(
        format!("duplicate heading \"{}\" twice in {}", heading, rel),
        Next::agent("renames one of the headings"),
    )
}

pub fn duplicate_heading_across_files(heading: &str, rel1: &str, rel2: &str) -> String {
    refuse(
        format!(
            "duplicate heading \"{}\" in {} and {}",
            heading, rel1, rel2
        ),
        Next::agent("renames one of the headings"),
    )
}

/// The naming half of the same decision as `duplicate_heading_across_files`
/// (per "What names a topic file?"): a topic file whose own file name isn't lowercase ASCII
/// words joined by single hyphens (`repo::valid_topic_file_name`). Not part
/// of `record_refusals`: `approve` refuses it through
/// `rules::own_name_refusals` and `check` through its plan check, from
/// the one message. No `owner runs:` clause: the fixed
/// name is the agent's own choice, so there is no concrete command to name
/// yet -- the agent's move covers telling the owner what re-approval a
/// rename costs, when it costs one.
pub fn bad_topic_file_name(rel: &str) -> String {
    refuse(
        format!(
            "{}: file name is not lowercase words joined by single hyphens",
            rel
        ),
        Next::agent(
            "renames the file to lowercase ascii words joined by single hyphens (e.g. \"topic-name.md\"); if it had already been approved, the owner re-approves it under the new name, since approval is keyed to the path",
        ),
    )
}

/// The "named by it" half of the file name rule (per "What names a topic
/// file?"): a topic file whose name is well
/// formed (`repo::valid_topic_file_name`) but is not its h1 topic text
/// lowercased and hyphenated (`repo::expected_topic_file_stem`). Read the
/// same way as `bad_topic_file_name`.
pub fn topic_file_name_mismatch(rel: &str, topic: &str, expected: &str) -> String {
    refuse(
        format!(
            "{}: file name does not match its topic \"{}\"",
            rel, topic
        ),
        Next::agent(format!(
            "renames the file to {}.md or changes the topic line to match the name; if it had already been approved, the owner re-approves it under the new name, since approval is keyed to the path",
            expected
        )),
    )
}

/// The file name rule ("What names a topic file?") names a topic
/// file by its topic "in lowercase words joined by hyphens, with no number": a topic whose text lowercased and hyphenated
/// (`repo::expected_topic_file_stem`) holds a digit, or holds no letter at
/// all, has no name any file can carry, so no rename clears it and the
/// topic line itself has to change. `approve` refuses it through
/// `rules::own_name_refusals` and `check` through its plan check, from the
/// one message.
pub fn topic_unnameable(rel: &str, topic: &str) -> String {
    refuse(
        format!(
            "{}: topic \"{}\" has no file name: a name holds lowercase words only, no number",
            rel, topic
        ),
        Next::agent(
            "rewords the topic line to letters and spaces only and names the file by it; if the file had already been approved, the owner re-approves it, since approval is keyed to the path",
        ),
    )
}

// --- adrdom/blame/*.md (blame files) --------------------------------------
// Shared by `blame::do_approve_blame`'s refusal collection (per "What does
// a blame file hold?", "Does a blame file cover its whole origin?", "What
// does a blame row name?", "Does the owner approve a blame file?").

pub fn blame_shape_error(rel: &str, e: &str) -> String {
    refuse(format!("{}: {}", rel, e), Next::agent("fixes the blame file"))
}

pub fn blame_blob_not_found(rel: &str, blob: &str) -> String {
    refuse(
        format!("{}: origin is missing and blob {} is not in git", rel, blob),
        Next::agent("restores the origin at its path, or fixes the blob identifier"),
    )
}

/// The origin sits at its path but its content hashes to a different blob
/// than the blame file names (per "What does a blame file hold?").
pub fn blame_blob_differs(rel: &str, origin: &str) -> String {
    refuse(
        format!("{}: blob does not match the content of {}", rel, origin),
        Next::agent(&format!(
            "fixes the blob identifier to the hash of {}, or restores {} to the content the rows map",
            origin, origin
        )),
    )
}

fn range_text(start: usize, end: usize) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{}-{}", start, end)
    }
}

pub fn blame_coverage_gap(rel: &str, start: usize, end: usize) -> String {
    refuse(
        format!(
            "{}: line(s) {} of the origin are covered by no row",
            rel,
            range_text(start, end)
        ),
        Next::agent("adds a row covering them"),
    )
}

pub fn blame_range_beyond(rel: &str, start: usize, end: usize, n: usize) -> String {
    refuse(
        format!(
            "{}: row {} is beyond the origin's {} line(s)",
            rel,
            range_text(start, end),
            n
        ),
        Next::agent("fixes the range"),
    )
}

pub fn blame_row_unknown_heading(rel: &str, text: &str) -> String {
    refuse(
        format!(
            "{}: row names heading \"{}\" which is no question in any topic file",
            rel, text
        ),
        Next::agent("names a heading that exists, or rewrites the row"),
    )
}

pub fn blame_row_ambiguous_heading(rel: &str, text: &str, files: &[String]) -> String {
    refuse(
        format!(
            "{}: row names heading \"{}\" which is a question in more than one file: {}",
            rel,
            text,
            files.join(", ")
        ),
        Next::agent("names it by id instead"),
    )
}

pub fn blame_row_unknown_term(rel: &str, name: &str) -> String {
    refuse(
        format!("{}: row names term \"{}\" which is no term", rel, name),
        Next::agent("adds the term to adrdom/terms.md or rewrites the row"),
    )
}

pub fn blame_row_retired_term(rel: &str, name: &str) -> String {
    refuse(
        format!("{}: row names term \"{}\" which is retired", rel, name),
        Next::agent("names a current term or rewrites the row"),
    )
}

pub fn blame_row_missing_prose_stem(rel: &str, stem: &str) -> String {
    refuse(
        format!(
            "{}: row names topic file \"{}\" which does not exist",
            rel, stem
        ),
        Next::agent("names an existing topic file or rewrites the row"),
    )
}

/// `path` had no trailing `?`, so it was read as a copy-of-origin path
/// (per "What does a blame row name?"), not a heading -- the extra clause
/// tells the agent that a heading is the other way to spell this row, in
/// case the missing `?` was the actual mistake.
pub fn blame_row_copy_path_missing(rel: &str, path: &str) -> String {
    let hint = if path.trim_end().ends_with('?') {
        ""
    } else {
        "; a question is named by its heading ending in ?"
    };
    refuse(
        format!(
            "{}: row's copy target \"{}\" is no file in the working tree and has no blame file of its own{}",
            rel, path, hint
        ),
        Next::agent("names an existing origin or rewrites the row"),
    )
}

/// The origin line, or a `dropped: copy of <path>` row, names a path
/// under `repo::ADRDOM_DIR`, or one that exists but is not a regular file
/// (per the "origin" term: "a prose file outside adrdom whose text the
/// records take in"). Shared by the origin line itself (34f0202a A) and a
/// copy-of path (38216d39 A) -- `blame::valid_origin_path` is the one
/// place both `approve` and `check` read this rule from.
pub fn blame_bad_origin_path(rel: &str, path: &str) -> String {
    refuse(
        format!("{}: \"{}\" is not a regular file outside adrdom", rel, path),
        Next::agent("names a regular file outside adrdom"),
    )
}

/// A `dropped: copy of <path>` row names the origin its own blame file
/// maps (per "What does a blame row name?": a copy is of a question or of
/// another origin).
pub fn blame_row_copy_of_own_origin(rel: &str, start: usize, end: usize, path: &str) -> String {
    refuse(
        format!(
            "{}: row {}: copy target \"{}\" is this blame file's own origin",
            rel,
            range_text(start, end),
            path
        ),
        Next::agent("names the question or the other origin that says the same thing, or rewrites the row"),
    )
}

pub fn blame_row_unknown_destination(rel: &str, start: usize, end: usize, text: &str) -> String {
    refuse(
        format!(
            "{}: row {}: \"{}\": blame row names no known destination",
            rel,
            range_text(start, end),
            text
        ),
        Next::agent(
            "rewrites the row as one of: decision: <heading>, term: <name>, prose: <topic file>, dropped: dead, dropped: detail, dropped: copy of <heading or origin path>",
        ),
    )
}

pub fn blame_approved(rel: &str) -> String {
    format!("approve: {}: blame approved", rel)
}

// --- commands/check/blame.rs -----------------------------------------------
// Shared by `check`'s `blame` category (per "Does check hold a blame file
// to its origin?", 5fddeeb5 A; "Does an origin leave the repository while
// its blame file has an open row?", 4bb5ef1b A; "Does the tool stop while
// a blame file waits for approval?", 9c3cc17e A).

/// `git hash-object`'s own blob sha for `origin`'s current content no
/// longer matches the blame file's `blob:` line (5fddeeb5 A). A missing
/// origin is never reported here -- that is `blame_origin_missing`.
pub fn blame_blob_mismatch(rel: &str, origin: &str) -> String {
    refuse(
        format!("{}: blob no longer matches {}", rel, origin),
        Next::agent(format!("updates the rows for {}'s current content", origin))
            .and_owner_runs(format!("adr approve {}", rel)),
    )
}

/// `origin` is missing while `rel` still holds it hostage: either the
/// blame file itself is a draft (`"the blame file's own approval"` in
/// `open`), or a `decision:` row it carries is not yet answered (4bb5ef1b
/// A). `open` is sorted, deduped, and always non-empty when this is
/// called.
pub fn blame_origin_missing(rel: &str, origin: &str, open: &[String]) -> String {
    refuse(
        format!(
            "{}: origin {} is missing while {} still open: {}",
            rel,
            origin,
            if open.len() == 1 { "this is" } else { "these are" },
            open.join(", ")
        ),
        Next::agent(format!(
            "restores {} from git; it may go once the blame file is approved and every row above is answered",
            origin
        )),
    )
}

/// A blame file itself (`rel`) could not be read as text at all (per "Does
/// check hold a blame file to its origin?", 5fddeeb5 A: fail-open here
/// means never silently skipping the file). Distinct from `blame_shape_error`
/// -- there is no content to have a shape at all.
pub fn blame_file_unreadable(rel: &str) -> String {
    refuse(
        format!("{}: cannot be read as text", rel),
        Next::agent("fixes the file so it can be read as text (e.g. its encoding)"),
    )
}

/// `git hash-object --stdin` itself failed on `origin`'s current content
/// (5fddeeb5 A: the blob comparison must fail closed, never silently pass
/// when git could not be asked at all).
pub fn blame_hash_failed(rel: &str, origin: &str) -> String {
    refuse(
        format!("{}: git could not hash the current content of {}", rel, origin),
        Next::agent(format!("retries; git must be able to read and hash {}", origin)),
    )
}

/// `origin` sits at its path (per `Source::exists`) but its content could
/// not be read as text (4bb5ef1b A: distinct from the origin being
/// missing outright -- a present, unreadable origin is its own failure,
/// never routed through the missing-origin branch).
pub fn blame_origin_unreadable(rel: &str, origin: &str) -> String {
    refuse(
        format!("{}: origin {} exists but cannot be read as text", rel, origin),
        Next::agent(format!("fixes {} so it can be read as text (e.g. its encoding)", origin)),
    )
}

// --- serve/mod.rs ------------------------------------------------------

pub fn bind_failed(port: u16, e: impl Display) -> String {
    refuse(
        format!("failed to bind 127.0.0.1:{}: {}", port, e),
        Next::owner_runs("adr serve --port <port>"),
    )
}

pub fn missing_answer_selection(id: &str, version: &str) -> String {
    refuse(
        "missing answer selection".to_string(),
        crate::moves::clause(MoveKind::Answer { id: id.to_string(), version: version.to_string(), heading: String::new() }),
    )
}

// --- check notes ------------------------------------------------------------

/// An untested current decision, a note and never a failure (per "What does
/// check do with an untested current decision?"), with its move (per "Does
/// check print a line that names no move?", 3fc35d4f A): `mv` is the move's
/// text, `moves::of_untested`.
pub fn untested(rel: &str, heading: &str, id: &str, mv: &str) -> String {
    format!("{}: \"{}\" is answered and no tracked file holds \"adr: {}\"; {}", rel, heading, id, mv)
}

/// A topic file `check` cannot read: a `parse` finding.
pub fn topic_file_unreadable(path: &str, e: impl Display) -> String {
    refuse(
        format!("{}: cannot be read: {}", path, e),
        Next::agent("fixes the file so it can be read as text (e.g. its encoding)"),
    )
}

/// A terms edit that adds, changes or removes more than one entry (per
/// "Does approve accept a terms edit that changes more than one entry?").
pub fn terms_edit_too_wide(names: &[String], approved_before: bool) -> String {
    let list = names.iter().map(|n| format!("\"{}\"", n)).collect::<Vec<_>>().join(", ");
    let what = if approved_before {
        format!("{} entries differ from the last approval", names.len())
    } else {
        format!("{} entries are new, and the file has no approval yet", names.len())
    };
    refuse(
        format!("adrdom/terms.md: {}: {}; the most is 1", what, list),
        Next::agent("keeps one of these edits in the file and leaves the rest for a later approve"),
    )
}

pub fn plan_approved(rel: &str) -> String {
    format!("approve: {}: plan approved", rel)
}

pub fn context_approved(rel: &str) -> String {
    format!("approve: {}: prose approved", rel)
}

/// The finding on unban's input when `n` (never 0) of the words given were
/// not banned. A count and never the words: a word given here may be a
/// misspelling of a banned one.
pub fn unban_not_banned(n: usize) -> String {
    refuse(
        format!("unban: {} of the words given were not banned", n),
        Next::owner_runs("adr unban"),
    )
}

/// `check --message <file>` could not read the commit message file.
pub fn message_file_unreadable(path: &str, e: impl Display) -> String {
    refuse(
        format!("cannot read the commit message file {}: {}", path, e),
        Next::agent("passes a commit message file that can be read as text"),
    )
}

/// The first part of every retired-term move: the agent takes the flag off
/// the one entry, leaving every other edit in the file as it is.
fn unretire(term: &str) -> String {
    format!("takes `retired: yes` off \"{}\" in adrdom/terms.md", term)
}

/// The `Next` for `term_retired_but_carried` when the dependent is a
/// current decision: take the retire back (approving a topic file is
/// refused while terms.md waits for approval), then supersede the decision.
pub fn term_retired_decision_next(term: &str, id: &str, heading: &str) -> Next {
    Next::agent(format!(
        "{}, then supersedes {} (\"{}\") with a new question",
        unretire(term),
        id,
        heading
    ))
}

/// The `Next` for `term_retired_but_carried` when the dependent is an
/// asked question: reword it so it no longer names the term and ask it
/// again, or strike it from the plan.
pub fn term_retired_asked_next(heading: &str, term: &str, rel: &str) -> Next {
    Next::agent(format!(
        "{}, then rewords \"{}\" so it does not name \"{}\" and runs: adr ask {}, or strikes it from the plan",
        unretire(term),
        heading,
        term,
        rel
    ))
}

/// The `Next` for `term_retired_but_carried` when the dependent is a
/// heading of an approved plan that is not asked yet: reword or strike the
/// heading, then the owner approves the changed plan.
pub fn term_retired_heading_next(term: &str, rel: &str) -> Next {
    Next::agent(format!("{}, then rewords the heading or strikes it from the plan", unretire(term)))
        .and_owner_runs(format!("adr approve {}", rel))
}

// --- the embedding model and the install of adrdom/.gitignore -----------------

pub fn gitignore_differs() -> String {
    refuse(
        "adrdom/.gitignore differs from what install writes".to_string(),
        Next::agent("runs: adr install"),
    )
}

pub fn gitignore_missing() -> String {
    refuse(
        "adrdom/.gitignore is missing".to_string(),
        Next::agent("runs: adr install"),
    )
}

/// The embedding model is not on this machine and this build cannot fetch it.
#[cfg(any(not(feature = "model"), feature = "fake-model"))]
pub fn model_missing() -> String {
    refuse(
        "model: the embedding model is missing".to_string(),
        Next::agent("fetches it by running a build of adr with the model feature, or runs it on another machine"),
    )
}

/// The embedding model cannot be loaded from, or downloaded into, the cache directory.
#[cfg(feature = "model")]
pub fn model_load_failed(cache: &str, e: impl Display) -> String {
    refuse(
        format!("model: cannot load the embedding model from {}: {}", cache, e),
        Next::agent("fetches the model by checking the network and the cache directory, then retries, or runs it on another machine"),
    )
}

pub fn embed_failed(e: impl Display) -> String {
    refuse(
        format!("model: cannot embed the texts: {}", e),
        Next::agent("runs the same adr command again; if the error stays, deletes adrdom/cache first"),
    )
}

/// The reader is not on this machine: no `claude` program on PATH.
pub fn reader_missing() -> String {
    refuse(
        "reader: the claude program is missing".to_string(),
        Next::agent("installs the claude CLI, or runs it on another machine"),
    )
}

/// The reader gave no usable word for a pair of texts; nothing of that
/// call is kept.
pub fn reader_failed(neighbour: &str, e: impl Display) -> String {
    refuse(
        format!("reader: no word for the neighbour {}: {}", neighbour, e),
        Next::agent("retries; if it persists, runs it on another machine"),
    )
}

pub fn reader_write_failed(e: impl Display) -> String {
    write_failed(crate::repo::READER_PATH, e, fix_then_same_command("the reader file"))
}

pub fn malformed_reader_row(lineno: usize, raw: &str) -> String {
    refuse(
        format!(
            "line {}: malformed row in the reader file: {}; the owner decides which rows are real before anything is committed",
            lineno, raw
        ),
        Next::owner_runs("git diff adrdom/state/reader.tsv"),
    )
}

/// A HEAD row of the reader file is missing or changed.
pub fn reader_not_append_only() -> String {
    refuse(
        "a row from HEAD's reader file is missing or changed in the working copy; a row may move, none may be dropped or edited, and what is missing or changed is restored by hand".to_string(),
        Next::owner_runs("git diff adrdom/state/reader.tsv"),
    )
}

pub fn reader_merge_drops_rows() -> String {
    refuse(
        "merge drops a branch's reader rows".to_string(),
        Next::owner_runs(
            "the rows of the merged branch are restored from that branch, by hand, only by adding them back",
        ),
    )
}

pub fn reader_push_drops_rows(local_ref: &str, remote_ref: &str) -> String {
    refuse(
        format!(
            "push of {} would drop reader rows the remote's {} already holds",
            local_ref, remote_ref
        ),
        Next::owner_runs("the rows already on the remote branch are restored, by hand, only by adding them back, then the push runs again"),
    )
}

pub fn reader_since_drops_rows(commit: &str) -> String {
    refuse(
        format!("reader file drops a row {} already holds", commit),
        Next::owner_runs("the rows already at that commit are restored, by hand, only by adding them back"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::DeriveError;

    /// Every refusal/failure function in this module, called once with
    /// sample args: the type system already forces each to carry a `Next`,
    /// this is the second guard -- the rendered text must actually name
    /// the agent's move or a command the owner runs.
    // adr: 48f4647d-8c81-4e8a-a936-6b32f5fb0699
    // adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
    //
    // "every stop names the move that clears it and who makes it" /
    // "every refusal names what failed, the command that clears it and who
    // runs that command, the agent or the owner": every refusal/failure
    // built in this module is asserted below to contain either "; agent "
    // or "owner runs: ", so a stop with no legal move (the failure mode an
    // override would otherwise paper over) is caught here rather than left
    // for callers to notice.
    #[test]
    fn every_refusal_names_its_next_move() {
        let e = "boom";
        let approve_x = crate::moves::Report {
            findings: Vec::new(),
            moves: vec![crate::moves::Move::new(crate::moves::MoveKind::Approve { path: "x.md".to_string() })],
        };
        let outputs: Vec<String> = vec![
            write_failed("x", e, Next::agent("retries")),
            read_failed("x", e, Next::agent("retries")),
            update_failed("x", e, Next::agent("retries")),
            create_failed("x", e, Next::agent("retries")),
            parse_failed("x", e, Next::agent("retries")),
            stdin_read_failed(Next::agent("retries")),
            not_a_repo("ask", "", Actor::Agent),
            not_a_repo("approve", "adrdom/decisions/x.md", Actor::Owner),
            owner_refusal("approve", "adrdom/decisions/x.md"),
            owner_refusal("unban", ""),
            terms_missing(),
            terms_load_error(e, Next::agent("retries")),
            terms_shape_error(e),
            terms_not_approved("adrdom/decisions/x.md", FileStatus::Changed),
            terms_file_not_approved(FileStatus::Changed),
            terms_brief_missing(),
            terms_brief_differs(),
            terms_brief_not_staged(),
            circular_definition("a -> b -> a"),
            terms_edit_too_wide(&["a".to_string(), "b".to_string()], true),
            terms_edit_too_wide(&["a".to_string(), "b".to_string()], false),
            too_long("x.md", 3, "heading", "option", "21 words, the limit is 20"),
            term_removed_but_carried("uuid-1", Some("word"), "term \"x\"", None),
            term_removed_but_carried("uuid-1", Some("word"), "id-1: \"heading\"", Some(("id-1", "heading"))),
            term_retired_but_carried("x", "term \"y\"", term_reword_and_approve()),
            term_retired_but_carried("x", "id-1: \"heading\"", term_retired_decision_next("x", "id-1", "heading")),
            word_moved_off_term("word", "term", "term \"y\"", term_reword_and_approve()),
            word_moved_off_term(
                "word",
                "term",
                "id-1: \"heading\"",
                Next::agent("restores the wording in adrdom/terms.md so \"word\" still names \"term\", or writes a question that supersedes id-1"),
            ),
            definition_too_long("x", 3),
            terms_banned_word(3, ""),
            shape_error("x.md", "missing h1"),
            section_shape_error("x.md", "heading", "stray line 4"),
            tell("x.md", 3, "context", "claude", "says who said it"),
            no_depends_line("x.md", "heading"),
            no_depends_line_bare("heading"),
            refused_summary("heading", "x.md"),
            section_no_terms_line("x.md", "heading", "id-1", false),
            section_no_terms_line("x.md", "heading", "id-1", true),
            section_terms_line_malformed("x.md", "heading", "id-1", false),
            section_terms_line_malformed("x.md", "heading", "id-1", true),
            section_term_not_found("x.md", "heading", "uuid-1", "word", "id-1", false),
            section_term_not_found("x.md", "heading", "uuid-1", "word", "id-1", true),
            section_term_retired("x.md", "heading", "term", "id-1", false),
            section_term_retired("x.md", "heading", "term", "id-1", true),
            term_definition_term_not_found("term", "uuid-1", "word"),
            term_definition_term_retired("term", "retired-term"),
            term_definition_terms_line_malformed("term"),
            frozen_depends_edited("x.md", "heading"),
            dep_not_current_decision("x.md", "heading", "id-1", Some("name")),
            dep_retired_term_id("x.md", "heading", "id-1", Some("name")),
            dep_no_term("x.md", "heading", "widget"),
            bad_option_letter("x.md", "dependent", "target", 'Z'),
            crate::moves::answer_waits(
                "heading",
                "x.md",
                vec![
                    crate::moves::Waits::Unanswered {
                        heading: "target".to_string(),
                        answer: crate::moves::Move::new(crate::moves::MoveKind::Answer {
                            id: "id-1".to_string(),
                            version: "abcd1234".to_string(),
                            heading: "target".to_string(),
                        }),
                    },
                    crate::moves::Waits::RefusedTarget { heading: "refused".to_string() },
                    crate::moves::Waits::RefusedTier { heading: "other".to_string() },
                ],
            ),
            crate::moves::answer_waits(
                "heading",
                "x.md",
                vec![crate::moves::Waits::Asking { heading: "target".to_string(), report: approve_x.clone() }],
            ),
            tier_cap_exceeded("x.md", &["heading".to_string()], 0, 4),
            tier_cap_exceeded("x.md", &["heading".to_string()], 3, 4),
            derive_error("x.md", &DeriveError::SelfDep("heading".to_string())),
            answered_id_missing("id-1", "heading"),
            id_not_found("id-1"),
            already_refused("heading"),
            already_answered("heading"),
            letter_not_an_option('Z', "heading", &['A', 'B'], "id-1", "abcd1234"),
            not_in_asked_state("heading", &approve_x),
            answer_missing_version("id-1", None),
            answer_missing_version("id-1", Some(("heading", "12345678"))),
            stale_version("heading", "x.md"),
            already_withdrawn("heading"),
            already_superseded("heading"),
            not_asked_yet("heading", &approve_x),
            not_written("heading", &approve_x),
            dep_retired_term("x.md", "heading", "widget"),
            show_not_a_file_approve_takes("x.md"),
            invalid_version_format("Z0ZZZZZZ"),
            no_terms_line("id-1", "heading"),
            malformed_terms_line("id-1", "heading"),
            terms_line_names_gone_term("id-1", "heading", "uuid-1", "word"),
            terms_line_names_retired_term("id-1", "heading", "term"),
            edited_after_answered("x.md", "heading"),
            edited_after_refused("x.md", "heading"),
            plan_not_approved("x.md", FileStatus::New, &approve_x),
            context_not_approved("x.md", FileStatus::New, &approve_x),
            missing_answer_set("x.md", "dependent", "target"),
            set_on_answered_question("x.md", "dependent", "target"),
            question_without_body("x.md", "heading"),
            ask_supersedes_not_current("x.md", "heading", "id-1", Some("name")),
            banned_word_in_candidate("x.md", "heading", ""),
            pairs_no_term("x.md", "heading"),
            pairs_no_term_at_write("x.md", "heading"),
            no_id_line("x.md", "heading"),
            invalid_letter("z", "id-1@abcd1234"),
            no_such_term("widget"),
            already_in_use(),
            settings_not_json_object("path", Next::agent("retries")),
            settings_permissions_not_object("path", Next::agent("retries")),
            settings_deny_not_array("path", Next::agent("retries")),
            check_parse_error("x.md", "heading", "duplicate id line"),
            duplicate_id("id-1", "x.md:heading", "y.md", "heading"),
            removed_from_plan("id-1", "heading", "asked", "x.md"),
            ledger_id_not_found("id-1", Some("heading")),
            withdrawn_id_present("id-1", "heading", "x.md"),
            refusal_without_row("x.md", "heading"),
            answer_without_row("x.md", "heading"),
            answered_text_changed("x.md", "heading", "id-1"),
            answered_row_without_line("x.md", "heading"),
            refused_row_without_line("x.md", "heading"),
            edited_after_asking(
                "x.md",
                "heading",
                &crate::moves::Report {
                    findings: Vec::new(),
                    moves: vec![crate::moves::Move::new(crate::moves::MoveKind::Ask {
                        rel: "x.md".into(),
                        headings: vec!["heading".into()],
                    })],
                },
            ),
            answer_set_changed("x.md", "dependent", "heading"),
            answer_conflict("x.md", "heading", "id-1", "A", "B"),
            banned_word_at("x.md:3", ""),
            banned_missing_from_file("abcdef123456"),
            banned_missing_from_ledger("abcdef123456"),
            banned_unbanned_but_in_file("abcdef123456"),
            agents_md_stale(),
            agents_md_missing_brief(),
            claude_md_missing_import(),
            missing_deny_entry("Bash(adr approve*)"),
            settings_unparseable(".claude/settings.json"),
            codex_rules_stale(),
            codex_rules_missing(),
            hook_differs(".githooks/pre-commit"),
            hook_missing(".githooks/pre-commit"),
            hook_not_executable(".githooks/pre-commit"),
            gitattributes_differs(),
            gitattributes_missing(),
            gitignore_differs(),
            gitignore_missing(),
            #[cfg(any(not(feature = "model"), feature = "fake-model"))]
            model_missing(),
            #[cfg(feature = "model")]
            model_load_failed("adrdom/cache/model", "no network"),
            embed_failed("x"),
            reader_missing(),
            unrelated_names_nothing("x.md", "Does X?", "Y"),
            unrelated_ambiguous("x.md", "Does X?", "Y", &["a.md".to_string(), "b.md".to_string()]),
            plan_rows_missing("x.md", "Does X?", "heading"),
            plan_line_missing("x.md", "Does X?", "term", "widget"),
            reader_failed("id-1", "no answer"),
            reader_write_failed("disk full"),
            malformed_reader_row(3, "bad row"),
            reader_not_append_only(),
            reader_merge_drops_rows(),
            reader_since_drops_rows("abc123"),
            reader_push_drops_rows("refs/heads/feat", "refs/heads/main"),
            malformed_ledger_row(3, "bad row"),
            ledger_not_append_only(),
            ledger_merge_drops_rows(),
            ledger_merge_parents_unreadable(),
            ledger_answered_without_asked("id-1", Some("heading")),
            ledger_push_drops_rows("refs/heads/feat", "refs/heads/main"),
            ledger_push_remote_unreadable("refs/heads/main"),
            ledger_since_drops_rows("abc123"),
            ledger_since_unreadable("abc123"),
            ledger_same_time_competition("2024-01-01T00:00:00Z", "id id-1"),
            stale_test("id-1", None, &["x.rs:3".to_string()]),
            duplicate_heading_in_file("heading", "x.md"),
            duplicate_heading_across_files("heading", "x.md", "y.md"),
            bad_topic_file_name("adrdom/decisions/Bad_Name.md"),
            topic_file_name_mismatch("adrdom/decisions/x.md", "Y", "y"),
            topic_unnameable("adrdom/decisions/rule.md", "Rule 2"),
            bind_failed(7357, e),
            missing_answer_selection("id-1", "abcd1234"),
            blame_shape_error("adrdom/blame/x.md", "line 1: missing \"origin: <path>\""),
            blame_blob_not_found("adrdom/blame/x.md", "abc123"),
            blame_blob_differs("adrdom/blame/x.md", "docs/x.md"),
            blame_coverage_gap("adrdom/blame/x.md", 3, 5),
            blame_range_beyond("adrdom/blame/x.md", 1, 10, 5),
            blame_row_unknown_heading("adrdom/blame/x.md", "Does X?"),
            blame_row_ambiguous_heading("adrdom/blame/x.md", "Does X?", &["a.md".to_string(), "b.md".to_string()]),
            blame_row_unknown_term("adrdom/blame/x.md", "widget"),
            blame_row_retired_term("adrdom/blame/x.md", "widget"),
            blame_row_missing_prose_stem("adrdom/blame/x.md", "no-such-topic"),
            blame_row_copy_path_missing("adrdom/blame/x.md", "docs/gone.md"),
            blame_row_unknown_destination("adrdom/blame/x.md", 1, 1, "free text"),
            blame_blob_mismatch("adrdom/blame/x.md", "docs/x.md"),
            blame_origin_missing("adrdom/blame/x.md", "docs/x.md", &["Does X?".to_string()]),
            blame_bad_origin_path("adrdom/blame/x.md", "adrdom/blame/notes.md"),
            blame_file_unreadable("adrdom/blame/x.md"),
            blame_hash_failed("adrdom/blame/x.md", "docs/x.md"),
            blame_origin_unreadable("adrdom/blame/x.md", "docs/x.md"),
        ];
        for o in &outputs {
            assert!(
                o.contains("; agent ") || o.contains("owner runs: ") || o.contains("; owner changes into"),
                "no next move: {}",
                o
            );
        }
        // What follows `owner runs: ` is what the agent posts to the owner
        // and the owner pastes: a command and nothing after it. The three
        // refusals about ledger rows a push, a merge or a commit dropped
        // name no command, since no command restores a row.
        for o in &outputs {
            let Some((_, command)) = o.split_once("owner runs: ") else {
                continue;
            };
            // A shell comment after the command names the heading and is
            // not part of the command.
            let command = command.split(" #").next().unwrap_or(command);
            if command.starts_with("the rows ") {
                continue;
            }
            assert!(
                command.starts_with("adr ") || command.starts_with("git "),
                "not a command: {}",
                o
            );
            for prose in [" again", " if ", " to ", " and ", " with ", " first", ";", ","] {
                assert!(!command.contains(prose), "{:?} after the command: {}", prose, o);
            }
        }
    }

    // adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
    //
    // "every refusal names what failed, the command that clears it and who
    // runs that command". `section_term_not_found`'s answered branch used to
    // add a parenthetical suggesting the restored term be marked `retired:
    // yes` -- doing that immediately trips `section_term_retired` instead,
    // so it was never a real move. The next move is exactly the two given
    // in `section_term_retired`'s own answered branch: restore the term
    // (live), or write a question that supersedes the decision.
    #[test]
    fn section_term_not_found_answered_never_suggests_restoring_as_retired() {
        let out = section_term_not_found("x.md", "heading", "uuid-1", "word", "id-1", true);
        assert!(!out.contains("retired"), "{}", out);
        assert!(out.contains("restores the term in adrdom/terms.md"), "{}", out);
        assert!(
            out.contains("writes a question that supersedes id-1"),
            "{}",
            out
        );
    }

    // adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
    //
    // The moves ban and unban print when their input cannot be read, the
    // ban list or the ledger cannot be written, or they run outside a
    // repository: each names the command as it runs, its words read on its
    // input as always, with nothing after it that the shell would take for
    // an argument.
    #[test]
    fn ban_and_unban_moves_name_a_command_that_runs_as_printed() {
        let e = "boom";
        for (line, command) in [
            (owner_refusal("unban", ""), "owner runs: adr unban"),
            (not_a_repo("unban", "", Actor::Owner), "owner changes into a git repository and runs: adr unban"),
            (stdin_read_failed(after_fix_owner_runs("adr unban")), "owner runs: adr unban"),
            (stdin_read_failed(after_fix_agent_runs("adr ban")), "then runs: adr ban"),
            (update_failed("banned list", e, after_fix_agent_runs("adr ban")), "then runs: adr ban"),
            (update_failed("ledger", e, after_fix_agent_runs("adr ban")), "then runs: adr ban"),
            (update_failed("banned list", e, after_fix_agent_runs("adr ban --rebuild")), "then runs: adr ban --rebuild"),
        ] {
            assert!(line.ends_with(command), "{} should end with {}", line, command);
        }
    }

    #[test]
    fn untested_names_the_decision_its_place_and_its_move() {
        let line = untested("x.md", "heading", "id-1", "agent builds it");
        assert_eq!(line, "x.md: \"heading\" is answered and no tracked file holds \"adr: id-1\"; agent builds it");
    }
}

/// `adr approve <path>` on a path the ledger never approved: the file was
/// never approved, so its removal needs no owner act; the agent commits the
/// deletion.
pub fn removal_not_a_topic_file(rel: &str) -> String {
    refuse(
        format!(
            "{} is absent and was never approved: its removal needs no owner act and there is nothing to approve",
            rel
        ),
        Next::agent("commits the deletion"),
    )
}

/// A removed topic file still held a current decision or an open question.
pub fn removal_holds_open(rel: &str, hold: &crate::moves::RemovalHold) -> String {
    refuse(
        format!(
            "{} cannot be removed: {} is {}",
            rel,
            named(&hold.id, Some(&hold.heading)),
            crate::moves::hold_words(hold.status)
        ),
        crate::moves::removal_next(rel, std::slice::from_ref(hold)),
    )
}


