//! Every user-facing message the tool prints lives here: one place for
//! wording, and a type (`Next`) that makes a refusal without a next move a
//! compile error. A refusal or failure always takes one, either
//! built by the caller (when the next move differs by call site) or fixed
//! inside the function (when every caller wants the same one).
//!
//! Shape: `<what happened>; <move>`, every clause of the move one act of
//! `Act`.
//! Some functions bake in their own `<cmd>: ` prefix (a single call site);
//! most don't, since the same text is reused under `approve: `, `check`'s
//! per-category prefix, and so on, exactly as before this module existed.

use crate::moves::{command, Cmd, MoveKind};
use crate::rule;
use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    Agent,
    Owner,
    /// The agent of a planner's session, when it takes a build move.
    Builder,
}

/// The fixed set of acts a move clause opens with, after its actor (per
/// "What does a finding line hold?", 50b31713 A). Every clause a `Next`
/// renders is one of these; `ACTS` holds their rendered openings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// `writes <thing>`: text that is not there yet.
    Writes(String),
    /// `rewrites <thing>`: text that is there.
    Rewrites(String),
    /// `removes <thing>`.
    Removes(String),
    /// `restores <thing> from git`.
    Restores(String),
    /// `runs: <command>`.
    Runs(Cmd),
    /// `builds <thing>`.
    Builds(String),
    /// `installs <thing>`: a program the tool calls and does not ship.
    Installs(String),
    /// `changes into <place>`: the directory a command runs from.
    ChangesInto(String),
    /// `runs the same adr command again`: the command that stopped, which
    /// the step that failed inside it does not know.
    RunsAgain,
}

/// The opening words of every act, as a move prints them, and of a clause
/// nobody takes yet.
pub const ACTS: &[&str] =
    &["writes ", "rewrites ", "removes ", "restores ", "runs: ", "runs the same adr command again", "builds ", "installs ", "changes into "];

/// The opening of a clause nobody takes yet.
pub const NOT_YET: &str = "not yet: ";

impl Act {
    fn render(&self) -> String {
        match self {
            Act::Writes(t) => format!("writes {}", t),
            Act::Rewrites(t) => format!("rewrites {}", t),
            Act::Removes(t) => format!("removes {}", t),
            Act::Restores(t) => format!("restores {} from git", t),
            Act::Runs(c) => format!("runs: {}", c),
            Act::Builds(t) => format!("builds {}", t),
            Act::Installs(t) => format!("installs {}", t),
            Act::ChangesInto(t) => format!("changes into {}", t),
            Act::RunsAgain => "runs the same adr command again".to_string(),
        }
    }

    fn map_thing(self, f: impl Fn(String) -> String) -> Act {
        match self {
            Act::Writes(t) => Act::Writes(f(t)),
            Act::Rewrites(t) => Act::Rewrites(f(t)),
            Act::Removes(t) => Act::Removes(f(t)),
            Act::Restores(t) => Act::Restores(f(t)),
            Act::Builds(t) => Act::Builds(f(t)),
            Act::Installs(t) => Act::Installs(f(t)),
            Act::ChangesInto(t) => Act::ChangesInto(f(t)),
            runs @ (Act::Runs(_) | Act::RunsAgain) => runs,
        }
    }
}

/// One clause of a move: an actor and the act it takes, or a wait nobody
/// takes yet. `then` marks a clause that follows the one before it by the
/// same actor, printed `, then <act>`.
#[derive(Clone, Debug)]
enum Clause {
    Do { actor: Actor, act: Act, then: bool },
    NotYet(String),
}

/// The thing the fix helpers (`after_fix_agent_runs`, `after_fix_owner_runs`)
/// rewrite is named by the failure that carries them; `Next::on` puts the
/// thing in its place.
const FIX_THE_THING: &str = "\u{1}";

/// One or more next-move clauses, joined with `, `. `Next::agent(..)`
/// reads as `agent <act>`; `Next::owner_runs(..)` as `owner runs:
/// <command>`. A clause that names a command to run takes it as a
/// `moves::Cmd`, built by `moves::command` from its words, so every word
/// taken from the records is quoted as a shell reads it and the command
/// runs as printed (per "Does a refusal name the command that clears
/// it?"). Every clause is one act of `Act`.
#[derive(Clone, Debug)]
pub struct Next(Vec<Clause>);

impl std::fmt::Display for Next {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}

impl Next {
    fn one(actor: Actor, act: Act) -> Next {
        Next(vec![Clause::Do { actor, act, then: false }])
    }

    /// `agent <act>`.
    pub fn agent(act: impl Into<Act>) -> Next {
        Next::one(Actor::Agent, act.into())
    }

    /// `builder <act>`: the build move in a planner's session.
    pub fn builder(act: impl Into<Act>) -> Next {
        Next::one(Actor::Builder, act.into())
    }

    /// `owner <act>`.
    pub fn owner(act: impl Into<Act>) -> Next {
        Next::one(Actor::Owner, act.into())
    }

    /// `<actor> <act>`.
    pub fn by(actor: Actor, act: impl Into<Act>) -> Next {
        Next::one(actor, act.into())
    }

    /// `not yet: <wait>`: a clause nobody takes yet.
    pub fn not_yet(wait: impl Into<String>) -> Next {
        Next(vec![Clause::NotYet(wait.into())])
    }

    /// `agent runs: <command>`.
    pub fn agent_runs(command: Cmd) -> Next {
        Next::agent(Act::Runs(command))
    }

    /// `agent <act>, then runs: <command>`.
    pub fn agent_then_runs(act: impl Into<Act>, command: Cmd) -> Next {
        Next::agent(act).then(Act::Runs(command))
    }

    /// `owner <act>, then runs: <command>`.
    pub fn owner_then_runs(act: impl Into<Act>, command: Cmd) -> Next {
        Next::owner(act).then(Act::Runs(command))
    }

    pub fn owner_runs(command: Cmd) -> Next {
        Next::owner(Act::Runs(command))
    }

    /// `<clauses>, then <act>`: a further act by the actor of the last clause.
    pub fn then(mut self, act: impl Into<Act>) -> Next {
        let actor = match self.0.last() {
            Some(Clause::Do { actor, .. }) => *actor,
            _ => panic!("then follows a clause somebody takes"),
        };
        self.0.push(Clause::Do { actor, act: act.into(), then: true });
        self
    }

    /// The clauses of every `Next` in `parts`, in order, as one.
    pub fn all(parts: Vec<Next>) -> Next {
        Next(parts.into_iter().flat_map(|n| n.0).collect())
    }

    pub fn and_owner_runs(mut self, command: Cmd) -> Next {
        self.0.push(Clause::Do { actor: Actor::Owner, act: Act::Runs(command), then: false });
        self
    }

    /// Names `thing` as what a generic fix clause (`FIX_THE_THING`) rewrites.
    pub fn on(self, thing: &str) -> Next {
        Next(
            self.0
                .into_iter()
                .map(|c| match c {
                    Clause::Do { actor, act, then } => {
                        Clause::Do { actor, act: act.map_thing(|t| t.replace(FIX_THE_THING, thing)), then }
                    }
                    w => w,
                })
                .collect(),
        )
    }

    pub fn render(&self) -> String {
        self.0
            .iter()
            .map(|c| match c {
                Clause::Do { act, then: true, .. } => format!("then {}", act.render()),
                Clause::Do { actor, act, then: false } => {
                    let who = match actor {
                        Actor::Agent => "agent",
                        Actor::Owner => "owner",
                        Actor::Builder => "builder",
                    };
                    format!("{} {}", who, act.render())
                }
                Clause::NotYet(w) => format!("{}{}", NOT_YET, w),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// What a message function says of a finding: the rule it breaks, what was
/// found and the move that clears it, apart; `Finding::said` joins found and
/// move into the line every command prints.
#[derive(Debug, Clone)]
pub struct Said {
    pub rule: &'static crate::rule::Rule,
    pub found: String,
    pub next: Next,
}

impl Said {
    /// The line the finding prints: `<found>; <move>`.
    pub fn text(&self) -> String {
        finding_line(&self.found, &self.next)
    }
}

/// The one line a finding prints, `Said::text` and `rules::Finding::text`
/// alike: `<found>; <move>`, nothing on what the rule protects (per "What
/// does a finding line hold?", 50b31713 A).
pub fn finding_line(found: &str, next: &Next) -> String {
    format!("{}; {}", found, next.render())
}

/// A finding of `rule`: `found` and the `next` move that clears it.
pub fn said(rule: &'static crate::rule::Rule, found: impl Into<String>, next: Next) -> Said {
    Said { rule, found: found.into(), next }
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
    refuse(format!("failed to write {}: {}", target, e), next.on(target))
}

pub fn read_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("cannot read {}: {}", target, e), next.on(target))
}

pub fn update_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("failed to update {}: {}", target, e), next.on(target))
}

pub fn create_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("failed to create {}: {}", target, e), next.on(target))
}

pub fn remove_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("failed to remove {}: {}", target, e), next.on(target))
}

/// `git config` did not take the change to `core.hooksPath`. `doing` is
/// "set" or "unset".
pub fn hooks_path_failed(doing: &str, next: Next) -> String {
    refuse(format!("git config failed to {} core.hooksPath", doing), next.on("the git configuration of core.hooksPath"))
}

pub fn parse_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("{}: {}", target, e), next.on(target))
}

pub fn stdin_read_failed(next: Next) -> String {
    refuse("failed to read stdin".to_string(), next.on("what is piped to adr"))
}

/// Standard output refused a line for a cause other than its reader going
/// away (a full disk, a closed descriptor): what was printed is incomplete.
/// `again` is the command line as it was run and `actor` who runs it: the
/// owner for an owner command, else the agent.
pub fn stdout_write_failed(e: impl Display, actor: Actor, again: Cmd) -> String {
    let next = match actor {
        Actor::Agent | Actor::Builder => Next::agent_runs(again),
        Actor::Owner => Next::owner_runs(again),
    };
    refuse(format!("output: standard output cannot be written: {}", e), next)
}

// --- repo.rs ----------------------------------------------------------------

/// `adr <cmd> <args>` as a command a move names, every word quoted as a
/// shell reads it.
/// The git command that puts `rel` back as the last commit holds it, its
/// path quoted so the hint runs as printed.
fn adr_command(cmd: &str, args: &[&str]) -> Cmd {
    let mut words = vec!["adr", cmd];
    words.extend_from_slice(args);
    command(&words)
}

/// `<cmd>: not inside a git repository; ...` -- printed by every command
/// before it does anything else. `actor` is who was already confirmed to
/// be running `cmd` (the owner, for the five owner-only commands, since
/// `owner_authorized` already gated by the time this fires; the agent for
/// every other command).
/// `args` is what the command was given, so the owner's line is the whole
/// command.
pub fn not_a_repo(cmd: &str, args: &[&str], actor: Actor) -> String {
    let command = adr_command(cmd, args);
    // What clears it is where the command runs, so the move names that
    // first: the command itself is run again from inside a repository.
    let into = Act::ChangesInto("a git repository".into());
    let next = match actor {
        Actor::Agent | Actor::Builder => Next::agent(into).then(Act::Runs(command)),
        Actor::Owner => Next::owner(into).then(Act::Runs(command)),
    };
    refuse(format!("{}: not inside a git repository", cmd), next)
}

/// The refusal every owner-only command prints when stdin isn't an
/// interactive terminal: `<cmd>: refusing: this is the owner's command;
/// owner runs: adr <cmd> <args>`.
pub fn owner_refusal(cmd: &str, args: &[&str]) -> String {
    let command = adr_command(cmd, args);
    refuse(
        format!("{}: refusing: this is the owner's command", cmd),
        Next::owner_runs(command),
    )
}

// --- adrdom/terms.md: shared by actions.rs, ask.rs, check/terms.rs --------

