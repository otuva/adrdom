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

    pub fn and_owner_runs(mut self, command: impl Into<String>) -> Next {
        self.0.push((Actor::Owner, format!("runs: {}", command.into())));
        self
    }

    fn render(&self) -> String {
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

pub fn parse_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("{}: {}", target, e), next)
}

pub fn stdin_read_failed(next: Next) -> String {
    refuse("failed to read stdin".to_string(), next)
}

// --- repo.rs ----------------------------------------------------------------

/// `<cmd>: not inside a git repository; ...` -- printed by every command
/// before it does anything else. `actor` is who was already confirmed to
/// be running `cmd` (the owner, for the five owner-only commands, since
/// `owner_authorized` already gated by the time this fires; the agent for
/// every other command).
pub fn not_a_repo(cmd: &str, actor: Actor) -> String {
    let next = match actor {
        Actor::Agent => Next::agent("changes into the repository and retries"),
        Actor::Owner => Next::owner_runs(format!("adr {} again from inside the repository", cmd)),
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
pub fn terms_not_approved(rel: &str) -> String {
    refuse(
        format!("{}: terms not approved", rel),
        Next::owner_runs("adr approve adrdom/terms.md"),
    )
}

/// `adrdom/terms.md` exists and parses, but isn't approved at its current
/// hash (per "Does the tool stop while the terms file waits for approval?",
/// aa475aa3 A): `check`'s own report of the same fact `terms_not_approved`
/// names from a dependent topic file -- this one names the file itself,
/// since `check` judges it on its own, not on behalf of one dependent.
pub fn terms_file_not_approved() -> String {
    refuse(
        "adrdom/terms.md not approved".to_string(),
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
pub fn term_removed_but_carried(uuid: &str, source: &str) -> String {
    refuse(
        format!(
            "adrdom/terms.md: term {} removed but carried by {}",
            uuid, source
        ),
        Next::owner_runs("adr approve adrdom/terms.md after restoring the term, marked retired: yes"),
    )
}

/// A dependent still names a term that is now retired. `next` differs by
/// dependent kind: a decision supersedes itself with a new question
/// (built by the caller, which knows the decision's id); a term's own
/// definition is reworded by the agent and re-approved by the owner --
/// terms are never superseded (per "Does a retired term keep its words?"; the fix for a term-dependent is
/// always this same pair, so it's fixed here).
pub fn term_retired_but_carried(name: &str, source: &str, next: Next) -> String {
    refuse(
        format!(
            "adrdom/terms.md: term \"{}\" retired but carried by {}",
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
            "adrdom/terms.md: word \"{}\" moved off term \"{}\", carried by {}",
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
// Shared by `actions::record_refusals` (approve + check) and, for the
// derive-error variant, by `ask` as well.

pub fn shape_error(rel: &str, e: &str) -> String {
    refuse(
        format!("{}: {}", rel, e),
        Next::agent("fixes the topic file"),
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

pub fn dep_not_current_decision(rel: &str, heading: &str, id: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" depends on {} which is not a current decision",
            rel, heading, id
        ),
        Next::agent("names a current decision or removes the line"),
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

pub fn derive_error(rel: &str, e: &DeriveError) -> String {
    refuse(derive_what(rel, e), derive_next(e))
}

/// `ask`'s own wording: a derivation failure means `approve` would also
/// refuse on this same file, so both fixes are named together.
pub fn derive_error_ask(rel: &str, e: &DeriveError) -> String {
    refuse(
        derive_what(rel, e),
        Next::agent("fixes the topic file").and_owner_runs(format!("adr approve {}", rel)),
    )
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
        format!("id {} not found in any topic file", id),
        Next::owner_runs("adr answer with a real id"),
    )
}

pub fn already_refused(heading: &str) -> String {
    refuse(
        format!("\"{}\" is already refused", heading),
        Next::owner_runs("adr answer a different, still-open id"),
    )
}

pub fn already_answered(heading: &str) -> String {
    refuse(
        format!("\"{}\" is already answered", heading),
        Next::owner_runs("adr answer a different, still-open id"),
    )
}

pub fn letter_not_an_option(letter: char, heading: &str) -> String {
    refuse(
        format!(
            "letter {} does not name an option in \"{}\"",
            letter, heading
        ),
        Next::owner_runs("adr answer with a letter the question actually has"),
    )
}

pub fn not_in_asked_state(heading: &str) -> String {
    refuse(
        format!(
            "\"{}\" is not currently in the 'asked' state at its current hash",
            heading
        ),
        Next::agent("runs adr ask so it is asked again at the current hash"),
    )
}

/// `adr answer <id> <letter>` with no `@<version>` (per "Does an answer
/// carry the version of the question the owner read?"): a bare id is
/// refused before it is ever looked up, since there is nothing yet to
/// check it against.
pub fn answer_missing_version(id: &str) -> String {
    refuse(
        format!("{}: no version given", id),
        Next::owner_runs("adr show to read the question and answer with the printed id@version"),
    )
}

/// The version on an answer (from the terminal's `<id>@<version>` or the
/// page's hidden field) no longer matches the question's current text (per
/// "Does an answer carry the version of the question the owner read?"):
/// the question moved -- usually a re-ask after an edit -- since that
/// version was read.
pub fn stale_version(heading: &str) -> String {
    refuse(
        format!("\"{}\" has moved since that version was read", heading),
        Next::owner_runs("adr show to read it again and answer with the printed id@version"),
    )
}

/// The version on `<id>@<version>` is not exactly `record::VERSION_LEN`
/// hex characters once lowercased (per "Does an answer carry the version
/// of the question the owner read?"): a malformed argument, never a
/// question that moved after being read, so it gets its own message
/// instead of `stale_version`.
pub fn invalid_version_format(version: &str) -> String {
    refuse(
        format!("\"{}\" is not a valid version", version),
        Next::owner_runs("adr show and copies the printed id@version"),
    )
}

pub fn no_terms_line(id: &str) -> String {
    refuse(format!("{}: no terms line", id), Next::agent("re-asks"))
}

pub fn malformed_terms_line(id: &str) -> String {
    refuse(
        format!("{}: malformed terms line", id),
        Next::agent("re-asks"),
    )
}

pub fn terms_line_names_gone_term(id: &str, uuid: &str) -> String {
    refuse(
        format!(
            "{}: terms line names term {} that no longer exists",
            id, uuid
        ),
        Next::agent("re-asks"),
    )
}

pub fn terms_line_names_retired_term(id: &str, name: &str) -> String {
    refuse(
        format!("{}: terms line names retired term \"{}\"", id, name),
        Next::agent("re-asks"),
    )
}

/// Printed after `adr answer <id>@<version> <letter>` succeeds: the heading
/// and the decision sentence the owner picked. Not a refusal, so it never
/// goes through `refuse()` -- there is no next move to name.
pub fn answered_summary(heading: &str, sentence: &str) -> String {
    format!("answered: {}\n  {}", heading, sentence)
}

/// Printed after `adr answer <id> refuse` succeeds: the heading is closed,
/// the plan around it likely needs another pass.
pub fn refused_summary(heading: &str, rel: &str) -> String {
    refuse(
        format!("refused \"{}\"", heading),
        Next::agent("re-plans").and_owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn supersedes_not_current(id: &str) -> String {
    refuse(
        format!("supersedes target {} is not a current decision", id),
        Next::agent("fixes the supersedes line"),
    )
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

pub fn plan_not_approved(rel: &str) -> String {
    refuse(
        format!("{}: plan not approved", rel),
        Next::owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn context_not_approved(rel: &str) -> String {
    refuse(
        format!("{}: context not approved", rel),
        Next::owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn missing_answer_set(rel: &str, dependent: &str, target: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" depends on \"{}\" without an answer set",
            rel, dependent, target
        ),
        Next::agent("writes =A, =A|B or =* on that depends line"),
    )
}

pub fn depends_on_refused(rel: &str, heading: &str, target: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\" stands on refused \"{}\"",
            rel, heading, target
        ),
        Next::agent("re-plans").and_owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn ask_supersedes_not_current(rel: &str, heading: &str, old: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\": supersedes {}, which is not a current decision",
            rel, heading, old
        ),
        Next::agent("fixes the supersedes line"),
    )
}

pub fn tier_blocked_refused(
    rel: &str,
    heading: &str,
    cand_tier: usize,
    open_tier: usize,
    blocker: &str,
) -> String {
    refuse(
        format!(
            "{}: \"{}\" is in tier {}; tier {} is open, waiting on refused \"{}\"",
            rel, heading, cand_tier, open_tier, blocker
        ),
        Next::agent("re-plans").and_owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn tier_blocked_waiting(
    rel: &str,
    heading: &str,
    cand_tier: usize,
    open_tier: usize,
    blocker: &str,
    blocker_id: &str,
    blocker_version: &str,
) -> String {
    refuse(
        format!(
            "{}: \"{}\" is in tier {}; tier {} is open, waiting on \"{}\"",
            rel, heading, cand_tier, open_tier, blocker
        ),
        Next::owner_runs(format!("adr answer {}@{} <letter> on it first", blocker_id, blocker_version)),
    )
}

/// The open tier still holds a bare skeleton heading (no body yet), so a
/// later tier's candidate waits on the agent, not the owner, to write one.
pub fn tier_blocked_unasked(
    rel: &str,
    heading: &str,
    cand_tier: usize,
    open_tier: usize,
    blocker: &str,
) -> String {
    tier_blocked_on(rel, heading, cand_tier, open_tier, blocker, "writes its body and asks it first")
}

/// The open tier still holds a question with a real body -- `state::
/// tier_blocker` only ever returns one, so this is never a skeleton --
/// that nobody has run `ask` on yet, so a later tier's candidate waits on
/// the agent to ask it, not to write a body that already exists.
pub fn tier_blocked_needs_ask(
    rel: &str,
    heading: &str,
    cand_tier: usize,
    open_tier: usize,
    blocker: &str,
) -> String {
    tier_blocked_on(rel, heading, cand_tier, open_tier, blocker, "asks it first")
}

/// The one sentence both tier-blocked refusals share; only the agent's
/// next move differs.
fn tier_blocked_on(
    rel: &str,
    heading: &str,
    cand_tier: usize,
    open_tier: usize,
    blocker: &str,
    next: &str,
) -> String {
    refuse(
        format!(
            "{}: \"{}\" is in tier {}; tier {} is open, waiting on unasked \"{}\"",
            rel, heading, cand_tier, open_tier, blocker
        ),
        Next::agent(next),
    )
}

pub fn moot_candidate(
    rel: &str,
    heading: &str,
    target: &str,
    set_str: &str,
    letter: char,
) -> String {
    refuse(
        format!(
            "{}: \"{}\" is moot: stands on \"{}\"={}, answered {}",
            rel, heading, target, set_str, letter
        ),
        Next::agent("drops or replaces the heading").and_owner_runs(format!("adr approve {}", rel)),
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
        format!("{}: \"{}\" pairs no term", rel, heading),
        Next::agent("adds the term to adrdom/terms.md or rewrites the question"),
    )
}

pub fn pairs_no_term_at_write(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: \"{}\": pairs no term at write time", rel, heading),
        Next::agent("re-runs adr ask"),
    )
}

pub fn no_id_line(rel: &str, heading: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\": no id line found to write the terms line after",
            rel, heading
        ),
        Next::owner_runs(format!("adr approve {} again", rel)),
    )
}

/// `ask` prints the file's next moves (`view::next_moves`) under this line.
pub fn nothing_to_ask(path: &str) -> String {
    format!("nothing to ask in {}", path)
}

// --- what is next in a topic file (`view::next_moves`) ---------------------
// One line per move, each opening with who makes it, in the words a
// refusal uses. `show` prints them under the file, `ask` when it has
// nothing to ask.

fn quoted(headings: &[String]) -> String {
    headings
        .iter()
        .map(|h| format!("\"{}\"", h))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn next_plan_unresolved() -> String {
    Next::agent("runs adr check and clears what it names in the plan").render()
}

/// A plan that waits for its approval and that `approve` would take.
pub fn next_plan_unapproved(rel: &str) -> String {
    Next::owner_runs(format!("adr approve {}", rel)).render()
}

/// A refusal `approve` would print for the plan, whole, with the move it
/// names: the agent reads it here and not among the lines of `check`.
pub fn next_clears(refusal: &str) -> String {
    Next::agent(format!("clears: {}", refusal)).render()
}

pub fn next_write_body(heading: &str) -> String {
    Next::agent(format!("writes the body of \"{}\"", heading)).render()
}

pub fn next_answer_set(dependent: &str, target: &str) -> String {
    Next::agent(format!(
        "writes =A, =A|B or =* on the depends line of \"{}\" where it names \"{}\"",
        dependent, target
    ))
    .render()
}

pub fn next_ask(rel: &str, headings: &[String]) -> String {
    Next::agent(format!("runs adr ask {} for {}", rel, quoted(headings))).render()
}

pub fn next_answer(id: &str, version: &str, heading: &str) -> String {
    format!(
        "{} for \"{}\"",
        Next::owner_runs(format!("adr answer {}@{} <letter>", id, version)).render(),
        heading
    )
}

pub fn next_refused(heading: &str) -> String {
    Next::agent(format!(
        "changes the plan: \"{}\" is refused and holds the questions that come after it",
        heading
    ))
    .render()
}

pub fn next_moot(rel: &str, heading: &str) -> String {
    Next::agent(format!("drops or replaces \"{}\", which is moot", heading))
        .and_owner_runs(format!("adr approve {}", rel))
        .render()
}

pub fn next_build(heading: &str, id: &str) -> String {
    Next::agent(format!(
        "builds \"{}\" and puts \"adr: {}\" in a test that shows it",
        heading, id
    ))
    .render()
}

pub fn next_build_waits(heading: &str, beside: &str) -> String {
    format!(
        "not yet: \"{}\" is answered and is built once \"{}\" is answered",
        heading, beside
    )
}

/// The question beside it is refused: no answer is coming, the plan changes.
pub fn next_build_waits_refused(heading: &str, beside: &str) -> String {
    format!(
        "not yet: \"{}\" is answered and is built once the plan changes, \"{}\" beside it is refused",
        heading, beside
    )
}

pub fn next_later(headings: &[String]) -> String {
    format!("not yet, after the questions named above: {}", quoted(headings))
}

// --- answer.rs / ban.rs / unban.rs / install.rs / uninstall.rs -------------

pub fn invalid_letter(letter: &str) -> String {
    refuse(
        format!("invalid letter {}", letter),
        Next::owner_runs("adr answer with a single uppercase letter, or \"refuse\""),
    )
}

pub fn no_such_term(name: &str) -> String {
    refuse(
        format!("no term named \"{}\" in adrdom/terms.md", name),
        Next::agent("names a term that exists in adrdom/terms.md"),
    )
}

pub fn already_in_use() -> String {
    refuse(
        "refusing: already in use".to_string(),
        Next::agent("removes the word from the repo text first, or bans a different word"),
    )
}

// --- settings.json shape (install --claude / check / uninstall) -----------

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

pub fn heading_plan_not_approved(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: {}: plan not approved", rel, heading),
        Next::owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn duplicate_id(id: &str, prev: &str, rel: &str, heading: &str) -> String {
    refuse(
        format!("duplicate id {} in {} and {}:{}", id, prev, rel, heading),
        Next::agent("changes one heading's id to a fresh uuid"),
    )
}

/// An asked/refused id missing from every current topic file, where at
/// least one currently-unapproved file was previously approved.
/// `removed_file` names that file when exactly one such file exists; when
/// more than one does, which one dropped this id can't be told apart, so
/// the owner is pointed at the generic fix instead of a guessed file.
pub fn removed_from_plan(id: &str, heading: &str, event: &str, removed_file: Option<&str>) -> String {
    let what = format!("{} (\"{}\", {}) was removed from the plan", id, heading, event);
    match removed_file {
        Some(rel) => refuse(what, Next::owner_runs(format!("adr approve {} again", rel))),
        None => refuse(what, Next::owner_runs("adr approve <the edited topic file>")),
    }
}

pub fn ledger_id_not_found(id: &str) -> String {
    refuse(
        format!("ledger id {} not found in any topic file", id),
        Next::agent("checks why this id's heading is not in any topic file"),
    )
}

pub fn withdrawn_id_present(id: &str, rel: &str) -> String {
    refuse(
        format!("withdrawn id {} still present in {}", id, rel),
        Next::agent("removes the heading").and_owner_runs(format!(
            "adr approve {} if it should come back",
            rel
        )),
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

pub fn edited_after_asking(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: {}: edited after asking", rel, heading),
        Next::agent("runs adr ask"),
    )
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
        Next::agent(format!("runs adr ask {}", rel))
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
pub fn section_term_not_found(rel: &str, heading: &str, uuid: &str, id: &str, answered: bool) -> String {
    let next = if answered {
        Next::agent(format!(
            "restores the term in adrdom/terms.md, or writes a question that supersedes {} with a new terms pairing",
            id
        ))
    } else {
        Next::agent("re-asks so the terms line matches current terms")
    };
    refuse(
        format!("{}: {}: term {} not found", rel, heading, uuid),
        next,
    )
}

/// A section's `terms:` line names a term that is now retired. Same split
/// as `section_no_terms_line`/`section_term_not_found`; the answered fix
/// mirrors `term_retired_but_carried`'s decision-dependent branch.
pub fn section_term_retired(rel: &str, heading: &str, name: &str, id: &str, answered: bool) -> String {
    let next = if answered {
        Next::agent(format!(
            "un-retires the term in adrdom/terms.md, or writes a question that supersedes {} with a new terms pairing",
            id
        ))
    } else {
        Next::agent("re-asks so the terms line drops the retired term")
    };
    refuse(
        format!("{}: {}: bound term \"{}\" is retired", rel, heading, name),
        next,
    )
}

/// A term's own `terms:` line (its definition's pairings) names a uuid
/// that no longer exists.
pub fn term_definition_term_not_found(name: &str, uuid: &str) -> String {
    refuse(
        format!("adrdom/terms.md: \"{}\": term {} not found", name, uuid),
        Next::agent("rewords the definition so it pairs a term that exists"),
    )
}

pub fn term_definition_term_retired(name: &str, retired_name: &str) -> String {
    refuse(
        format!(
            "adrdom/terms.md: \"{}\": bound term \"{}\" is retired",
            name, retired_name
        ),
        Next::agent("rewords the definition so it no longer pairs the retired term"),
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
    refuse(
        format!("{} cannot be parsed", path),
        Next::agent("runs: adr install --claude"),
    )
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
        format!("line {}: malformed row: {}", lineno, raw),
        Next::owner_runs("git diff adrdom/state/ledger.tsv and decides which rows are real before anything is committed"),
    )
}

/// Per "Does check fail a ledger whose rows changed place?"
/// (c10c3a49-e993-4c9f-a46b-4a1ba3646dcf A): a HEAD row is missing or
/// changed -- a row's place in the file is free, so this never fires for
/// one that only moved.
pub fn ledger_not_append_only() -> String {
    refuse(
        "a row from HEAD's ledger is missing or changed in the working copy".to_string(),
        Next::owner_runs("git diff adrdom/state/ledger.tsv; a row may move, but none may be dropped or edited -- restore by hand whatever is missing or changed"),
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

pub fn ledger_asked_id_missing(id: &str) -> String {
    refuse(
        format!("asked row id {} not found in any topic file", id),
        Next::agent("restores the heading in the topic file").and_owner_runs("adr approve <file> if the heading was removed on purpose"),
    )
}

pub fn ledger_answered_without_asked(id: &str) -> String {
    refuse(
        format!(
            "answered row for {} has no matching earlier asked hash",
            id
        ),
        Next::owner_runs("git diff adrdom/state/ledger.tsv; an answered row needs the asked row it answers"),
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

pub fn untested(id: &str) -> String {
    refuse(
        format!("{}: untested", id),
        Next::agent(format!(
            "writes a test that shows the code follows this decision and puts \"adr: {}\" in it",
            id
        )),
    )
}

pub fn stale_test(id: &str) -> String {
    refuse(
        format!("{}: stale (not a current decision)", id),
        Next::agent(format!("removes the stale \"adr: {}\" tag from the test", id)),
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
/// (76bd7ef0 A): a topic file whose own file name isn't lowercase ASCII
/// words joined by single hyphens (`repo::valid_topic_file_name`). Check-
/// only, like its duplicate-heading sibling -- neither is part of
/// `record_refusals`, so `approve` doesn't stop on either one; naming, like
/// cross-file duplication, needs every file in the directory, not just the
/// one file `approve` was pointed at. No `owner runs:` clause: the fixed
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

/// The "named by it" half of 76bd7ef0 A: a topic file whose name is well
/// formed (`repo::valid_topic_file_name`) but is not its h1 topic text
/// lowercased and hyphenated (`repo::expected_topic_file_stem`). Check-only,
/// like `bad_topic_file_name`.
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

/// 76bd7ef0 A names a topic file by its topic "in lowercase words joined by
/// hyphens, with no number": a topic whose text lowercased and hyphenated
/// (`repo::expected_topic_file_stem`) holds a digit, or holds no letter at
/// all, has no name any file can carry, so no rename clears it and the
/// topic line itself has to change. Check-only, like `bad_topic_file_name`.
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

pub fn body_under_unapproved_heading(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: body under unapproved heading \"{}\"", rel, heading),
        Next::owner_runs(format!("adr approve {}", rel)),
    )
}

pub fn bodies_under_unapproved_headings(rel: &str, count: usize) -> String {
    refuse(
        format!("{}: {} bodies under unapproved headings", rel, count),
        Next::owner_runs(format!("adr approve {}", rel)),
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

pub fn blame_unchanged(rel: &str) -> String {
    format!("approve: {}: blame unchanged", rel)
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
            .and_owner_runs(format!("adr approve {} again", rel)),
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

/// `count` blame files awaiting approval (9c3cc17e A: a draft passes, it
/// only ever earns a count line, the same shape `plan_awaiting` already
/// gives an unapproved topic file).
pub fn blame_awaiting(count: usize, list: &str) -> String {
    format!("blame: {} file(s) awaiting approval: {}", count, list)
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
        Next::owner_runs("adr serve --port <a free port>"),
    )
}

pub fn missing_answer_selection(id: &str, version: &str) -> String {
    refuse(
        "missing answer selection".to_string(),
        Next::owner_runs(format!("adr answer {}@{} <letter>", id, version)),
    )
}

// --- plain notes (no Next: nothing is refused) -----------------------------

pub fn tested_count(n: usize) -> String {
    format!("tested: {}", n)
}

pub fn plan_awaiting(count: usize, list: &str) -> String {
    format!("plan: {} file(s) awaiting approval: {}", count, list)
}

pub fn done(rel: &str) -> String {
    format!("done: {}", rel)
}

pub fn moot_note(rel: &str, note: &str) -> String {
    format!("moot: {}: {}", rel, note)
}

pub fn tiers_note_check(rel: &str, sizes: &str) -> String {
    format!("tiers: {}: {}", rel, sizes)
}

pub fn tiers_note_approve(sizes: &str) -> String {
    format!("approve: tiers: {}", sizes)
}

pub fn terms_unpaired(count: usize, list: &str) -> String {
    format!("terms: {} terms paired by nothing: {}", count, list)
}

pub fn topic_file_unreadable(path: &str, e: impl Display) -> String {
    format!("parse: {}: cannot read: {}", path, e)
}

pub fn terms_ids_filled(n: usize) -> String {
    format!("approve: adrdom/terms.md: {} missing id(s) filled", n)
}

pub fn terms_already_approved() -> String {
    "approve: adrdom/terms.md: already approved".to_string()
}

pub fn terms_approved_now() -> String {
    "approve: adrdom/terms.md: approved".to_string()
}

pub fn plan_approved(rel: &str) -> String {
    format!("approve: {}: plan approved", rel)
}

pub fn plan_unchanged(rel: &str) -> String {
    format!("approve: {}: plan unchanged", rel)
}

pub fn context_approved(rel: &str) -> String {
    format!("approve: {}: context approved", rel)
}

pub fn context_unchanged(rel: &str) -> String {
    format!("approve: {}: context unchanged", rel)
}

pub fn withdrawn(detail: &str) -> String {
    format!("approve: withdrawn: {}", detail)
}

pub fn supersedes_ignored_on_refusal(heading: &str) -> String {
    format!("{}: supersedes ignored on a refused question", heading)
}

pub fn nothing_to_remove() -> String {
    "nothing to remove".to_string()
}

pub fn removed_entries(n: usize) -> String {
    format!("removed {} entries", n)
}

pub fn no_new_entries() -> String {
    "no new entries".to_string()
}

pub fn banned_added(n: usize) -> String {
    format!("added {} entries", n)
}

/// Printed after `adr ban --rebuild` writes the ban list straight from the
/// ledger (per "Does the tool rewrite the ban list from the ledger?"): `n`
/// is the number of lines it wrote, in the same style `banned_added` and
/// `removed_entries` already use for the plain `ban`/`unban` counts.
pub fn banned_rebuilt(n: usize) -> String {
    format!("rebuilt {} entries", n)
}

pub fn agents_md_created() -> String {
    "install: AGENTS.md: created with the brief".to_string()
}

pub fn agents_md_up_to_date() -> String {
    "install: AGENTS.md: brief already up to date".to_string()
}

pub fn agents_md_replaced() -> String {
    "install: AGENTS.md: replaced the stale brief section".to_string()
}

pub fn agents_md_appended() -> String {
    "install: AGENTS.md: appended the brief".to_string()
}

pub fn claude_md_created() -> String {
    "install: CLAUDE.md: created with the AGENTS.md import".to_string()
}

pub fn claude_md_import_added() -> String {
    "install: CLAUDE.md: added the AGENTS.md import".to_string()
}

/// `adr install` wrote a hook (`path`, e.g. `.githooks/pre-commit`) that
/// was either missing or held stale content.
pub fn hook_written(path: &str) -> String {
    format!("install: {}: written", path)
}

/// `adr install` found a hook (`path`) already holding exactly what it
/// writes, and left it untouched.
pub fn hook_up_to_date(path: &str) -> String {
    format!("install: {}: already up to date", path)
}

pub fn agents_md_removed_whole() -> String {
    "uninstall: AGENTS.md: removed (held only the brief)".to_string()
}

pub fn agents_md_section_removed() -> String {
    "uninstall: AGENTS.md: removed the brief section".to_string()
}

pub fn claude_md_deleted() -> String {
    "uninstall: CLAUDE.md: deleted (held only the AGENTS.md import)".to_string()
}

pub fn claude_md_import_removed() -> String {
    "uninstall: CLAUDE.md: removed the AGENTS.md import".to_string()
}

pub fn hook_removed(name: &str) -> String {
    format!("uninstall: removed .githooks/{}", name)
}

pub fn githooks_dir_removed() -> String {
    "uninstall: removed empty .githooks".to_string()
}

pub fn hooks_path_unset() -> String {
    "uninstall: unset core.hooksPath".to_string()
}

pub fn codex_rules_removed() -> String {
    "uninstall: removed .codex/rules/adrdom.rules".to_string()
}

pub fn codex_rules_dir_removed() -> String {
    "uninstall: removed empty .codex/rules".to_string()
}

pub fn codex_dir_removed() -> String {
    "uninstall: removed empty .codex".to_string()
}

pub fn deny_entry_present(entry: &str) -> String {
    format!(
        "install: .claude/settings.json: deny entry {} already present",
        entry
    )
}

pub fn deny_entry_added(entry: &str) -> String {
    format!("install: .claude/settings.json: added deny entry {}", entry)
}

pub fn deny_entry_removed(entry: &str) -> String {
    format!(
        "uninstall: .claude/settings.json: removed deny entry {}",
        entry
    )
}

pub fn settings_json_deleted() -> String {
    "uninstall: .claude/settings.json: deleted (held only the deny entries install added)"
        .to_string()
}

pub fn claude_dir_removed() -> String {
    "uninstall: removed empty .claude".to_string()
}

pub fn codex_rules_up_to_date() -> String {
    "install: .codex/rules/adrdom.rules: already up to date".to_string()
}

pub fn codex_rules_written() -> String {
    "install: .codex/rules/adrdom.rules: written".to_string()
}

pub fn codex_trust_note() -> String {
    "install: Codex enforces project rules only for a project marked trusted in the owner's Codex config."
        .to_string()
}

/// `adr install` wrote `adrdom/.gitattributes` (Decision 7,
/// db56a285-946f-48b1-b073-bb1e90192185) because it was either missing or
/// held stale content.
pub fn gitattributes_written() -> String {
    "install: adrdom/.gitattributes: written".to_string()
}

/// `adr install` found `adrdom/.gitattributes` already holding exactly
/// what it writes, and left it untouched.
pub fn gitattributes_up_to_date() -> String {
    "install: adrdom/.gitattributes: already up to date".to_string()
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
        let outputs: Vec<String> = vec![
            write_failed("x", e, Next::agent("retries")),
            read_failed("x", e, Next::agent("retries")),
            update_failed("x", e, Next::agent("retries")),
            create_failed("x", e, Next::agent("retries")),
            parse_failed("x", e, Next::agent("retries")),
            stdin_read_failed(Next::agent("retries")),
            not_a_repo("ask", Actor::Agent),
            not_a_repo("approve", Actor::Owner),
            owner_refusal("approve", "adrdom/decisions/x.md"),
            owner_refusal("unban", ""),
            terms_missing(),
            terms_load_error(e, Next::agent("retries")),
            terms_shape_error(e),
            terms_not_approved("adrdom/decisions/x.md"),
            terms_file_not_approved(),
            terms_brief_missing(),
            terms_brief_differs(),
            terms_brief_not_staged(),
            circular_definition("a -> b -> a"),
            term_removed_but_carried("uuid-1", "term \"x\""),
            term_retired_but_carried("x", "term \"y\"", term_reword_and_approve()),
            term_retired_but_carried("x", "id-1: \"heading\"", Next::agent("supersedes id-1 with a new question first")),
            word_moved_off_term("word", "term", "term \"y\"", term_reword_and_approve()),
            word_moved_off_term(
                "word",
                "term",
                "id-1: \"heading\"",
                Next::agent("restores the wording in adrdom/terms.md so \"word\" still names \"term\", or writes a question that supersedes id-1 with a new terms pairing"),
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
            section_term_not_found("x.md", "heading", "uuid-1", "id-1", false),
            section_term_not_found("x.md", "heading", "uuid-1", "id-1", true),
            section_term_retired("x.md", "heading", "term", "id-1", false),
            section_term_retired("x.md", "heading", "term", "id-1", true),
            term_definition_term_not_found("term", "uuid-1"),
            term_definition_term_retired("term", "retired-term"),
            term_definition_terms_line_malformed("term"),
            frozen_depends_edited("x.md", "heading"),
            dep_not_current_decision("x.md", "heading", "id-1"),
            dep_no_term("x.md", "heading", "widget"),
            bad_option_letter("x.md", "dependent", "target", 'Z'),
            tier_cap_exceeded("x.md", &["heading".to_string()], 0, 4),
            tier_cap_exceeded("x.md", &["heading".to_string()], 3, 4),
            derive_error("x.md", &DeriveError::SelfDep("heading".to_string())),
            derive_error_ask("x.md", &DeriveError::SelfDep("heading".to_string())),
            answered_id_missing("id-1", "heading"),
            id_not_found("id-1"),
            already_refused("heading"),
            already_answered("heading"),
            letter_not_an_option('Z', "heading"),
            not_in_asked_state("heading"),
            answer_missing_version("id-1"),
            stale_version("heading"),
            invalid_version_format("Z0ZZZZZZ"),
            no_terms_line("id-1"),
            malformed_terms_line("id-1"),
            terms_line_names_gone_term("id-1", "uuid-1"),
            terms_line_names_retired_term("id-1", "term"),
            supersedes_not_current("id-1"),
            edited_after_answered("x.md", "heading"),
            edited_after_refused("x.md", "heading"),
            plan_not_approved("x.md"),
            context_not_approved("x.md"),
            missing_answer_set("x.md", "dependent", "target"),
            depends_on_refused("x.md", "heading", "target"),
            ask_supersedes_not_current("x.md", "heading", "id-1"),
            tier_blocked_refused("x.md", "heading", 2, 1, "blocker"),
            tier_blocked_waiting("x.md", "heading", 2, 1, "blocker", "id-1", "abcd1234"),
            tier_blocked_unasked("x.md", "heading", 2, 1, "blocker"),
            tier_blocked_needs_ask("x.md", "heading", 2, 1, "blocker"),
            moot_candidate("x.md", "heading", "target", "A", 'B'),
            banned_word_in_candidate("x.md", "heading", ""),
            pairs_no_term("x.md", "heading"),
            pairs_no_term_at_write("x.md", "heading"),
            no_id_line("x.md", "heading"),
            invalid_letter("z"),
            no_such_term("widget"),
            already_in_use(),
            settings_not_json_object("path", Next::agent("retries")),
            settings_permissions_not_object("path", Next::agent("retries")),
            settings_deny_not_array("path", Next::agent("retries")),
            check_parse_error("x.md", "heading", "duplicate id line"),
            heading_plan_not_approved("x.md", "heading"),
            duplicate_id("id-1", "x.md:heading", "y.md", "heading"),
            removed_from_plan("id-1", "heading", "asked", Some("x.md")),
            removed_from_plan("id-1", "heading", "asked", None),
            ledger_id_not_found("id-1"),
            withdrawn_id_present("id-1", "x.md"),
            refusal_without_row("x.md", "heading"),
            answer_without_row("x.md", "heading"),
            answered_text_changed("x.md", "heading", "id-1"),
            answered_row_without_line("x.md", "heading"),
            refused_row_without_line("x.md", "heading"),
            edited_after_asking("x.md", "heading"),
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
            malformed_ledger_row(3, "bad row"),
            ledger_not_append_only(),
            ledger_merge_drops_rows(),
            ledger_merge_parents_unreadable(),
            ledger_asked_id_missing("id-1"),
            ledger_answered_without_asked("id-1"),
            ledger_push_drops_rows("refs/heads/feat", "refs/heads/main"),
            ledger_push_remote_unreadable("refs/heads/main"),
            ledger_since_drops_rows("abc123"),
            ledger_since_unreadable("abc123"),
            ledger_same_time_competition("2024-01-01T00:00:00Z", "id id-1"),
            untested("id-1"),
            stale_test("id-1"),
            duplicate_heading_in_file("heading", "x.md"),
            duplicate_heading_across_files("heading", "x.md", "y.md"),
            bad_topic_file_name("adrdom/decisions/Bad_Name.md"),
            topic_file_name_mismatch("adrdom/decisions/x.md", "Y", "y"),
            topic_unnameable("adrdom/decisions/rule.md", "Rule 2"),
            body_under_unapproved_heading("x.md", "heading"),
            bodies_under_unapproved_headings("x.md", 3),
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
                o.contains("; agent ") || o.contains("owner runs: "),
                "no next move: {}",
                o
            );
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
        let out = section_term_not_found("x.md", "heading", "uuid-1", "id-1", true);
        assert!(!out.contains("retired"), "{}", out);
        assert!(out.contains("restores the term in adrdom/terms.md"), "{}", out);
        assert!(
            out.contains("writes a question that supersedes id-1 with a new terms pairing"),
            "{}",
            out
        );
    }
}
