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

/// The same "not approved" state, reported about `adrdom/terms.md` itself
/// rather than a dependent topic file (`check`'s own terms check, when no
/// topic file exists yet to name).
pub fn terms_md_not_approved() -> String {
    refuse(
        "adrdom/terms.md not approved".to_string(),
        Next::owner_runs("adr approve adrdom/terms.md"),
    )
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
/// terms are never superseded (9.3 B; the fix for a term-dependent is
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

pub fn terms_banned_word(line: usize, word: &str, suffix: &str) -> String {
    refuse(
        format!(
            "adrdom/terms.md:{}: banned word \"{}\"{}",
            line, word, suffix
        ),
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

/// One provenance/code-state/date/loaded-heading tell (9.5 A).
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

pub fn tier_cap_exceeded(rel: &str, tier: usize, count: usize, cap: usize) -> String {
    refuse(
        format!(
            "{}: tier {} holds {} questions, the cap is {}",
            rel, tier, count, cap
        ),
        Next::agent("moves questions under a higher heading or splits the topic"),
    )
}

// `derive_tiers`' three failure shapes (record::DeriveError), each with two
// wordings: the plain one (`approve`/`check`'s `plan` category) and `ask`'s,
// which always points at re-approving afterward.

use crate::record::DeriveError;

fn derive_what(rel: &str, e: &DeriveError) -> String {
    match e {
        DeriveError::Dangling { section, entry } => format!(
            "{}: \"{}\" depends on \"{}\" which is no heading in this file",
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
    }
}

fn derive_next(e: &DeriveError) -> Next {
    match e {
        DeriveError::Dangling { .. } => Next::agent("adds the heading or fixes the text"),
        DeriveError::Cycle(_) => Next::agent("breaks it"),
        DeriveError::SelfDep(_) => Next::agent("removes the line or names something else"),
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
) -> String {
    refuse(
        format!(
            "{}: \"{}\" is in tier {}; tier {} is open, waiting on \"{}\"",
            rel, heading, cand_tier, open_tier, blocker
        ),
        Next::owner_runs(format!("adr answer {} <letter> on it first", blocker_id)),
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

pub fn banned_word_in_candidate(rel: &str, heading: &str, word: &str, suffix: &str) -> String {
    refuse(
        format!(
            "{}: \"{}\": banned word \"{}\"{}",
            rel, heading, word, suffix
        ),
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

pub fn nothing_to_ask(path: &str) -> String {
    format!(
        "nothing to ask in {}: no question in the open tier has a body that is not already asked; agent writes the bodies of the open tier (adr show names it)",
        path
    )
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

pub fn answer_without_row(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: {}: answer line without ledger row", rel, heading),
        Next::agent("removes the hand-written answer line; only adr answer writes one backed by the ledger"),
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

pub fn banned_word_at(location: &str, word: &str, suffix: &str) -> String {
    refuse(
        format!("{}: banned word \"{}\"{}", location, word, suffix),
        Next::agent("rewrites the line without the banned word"),
    )
}

pub fn banned_missing_from_file(hash_prefix: &str) -> String {
    refuse(
        format!(
            "{} in ledger, missing from adrdom/state/banned.sha256",
            hash_prefix
        ),
        Next::owner_runs(
            "git diff adrdom/state/banned.sha256 adrdom/state/ledger.tsv; restore by hand whatever is not real",
        ),
    )
}

pub fn banned_missing_from_ledger(hash_prefix: &str) -> String {
    refuse(
        format!("{} in file, no ledger row", hash_prefix),
        Next::owner_runs(
            "git diff adrdom/state/banned.sha256 adrdom/state/ledger.tsv; restore by hand whatever is not real",
        ),
    )
}

/// An asked/answered section that carries no `terms:` line at all.
pub fn section_no_terms_line(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: {}: no terms line", rel, heading),
        Next::agent("re-asks so the terms line is written"),
    )
}

pub fn section_term_not_found(rel: &str, heading: &str, uuid: &str) -> String {
    refuse(
        format!("{}: {}: term {} not found", rel, heading, uuid),
        Next::agent("re-asks so the terms line matches current terms"),
    )
}

pub fn section_term_retired(rel: &str, heading: &str, name: &str) -> String {
    refuse(
        format!("{}: {}: bound term \"{}\" is retired", rel, heading, name),
        Next::agent("re-asks so the terms line drops the retired term"),
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
        Next::agent("runs: adr install"),
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

pub fn malformed_ledger_row(lineno: usize, raw: &str) -> String {
    refuse(
        format!("line {}: malformed row: {}", lineno, raw),
        Next::owner_runs("git diff adrdom/state/ledger.tsv and decides which rows are real before anything is committed"),
    )
}

pub fn ledger_not_append_only() -> String {
    refuse(
        "not append-only: working copy diverges from HEAD".to_string(),
        Next::owner_runs("git diff adrdom/state/ledger.tsv; only new rows may be appended, the rest is restored by hand"),
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

pub fn body_under_unapproved_heading(rel: &str, heading: &str) -> String {
    refuse(
        format!("{}: body under unapproved heading \"{}\"", rel, heading),
        Next::owner_runs(format!("adr approve {}", rel)),
    )
}

// --- serve/mod.rs ------------------------------------------------------

pub fn bind_failed(port: u16, e: impl Display) -> String {
    refuse(
        format!("failed to bind 127.0.0.1:{}: {}", port, e),
        Next::owner_runs("adr serve --port <a free port>"),
    )
}

pub fn missing_answer_selection(id: &str) -> String {
    refuse(
        "missing answer selection".to_string(),
        Next::owner_runs(format!("adr answer {} <letter>", id)),
    )
}

// --- plain notes (no Next: nothing is refused) -----------------------------

pub fn tested_count(n: usize) -> String {
    format!("tested: {}", n)
}

pub fn plan_awaiting(count: usize, list: &str) -> String {
    format!("plan: {} file(s) awaiting approval: {}", count, list)
}

pub fn terms_awaiting() -> String {
    "terms: awaiting approval".to_string()
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

pub fn term_definition_changed(name: &str) -> String {
    format!("approve: term \"{}\" definition changed", name)
}

pub fn term_used_by_current(rel: &str, heading: &str) -> String {
    format!("  current: {}: {}", rel, heading)
}

pub fn term_used_by_term(name: &str) -> String {
    format!("  term: {}", name)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::DeriveError;

    /// Every refusal/failure function in this module, called once with
    /// sample args: the type system already forces each to carry a `Next`,
    /// this is the second guard -- the rendered text must actually name
    /// the agent's move or a command the owner runs.
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
            terms_md_not_approved(),
            circular_definition("a -> b -> a"),
            term_removed_but_carried("uuid-1", "term \"x\""),
            term_retired_but_carried("x", "term \"y\"", term_reword_and_approve()),
            term_retired_but_carried("x", "id-1: \"heading\"", Next::agent("supersedes id-1 with a new question first")),
            word_moved_off_term("word", "term", "term \"y\"", term_reword_and_approve()),
            word_moved_off_term("word", "term", "id-1: \"heading\"", Next::agent("re-asks id-1")),
            terms_banned_word(3, "word", ""),
            shape_error("x.md", "missing h1"),
            section_shape_error("x.md", "heading", "stray line 4"),
            tell("x.md", 3, "context", "claude", "says who said it"),
            no_depends_line("x.md", "heading"),
            no_depends_line_bare("heading"),
            refused_summary("heading", "x.md"),
            section_no_terms_line("x.md", "heading"),
            section_term_not_found("x.md", "heading", "uuid-1"),
            section_term_retired("x.md", "heading", "term"),
            term_definition_term_not_found("term", "uuid-1"),
            term_definition_term_retired("term", "retired-term"),
            frozen_depends_edited("x.md", "heading"),
            dep_not_current_decision("x.md", "heading", "id-1"),
            dep_no_term("x.md", "heading", "widget"),
            bad_option_letter("x.md", "dependent", "target", 'Z'),
            tier_cap_exceeded("x.md", 1, 5, 4),
            derive_error("x.md", &DeriveError::SelfDep("heading".to_string())),
            derive_error_ask("x.md", &DeriveError::SelfDep("heading".to_string())),
            answered_id_missing("id-1", "heading"),
            id_not_found("id-1"),
            already_refused("heading"),
            already_answered("heading"),
            letter_not_an_option('Z', "heading"),
            not_in_asked_state("heading"),
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
            tier_blocked_waiting("x.md", "heading", 2, 1, "blocker", "id-1"),
            moot_candidate("x.md", "heading", "target", "A", 'B'),
            banned_word_in_candidate("x.md", "heading", "word", ""),
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
            answered_row_without_line("x.md", "heading"),
            refused_row_without_line("x.md", "heading"),
            edited_after_asking("x.md", "heading"),
            answer_set_changed("x.md", "dependent", "heading"),
            banned_word_at("x.md:3", "word", ""),
            banned_missing_from_file("abcdef123456"),
            banned_missing_from_ledger("abcdef123456"),
            agents_md_stale(),
            agents_md_missing_brief(),
            claude_md_missing_import(),
            missing_deny_entry("Bash(adr approve*)"),
            settings_unparseable(".claude/settings.json"),
            codex_rules_stale(),
            codex_rules_missing(),
            malformed_ledger_row(3, "bad row"),
            ledger_not_append_only(),
            ledger_asked_id_missing("id-1"),
            ledger_answered_without_asked("id-1"),
            untested("id-1"),
            stale_test("id-1"),
            duplicate_heading_in_file("heading", "x.md"),
            duplicate_heading_across_files("heading", "x.md", "y.md"),
            body_under_unapproved_heading("x.md", "heading"),
            bind_failed(7357, e),
            missing_answer_selection("id-1"),
        ];
        for o in &outputs {
            assert!(
                o.contains("; agent ") || o.contains("owner runs: "),
                "no next move: {}",
                o
            );
        }
    }
}