pub fn terms_missing() -> Said {
    said(
        &crate::rule::TERMS_FILE_EXISTS,
        "adrdom/terms.md missing",
        Next::agent(Act::Writes("adrdom/terms.md".into())).and_owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// A read/parse failure on `adrdom/terms.md` itself (an I/O error, not one
/// of its shape rules) -- rare enough that the caller's own context decides
/// who fixes it.
pub fn terms_load_error(e: &str, next: Next) -> String {
    refuse(format!("adrdom/terms.md: {}", e), next.on("adrdom/terms.md"))
}

/// One of `adrdom/terms.md`'s shape errors (from `terms::parse`): always
/// the agent's fix, regardless of which command found it.
pub fn terms_shape_error(e: &str) -> Said {
    let fault = e.split_once(": ").map(|(_, f)| f).unwrap_or(e);
    let found = format!("adrdom/terms.md: {}", e);
    if fault.contains("used in both") {
        said(&crate::rule::WORD_NAMES_ONE_TERM, found, Next::agent(Act::Removes("the word from one of the two entries".into())))
    } else if fault.starts_with("missing h1") || fault.starts_with("duplicate h1") || fault == "unknown line" {
        said(&crate::rule::TERMS_FILE_SHAPE, found, Next::agent(Act::Rewrites("the line".into())))
    } else if fault.starts_with("duplicate id ") {
        said(&crate::rule::TERM_IDENTIFIER_IS_UNIQUE, found, Next::agent(Act::Rewrites("the entry".into())))
    } else {
        said(&crate::rule::TERM_ENTRY_FIELDS, found, Next::agent(Act::Rewrites("the entry".into())))
    }
}

/// `adrdom/terms.md` exists and parses, but isn't approved at its current
/// hash -- named from a dependent topic file's own path (`rel`), the way
/// `ask`/`answer`/`approve` all report it.
pub fn terms_not_approved(rel: &str, status: FileStatus) -> Said {
    said(
        &crate::rule::TERMS_FILE_APPROVED,
        format!("{}: adrdom/terms.md is {}", rel, status.name()),
        Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// `adrdom/terms.md` exists and parses, but isn't approved at its current
/// hash (per "Does the tool stop while the terms file waits for approval?",
/// aa475aa3 A): `check`'s own report of the same fact `terms_not_approved`
/// names from a dependent topic file -- this one names the file itself,
/// since `check` judges it on its own, not on behalf of one dependent.
pub fn terms_file_not_approved(status: FileStatus) -> Said {
    said(
        &crate::rule::TERMS_FILE_APPROVED,
        format!("adrdom/terms.md is {}", status.name()),
        Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// `check`: no terms brief where the terms file is approved (per "Does
/// check hold the terms brief to the terms file?", ec3f875c A). `approve`
/// is the one writer (per "Does approve write the terms brief?", fcab4bf9
/// A), so the move is the owner's.
pub fn terms_brief_missing() -> Said {
    said(
        &crate::rule::TERMS_BRIEF_MATCHES_TERMS,
        format!("{} missing", crate::repo::TERMS_BRIEF_PATH),
        Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// `check`: the terms brief does not hold what the terms file gives.
pub fn terms_brief_differs() -> Said {
    said(
        &crate::rule::TERMS_BRIEF_MATCHES_TERMS,
        format!("{} differs from adrdom/terms.md", crate::repo::TERMS_BRIEF_PATH),
        Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// `check` at a commit: the working tree holds the right terms brief and
/// the staged files do not.
pub fn terms_brief_not_staged() -> Said {
    said(
        &crate::rule::STAGED_FILES_ARE_JUDGED_FILES,
        format!("{} not staged", crate::repo::TERMS_BRIEF_PATH),
        Next::agent_runs(command(&["git", "add", crate::repo::TERMS_BRIEF_PATH])),
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

pub fn circular_definition(names: &[String]) -> Said {
    let mut all = names.to_vec();
    if let Some(first) = names.first() {
        all.push(first.clone());
    }
    let last = all.pop().unwrap_or_default();
    let run = if all.is_empty() { last } else { format!("{} and {}", all.join(", "), last) };
    said(
        &crate::rule::DEFINITION_NOT_CIRCULAR,
        format!("adrdom/terms.md: circular definition: a cycle runs through {}", run),
        Next::agent(Act::Rewrites("one of these definitions".into())),
    )
}

/// A dependent (a decision's or a term's own `terms:` line) still names a
/// uuid that no longer exists in `adrdom/terms.md` at all.
/// `word` is the word `source` names the term by, when it names it by a
/// word (its terms line); a depends line names it by the identifier alone.
/// `decision` is the dependent's id and heading when it is a current
/// decision. Marking the term retired is not offered: a retire is held while
/// the dependent stands (per "What holds a retire of a term?"), so the move
/// is restoring the term as it was, then superseding the decision, named
/// by its identifier and heading (`named`).
pub fn term_removed_but_carried(uuid: &str, word: Option<&str>, source: &str, decision: Option<(&str, &str)>) -> Said {
    let next = match decision {
        Some((id, heading)) => Next::agent(Act::Restores("the term in adrdom/terms.md".into()))
            .then(Act::Writes(format!("a question that supersedes {}", named(id, Some(heading))))),
        None => Next::agent(Act::Restores("the term in adrdom/terms.md".into())).and_owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    };
    said(&crate::rule::TERM_STAYS_WHILE_USED, format!("adrdom/terms.md: term {} removed but used by {}", named(uuid, word), source), next)
}

/// A dependent still names a term that is now retired. `next` differs by
/// dependent kind: a decision supersedes itself with a new question
/// (built by the caller, which knows the decision's id); a term's own
/// definition is reworded by the agent and re-approved by the owner --
/// terms are never superseded (per "What holds a retire of a term?"; the fix for a term-dependent is
/// always this same pair, so it's fixed here).
pub fn term_retired_but_carried(name: &str, source: &str, user: RetiredUser) -> Said {
    let found = format!("adrdom/terms.md: term \"{}\" retired but used by {}", name, source);
    match user {
        RetiredUser::Decision(id, heading) => said(
            &crate::rule::TERM_NOT_RETIRED_WHILE_NAMED,
            found,
            Next::agent(Act::Rewrites(format!("the entry of \"{}\"", name)))
                .then(Act::Writes(format!("a question that supersedes {}", named(id, Some(heading))))),
        ),
        RetiredUser::Asked(rel) => said(
            &crate::rule::TERM_NOT_RETIRED_WHILE_NAMED,
            found,
            Next::agent_then_runs(Act::Rewrites("the question".into()), command(&["adr", "ask", rel])),
        ),
        RetiredUser::PlanHeading(rel) => said(
            &crate::rule::TERM_NOT_RETIRED_WHILE_NAMED,
            found,
            Next::agent(Act::Rewrites("the heading".into())).and_owner_runs(command(&["adr", "approve", rel])),
        ),
        RetiredUser::Definition => said(
            &crate::rule::DEFINITION_USES_LIVE_TERMS,
            found,
            Next::agent(Act::Rewrites("the definition".into())).and_owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
        ),
    }
}

/// What still names a retired term, with the part of it a move needs.
pub enum RetiredUser<'a> {
    /// A current decision, by its identifier and heading.
    Decision(&'a str, &'a str),
    /// An asked question, by its topic file.
    Asked(&'a str),
    /// A heading of an approved plan, by its topic file.
    PlanHeading(&'a str),
    /// Another term's definition.
    Definition,
}



/// A dependent's pairing (`word=uuid`) no longer resolves that word to
/// that term at all -- the wording moved, without the term being retired
/// or removed. `next` mirrors `term_retired_but_carried`'s split.
pub fn word_moved_off_term(word: &str, term_name: &str, source: &str, decision_id: Option<&str>) -> Said {
    let next = match decision_id {
        // A current decision is never re-asked: the wording in the terms
        // file returns.
        Some(_) => Next::agent(Act::Rewrites(format!("the entry of \"{}\" so \"{}\" names it", term_name, word))),
        None => Next::agent(Act::Rewrites("the definition".into())).and_owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    };
    said(
        &crate::rule::WORD_STAYS_ON_TERM,
        format!("adrdom/terms.md: word \"{}\" no longer names term \"{}\", used by {}", word, term_name, source),
        next,
    )
}

/// A term's definition runs past two sentences (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76:
/// a definition says what a thing is "in one or two sentences").
pub fn definition_too_long(name: &str, count: usize) -> Said {
    said(
        &crate::rule::DEFINITION_SENTENCE_LIMIT,
        format!("adrdom/terms.md: term \"{}\" definition has {} sentences, the most is 2", name, count),
        Next::agent(Act::Rewrites("the definition".into())),
    )
}

pub fn terms_banned_word(line: usize, suffix: &str) -> Said {
    said(
        &crate::rule::BANNED_WORD_ABSENT_FROM_RECORDS,
        format!("adrdom/terms.md:{}: banned word{}", line, suffix),
        Next::agent(Act::Rewrites("the line".into())),
    )
}

// --- a topic file's own shape/tells/depends/tier refusals ------------------
// Shared by `rules::record_refusals` (approve + check) and, for the
// derive-error variant, by `ask` as well.

pub fn shape_error(rel: &str, e: &str) -> Said {
    if e.starts_with("tier heading") {
        return said(
            &rule::TIER_HEADING_ABSENT,
            format!("{}: {}", rel, e),
            Next::agent(Act::Removes("the heading".into()))
                .then(Act::Writes("what each question stands on in its depends line".into())),
        );
    }
    said(&rule::TOPIC_FILE_SHAPE, format!("{}: {}", rel, e), Next::agent(Act::Rewrites("the file".into())))
}

/// A `### ` line that does not end in `?` (per "What does a topic file
/// hold?": a heading is a question).
pub fn heading_not_a_question(rel: &str, line: usize) -> Said {
    said(
        &rule::QUESTION_HEADING_ENDS_IN_MARK,
        format!("{}: line {}: a heading is not a question ending in ?", rel, line),
        Next::agent(Act::Rewrites("the line".into())),
    )
}

pub fn section_shape_error(rel: &str, heading: &str, e: &str) -> Said {
    let at = format!("{}: \"{}\": ", rel, heading);
    if e.starts_with("depends: duplicate entry") {
        return said(&rule::DEPENDS_ENTRY_ONCE, format!("{}{}", at, e), Next::agent(Act::Removes("the entry".into())));
    }
    if e.starts_with("second answer line") {
        return said(&rule::ANSWER_LINE_IS_LAST, format!("{}{}", at, e), Next::agent(Act::Removes("the second answer line".into())));
    }
    if e.starts_with("answer line not last") {
        return said(
            &rule::ANSWER_LINE_IS_LAST,
            format!("{}{}", at, e),
            Next::agent(Act::Removes("the lines after the answer line".into())),
        );
    }
    for kind in ["depends", "supersedes", "unrelated", "blame", "terms"] {
        let Some(rest) = e.strip_prefix(&format!("{}: ", kind)) else { continue };
        if rest.starts_with("second ") {
            let n = e.rsplit(' ').next().unwrap_or("");
            return said(
                &rule::REFERENCE_LINE_ONCE,
                format!("{}{} second line, line {}", at, kind, n),
                Next::agent(Act::Rewrites(format!("the {} line", kind))),
            );
        }
        return said(&rule::REFERENCE_LINE_ONCE, format!("{}{}", at, e), Next::agent(Act::Rewrites(format!("the {} line", kind))));
    }
    said(&rule::QUESTION_SECTION_SHAPE, format!("{}{}", at, e), Next::agent(Act::Rewrites("the question".into())))
}

/// One provenance/code-state/date/loaded-heading tell (per "Does the tool
/// stop loaded phrases in a question?").
pub fn tell(rel: &str, line: usize, label: &str, phrase: &str, kind: crate::tells::TellKind) -> Said {
    use crate::tells::TellKind;
    let at = format!("{}:{}: \"{}\": ", rel, line + 1, label);
    let (r, found, mv) = match kind {
        TellKind::Provenance => (&rule::TELL_NAMES_SPEAKER, format!("\"{}\" names who said it", phrase), "the line"),
        TellKind::CodeState => (&rule::TELL_STATES_CODE_NOW, format!("\"{}\" states what the code does", phrase), "the line"),
        TellKind::Date => (&rule::TELL_CARRIES_DATE, format!("\"{}\" is a date", phrase), "the line"),
        TellKind::LoadedOpening => {
            (&rule::TELL_ASSUMES_ANSWER, format!("the heading opens with \"{}\"", phrase), "the heading")
        }
        TellKind::Join => (&rule::TELL_JOINS_TWO_QUESTIONS, format!("the heading holds \"{}\"", phrase), "the heading"),
    };
    said(r, format!("{}{}", at, found), Next::agent(Act::Rewrites(mv.into())))
}

/// An option, Why line or Costs line over the length rule (per "Does ask
/// refuse an option over a word limit?"); `words` is the line's word count.
pub fn too_long(rel: &str, line: usize, label: &str, kind: crate::tells::LengthKind, words: usize) -> Said {
    use crate::tells::LengthKind;
    let (r, thing, most) = match kind {
        LengthKind::Why => (&rule::WHY_WORD_LIMIT, "Why line", crate::tells::WORD_LIMIT),
        LengthKind::Costs => (&rule::COSTS_WORD_LIMIT, "Costs line", crate::tells::WORD_LIMIT),
        LengthKind::OneWordOption => (&rule::DOES_OPTION_IS_ONE_WORD, "option", 1),
        LengthKind::Option => (&rule::OPTION_WORD_LIMIT, "option", crate::tells::WORD_LIMIT),
    };
    said(
        r,
        format!("{}:{}: \"{}\": the {} has {} words, the most is {}", rel, line + 1, label, thing, words, most),
        Next::agent(Act::Rewrites(format!("the {}", thing))),
    )
}

pub fn no_depends_line(rel: &str, heading: &str) -> Said {
    said(
        &rule::QUESTION_HAS_DEPENDS_LINE,
        format!("{}: \"{}\" has no depends line", rel, heading),
        Next::agent(Act::Writes(format!("the depends line of \"{}\"", heading))),
    )
}

/// Same as `no_depends_line`, without naming the file: `approve`'s own
/// defensive fallback in `insert_missing_ids`, unreachable in practice
/// since `record_refusals` already refuses on this first.
pub fn no_depends_line_bare(heading: &str) -> Said {
    said(
        &rule::QUESTION_HAS_DEPENDS_LINE,
        format!("\"{}\" has no depends line", heading),
        Next::agent(Act::Writes(format!("the depends line of \"{}\"", heading))),
    )
}

/// A closed question's depends line edited: `st` is its status (answered,
/// refused or superseded), named as the listing names it.
pub fn frozen_depends_edited(rel: &str, heading: &str, st: crate::status::QuestionStatus) -> Said {
    said(
        &rule::ANSWERED_DEPENDS_LINE_FROZEN,
        format!("{}: \"{}\" is [{}] and its depends line was edited", rel, heading, st.name()),
        Next::agent(Act::Restores("the line".into())),
    )
}

pub fn dep_not_current_decision(rel: &str, heading: &str, id: &str, name: Option<&str>) -> Said {
    said(
        &rule::DEPENDS_ENTRY_RESOLVES,
        format!("{}: \"{}\" depends on {} which names no term and no question of this file", rel, heading, named(id, name)),
        Next::agent(Act::Rewrites("the entry to name a term or a question of this file".into())),
    )
}

/// A live section's depends line names, by identifier, a term that is
/// retired.
pub fn dep_retired_term_id(rel: &str, heading: &str, id: &str, name: Option<&str>) -> Said {
    said(
        &rule::DEPENDS_TERM_IS_LIVE,
        format!(
            "{}: \"{}\" depends on the term {}, which is retired",
            rel,
            heading,
            name.map_or_else(|| id.to_string(), |n| format!("\"{}\"", n))
        ),
        Next::agent(Act::Rewrites("the entry to name a term that is not retired".into())),
    )
}

pub fn dep_no_term(rel: &str, heading: &str, text: &str) -> Said {
    said(
        &rule::DEPENDS_TERM_EXISTS,
        format!("{}: \"{}\" depends on \"{}\" which is no term", rel, heading, text),
        Next::agent(Act::Writes("the term in adrdom/terms.md".into())),
    )
}

/// A `depends:` entry, on a question being judged, names a current decision
/// of another topic file by its identifier.
pub fn dep_names_decision_of_other_file(rel: &str, heading: &str, id: &str, name: Option<&str>) -> Said {
    dep_names_other_file(rel, heading, &named(id, name))
}

/// The same, written as the heading of that decision.
pub fn dep_names_heading_of_other_file(rel: &str, heading: &str, text: &str) -> Said {
    dep_names_other_file(rel, heading, &format!("\"{}\"", text))
}

fn dep_names_other_file(rel: &str, heading: &str, target: &str) -> Said {
    said(
        &rule::DEPENDS_ENTRY_NAMES_TERM_OR_FILE_QUESTION,
        format!(
            "{}: \"{}\" depends on {}, a decision of another topic file",
            rel, heading, target
        ),
        Next::agent(Act::Rewrites(format!("the depends line of \"{}\" to name terms and questions of this file", heading))),
    )
}

/// The depends line of a question being judged names more questions than the
/// cap: `names` are the questions its entries name, in line order.
pub fn depends_line_over_limit(rel: &str, heading: &str, names: &[String], cap: usize) -> Said {
    let list = names.iter().map(|n| format!("\"{}\"", n)).collect::<Vec<_>>().join(", ");
    said(
        &rule::DEPENDS_LINE_QUESTION_LIMIT,
        format!("{}: \"{}\" depends on {} questions, the most is {}: {}", rel, heading, names.len(), cap, list),
        Next::agent(Act::Rewrites(format!("the depends line of \"{}\"", heading))),
    )
}

/// `show <file>` on a file that is not one of the topic files.
/// `show` names a path approve does not take (per "Does show print what a file
/// holds past its latest approval?": show takes every file approve
/// takes).
/// `ask` names a path that is no topic file: it asks the questions of topic
/// files alone.
pub fn plan_not_a_topic_file(rel: &str) -> String {
    refuse(
        format!("{} is not a topic file", rel),
        Next::agent(Act::Rewrites("the path to a topic file under adrdom/decisions".into())),
    )
}

/// `adr approve` on a path outside the repository or outside what approve
/// takes: the owner names one it takes.
pub fn approve_not_a_file_approve_takes(path: &str) -> String {
    refuse(
        format!("{} is not a file approve takes", path),
        Next::owner(Act::Rewrites("the path to a topic file under adrdom/decisions, adrdom/terms.md or a blame file under adrdom/blame".into())),
    )
}

pub fn show_not_a_file_approve_takes(rel: &str) -> String {
    refuse(
        format!("{} is not a file approve takes", rel),
        Next::agent(Act::Rewrites("the path to a topic file under adrdom/decisions, adrdom/terms.md or a blame file under adrdom/blame".into())),
    )
}

pub fn dep_retired_term(rel: &str, heading: &str, text: &str) -> Said {
    said(
        &rule::DEPENDS_TERM_IS_LIVE,
        format!("{}: \"{}\" names the retired term \"{}\"", rel, heading, text),
        Next::agent(Act::Rewrites("the entry".into())),
    )
}

fn ambiguous_heading_next() -> Next {
    Next::agent(Act::Writes("the identifier of the one it means".into()))
}

/// A heading `depends:` entry's `=A|B` set names a letter its target
/// doesn't have.
pub fn bad_option_letter(rel: &str, dependent: &str, target: &str, letter: char) -> Said {
    said(
        &rule::ANSWER_SET_NAMES_TARGET_OPTIONS,
        format!(
            "{}: \"{}\" depends on \"{}\" under an answer set naming option {}, which it does not have",
            rel, dependent, target, letter
        ),
        Next::agent(Act::Rewrites("the answer set".into())),
    )
}

/// `open` are the questions of the tier that are not answered yet, in file
/// order; `answered` counts the ones the owner answered, which hold their
/// place and are never moved.
pub fn tier_cap_exceeded(rel: &str, open: &[String], answered: usize, cap: usize) -> Said {
    let names = open.iter().map(|h| format!("\"{}\"", h)).collect::<Vec<_>>().join(", ");
    let found = if answered == 0 {
        format!("{}: {} questions in one tier, the most is {}: {}", rel, open.len(), cap, names)
    } else {
        format!(
            "{}: {} questions in one tier, {} answered, the most is {}: {}",
            rel,
            open.len() + answered,
            answered,
            cap,
            names
        )
    };
    let next = Next::agent(Act::Rewrites("the depends lines of these questions".into()));
    said(&rule::TIER_QUESTION_LIMIT, found, next)
}

// `derive_tiers`' three failure shapes (record::DeriveError), each with two
// wordings: the plain one (`approve`/`check`'s `plan` category) and `ask`'s,
// which always points at re-approving afterward.

use crate::record::DeriveError;

fn derive_what(rel: &str, e: &DeriveError) -> (&'static rule::Rule, String, Next) {
    match e {
        DeriveError::Dangling { section, entry } => (
            &rule::DEPENDS_ENTRY_RESOLVES,
            format!(
                "{}: \"{}\" depends on \"{}\" which is no heading in this file",
                rel, section, entry
            ),
            Next::agent(Act::Rewrites("the entry".into())),
        ),
        DeriveError::Cycle(names) => (
            &rule::DEPENDS_LINES_HAVE_NO_CYCLE,
            format!(
                "{}: depends lines lead back to their own question: {}",
                rel,
                names.iter().map(|n| format!("\"{}\"", n)).collect::<Vec<_>>().join(", ")
            ),
            Next::agent(Act::Rewrites("the depends lines of these questions".into())),
        ),
        DeriveError::SelfDep(h) => (
            &rule::QUESTION_NOT_ON_ITSELF,
            format!("{}: \"{}\" depends on itself", rel, h),
            Next::agent(Act::Removes("the entry".into())),
        ),
    }
}



pub fn derive_error(rel: &str, e: &DeriveError) -> Said {
    let (r, found, next) = derive_what(rel, e);
    said(r, found, next)
}

pub fn answered_id_missing(id: &str, heading: &str) -> Said {
    said(
        &crate::rule::ANSWERED_QUESTION_STAYS_IN_FILES,
        format!("id {} (\"{}\") is answered and current but missing from every topic file", id, heading),
        Next::agent(Act::Restores("the section".into())),
    )
}

// --- actions.rs: answer/refuse --------------------------------------------

pub fn id_not_found(id: &str) -> String {
    refuse(
        format!(
            "id {} not found in any topic file; show prints the id of every question that waits for an answer",
            id
        ),
        Next::owner_runs(Cmd::fixed("adr show")),
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
        Next::agent(Act::Rewrites("each file that cannot be read".into())),
    )
}

/// The question `answer` was given is closed: its latest ledger state is
/// `st`, an answer, a refusal, a supersede, a withdrawal or a drop.
pub fn already_closed(heading: &str, st: crate::status::QuestionStatus) -> String {
    refuse(
        format!("\"{}\" is already {}; show prints every question that waits for an answer", heading, st.name()),
        Next::owner_runs(Cmd::fixed("adr show")),
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
    format!("\"{}\" is edited since it was asked; {}", heading, report.moves_tail(Next::agent(Act::Restores("the question".into()))).render())
}




/// A question with a body that no `ask` has put to the owner.
/// `report`: what the rules find in the question's file and its moves
/// (`moves::of_unasked_question_in`).
pub fn not_asked_yet(heading: &str, rel: &str, report: &crate::moves::Report) -> String {
    format!("\"{}\" is {}; {}", heading, crate::status::QuestionStatus::NotAsked.name(), report.moves_tail(Next::agent_runs(command(&["adr", "ask", rel]))).render())
}

/// An answer that would remove a question with no identifier.
pub fn drop_without_id(heading: &str, rel: &str) -> Said {
    said(
        &crate::rule::DROP_NAMES_AN_IDENTIFIER,
        format!("answering would remove \"{}\", which has no identifier", heading),
        Next::owner_runs(command(&["adr", "approve", rel])),
    )
}

/// An answer that would remove a question that holds a decision or a
/// refusal (per "Does a decision change after it is answered?").
pub fn drop_of_closed(heading: &str, refused: bool, rel: &str) -> Said {
    said(
        &crate::rule::ANSWER_NEVER_REMOVES_CLOSED_QUESTION,
        format!(
            "answering would remove \"{}\", which is {}",
            heading,
            if refused { "refused" } else { "answered" }
        ),
        Next::agent(Act::Writes(format!("a question that supersedes \"{}\"", heading))).and_owner_runs(command(&["adr", "approve", rel])),
    )
}

/// An answer that would remove a question whose text matches no approval row.
pub fn drop_not_approved(heading: &str, rel: &str) -> Said {
    said(
        &crate::rule::DROP_HAS_APPROVED_TEXT,
        format!(
            "answering would remove \"{}\", whose text matches no approved row of {}",
            heading, rel
        ),
        Next::owner_runs(command(&["adr", "approve", rel])),
    )
}

/// A question with no body yet: nothing to ask or answer.
pub fn not_written(heading: &str, report: &crate::moves::Report) -> String {
    format!("\"{}\" is [{}]; {}", heading, crate::status::QuestionStatus::NoBody.name(), report.moves_tail(Next::agent(Act::Writes(format!("the body of \"{}\"", heading)))).render())
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
            Next::owner_runs(Cmd::fixed("adr show")),
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
        Next::owner_runs(command(&["adr", "show", rel])),
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
        Next::owner_runs(Cmd::fixed("adr show")),
    )
}

/// The move that writes an asked question's terms line again: `ask` of its
/// file, which rewrites the line in place.
fn ask_again(rel: &str) -> Next {
    Next::agent_runs(command(&["adr", "ask", rel]))
}

pub fn no_terms_line(rel: &str, id: &str, heading: &str) -> Said {
    said(&crate::rule::ASKED_QUESTION_HAS_TERMS_LINE, format!("{}: no terms line", named(id, Some(heading))), ask_again(rel))
}

pub fn malformed_terms_line(rel: &str, id: &str, heading: &str) -> Said {
    said(&crate::rule::ASKED_TERMS_LINE_PARSES, format!("{}: malformed terms line", named(id, Some(heading))), ask_again(rel))
}

/// `word` is the word of the question the gone term was named by.
pub fn terms_line_names_gone_term(rel: &str, id: &str, heading: &str, uuid: &str, word: &str) -> Said {
    said(
        &crate::rule::ANSWERED_TERMS_LINE_TERM_EXISTS,
        format!(
            "{}: terms line names term {} that no longer exists",
            named(id, Some(heading)),
            named(uuid, Some(word))
        ),
        ask_again(rel),
    )
}

pub fn terms_line_names_retired_term(rel: &str, id: &str, heading: &str, name: &str) -> Said {
    said(
        &crate::rule::ANSWERED_TERMS_LINE_TERM_IS_LIVE,
        format!(
            "{}: terms line names retired term \"{}\"",
            named(id, Some(heading)),
            name
        ),
        ask_again(rel),
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

/// A plan that differs from its latest approval, with the moves of its file
/// (`moves::of_topic_file_in`): the same moves `show` names for the file, so
/// the refusal never names the owner's approve while approve would refuse
/// (per "Does a command compute a move of its own?", aac5c313 A).
pub fn plan_not_approved(rel: &str, status: FileStatus, report: &crate::moves::Report) -> Said {
    said(&crate::rule::PLAN_APPROVED_BEFORE_ASK, format!("{}: plan is {}", rel, status.name()), report.moves_tail(Next::owner_runs(command(&["adr", "approve", rel]))))
}

/// `plan_not_approved` for the prose above the first question.
pub fn context_not_approved(rel: &str, status: FileStatus, report: &crate::moves::Report) -> Said {
    said(&crate::rule::PROSE_APPROVED_BEFORE_ASK, format!("{}: prose is {}", rel, status.name()), report.moves_tail(Next::owner_runs(command(&["adr", "approve", rel]))))
}

pub fn missing_answer_set(rel: &str, dependent: &str, target: &str) -> Said {
    said(
        &rule::DEPENDS_ENTRY_HAS_ANSWER_SET,
        format!("{}: \"{}\" depends on \"{}\" without an answer set", rel, dependent, target),
        Next::agent(Act::Writes("an answer set (=A, =A|B or =*) on that depends entry".into())).and_owner_runs(command(&["adr", "approve", rel])),
    )
}

/// A `depends:` entry that names what an earlier entry of the same line
/// already names, written another way.
pub fn depends_names_twice(rel: &str, dependent: &str, entry: &str) -> Said {
    said(
        &rule::DEPENDS_ENTRY_ONCE,
        format!("{}: \"{}\": the depends entry \"{}\" names what an earlier entry of the line names", rel, dependent, entry),
        Next::agent(Act::Removes("the entry".into())),
    )
}

/// A supersedes line naming one decision twice.
pub fn supersedes_names_twice(rel: &str, heading: &str, entry: &str) -> Said {
    said(
        &rule::SUPERSEDES_ENTRY_ONCE,
        format!("{}: \"{}\": the supersedes entry \"{}\" names what an earlier entry of the line names", rel, heading, entry),
        Next::agent(Act::Removes("the entry".into())),
    )
}

pub fn set_on_term(rel: &str, dependent: &str) -> Said {
    said(
        &rule::ANSWER_SET_NOT_ON_TERM,
        format!("{}: \"{}\" depends on a term under an answer set", rel, dependent),
        Next::agent(Act::Removes("the answer set from that entry".into())).and_owner_runs(command(&["adr", "approve", rel])),
    )
}

pub fn set_on_answered_question(rel: &str, dependent: &str, target: &str) -> Said {
    said(
        &rule::ANSWER_SET_ON_OPEN_QUESTION_ONLY,
        format!("{}: \"{}\" depends on \"{}\", which is answered, under an answer set", rel, dependent, target),
        Next::agent(Act::Removes("the answer set from that entry".into())).and_owner_runs(command(&["adr", "approve", rel])),
    )
}

pub fn question_without_body(rel: &str, heading: &str) -> Said {
    said(
        &rule::QUESTION_HAS_BODY,
        format!("{}: \"{}\" is [{}]", rel, heading, crate::status::QuestionStatus::NoBody.name()),
        Next::agent(Act::Writes(format!("the body of \"{}\"", heading))),
    )
}

/// An `unrelated:` entry names no current decision, no term, no other
/// question of the file and no dropped question.
pub fn unrelated_names_nothing(rel: &str, heading: &str, entry: &str) -> Said {
    said(
        &rule::UNRELATED_ENTRY_NAMES_SOMETHING,
        format!(
            "{}: \"{}\": the unrelated entry \"{}\" names no current decision, term or question of this file",
            rel, heading, entry
        ),
        Next::agent(Act::Rewrites("the entry".into())),
    )
}

/// An `unrelated:` entry is the heading of current decisions in several files.
pub fn unrelated_ambiguous(rel: &str, heading: &str, entry: &str, files: &[String]) -> Said {
    said(
        &rule::UNRELATED_ENTRY_NAMES_ONE_DECISION,
        format!(
            "{}: \"{}\": the unrelated entry \"{}\" is the heading of current decisions in {}",
            rel,
            heading,
            entry,
            files.join(", ")
        ),
        ambiguous_heading_next(),
    )
}

// --- the blame line of a question -------------------------------------------
// One line under a heading naming the approved blame file and the row ranges
// the question was born from (per "What does a question carry of the blame
// file it came from?"), judged by `rules::blame_line_refusals`.

/// A `blame:` entry that names no row range (`entry` as the line writes it):
/// a name alone.
pub fn blame_entry_without_range(rel: &str, heading: &str, entry: &str) -> Said {
    blame_entry_incomplete(rel, heading, entry, "row range")
}

fn blame_entry_incomplete(rel: &str, heading: &str, entry: &str, lacks: &str) -> Said {
    said(
        &rule::BLAME_LINE_NAMES_ROW_RANGES,
        format!("{}: \"{}\": the blame entry \"{}\" names no {}", rel, heading, entry, lacks),
        Next::agent(Act::Writes("the name of a blame file and the ranges of its rows".into())),
    )
}

/// A `blame:` entry whose name or identifier is no blame file.
pub fn blame_names_no_file(rel: &str, heading: &str, name: &str) -> Said {
    said(
        &rule::BLAME_LINE_NAMES_APPROVED_FILE,
        format!("{}: \"{}\": the blame entry \"{}\" names no blame file", rel, heading, name),
        Next::agent(Act::Writes("the path of a blame file under adrdom/blame without .md".into())),
    )
}

/// A `blame:` entry names the blame file `blame_rel`, which awaits its
/// approval (`status` is new or changed): the line waits for it.
pub fn blame_file_awaits_approval(rel: &str, heading: &str, blame_rel: &str, status: FileStatus) -> Said {
    said(
        &rule::BLAME_LINE_NAMES_APPROVED_FILE,
        format!("{}: \"{}\": the blame line names {}, which is {}", rel, heading, blame_rel, status.name()),
        crate::moves::approval_wait(),
    )
}

/// A `blame:` entry names a range that is the range of no row of the blame
/// file `blame_rel`.
pub fn blame_range_of_no_row(rel: &str, heading: &str, blame_rel: &str, from: usize, to: usize) -> Said {
    said(
        &rule::BLAME_LINE_NAMES_ROW_RANGES,
        format!(
            "{}: \"{}\": the blame line names {}-{}, the range of no row of {}",
            rel, heading, from, to, blame_rel
        ),
        Next::agent(Act::Writes("the range of a row of that blame file".into())),
    )
}

/// A question not yet asked has no neighbours on record for its heading.
pub fn plan_neighbours_missing(rel: &str, heading: &str) -> Said {
    said(
        &rule::PLAN_STEP_HAS_ITS_RECORD,
        format!("{}: \"{}\" has no neighbours on record for its heading", rel, heading),
        crate::moves::clause(MoveKind::Plan { rel: rel.to_string() }),
    )
}

/// A question not yet asked has no rows of the reader at its body hash.
pub fn plan_rows_missing(rel: &str, heading: &str) -> Said {
    said(
        &rule::PLAN_STEP_HAS_ITS_RECORD,
        format!("{}: \"{}\" has no reader rows for its heading with its body", rel, heading),
        crate::moves::clause(MoveKind::Plan { rel: rel.to_string() }),
    )
}

/// A neighbour in the reader's rows is on no depends, supersedes or
/// unrelated line of the question.
pub fn plan_line_missing(rel: &str, heading: &str, kind: &str, name: &str) -> Said {
    said(
        &rule::NEIGHBOUR_IS_ON_A_LINE,
        format!("{}: the {} \"{}\" is a neighbour of \"{}\" and is on no line", rel, kind, name, heading),
        Next::agent(Act::Writes(format!("the missing line under \"{}\"", heading))),
    )
}

/// One question `adr plan` read the neighbours of, and the step it read them
/// at: its heading, or its heading with its body.
pub fn plan_question_read(heading: &str, what: &str) -> String {
    format!("\"{}\": {}", heading, what)
}

/// One neighbour of a question as every command prints it (`adr plan` for
/// what it read, `adr show --neighbours`): its kind and name, the step whose
/// text it was brought for and the reader's word (a neighbour of a heading
/// has none), the line the agent put it on, and the reader's sentence of
/// reason (the same).
pub fn neighbour(kind: &str, name: &str, step: &str, word: Option<&str>, line: Option<&str>, reason: &str) -> String {
    let on = match line {
        Some(l) => format!("on the {} line", l),
        None => "no line".to_string(),
    };
    let at = match word {
        Some(w) => format!("{} {}", step, w),
        None => step.to_string(),
    };
    let text = format!("{} \"{}\": {}; {}", kind, name, at, on);
    if reason.is_empty() {
        text
    } else {
        format!("{}; {}", text, reason)
    }
}

/// One pair of current decisions the reader marked same or inconsistent,
/// as `adr read` prints it; no other word marks a pair.
pub fn read_pair(word: &str, first: &str, second: &str, reason: &str, next: Next) -> Said {
    debug_assert!(matches!(word, "same" | "inconsistent"), "{}", word);
    let rule = match word {
        "same" => &crate::rule::READ_PAIR_SAME,
        _ => &crate::rule::READ_PAIR_INCONSISTENT,
    };
    said(rule, format!("{}: \"{}\" and \"{}\": {}", word, first, second, reason), next)
}

pub fn ask_supersedes_not_current(rel: &str, heading: &str, old: &str, name: Option<&str>) -> Said {
    let target = if crate::record::is_uuid(old) {
        named(old, name)
    } else {
        format!("\"{}\"", old)
    };
    said(
        &rule::SUPERSEDES_TARGET_IS_CURRENT,
        format!("{}: \"{}\": supersedes {}, which is not a current decision", rel, heading, target),
        Next::agent(Act::Rewrites("the supersedes entry".into())),
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
) -> Said {
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
    let add = Act::Rewrites(format!("the supersedes line of \"{}\" to add {}", heading, names));
    said(
        &rule::SUPERSEDES_NAMES_DEPENDENTS,
        format!(
            "{}: \"{}\": supersedes a decision that current decisions stand on, and the line leaves out {}",
            rel, heading, names
        ),
        match moment {
            SupersedesMoment::Approve => Next::agent(add),
            SupersedesMoment::Asked => Next::agent_then_runs(add, command(&["adr", "ask", rel])),
        },
    )
}

/// A question whose depends line names a decision that its own supersedes
/// line replaces.
pub fn stands_on_what_it_supersedes(rel: &str, heading: &str, target: &str) -> Said {
    said(
        &rule::QUESTION_NOT_ON_WHAT_IT_SUPERSEDES,
        format!("{}: \"{}\" stands on \"{}\", which it supersedes", rel, heading, target),
        Next::agent(Act::Removes("the entry from its depends line".into())),
    )
}

/// A question of the plan stands on a decision that another question of the
/// same plan supersedes.
pub fn stands_on_what_the_plan_supersedes(rel: &str, heading: &str, target: &str, replacer: &str) -> Said {
    said(
        &rule::NO_QUESTION_ON_WHAT_PLAN_SUPERSEDES,
        format!("{}: \"{}\" stands on \"{}\", which \"{}\" supersedes", rel, heading, target, replacer),
        Next::agent(Act::Rewrites(format!("the depends entry to name \"{}\"", replacer))),
    )
}

/// A `supersedes:` entry written as a heading that more than one current
/// decision carries.
pub fn ask_supersedes_ambiguous(rel: &str, heading: &str, entry: &str, files: &[String]) -> Said {
    said(
        &rule::SUPERSEDES_ENTRY_NAMES_ONE_DECISION,
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

pub fn banned_word_in_candidate(rel: &str, heading: &str, suffix: &str) -> Said {
    said(
        &rule::BANNED_WORD_ABSENT_FROM_RECORDS,
        format!("{}: \"{}\": banned word{}", rel, heading, suffix),
        Next::agent(Act::Rewrites("the line".into())),
    )
}

pub fn pairs_no_term(rel: &str, heading: &str) -> Said {
    said(
        &rule::QUESTION_PAIRS_TO_TERM,
        format!("{}: \"{}\" uses no term", rel, heading),
        Next::agent(Act::Writes("the term in adrdom/terms.md".into())),
    )
}

pub fn pairs_no_term_at_write(rel: &str, heading: &str) -> Said {
    pairs_no_term(rel, heading)
}

// --- what is next in a topic file (`moves::of_topic_file`) ----------------
// One line per move, each opening with who makes it, in the words a
// refusal uses. `show` prints them under the file, `ask` when it has
// nothing to ask.


// --- show: the lines of its listing ----------------------------------------
// Labels and layout, no refusal among them: a note carries no `Next`.

use crate::status::FileStatus;

/// The header of a group of findings on a terminal (per "What does the tool
/// add to the shape of what it prints to a terminal?", f3c9e736 A).
pub fn group_findings() -> String {
    "Findings".to_string()
}

/// The header of a group of moves on a terminal.
pub fn group_moves() -> String {
    "Moves".to_string()
}

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


/// Under a question not yet asked: no neighbour of its heading and no word of
/// the reader on its body is on record.
/// The move that reads them is the file's own (`moves::of_planning`), not
/// computed here a second time.
pub fn show_no_records() -> String {
    "neighbours: none on record yet".to_string()
}

/// After the rows of a blame file under `show --questions`: how many of its
/// `decision` rows a question names and how many none names (per "Does check
/// report a decision row that no question names?", ee27fd4c A).
pub fn show_decision_rows(with: usize, without: usize) -> String {
    format!("decision rows: {} with a question, {} with none", with, without)
}

/// Above the lines of a changed part whose approved text no commit holds:
/// every line is shown as added, and why.
pub fn show_no_baseline() -> String {
    "the approved text is in no commit: every line shows as added".to_string()
}

/// Above the questions struck from a plan whose approved version no commit
/// holds, when the ledger alone cannot tell which of them the latest plan
/// held.
pub fn show_struck_unsure() -> String {
    "the approved plan is in no commit: every question approved for this file and gone from it is named, some may have left by an earlier approval".to_string()
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
        Next::owner_runs(command(&["adr", "answer", id_arg]).then("<letter>")),
    )
}

pub fn no_such_term(name: &str) -> String {
    refuse(
        format!("no term named \"{}\" in adrdom/terms.md", name),
        Next::agent(Act::Rewrites("the name to a term that exists in adrdom/terms.md".into())),
    )
}

/// `ban --term` with no terms file to look the name up in.
pub fn no_terms_file_for_term(name: &str) -> String {
    refuse(
        format!("cannot look up \"{}\": adrdom/terms.md is missing or cannot be read", name),
        Next::agent(Act::Writes("adrdom/terms.md".into())),
    )
}

pub fn already_in_use() -> Said {
    said(&crate::rule::BAN_NOT_ON_USED_WORD, "refusing: already in use", Next::agent(Act::Removes("the word from the repository".into())))
}

/// One place a word `ban` was given already stands: its file and line,
/// with the move that clears it, as check names a banned word's place.
pub fn in_use_at(location: &str, again: Cmd) -> Said {
    said(
        &crate::rule::BAN_NOT_ON_USED_WORD,
        format!("{}: holds a word given", location),
        Next::agent_then_runs(Act::Rewrites("the line".into()), again),
    )
}

// --- settings.json shape (install --claude / check / uninstall) -----------

/// The move on a failure of an owner command the owner cannot clear by
/// running it again (a file it cannot read or write): the agent fixes what
/// the error names, then the owner runs `command`.
pub fn after_fix_owner_runs(command: Cmd) -> Next {
    Next::agent(Act::Rewrites(FIX_THE_THING.to_string())).and_owner_runs(command)
}

/// The agent's move after a command of its own failed to read or write what
/// the error names: fix that, then run `command` again.
pub fn after_fix_agent_runs(command: Cmd) -> Next {
    Next::agent_then_runs(Act::Rewrites(FIX_THE_THING.to_string()), command)
}

/// The agent's move on a topic file `cmd` could not read at `rel`: a path
/// that does not exist is named again, any other failure is fixed first.
pub fn topic_file_unread(cmd: &str, rel: &str, e: &std::io::Error) -> Next {
    if e.kind() == std::io::ErrorKind::NotFound {
        // The file meant is not known, so the command is not named.
        Next::agent(Act::Rewrites(format!("the topic file given to adr {} to one under adrdom/decisions that exists", cmd)))
    } else {
        after_fix_agent_runs(command(&["adr", cmd, rel]))
    }
}

/// The move of a failure inside a step several commands share (the
/// embedding cache, the reader's file, the terms file as loaded): fix `what`,
/// then run the command that stopped, named at the head of its line, again.
pub fn fix_then_same_command(what: &str) -> Next {
    Next::agent(Act::Rewrites(what.to_string())).then(Act::RunsAgain)
}

/// The agent's move on a file a command cannot take as it stands: fix the
/// file at `path` so `so` holds, then run `command` again.
pub fn fix_then(path: &str, command: Cmd) -> Next {
    Next::agent_then_runs(Act::Rewrites(path.to_string()), command)
}

pub fn settings_not_json_object(path: &str, next: Next) -> String {
    refuse(format!("{} is not a JSON object", path), next.on(path))
}

pub fn settings_permissions_not_object(path: &str, next: Next) -> String {
    refuse(format!("{}: permissions is not an object", path), next.on(path))
}

pub fn settings_deny_not_array(path: &str, next: Next) -> String {
    refuse(format!("{}: permissions.deny is not an array", path), next.on(path))
}

// --- check/*.rs --------------------------------------------------------

pub fn check_parse_error(rel: &str, heading: &str, e: &str) -> Said {
    let found = format!("{}: {}: {}", rel, heading, e);
    if e == "duplicate id line" || e.starts_with("id line holds") {
        said(
            &rule::ID_ASSIGNED_BY_TOOL,
            found,
            Next::agent(Act::Removes("the id line".into())).and_owner_runs(command(&["adr", "approve", rel])),
        )
    } else if e.starts_with("answer names missing option") {
        said(&rule::ANSWER_NAMES_AN_OPTION, found, Next::agent(Act::Restores("the answer line".into())))
    } else {
        said(&rule::OPTION_IS_ONE_SENTENCE, found, Next::agent(Act::Rewrites("the option".into())))
    }
}

pub fn duplicate_id(id: &str, prev: &str, rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::ID_NAMES_ONE_QUESTION,
        format!("duplicate id {} in {} and {}:{}", id, prev, rel, heading),
        Next::agent(Act::Rewrites("the copied section".into())).then(Act::Removes("the id line copied with it".into()))
            .and_owner_runs(command(&["adr", "approve", rel])),
    )
}

/// An asked/refused id missing from every current topic file, struck from
/// the plan of `rel`, the file awaiting approval it was approved in: the
/// owner's approve of that file writes its withdrawn row.
pub fn removed_from_plan(id: &str, heading: &str, event: &str, rel: &str) -> Said {
    said(
        &crate::rule::ASKED_QUESTION_LEAVES_BY_APPROVAL,
        format!("{} (\"{}\", {}) was removed from the plan", id, heading, event),
        Next::owner_runs(command(&["adr", "approve", rel])),
    )
}

/// An asked/refused id whose file is removed, its removal awaiting approval:
/// `next` is the move `moves::removal_next` names for that file.
pub fn removed_with_file(id: &str, heading: &str, event: &str, next: Next) -> Said {
    said(
        &crate::rule::FILE_LEAVES_WITH_NO_LIVE_QUESTION,
        format!("{} (\"{}\", {}) was removed with its file", id, heading, event),
        next,
    )
}

/// `heading` is the heading the ledger holds for the question, from the
/// row written when it was asked.
pub fn ledger_id_not_found(id: &str, heading: Option<&str>) -> Said {
    said(
        &crate::rule::LEDGER_IDENTIFIER_IN_A_FILE,
        format!("ledger id {} not found in any topic file", named(id, heading)),
        Next::agent(Act::Restores("the question's section".into())),
    )
}

pub fn withdrawn_id_present(id: &str, heading: &str, rel: &str) -> Said {
    said(
        &crate::rule::WITHDRAWN_ID_STAYS_OUT,
        format!("withdrawn id {} still present in {}", named(id, Some(heading)), rel),
        Next::agent(Act::Removes(format!("the section from {}", rel))),
    )
}

pub fn refusal_without_row(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::REFUSAL_LINE_HAS_ROW,
        format!("{}: {}: refusal line without ledger row", rel, heading),
        Next::agent(Act::Removes("the answer line".into())),
    )
}

/// A hand-written `**Answer:** <letter>` line with no genuine `answered`
/// ledger row behind it at all -- distinct from `answered_text_changed`,
/// whose answer line *is* backed by a real row at that same letter, but the
/// question's own text moved since. Removing the line is only the right fix
/// here, where nothing genuine is being thrown away.
pub fn answer_without_row(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::ANSWER_LINE_HAS_ROW,
        format!("{}: {}: answer line without ledger row", rel, heading),
        Next::agent(Act::Removes("the answer line".into())),
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
pub fn answered_text_changed(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::ANSWERED_TEXT_FROZEN,
        format!("{}: {}: text changed since being answered", rel, heading),
        Next::agent(Act::Restores(rel.to_string())),
    )
}

/// A refused section (a genuine `refused` row exists for its id) whose
/// text no longer hashes to that row: the refusal stands, the question's
/// text moved since. The refused question leaves at the next plan change,
/// so its text goes back to what was refused.
pub fn refused_text_changed(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::REFUSED_TEXT_FROZEN,
        format!("{}: {}: text changed since being refused", rel, heading),
        Next::agent(Act::Restores(rel.to_string())),
    )
}

pub fn answered_row_without_line(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::ANSWER_LINE_KEPT,
        format!("{}: {}: answered row without answer line", rel, heading),
        Next::agent(Act::Restores(rel.to_string())),
    )
}

pub fn refused_row_without_line(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::REFUSAL_LINE_KEPT,
        format!("{}: {}: refused row without answer line", rel, heading),
        Next::agent(Act::Restores(rel.to_string())),
    )
}

/// An asked question whose text no longer hashes as asked. `report`: the
/// findings and moves of the question's file (`moves::of_unasked_question_in`), the ones
/// `show` names for it (per "Does a command compute a move of its own?",
/// aac5c313 A).
pub fn edited_after_asking(rel: &str, heading: &str, report: &crate::moves::Report) -> Said {
    said(
        &crate::rule::ASKED_TEXT_FROZEN,
        format!("{}: {}: edited after asking", rel, heading),
        report.moves_tail(Next::agent(Act::Restores(rel.to_string()))),
    )
}

pub fn answer_set_changed(rel: &str, dep_heading: &str, heading: &str) -> Said {
    said(
        &crate::rule::ANSWER_SET_FROZEN_AT_ASK,
        format!("{}: {} answer set on {} changed after ask", rel, dep_heading, heading),
        Next::agent(Act::Restores("the answer set".into())),
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
pub fn answer_conflict(rel: &str, heading: &str, id: &str, a: &str, b: &str) -> Said {
    said(
        &crate::rule::ONE_STANDING_ANSWER,
        format!("{}: {}: two standing answers, {} and {}", rel, heading, a, b),
        Next::agent_runs(command(&["adr", "ask", rel]))
            .and_owner_runs(command(&["adr", "answer", &format!("{}@", id)]).then("<version> <letter>")),
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

pub fn banned_word_at(location: &str, suffix: &str) -> Said {
    said(
        &crate::rule::BANNED_WORD_ABSENT_FROM_FILES,
        format!("{}: banned word{}", location, suffix),
        Next::agent(Act::Rewrites("the line".into())),
    )
}

pub fn banned_missing_from_file(hash_prefix: &str) -> Said {
    said(
        &crate::rule::BAN_LIST_MATCHES_LEDGER,
        format!("{} in ledger, missing from the ban list", hash_prefix),
        Next::agent_runs(Cmd::fixed("adr ban --rebuild")),
    )
}

pub fn banned_missing_from_ledger(hash_prefix: &str) -> Said {
    said(
        &crate::rule::BAN_LIST_MATCHES_LEDGER,
        format!("{} in the ban list, no ledger row", hash_prefix),
        Next::agent_runs(Cmd::fixed("adr ban --rebuild")),
    )
}

/// A hash that stands in the file, was banned at some time, but the
/// ledger's latest row for it is `unbanned` -- the file still carries a
/// line the ledger already voided (per "Does check hold the ban list
/// equal to the ledger?"). `adr ban --rebuild` clears this: it rewrites the
/// file to exactly what the ledger currently says is banned, whatever the
/// file held before.
pub fn banned_unbanned_but_in_file(hash_prefix: &str) -> Said {
    said(
        &crate::rule::BAN_LIST_MATCHES_LEDGER,
        format!("{} in the ban list, ledger says unbanned", hash_prefix),
        Next::agent_runs(Cmd::fixed("adr ban --rebuild")),
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
pub fn section_no_terms_line(rel: &str, heading: &str, answered: bool) -> Said {
    let next = if answered {
        Next::agent(Act::Restores("the terms line".into()))
    } else {
        ask_again(rel)
    };
    said(&crate::rule::ASKED_QUESTION_HAS_TERMS_LINE, format!("{}: {}: no terms line", rel, heading), next)
}

/// approve of the terms file found an asked question whose `terms:` line
/// cannot be read, so it cannot tell which terms the question names.
pub fn asked_terms_line_unreadable(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::RETIRE_READS_ASKED_TERMS_LINES,
        format!("{}: {}: terms line cannot be read", rel, heading),
        Next::agent(Act::Restores(rel.to_string())),
    )
}

/// A section's `terms:` line is present but doesn't parse (per
/// `terms::validate_pairs`'s `Malformed`). Same answered/asked split as
/// `section_no_terms_line`.
pub fn section_terms_line_malformed(rel: &str, heading: &str, answered: bool) -> Said {
    let next = if answered {
        Next::agent(Act::Restores("the terms line".into()))
    } else {
        ask_again(rel)
    };
    said(&crate::rule::SECTION_TERMS_LINE_PARSES, format!("{}: {}: malformed terms line", rel, heading), next)
}

/// A section's `terms:` line names a uuid that isn't in `adrdom/terms.md`
/// at all. Same split as `section_no_terms_line`: an asked section can
/// still be re-asked; an answered one is frozen, so the fix moves to
/// restoring the term in `adrdom/terms.md` (mirrors `term_removed_but_carried`)
/// or superseding the decision.
pub fn section_term_not_found(rel: &str, heading: &str, word: &str, answered: bool) -> Said {
    let next = if answered {
        Next::agent(Act::Restores("the term in adrdom/terms.md".into()))
    } else {
        ask_again(rel)
    };
    said(
        &crate::rule::ASKED_TERMS_LINE_TERM_EXISTS,
        format!("{}: {}: term {} not found", rel, heading, word),
        next,
    )
}

/// A section's `terms:` line names a term that is now retired. Same split
/// as `section_no_terms_line`/`section_term_not_found`; the answered fix
/// mirrors `term_retired_but_carried`'s decision-dependent branch.
pub fn section_term_retired(rel: &str, heading: &str, name: &str, answered: bool) -> Said {
    let next = if answered {
        Next::agent(Act::Rewrites("the term in adrdom/terms.md".into()))
    } else {
        ask_again(rel)
    };
    said(
        &crate::rule::ASKED_TERMS_LINE_TERM_IS_LIVE,
        format!("{}: {}: uses the retired term {}", rel, heading, name),
        next,
    )
}

/// A term's own `terms:` line (its definition's pairings) names a uuid
/// that no longer exists.
pub fn term_definition_term_not_found(name: &str, word: &str) -> Said {
    said(
        &crate::rule::DEFINITION_TERM_EXISTS,
        format!("adrdom/terms.md: {}: term {} not found", name, word),
        Next::agent(Act::Rewrites("the definition".into())),
    )
}

pub fn term_definition_term_retired(name: &str, retired_name: &str) -> Said {
    said(
        &crate::rule::DEFINITION_USES_LIVE_TERMS,
        format!("adrdom/terms.md: {}: uses the retired term {}", name, retired_name),
        Next::agent(Act::Rewrites("the definition".into())),
    )
}

/// A term's own `terms:` line (machine-written by `approve` from its
/// definition) doesn't parse at all (per `terms::validate_pairs`'s
/// `Malformed`) -- never an agent's line to hand-fix, since only `approve`
/// ever writes it; the move is the same re-approve that regenerates it.
pub fn term_definition_terms_line_malformed(name: &str) -> Said {
    said(
        &crate::rule::DEFINITION_TERMS_LINE_PARSES,
        format!("adrdom/terms.md: {}: malformed terms line", name),
        Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// `check` at a commit: the working tree holds `path` as install writes it
/// and the staged files do not; staging it is the move.
pub fn install_file_not_staged(path: &str) -> Said {
    said(
        &crate::rule::STAGED_FILES_ARE_JUDGED_FILES,
        format!("{} not staged", path),
        Next::agent_runs(command(&["git", "add", path])),
    )
}

pub fn agents_md_stale() -> Said {
    said(
        &crate::rule::AGENTS_BRIEF_MATCHES_INSTALL,
        "AGENTS.md brief differs from this binary's",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn agents_md_missing_brief() -> Said {
    said(
        &crate::rule::AGENTS_BRIEF_MATCHES_INSTALL,
        "AGENTS.md lacks the adrdom brief",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn brief_file_differs() -> Said {
    said(
        &crate::rule::AGENTS_BRIEF_MATCHES_INSTALL,
        "adrdom/state/brief.md differs from this binary's brief",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn brief_file_missing() -> Said {
    said(
        &crate::rule::AGENTS_BRIEF_MATCHES_INSTALL,
        "adrdom/state/brief.md is missing",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn claude_md_missing_import() -> Said {
    said(
        &crate::rule::AGENT_FILE_IMPORTS_AGENTS_MD,
        "CLAUDE.md does not import AGENTS.md",
        Next::agent_runs(Cmd::fixed("adr install --claude")),
    )
}

pub fn missing_deny_entry(entry: &str) -> Said {
    said(
        &crate::rule::AGENT_SETTINGS_DENY_OWNER_COMMANDS,
        format!("{} lacks deny entry {}", ".claude/settings.json", entry),
        Next::agent_runs(Cmd::fixed("adr install --claude")),
    )
}

pub fn settings_unparseable(path: &str) -> Said {
    said(
        &crate::rule::AGENT_SETTINGS_FILE_PARSES,
        format!("{} cannot be parsed", path),
        Next::agent_then_runs(Act::Rewrites("the file".into()), Cmd::fixed("adr install --claude")),
    )
}

/// `check`'s `install` check on a git hook (`.githooks/pre-commit` or
/// `.githooks/commit-msg`, per `path`): the file exists but its content
/// differs from what `adr install` writes.
pub fn hook_differs(path: &str) -> Said {
    said(
        &crate::rule::HOOK_MATCHES_INSTALL,
        format!("{} differs from what install writes", path),
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// Same check as `hook_differs`, but the hook's content is current and the
/// file is not executable, so git would skip it without a word; install
/// repairs the mode.
pub fn hook_not_executable(path: &str) -> Said {
    said(
        &crate::rule::HOOK_IS_EXECUTABLE,
        format!("{} is not executable", path),
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// Same check as `hook_differs`, but the hook file is missing outright.
pub fn hook_missing(path: &str) -> Said {
    said(
        &crate::rule::HOOK_MATCHES_INSTALL,
        format!("{} is missing", path),
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn codex_rules_stale() -> Said {
    said(
        &crate::rule::CODEX_RULES_MATCH_INSTALL,
        ".codex/rules/adrdom.rules differs from this binary's",
        Next::agent_runs(Cmd::fixed("adr install --codex")),
    )
}

pub fn codex_rules_missing() -> Said {
    said(
        &crate::rule::CODEX_RULES_MATCH_INSTALL,
        ".codex/rules/adrdom.rules missing",
        Next::agent_runs(Cmd::fixed("adr install --codex")),
    )
}

/// `check`'s `install` check on `adrdom/.gitattributes`: the file exists
/// but its content differs from what `adr install` writes.
pub fn gitattributes_differs() -> Said {
    said(
        &crate::rule::GITATTRIBUTES_MATCH_INSTALL,
        "adrdom/.gitattributes differs from what install writes",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// Same check as `gitattributes_differs`, but the file is missing
/// outright.
pub fn gitattributes_missing() -> Said {
    said(
        &crate::rule::GITATTRIBUTES_MATCH_INSTALL,
        "adrdom/.gitattributes is missing",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn malformed_ledger_row(lineno: usize, raw: &str) -> Said {
    said(
        &crate::rule::LEDGER_ROW_FIELDS,
        format!("line {}: malformed row: {}", lineno, raw),
        Next::owner(Act::Removes(format!("the row at line {}", lineno))),
    )
}

/// Per "Does check fail a ledger whose rows changed place?"
/// (c10c3a49-e993-4c9f-a46b-4a1ba3646dcf A): a HEAD row is missing or
/// changed -- a row's place in the file is free, so this never fires for
/// one that only moved.
pub fn ledger_not_append_only() -> Said {
    said(
        &crate::rule::LEDGER_ROWS_KEPT,
        "a row from HEAD's ledger is missing or changed",
        Next::owner(Act::Restores("the missing or changed row".into())),
    )
}

/// A merge (pending, via `MERGE_HEAD`, or already committed, via `HEAD`'s
/// second parent) is missing a ledger row one of the two branches wrote
/// (per "Does check at a merge look for the ledger rows of both
/// branches?"): the merge itself dropped a row instead of carrying it
/// forward, the one thing an append-only ledger can never do.
pub fn ledger_merge_drops_rows() -> Said {
    said(
        &crate::rule::MERGE_KEEPS_LEDGER_ROWS,
        "merge drops a ledger row of a merged branch",
        Next::owner(Act::Restores("the missing rows of the ledger".into())),
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
        Next::agent_runs(Cmd::fixed("git fetch --deepen=1")),
    )
}

pub fn ledger_answered_without_asked(id: &str, heading: Option<&str>) -> Said {
    said(
        &crate::rule::ANSWER_ROW_HAS_ASKED_ROW,
        format!("answered row for {} matches no asked row", named(id, heading)),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// A `dropped` row whose detail is neither `<id> <letter>` nor `<id>`.
pub fn ledger_dropped_malformed(id: &str, detail: &str) -> Said {
    said(
        &crate::rule::LEDGER_DROPPED_ROW_NAMES_ITS_CAUSE,
        format!("dropped row for {} has the detail \"{}\", not an identifier with a letter or an identifier", id, detail),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// A `dropped` row with no answer row (direct) or no dropped row (dependent) behind it.
pub fn ledger_dropped_without_cause(id: &str, cause: &str) -> Said {
    said(
        &crate::rule::DROPPED_ROW_HAS_CAUSE,
        format!("dropped row for {} has no cause: {}", id, cause),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// A `dropped` row whose id and hash match no per-question approved row.
pub fn ledger_dropped_not_approved(id: &str) -> Said {
    said(
        &crate::rule::LEDGER_DROPPED_HASH_IS_APPROVED,
        format!("dropped row for {} matches no approved row of that question", id),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// A `dropped` row of a question with an earlier `answered` or `refused` row.
pub fn ledger_dropped_closed(id: &str, event: &str) -> Said {
    said(
        &crate::rule::LEDGER_DROPPED_NOT_AFTER_CLOSE,
        format!("dropped row for {} follows its {} row", id, event),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// A dropped question that is still in a topic file.
pub fn ledger_dropped_still_in_file(id: &str, rel: &str) -> Said {
    said(
        &crate::rule::DROPPED_QUESTION_LEAVES_FILES,
        format!("{} has a dropped row but is still in {}", id, rel),
        Next::agent(Act::Removes("the question from the file".into())),
    )
}

/// A superseded question whose section is still in a topic file.
pub fn ledger_superseded_still_in_file(heading: &str, rel: &str) -> Said {
    said(
        &crate::rule::SUPERSEDED_QUESTION_LEAVES_FILES,
        format!("\"{}\" is superseded but its section is still in {}", heading, rel),
        Next::agent(Act::Removes("the question from the file".into())),
    )
}

/// At the commit that removes a dropped question: HEAD's text hashes to something else.
pub fn ledger_dropped_text_differs(id: &str) -> Said {
    said(
        &crate::rule::LEDGER_DROPPED_HASH_IS_REMOVED_TEXT,
        format!("dropped row for {} holds a hash that is not the hash of the text HEAD holds", id),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// At the commit that removes a dropped question: HEAD's set already held the letter.
pub fn ledger_dropped_set_holds_letter(id: &str, letter: &str) -> Said {
    said(
        &crate::rule::LEDGER_DROPPED_LETTER_NOT_IN_SET,
        format!("dropped row for {} names answer {}, which the set at HEAD holds", id, letter),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// At the commit that removes a dropped dependent: HEAD's text does not stand on the named question.
pub fn ledger_dropped_not_standing_on(id: &str, on: &str) -> Said {
    said(
        &crate::rule::DROPPED_ROW_NAMES_GROUND,
        format!("dropped row for {} says it stood on {}, which the question at HEAD does not", id, on),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// `adr check --push`, one line pre-push hands it: `local_ref`'s ledger, at
/// the sha about to be pushed, is missing a row `remote_ref` already holds
/// at its own sha (per "Does check look at the ledger before a push?",
/// 19e1c023-a678-4842-a414-bb0e45c41601 P). Same remedy as
/// `ledger_merge_drops_rows` -- the missing rows are restored, only by
/// adding them back -- since a push that drops a row is the same kind of
/// loss, just found before it reaches the remote instead of after a merge.
pub fn ledger_push_drops_rows(local_ref: &str, remote_ref: &str) -> Said {
    said(
        &crate::rule::PUSH_KEEPS_LEDGER_ROWS,
        format!("push of {} would drop ledger rows the remote's {} holds", local_ref, remote_ref),
        Next::owner(Act::Restores("the missing rows of the ledger".into())),
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
        Next::agent_runs(Cmd::fixed("git fetch")),
    )
}

/// `adr check --since <commit>` (per "Does check in CI compare the ledger
/// with the commit before the push?", a26876ba-5789-4d05-965e-853d00d9edd2
/// C): the ledger being judged is missing a row `<commit>`'s own ledger
/// already holds. Same remedy as `ledger_push_drops_rows`.
pub fn ledger_since_drops_rows(commit: &str) -> Said {
    said(
        &crate::rule::SINCE_KEEPS_LEDGER_ROWS,
        format!("ledger drops a row {} already holds", commit),
        Next::owner(Act::Restores("the missing rows of the ledger".into())),
    )
}

/// `adr check --since <commit>`: `<commit>` itself is not in this
/// repository (too shallow a fetch), so its ledger cannot be read at all.
pub fn ledger_since_unreadable(commit: &str) -> String {
    refuse(
        format!("{} cannot be read here", commit),
        Next::agent_runs(Cmd::fixed("git fetch")),
    )
}

/// Per "Does check fail two ledger rows on one subject with the same
/// time?" (ce19c883-c61d-477b-a6d4-fee5134c6e9a T): two rows that are not
/// byte-identical, carry the same time, and compete for one subject --
/// `subject` already names which kind it is and the value (`"id <uuid>"`,
/// `"hash <prefix>"`, or `"file <path>"`). The remedy is always the same
/// command run again, since that writes a later row, and a later row
/// settles which one stands.
pub fn ledger_same_time_competition(time: &str, subject: &str) -> Said {
    said(
        &crate::rule::LEDGER_LATEST_ROW_IS_ONE,
        format!("two ledger rows at {} compete for {}", time, subject),
        Next::owner(Act::RunsAgain),
    )
}

/// `locations` are the `path:line` places the tag was found at; `mv` is the
/// retie move (`moves::MoveKind::Retie`) that clears it.
pub fn stale_test(id: &str, heading: Option<&str>, locations: &[String], mv: Next) -> Said {
    let at = if locations.is_empty() {
        String::new()
    } else {
        format!(", at {}", locations.join(", "))
    };
    said(
        &crate::rule::TEST_TIE_NAMES_CURRENT_DECISION,
        format!("{} tied to a test and not a current decision{}", named(id, heading), at),
        mv,
    )
}

pub fn duplicate_heading_in_file(heading: &str, rel: &str) -> Said {
    said(
        &rule::HEADING_ONCE_IN_FILE,
        format!("{}: the heading \"{}\" appears twice", rel, heading),
        Next::agent(Act::Rewrites("one of the headings".into())),
    )
}

pub fn duplicate_heading_across_files(heading: &str, rel1: &str, rel2: &str) -> Said {
    said(
        &crate::rule::HEADING_IN_ONE_FILE,
        format!("duplicate heading \"{}\" in {} and {}", heading, rel1, rel2),
        Next::agent(Act::Rewrites("one of the two sections".into())),
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
pub fn bad_topic_file_name(rel: &str, new_path: &str) -> Said {
    said(
        &crate::rule::FILE_NAME_IS_LOWERCASE_HYPHENATED,
        format!("{}: file name is not lowercase words joined by single hyphens", rel),
        Next::agent(Act::Rewrites(format!("the name of {} to {}", rel, new_path))).and_owner_runs(command(&["adr", "approve", new_path])),
    )
}

/// The "named by it" half of the file name rule (per "What names a topic
/// file?"): a topic file whose name is well
/// formed (`repo::valid_topic_file_name`) but is not its h1 topic text
/// lowercased and hyphenated (`repo::expected_topic_file_stem`). Read the
/// same way as `bad_topic_file_name`.
pub fn topic_file_name_mismatch(rel: &str, topic: &str) -> Said {
    said(
        &crate::rule::FILE_NAME_MATCHES_TOPIC,
        format!("{}: file name does not match its topic \"{}\" in lowercase words joined by hyphens", rel, topic),
        Next::agent(Act::Rewrites("the topic line".into())).and_owner_runs(command(&["adr", "approve", rel])),
    )
}

/// The file name rule ("What names a topic file?") names a topic
/// file by its topic "in lowercase words joined by hyphens, with no number": a topic whose text lowercased and hyphenated
/// (`repo::expected_topic_file_stem`) holds a digit, or holds no letter at
/// all, has no name any file can carry, so no rename clears it and the
/// topic line itself has to change. `approve` refuses it through
/// `rules::own_name_refusals` and `check` through its plan check, from the
/// one message.
pub fn topic_unnameable(rel: &str, topic: &str) -> Said {
    said(
        &crate::rule::TOPIC_HAS_A_FILE_NAME,
        format!("{}: topic \"{}\" has no file name: a name holds lowercase words only", rel, topic),
        Next::agent(Act::Rewrites("the topic line".into())).and_owner_runs(command(&["adr", "approve", rel])),
    )
}

// --- adrdom/blame/*.md (blame files) --------------------------------------
// Shared by `blame::file_state` and the rules on a blame file (per "What
// does a blame file hold of its origin?", "Does a blame file cover its whole
// origin?"), read by `approve`, `check` and `show` alike.

/// A fault of the structure of a blame file: a header line missing, the
/// blank line after the header, a line that is no row.
pub fn blame_shape_error(rel: &str, e: &str) -> Said {
    said(&crate::rule::BLAME_FILE_SHAPE, format!("{}: {}", rel, e), Next::agent(Act::Rewrites("the blame file".into())))
}

/// A row whose value is neither `decision` nor `detail`.
pub fn blame_row_value_error(rel: &str, e: &str) -> Said {
    said(
        &crate::rule::BLAME_ROW_SAYS_DECISION_OR_DETAIL,
        format!("{}: {}", rel, e),
        Next::agent(Act::Writes("decision or detail after the tab".into())),
    )
}

/// An id line that holds no identifier, or sits below the header: the tool
/// writes the identifier of a blame file, at approval, as line 3.
pub fn blame_id_error(rel: &str, e: &str, approved: Option<&str>) -> Said {
    // `e` opens with the line it found, as "line N: ...". A file with an
    // approval gets its approved id line back instead.
    let next = match approved {
        Some(id) => Next::agent(Act::Writes(format!("back the id line \"id: {}\"", id))),
        None => match e.strip_prefix("line ").and_then(|r| r.split(':').next()).filter(|n| n.parse::<usize>().is_ok()) {
        Some(n) => Next::agent(Act::Removes(format!("the id line, line {}", n))),
        None => Next::agent(Act::Removes("the id line".into())),
        },
    };
    said(&crate::rule::BLAME_ID_ASSIGNED_BY_TOOL, format!("{}: {}", rel, e), next)
}

/// `rel` was approved and has no id line now: the identifier its approval
/// gave is gone, and approve does not assign another.
pub fn blame_id_line_gone(rel: &str, id: &str) -> Said {
    said(
        &crate::rule::BLAME_ID_ASSIGNED_BY_TOOL,
        format!("{}: the id line of its approval is gone", rel),
        Next::agent(Act::Writes(format!("back the id line \"id: {}\"", id))),
    )
}

/// The id line of `rel` holds an identifier no `blame` ledger row holds: the
/// tool did not write it.
pub fn blame_id_not_written(rel: &str) -> Said {
    said(
        &crate::rule::BLAME_ID_ASSIGNED_BY_TOOL,
        format!("{}: its id line holds an identifier the tool did not write", rel),
        Next::agent(Act::Removes("the id line".into())),
    )
}

/// `rel` was approved and its id line differs from the identifier its
/// approval gave (`written`).
pub fn blame_id_edited(rel: &str, written: &str) -> Said {
    said(
        &crate::rule::BLAME_ID_ASSIGNED_BY_TOOL,
        format!("{}: its id line differs from the identifier of its approval", rel),
        Next::agent(Act::Writes(format!("back the id line \"id: {}\"", written))),
    )
}

/// The id line of `rel` holds the identifier of another blame file
/// (`other_rel`, the one that keeps it): a copy of that file.
pub fn blame_id_shared(rel: &str, other_rel: &str) -> Said {
    said(
        &crate::rule::BLAME_ID_NAMES_ONE_FILE,
        format!("{}: its id line holds the identifier of {}", rel, other_rel),
        Next::agent(Act::Removes(format!("the id line from {}", rel))),
    )
}

fn range_text(start: usize, end: usize) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{}-{}", start, end)
    }
}

pub fn blame_coverage_gap(rel: &str, start: usize, end: usize) -> Said {
    said(
        &crate::rule::BLAME_COVERS_ORIGIN,
        format!("{}: line(s) {} of the origin are covered by no row", rel, range_text(start, end)),
        Next::agent(Act::Writes(format!("a row for {} {}", if start == end { "line" } else { "lines" }, range_text(start, end)))),
    )
}

pub fn blame_range_beyond(rel: &str, start: usize, end: usize, n: usize) -> Said {
    said(
        &crate::rule::BLAME_ROW_RANGE_WITHIN_ORIGIN,
        format!("{}: row {} is beyond the origin's {} line(s)", rel, range_text(start, end), n),
        Next::agent(Act::Rewrites("the row".into())),
    )
}

/// The origin line names a path under `repo::ADRDOM_DIR`, or one that exists
/// but is not a regular file (per the "origin" term: "a prose file outside
/// adrdom whose text the records take in"). `blame::valid_origin_path` is
/// the one place `approve`, `check` and `show` read this rule from.
pub fn blame_bad_origin_path(rel: &str, path: &str) -> Said {
    said(
        &crate::rule::BLAME_ORIGIN_IS_A_FILE,
        format!("{}: \"{}\" is not a regular file outside adrdom", rel, path),
        Next::agent(Act::Rewrites("the origin line".into())),
    )
}

// --- commands/check/blame.rs -----------------------------------------------
// Shared by `check`'s `blame` category (per "What does check do when an origin
// differs from its blame file's blob identifier?", 6f6e7f4a A; "What does
// check do when an origin is absent?", 0c3d498a A).

/// `git hash-object`'s own blob sha for `origin`'s current content no
/// longer matches the blame file's `blob:` line (6f6e7f4a A). A missing
/// origin is never reported here -- that is `blame_origin_missing`.
pub fn blame_blob_mismatch(rel: &str, origin: &str) -> Said {
    said(
        &crate::rule::BLAME_BLOB_MATCHES_ORIGIN,
        format!("{}: blob no longer matches {}", rel, origin),
        Next::agent(Act::Rewrites(format!("the rows of {} to the current origin", rel))).and_owner_runs(command(&["adr", "approve", rel])),
    )
}

/// `origin` is missing while the blame file `rel` awaits approval (`status`,
/// new or changed): an origin stays until its blame file is approved
/// (0c3d498a A). Once the file is approved the origin may go, and nothing
/// is reported.
pub fn blame_origin_missing(rel: &str, origin: &str, status: FileStatus) -> Said {
    said(
        &crate::rule::BLAME_ORIGIN_STAYS_UNTIL_APPROVED,
        format!("{}: origin {} is missing while the blame file is {}", rel, origin, status.name()),
        Next::agent(Act::Restores(origin.to_string())),
    )
}

/// A blame file itself (`rel`) could not be read as text at all (per "Does
/// check compare a blame file's blob identifier with its origin?", 6f6e7f4a
/// A: fail-open here means never silently skipping the file). Distinct from
/// `blame_shape_error` -- there is no content to have a shape at all.
pub fn blame_file_unreadable(rel: &str) -> String {
    refuse(
        format!("{}: cannot be read as text (e.g. its encoding)", rel),
        Next::agent(Act::Rewrites(rel.to_string())),
    )
}

/// `git hash-object --stdin` itself failed on `origin`'s current content
/// (6f6e7f4a A: the blob comparison must fail closed, never silently pass
/// when git could not be asked at all).
pub fn blame_hash_failed(rel: &str, origin: &str) -> String {
    refuse(
        format!("{}: git could not hash the current content of {}", rel, origin),
        Next::agent_then_runs(Act::Rewrites(origin.to_string()), Cmd::fixed("adr check")),
    )
}

/// `origin` sits at its path (per `Source::exists`) but its content could
/// not be read as text (0c3d498a A: distinct from the origin being
/// missing outright -- a present, unreadable origin is its own failure,
/// never routed through the missing-origin branch).
pub fn blame_origin_unreadable(rel: &str, origin: &str) -> String {
    refuse(
        format!("{}: origin {} exists but cannot be read as text (e.g. its encoding)", rel, origin),
        Next::agent(Act::Rewrites(origin.to_string())),
    )
}

// --- serve/mod.rs ------------------------------------------------------

pub fn bind_failed(port: u16, e: impl Display) -> String {
    refuse(
        format!("failed to bind 127.0.0.1:{}: {}", port, e),
        Next::owner_runs(Cmd::fixed("adr serve --port").then("<port>")),
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
/// check print a line with no move on it?", bd83aa22 A): `mv` is the move's
/// text, `moves::of_untested`.
pub fn untested(rel: &str, heading: &str, mv: Next) -> Said {
    said(
        &crate::rule::DECISION_HAS_TEST_TIE,
        format!("{}: {} is answered and no tracked file holds its tie", rel, heading),
        mv,
    )
}

/// A topic file `check` cannot read: a `parse` finding.
pub fn topic_file_unreadable(path: &str, e: impl Display) -> String {
    file_unreadable(path, e)
}

/// A file of the records that exists and cannot be read as text: a topic
/// file, or adrdom/terms.md.
pub fn file_unreadable(path: &str, e: impl Display) -> String {
    refuse(
        format!("{}: cannot be read as text (e.g. its encoding): {}", path, e),
        Next::agent(Act::Rewrites(path.to_string())),
    )
}

/// The owner's approve of one entry that waits, `adr approve adrdom/terms.md
/// <name>` (per "What does approve take for a terms file holding several
/// entries that wait?", da0dc8b1 A).
pub fn approve_entry_cmd(name: &str) -> Cmd {
    command(&["adr", "approve", crate::repo::TERMS_PATH, name])
}

/// The owner's approve of one entry of those the finding names: the owner
/// writes the name in place of `<name>`, as `<letter>` in an answer.
fn approve_one_of() -> Next {
    Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md").then("<name>"))
}

fn quoted_list(names: &[String]) -> String {
    names.iter().map(|n| format!("\"{}\"", n)).collect::<Vec<_>>().join(", ")
}

/// One entry of `adrdom/terms.md` waits for the owner's approve (per "Does
/// the tool stop while the terms file waits for approval?", aa475aa3 A, and
/// "What does approve take for a terms file holding several entries that
/// wait?", da0dc8b1 A): one finding per entry, each with its own approve.
/// `rel` is the topic file a gated command named, as `terms_not_approved`.
pub fn terms_entry_waits(rel: Option<&str>, name: &str, status: crate::status::TermStatus) -> Said {
    let at = rel.map(|r| format!("{}: ", r)).unwrap_or_default();
    said(
        &crate::rule::TERMS_FILE_APPROVED,
        format!("{}adrdom/terms.md: \"{}\" is {}", at, name, status.name()),
        Next::owner_runs(approve_entry_cmd(name)),
    )
}

/// `adr approve adrdom/terms.md` with no name while entries wait: the
/// owner names one (per da0dc8b1 A).
pub fn approve_terms_names_no_entry(waiting: &[String]) -> Said {
    said(
        &crate::rule::APPROVE_NAMES_ONE_TERM_ENTRY,
        format!("adrdom/terms.md: {} wait, and approve names none of them", quoted_list(waiting)),
        approve_one_of(),
    )
}

/// `adr approve adrdom/terms.md <name>` for a name no entry that waits
/// holds; with none waiting, the file is approved as a whole.
pub fn approve_terms_entry_not_waiting(name: &str, waiting: &[String]) -> Said {
    let (found, next) = if waiting.is_empty() {
        (
            format!("adrdom/terms.md: \"{}\" does not wait, and no entry does", name),
            Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
        )
    } else {
        (
            format!("adrdom/terms.md: \"{}\" does not wait; {} wait", name, quoted_list(waiting)),
            approve_one_of(),
        )
    };
    said(&crate::rule::APPROVE_NAMES_ONE_TERM_ENTRY, found, next)
}

/// An entry name given to approve of a file other than the terms file.
pub fn approve_entry_on_other_file(rel: &str) -> Said {
    said(
        &crate::rule::APPROVE_NAMES_ONE_TERM_ENTRY,
        format!("{}: approve takes an entry name for adrdom/terms.md only", rel),
        Next::owner_runs(command(&["adr", "approve", rel])),
    )
}

/// The finding on unban's input when `n` (never 0) of the words given were
/// not banned. A count and never the words: a word given here may be a
/// misspelling of a banned one.
pub fn unban_not_banned(n: usize) -> String {
    refuse(
        format!("unban: {} of the words given were not banned", n),
        Next::owner_runs(Cmd::fixed("adr unban")),
    )
}

/// `check --message <file>` could not read the commit message file.
pub fn message_file_unreadable(path: &str, e: impl Display) -> String {
    refuse(
        format!("cannot read the commit message file {}: {}", path, e),
        Next::agent(Act::Rewrites(format!("the commit message file {}", path))),
    )
}









// --- the embedding model and the install of adrdom/.gitignore -----------------

pub fn gitignore_differs() -> Said {
    said(
        &crate::rule::GITIGNORE_MATCHES_INSTALL,
        "adrdom/.gitignore differs from what install writes",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn gitignore_missing() -> Said {
    said(
        &crate::rule::GITIGNORE_MATCHES_INSTALL,
        "adrdom/.gitignore is missing",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// The embedding model is not on this machine and this build cannot fetch it.
#[cfg(any(not(feature = "model"), feature = "fake-model"))]
pub fn model_missing() -> String {
    refuse(
        "model: the embedding model is missing".to_string(),
        Next::agent(Act::Builds("adr with the model feature".into())),
    )
}

/// The embedding model cannot be loaded from, or downloaded into, the cache directory.
#[cfg(feature = "model")]
pub fn model_load_failed(cache: &str, e: impl Display) -> String {
    refuse(
        format!("model: cannot load the embedding model from {}, which needs the network once: {}", cache, e),
        Next::agent(Act::RunsAgain),
    )
}

pub fn embed_failed(e: impl Display) -> String {
    refuse(
        format!("model: cannot embed the texts: {}", e),
        Next::agent(Act::RunsAgain),
    )
}

/// The reader is not on this machine: no `claude` program on PATH.
pub fn reader_missing() -> String {
    refuse(
        "reader: the claude program is missing".to_string(),
        Next::agent(Act::Installs("the claude program".into())),
    )
}

/// The reader gave no usable word for a pair of texts; nothing of that
/// call is kept.
/// The reader's own error is flattened to one line: a stack trace of
/// several lines never runs into the move.
pub fn reader_failed(neighbour: &str, e: impl Display) -> String {
    refuse(
        format!("reader: no word for the neighbour {}: {}", neighbour, crate::reader::flatten(&e.to_string())),
        Next::agent(Act::RunsAgain),
    )
}

pub fn reader_write_failed(e: impl Display) -> String {
    write_failed(crate::repo::READER_PATH, e, fix_then_same_command("the reader file"))
}

pub fn neighbours_write_failed(e: impl Display) -> String {
    write_failed(crate::repo::NEIGHBOURS_PATH, e, fix_then_same_command("the neighbours file"))
}

pub fn malformed_reader_row(lineno: usize, raw: &str) -> Said {
    said(
        &crate::rule::READER_ROW_FIELDS,
        format!("line {}: malformed row in the reader file: {}", lineno, raw),
        Next::owner(Act::Removes(format!("the row at line {}", lineno))),
    )
}

/// A row of HEAD's reader file is edited: its line is not there byte for
/// byte, and a line with the same time, text, neighbour, prompt and model is.
pub fn reader_row_edited() -> Said {
    said(
        &crate::rule::READER_ROWS_UNEDITED,
        "a row from HEAD's reader file is edited by hand",
        Next::owner(Act::Restores("the row".into())),
    )
}

pub fn reader_merge_row_edited() -> Said {
    said(
        &crate::rule::MERGE_LEAVES_READER_ROWS_UNEDITED,
        "merge edits a reader row of a merged branch",
        Next::owner(Act::Restores("the row".into())),
    )
}

pub fn reader_push_row_edited(local_ref: &str, remote_ref: &str) -> Said {
    said(
        &crate::rule::PUSH_LEAVES_READER_ROWS_UNEDITED,
        format!("push of {} would change a reader row the remote's {} holds", local_ref, remote_ref),
        Next::owner(Act::Restores("the row".into())),
    )
}

pub fn reader_since_row_edited(commit: &str) -> Said {
    said(
        &crate::rule::SINCE_LEAVES_READER_ROWS_UNEDITED,
        format!("reader file edits a row {} already holds", commit),
        Next::owner(Act::Restores("the row".into())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::DeriveError;

    // Every act renders with one opening of the fixed set, and a move built
    // of acts prints each clause as who takes it and that act.
    #[test]
    fn every_act_opens_with_one_of_the_set() {
        // adr: 50b31713-4fea-4aa3-842a-79c2750660ba
        let acts = [
            Act::Writes("x".into()),
            Act::Rewrites("x".into()),
            Act::Removes("x".into()),
            Act::Restores("x".into()),
            Act::Runs(Cmd::fixed("adr check")),
            Act::RunsAgain,
            Act::Builds("x".into()),
            Act::Installs("x".into()),
            Act::ChangesInto("x".into()),
        ];
        for act in acts {
            let text = act.render();
            assert_eq!(ACTS.iter().filter(|a| text.starts_with(**a)).count(), 1, "{}", text);
        }
        let mv = Next::agent(Act::Rewrites("the line".into()))
            .then(Act::Runs(Cmd::fixed("adr ask x.md")))
            .and_owner_runs(Cmd::fixed("adr approve x.md"));
        assert_eq!(mv.render(), "agent rewrites the line, then runs: adr ask x.md, owner runs: adr approve x.md");
        assert_eq!(Next::not_yet("after it is approved").render(), "not yet: after it is approved");
    }

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
            stops: Vec::new(),
            findings: Vec::new(),
            moves: vec![crate::moves::Move::new(crate::moves::MoveKind::Approve { path: "x.md".to_string() })],
        };
        let outputs: Vec<String> = vec![
            write_failed("x", e, Next::agent(Act::RunsAgain)),
            read_failed("x", e, Next::agent(Act::RunsAgain)),
            update_failed("x", e, Next::agent(Act::RunsAgain)),
            create_failed("x", e, Next::agent(Act::RunsAgain)),
            parse_failed("x", e, Next::agent(Act::RunsAgain)),
            stdin_read_failed(Next::agent(Act::RunsAgain)),
            not_a_repo("ask", &[], Actor::Agent),
            not_a_repo("approve", &["adrdom/decisions/x.md"], Actor::Owner),
            owner_refusal("approve", &["adrdom/decisions/x.md"]),
            owner_refusal("unban", &[]),
            terms_missing().text(),
            terms_load_error(e, Next::agent(Act::RunsAgain)),
            terms_shape_error(e).text(),
            terms_not_approved("adrdom/decisions/x.md", FileStatus::Changed).text(),
            terms_file_not_approved(FileStatus::Changed).text(),
            terms_brief_missing().text(),
            terms_brief_differs().text(),
            terms_brief_not_staged().text(),
            circular_definition(&["a".to_string(), "b".to_string()]).text(),
            terms_entry_waits(None, "a", crate::status::TermStatus::New).text(),
            terms_entry_waits(Some("x.md"), "a b", crate::status::TermStatus::Removed).text(),
            approve_terms_names_no_entry(&["a".to_string(), "b".to_string()]).text(),
            approve_terms_entry_not_waiting("c", &["a".to_string()]).text(),
            approve_terms_entry_not_waiting("c", &[]).text(),
            approve_entry_on_other_file("adrdom/decisions/x.md").text(),
            too_long("x.md", 3, "heading", crate::tells::LengthKind::Option, 21).text(),
            term_removed_but_carried("uuid-1", Some("word"), "term \"x\"", None).text(),
            term_retired_but_carried("x", "term \"y\"", RetiredUser::Definition).text(),
            term_retired_but_carried("x", "id-1: \"heading\"", RetiredUser::Decision("id-1", "heading")).text(),
            term_retired_but_carried("x", "x.md: \"heading\"", RetiredUser::Asked("x.md")).text(),
            term_retired_but_carried("x", "x.md: \"heading\"", RetiredUser::PlanHeading("x.md")).text(),
            word_moved_off_term("word", "term", "id-1: \"heading\"", Some("id-1")).text(),
            definition_too_long("x", 3).text(),
            terms_banned_word(3, "").text(),
            shape_error("x.md", "missing h1").text(),
            section_shape_error("x.md", "heading", "stray line 4").text(),
            tell("x.md", 3, "context", "claude", crate::tells::TellKind::Provenance).text(),
            no_depends_line("x.md", "heading").text(),
            no_depends_line_bare("heading").text(),
            section_no_terms_line("x.md", "heading", false).text(),
            section_no_terms_line("x.md", "heading", true).text(),
            section_terms_line_malformed("x.md", "heading", false).text(),
            section_terms_line_malformed("x.md", "heading", true).text(),
            section_term_not_found("x.md", "heading", "word", false).text(),
            section_term_not_found("x.md", "heading", "word", true).text(),
            section_term_retired("x.md", "heading", "term", false).text(),
            section_term_retired("x.md", "heading", "term", true).text(),
            term_definition_term_not_found("term", "word").text(),
            term_definition_term_retired("term", "retired-term").text(),
            term_definition_terms_line_malformed("term").text(),
            frozen_depends_edited("x.md", "heading", crate::status::QuestionStatus::Answered).text(),
            dep_not_current_decision("x.md", "heading", "id-1", Some("name")).text(),
            dep_retired_term_id("x.md", "heading", "id-1", Some("name")).text(),
            dep_no_term("x.md", "heading", "widget").text(),
            bad_option_letter("x.md", "dependent", "target", 'Z').text(),
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
            ).0.text(),
            crate::moves::answer_waits(
                "heading",
                "x.md",
                vec![crate::moves::Waits::Asking { heading: "target".to_string(), report: approve_x.clone() }],
            ).0.text(),
            tier_cap_exceeded("x.md", &["heading".to_string()], 0, 4).text(),
            tier_cap_exceeded("x.md", &["heading".to_string()], 3, 4).text(),
            derive_error("x.md", &DeriveError::SelfDep("heading".to_string())).text(),
            answered_id_missing("id-1", "heading").text(),
            id_not_found("id-1"),
            already_closed("heading", crate::status::QuestionStatus::Refused),
            already_closed("heading", crate::status::QuestionStatus::Answered),
            letter_not_an_option('Z', "heading", &['A', 'B'], "id-1", "abcd1234"),
            not_in_asked_state("heading", &approve_x),
            answer_missing_version("id-1", None),
            answer_missing_version("id-1", Some(("heading", "12345678"))),
            stale_version("heading", "x.md"),
            already_closed("heading", crate::status::QuestionStatus::Withdrawn),
            already_closed("heading", crate::status::QuestionStatus::Superseded),
            not_asked_yet("heading", "x.md", &approve_x),
            not_written("heading", &approve_x),
            dep_retired_term("x.md", "heading", "widget").text(),
            show_not_a_file_approve_takes("x.md"),
            invalid_version_format("Z0ZZZZZZ"),
            no_terms_line("x.md", "id-1", "heading").text(),
            malformed_terms_line("x.md", "id-1", "heading").text(),
            terms_line_names_gone_term("x.md", "id-1", "heading", "uuid-1", "word").text(),
            terms_line_names_retired_term("x.md", "id-1", "heading", "term").text(),
            plan_not_approved("x.md", FileStatus::New, &approve_x).text(),
            context_not_approved("x.md", FileStatus::New, &approve_x).text(),
            missing_answer_set("x.md", "dependent", "target").text(),
            set_on_answered_question("x.md", "dependent", "target").text(),
            question_without_body("x.md", "heading").text(),
            ask_supersedes_not_current("x.md", "heading", "id-1", Some("name")).text(),
            banned_word_in_candidate("x.md", "heading", "").text(),
            pairs_no_term("x.md", "heading").text(),
            pairs_no_term_at_write("x.md", "heading").text(),
            invalid_letter("z", "id-1@abcd1234"),
            no_such_term("widget"),
            already_in_use().text(),
            settings_not_json_object("path", Next::agent(Act::RunsAgain)),
            settings_permissions_not_object("path", Next::agent(Act::RunsAgain)),
            settings_deny_not_array("path", Next::agent(Act::RunsAgain)),
            check_parse_error("x.md", "heading", "duplicate id line").text(),
            duplicate_id("id-1", "x.md:heading", "y.md", "heading").text(),
            removed_from_plan("id-1", "heading", "asked", "x.md").text(),
            ledger_id_not_found("id-1", Some("heading")).text(),
            withdrawn_id_present("id-1", "heading", "x.md").text(),
            refusal_without_row("x.md", "heading").text(),
            answer_without_row("x.md", "heading").text(),
            answered_text_changed("x.md", "heading").text(),
            answered_row_without_line("x.md", "heading").text(),
            refused_row_without_line("x.md", "heading").text(),
            edited_after_asking(
                "x.md",
                "heading",
                &crate::moves::Report {
                    stops: Vec::new(),
                    findings: Vec::new(),
                    moves: vec![crate::moves::Move::new(crate::moves::MoveKind::Ask {
                        rel: "x.md".into(),
                        headings: vec!["heading".into()],
                    })],
                },
            )
            .text(),
            answer_set_changed("x.md", "dependent", "heading").text(),
            answer_conflict("x.md", "heading", "id-1", "A", "B").text(),
            banned_word_at("x.md:3", "").text(),
            banned_missing_from_file("abcdef123456").text(),
            banned_missing_from_ledger("abcdef123456").text(),
            banned_unbanned_but_in_file("abcdef123456").text(),
            agents_md_stale().text(),
            agents_md_missing_brief().text(),
            brief_file_differs().text(),
            brief_file_missing().text(),
            claude_md_missing_import().text(),
            missing_deny_entry("Bash(adr approve*)").text(),
            settings_unparseable(".claude/settings.json").text(),
            codex_rules_stale().text(),
            codex_rules_missing().text(),
            hook_differs(".githooks/pre-commit").text(),
            hook_missing(".githooks/pre-commit").text(),
            hook_not_executable(".githooks/pre-commit").text(),
            gitattributes_differs().text(),
            gitattributes_missing().text(),
            gitignore_differs().text(),
            gitignore_missing().text(),
            #[cfg(any(not(feature = "model"), feature = "fake-model"))]
            model_missing(),
            #[cfg(feature = "model")]
            model_load_failed("adrdom/cache/model", "no network"),
            embed_failed("x"),
            reader_missing(),
            unrelated_names_nothing("x.md", "Does X?", "Y").text(),
            unrelated_ambiguous("x.md", "Does X?", "Y", &["a.md".to_string(), "b.md".to_string()]).text(),
            blame_entry_without_range("x.md", "Does X?", "notes").text(),
            blame_names_no_file("x.md", "Does X?", "notes").text(),
            blame_range_of_no_row("x.md", "Does X?", "adrdom/blame/notes.md", 3, 4).text(),
            plan_neighbours_missing("x.md", "Does X?").text(),
            plan_rows_missing("x.md", "Does X?").text(),
            plan_line_missing("x.md", "Does X?", "term", "widget").text(),
            reader_failed("id-1", "no answer"),
            reader_write_failed("disk full"),
            malformed_reader_row(3, "bad row").text(),
            reader_row_edited().text(),
            reader_merge_row_edited().text(),
            reader_since_row_edited("abc123").text(),
            reader_push_row_edited("refs/heads/feat", "refs/heads/main").text(),
            malformed_ledger_row(3, "bad row").text(),
            ledger_not_append_only().text(),
            ledger_merge_drops_rows().text(),
            ledger_merge_parents_unreadable(),
            ledger_answered_without_asked("id-1", Some("heading")).text(),
            ledger_push_drops_rows("refs/heads/feat", "refs/heads/main").text(),
            ledger_push_remote_unreadable("refs/heads/main"),
            ledger_since_drops_rows("abc123").text(),
            ledger_since_unreadable("abc123"),
            ledger_same_time_competition("2024-01-01T00:00:00Z", "id id-1").text(),
            stale_test("id-1", None, &["x.rs:3".to_string()], Next::agent(Act::Removes("the tie comment from the test".into()))).text(),
            duplicate_heading_in_file("heading", "x.md").text(),
            duplicate_heading_across_files("heading", "x.md", "y.md").text(),
            bad_topic_file_name("adrdom/decisions/Bad_Name.md", "adrdom/decisions/bad-name.md").text(),
            topic_file_name_mismatch("adrdom/decisions/x.md", "Y").text(),
            topic_unnameable("adrdom/decisions/rule.md", "Rule 2").text(),
            bind_failed(7357, e),
            missing_answer_selection("id-1", "abcd1234"),
            blame_shape_error("adrdom/blame/x.md", "line 1: missing \"origin: <path>\"").text(),
            blame_row_value_error("adrdom/blame/x.md", "line 4: row value \"term: pass\" is neither decision nor detail").text(),
            blame_id_error("adrdom/blame/x.md", "line 3: id line holds \"x\", which is no identifier the tool assigns", None).text(),
            blame_id_line_gone("adrdom/blame/x.md", "11111111-1111-4111-8111-111111111111").text(),
            blame_id_not_written("adrdom/blame/x.md").text(),
            blame_id_edited("adrdom/blame/x.md", "11111111-1111-4111-8111-111111111111").text(),
            blame_id_shared("adrdom/blame/x.md", "adrdom/blame/y.md").text(),
            blame_coverage_gap("adrdom/blame/x.md", 3, 5).text(),
            blame_range_beyond("adrdom/blame/x.md", 1, 10, 5).text(),
            blame_blob_mismatch("adrdom/blame/x.md", "docs/x.md").text(),
            blame_origin_missing("adrdom/blame/x.md", "docs/x.md", FileStatus::New).text(),
            blame_origin_missing("adrdom/blame/x.md", "docs/x.md", FileStatus::Changed).text(),
            blame_bad_origin_path("adrdom/blame/x.md", "adrdom/blame/notes.md").text(),
            blame_file_unreadable("adrdom/blame/x.md"),
            blame_hash_failed("adrdom/blame/x.md", "docs/x.md"),
            blame_origin_unreadable("adrdom/blame/x.md", "docs/x.md"),
            // Added with the one-printer build: each names its move too.
            already_closed("heading", crate::status::QuestionStatus::Dropped),
            in_use_at("x.md:3", command(&["adr", "ban"])).text(),
            supersedes_names_twice("x.md", "heading", "entry").text(),
            depends_names_twice("x.md", "heading", "entry").text(),
            stdout_write_failed(e, Actor::Owner, command(&["adr", "approve", "x.md"])),
            stdout_write_failed(e, Actor::Agent, command(&["adr", "show"])),
            file_unreadable("adrdom/terms.md", e),
            plan_not_a_topic_file("adrdom/terms.md"),
            no_terms_line("x.md", "id-1", "heading").text(),
            remove_failed("x", e, Next::agent(Act::Rewrites("it".into()))),
            hooks_path_failed("set", Next::agent(Act::Rewrites("it".into()))),
            heading_not_a_question("x.md", 3).text(),
            approve_not_a_file_approve_takes("README.md"),
            dep_names_decision_of_other_file("x.md", "H?", "id-1", None).text(),
            dep_names_heading_of_other_file("x.md", "H?", "E?").text(),
            depends_line_over_limit("x.md", "H?", &["A?".to_string(), "B?".to_string(), "C?".to_string()], 2).text(),
            id_not_found_some_unread("id-1", &["y.md".to_string()]),
            drop_without_id("H?", "x.md").text(),
            drop_of_closed("H?", true, "x.md").text(),
            drop_of_closed("H?", false, "x.md").text(),
            drop_not_approved("H?", "x.md").text(),
            set_on_term("x.md", "H?").text(),
            supersedes_leaves_out("x.md", "H?", &[("y.md".to_string(), "G?".to_string(), "id-2".to_string())], SupersedesMoment::Approve).text(),
            supersedes_leaves_out("x.md", "H?", &[("x.md".to_string(), "G?".to_string(), "id-2".to_string())], SupersedesMoment::Asked).text(),
            stands_on_what_it_supersedes("x.md", "H?", "G?").text(),
            stands_on_what_the_plan_supersedes("x.md", "H?", "G?", "F?").text(),
            ask_supersedes_ambiguous("x.md", "H?", "G?", &["y.md".to_string(), "z.md".to_string()]).text(),
            no_terms_file_for_term("widget"),
            removed_with_file("id-1", "H?", "asked", Next::owner_runs(command(&["adr", "approve", "x.md"]))).text(),
            refused_text_changed("x.md", "H?").text(),
            asked_terms_line_unreadable("x.md", "H?").text(),
            install_file_not_staged(".githooks/pre-commit").text(),
            ledger_dropped_malformed("id-1", "x y z").text(),
            ledger_dropped_without_cause("id-1", "id-2").text(),
            ledger_dropped_not_approved("id-1").text(),
            ledger_dropped_closed("id-1", "answered").text(),
            ledger_dropped_still_in_file("id-1", "x.md").text(),
            ledger_superseded_still_in_file("Q?", "x.md").text(),
            ledger_dropped_text_differs("id-1").text(),
            ledger_dropped_set_holds_letter("id-1", "A").text(),
            ledger_dropped_not_standing_on("id-1", "id-2").text(),
            unban_not_banned(2),
            message_file_unreadable("m.txt", e),
        ];
        for o in &outputs {
            assert!(
                o.contains("; agent ") || o.contains("; owner "),
                "no next move: {}",
                o
            );
        }
        // What follows `owner runs: ` is what the agent posts to the owner
        // and the owner pastes: a command and nothing after it. A move with
        // no command to name (a ledger row a push, a merge or a commit
        // dropped: no command restores a row) says it in words, without
        // `runs:`.
        for o in &outputs {
            let Some((_, command)) = o.split_once("owner runs: ") else {
                continue;
            };
            // A shell comment after the command names the heading and is
            // not part of the command.
            let command = command.split(" #").next().unwrap_or(command);
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

    // adr: a469ecb9-a4f1-4a2a-89c5-7378cc1138b4
    //
    // The words of the count `show --questions` prints.
    #[test]
    fn the_count_of_decision_rows_reads_word_for_word() {
        assert_eq!(show_decision_rows(1, 2), "decision rows: 1 with a question, 2 with none");
    }

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    //
    // The five findings of a blame line, word for word: each ends in a move,
    // and the one that waits for a blame file's approval names the status of
    // the file by its name in status.rs and ends in the tail moves.rs holds.
    #[test]
    fn the_blame_line_findings_read_as_the_decision_words_them() {
        let (r, h) = ("x.md", "Does X?");
        assert_eq!(
            blame_entry_without_range(r, h, "notes").text(),
            "x.md: \"Does X?\": the blame entry \"notes\" names no row range; agent writes the name of a blame file and the ranges of its rows"
        );
        assert_eq!(
            blame_names_no_file(r, h, "notes").text(),
            "x.md: \"Does X?\": the blame entry \"notes\" names no blame file; agent writes the path of a blame file under adrdom/blame without .md"
        );
        for status in [FileStatus::New, FileStatus::Changed] {
            assert_eq!(
                blame_file_awaits_approval(r, h, "adrdom/blame/notes.md", status).text(),
                format!(
                    "x.md: \"Does X?\": the blame line names adrdom/blame/notes.md, which is {}; {}",
                    status.name(),
                    crate::moves::approval_wait().render()
                )
            );
        }
        assert_eq!(
            blame_range_of_no_row(r, h, "adrdom/blame/notes.md", 3, 3).text(),
            "x.md: \"Does X?\": the blame line names 3-3, the range of no row of adrdom/blame/notes.md; agent writes the range of a row of that blame file"
        );
    }

    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    #[test]
    fn a_second_blame_line_is_a_reference_line_given_twice() {
        assert_eq!(
            section_shape_error("x.md", "Does X?", "blame: second blame line, line 6").text(),
            "x.md: \"Does X?\": blame second line, line 6; agent rewrites the blame line"
        );
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
        let out = section_term_not_found("x.md", "heading", "word", true).text();
        assert!(!out.contains("retired"), "{}", out);
        assert!(out.contains("agent restores the term in adrdom/terms.md"), "{}", out);
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
            (owner_refusal("unban", &[]), "owner runs: adr unban"),
            (not_a_repo("unban", &[], Actor::Owner), "owner changes into a git repository, then runs: adr unban"),
            (stdin_read_failed(after_fix_owner_runs(Cmd::fixed("adr unban"))), "owner runs: adr unban"),
            (stdin_read_failed(after_fix_agent_runs(Cmd::fixed("adr ban"))), "then runs: adr ban"),
            (update_failed("banned list", e, after_fix_agent_runs(Cmd::fixed("adr ban"))), "then runs: adr ban"),
            (update_failed("ledger", e, after_fix_agent_runs(Cmd::fixed("adr ban"))), "then runs: adr ban"),
            (update_failed("banned list", e, after_fix_agent_runs(Cmd::fixed("adr ban --rebuild"))), "then runs: adr ban --rebuild"),
        ] {
            assert!(line.ends_with(command), "{} should end with {}", line, command);
        }
    }

    #[test]
    fn untested_names_the_decision_its_place_and_its_move() {
        let line = untested("x.md", "heading", Next::agent(Act::Builds("it".into()))).text();
        assert_eq!(
            line,
            "x.md: heading is answered and no tracked file holds its tie; agent builds it"
        );
    }

    // adr: 50b31713-4fea-4aa3-842a-79c2750660ba
    //
    // A move that has two legitimate acts names both, and the one for a
    // current decision names the question by identifier and heading.
    #[test]
    fn moves_name_one_act_each() {
        let asked = term_retired_but_carried("x", "x.md: \"H?\"", RetiredUser::Asked("x.md")).text();
        assert!(asked.ends_with("agent rewrites the question, then runs: adr ask x.md"), "{}", asked);
        assert!(!asked.contains("strikes it"), "{}", asked);
        let heading = term_retired_but_carried("x", "x.md: \"H?\"", RetiredUser::PlanHeading("x.md")).text();
        assert!(heading.ends_with("agent rewrites the heading, owner runs: adr approve x.md"), "{}", heading);
        assert!(!heading.contains("strikes it"), "{}", heading);
        let decision = term_retired_but_carried("x", "id-1", RetiredUser::Decision("id-1", "H?")).text();
        assert!(decision.contains("then writes a question that supersedes id-1 (\"H?\")"), "{}", decision);
        let removed = term_removed_but_carried("u", Some("w"), "id-1", Some(("id-1", "H?"))).text();
        assert!(removed.contains("then writes a question that supersedes id-1 (\"H?\")"), "{}", removed);
        let moved = word_moved_off_term("w", "t", "id-1", Some("id-1")).text();
        assert!(moved.ends_with("agent rewrites the entry of \"t\" so \"w\" names it"), "{}", moved);
        assert!(!moved.contains("or writes a question"), "{}", moved);
        assert!(section_no_terms_line("x.md", "H?", true).text().contains("agent restores the terms line from git"));
        let no_term = dep_no_term("x.md", "H?", "w").text();
        assert!(no_term.ends_with("agent writes the term in adrdom/terms.md"), "{}", no_term);
        assert!(!no_term.contains("or names another"), "{}", no_term);
        let not_current = dep_not_current_decision("x.md", "H?", "id-1", None).text();
        assert!(not_current.ends_with("agent rewrites the entry to name a term or a question of this file"), "{}", not_current);
    }

    #[test]
    fn dropped_malformed_row_prints_its_detail_and_a_removal_names_its_move() {
        assert!(ledger_dropped_malformed("id-1", "x y z").text().contains("\"x y z\""));
        let hold = crate::moves::RemovalHold {
            id: "id-1".into(),
            heading: "H?".into(),
            status: crate::status::QuestionStatus::Answered,
        };
        let line = removal_holds_open("x.md", &hold).text();
        assert_eq!(
            line.split("; ").last().unwrap(),
            crate::moves::removal_next("x.md", std::slice::from_ref(&hold)).render()
        );
    }

    #[test]
    fn a_duplicate_term_identifier_is_judged_by_its_own_rule() {
        let said = terms_shape_error("line 9: duplicate id \"u\" (also used at line 3)");
        assert_eq!(said.rule.name, "term_identifier_is_unique");
        assert_eq!(terms_shape_error("line 4: duplicate field \"id\"").rule.name, "term_entry_fields");
    }

    #[test]
    fn a_tell_and_a_length_hit_pick_their_rule_from_their_kind() {
        use crate::tells::{LengthKind, TellKind};
        assert_eq!(tell("x.md", 1, "H?", "said", TellKind::Provenance).rule.name, rule::TELL_NAMES_SPEAKER.name);
        assert_eq!(tell("x.md", 1, "H?", "and", TellKind::Join).rule.name, rule::TELL_JOINS_TWO_QUESTIONS.name);
        assert_eq!(too_long("x.md", 1, "H?", LengthKind::OneWordOption, 3).rule.name, rule::DOES_OPTION_IS_ONE_WORD.name);
        assert_eq!(too_long("x.md", 1, "H?", LengthKind::Option, 30).rule.name, rule::OPTION_WORD_LIMIT.name);
        assert_eq!(too_long("x.md", 1, "H?", LengthKind::Why, 30).rule.name, rule::WHY_WORD_LIMIT.name);
        assert_eq!(too_long("x.md", 1, "H?", LengthKind::Costs, 30).rule.name, rule::COSTS_WORD_LIMIT.name);
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
        Next::agent_runs(command(&["git", "commit", rel])),
    )
}

/// A removed topic file still held a current decision or an open question.
pub fn removal_holds_open(rel: &str, hold: &crate::moves::RemovalHold) -> Said {
    said(
        &crate::rule::FILE_REMOVAL_KEEPS_LIVE_QUESTIONS,
        format!(
            "{} cannot be removed: {} is {}",
            rel,
            named(&hold.id, Some(&hold.heading)),
            crate::moves::hold_words(hold.status)
        ),
        crate::moves::removal_next(rel, std::slice::from_ref(hold)),
    )
}



/// A commit in a planner's session that stages paths outside adrdom: the
/// first `named` paths, then "and N more".
pub fn planner_commit_outside(paths: &[String], named: usize) -> Said {
    let shown = paths.iter().take(named).cloned().collect::<Vec<_>>().join(", ");
    let rest = paths.len().saturating_sub(named);
    let more = if rest > 0 { format!(" and {} more", rest) } else { String::new() };
    said(
        &crate::rule::PLANNER_COMMIT_STAYS_IN_ADRDOM,
        format!("this planner's commit stages {}{} outside adrdom", shown, more),
        Next::builder(Act::Runs(command(&["git", "commit", "--", ":!adrdom"]))),
    )
}

/// The roles file of a session with an identifier cannot be read, at a commit.
pub fn planner_roles_unreadable(path: &str, err: &std::io::Error) -> Said {
    said(
        &crate::rule::PLANNER_COMMIT_STAYS_IN_ADRDOM,
        format!("{} cannot be read, so the role of this session is unknown: {}", path, err),
        Next::owner(Act::Rewrites(path.to_string())),
    )
}

/// The staged paths of a planner's commit could not be read from git.
pub fn planner_staged_unreadable() -> Said {
    said(
        &crate::rule::PLANNER_COMMIT_STAYS_IN_ADRDOM,
        "the staged paths of this planner's commit could not be read from git",
        Next::agent_then_runs(Act::Rewrites("the git repository".into()), Cmd::fixed("git commit")),
    )
}
