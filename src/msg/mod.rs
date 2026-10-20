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

mod answer;
mod ask;
mod blame;
mod check;
mod check_notes;
mod commands;
mod repo;
mod satellite;
mod serve;
mod show;
mod terms;
mod topic;

pub use answer::*;
pub use ask::*;
pub use blame::*;
pub use check::*;
pub use check_notes::*;
pub use commands::*;
pub use repo::*;
pub use satellite::*;
pub use serve::*;
pub use show::*;
pub use terms::*;
pub use topic::*;

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
pub const ACTS: &[&str] = &[
    "writes ",
    "rewrites ",
    "removes ",
    "restores ",
    "runs: ",
    "runs the same adr command again",
    "builds ",
    "installs ",
    "changes into ",
];

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
        Next(vec![Clause::Do {
            actor,
            act,
            then: false,
        }])
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
        self.0.push(Clause::Do {
            actor,
            act: act.into(),
            then: true,
        });
        self
    }

    /// The clauses of every `Next` in `parts`, in order, as one.
    pub fn all(parts: Vec<Next>) -> Next {
        Next(parts.into_iter().flat_map(|n| n.0).collect())
    }

    pub fn and_owner_runs(mut self, command: Cmd) -> Next {
        self.0.push(Clause::Do {
            actor: Actor::Owner,
            act: Act::Runs(command),
            then: false,
        });
        self
    }

    /// Names `thing` as what a generic fix clause (`FIX_THE_THING`) rewrites.
    pub fn on(self, thing: &str) -> Next {
        Next(
            self.0
                .into_iter()
                .map(|c| match c {
                    Clause::Do { actor, act, then } => Clause::Do {
                        actor,
                        act: act.map_thing(|t| t.replace(FIX_THE_THING, thing)),
                        then,
                    },
                    w => w,
                })
                .collect(),
        )
    }

    pub fn render(&self) -> String {
        self.0
            .iter()
            .map(|c| match c {
                Clause::Do {
                    act, then: true, ..
                } => format!("then {}", act.render()),
                Clause::Do {
                    actor,
                    act,
                    then: false,
                } => {
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
    Said {
        rule,
        found: found.into(),
        next,
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
    refuse(
        format!("failed to write {}: {}", target, e),
        next.on(target),
    )
}

pub fn read_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("cannot read {}: {}", target, e), next.on(target))
}

pub fn update_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(
        format!("failed to update {}: {}", target, e),
        next.on(target),
    )
}

pub fn create_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(
        format!("failed to create {}: {}", target, e),
        next.on(target),
    )
}

pub fn remove_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(
        format!("failed to remove {}: {}", target, e),
        next.on(target),
    )
}

/// `git config` did not take the change to `core.hooksPath`. `doing` is
/// "set" or "unset".
pub fn hooks_path_failed(doing: &str, next: Next) -> String {
    refuse(
        format!("git config failed to {} core.hooksPath", doing),
        next.on("the git configuration of core.hooksPath"),
    )
}

/// Install stops: git's `core.hooksPath` is set away from `.githooks`, so
/// another program holds it (adr-code: 897b52a7-f6d2-467e-ba14-9f4ce4865664).
pub fn hooks_path_foreign(value: &str, install: Cmd) -> String {
    refuse(
        format!("core.hooksPath is {}, not .githooks", value),
        hooks_path_foreign_next(install),
    )
}

/// The move that clears a foreign `core.hooksPath`, for install's refusal and
/// for check's finding alike: install alone leaves it standing.
fn hooks_path_foreign_next(install: Cmd) -> Next {
    Next::owner_then_runs(
        Act::Removes("the hook manager that set core.hooksPath".into()),
        install,
    )
}

/// Install stops: the hook at `from` moves to `own`, and a file is there
/// already (adr-code: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0).
pub fn hook_move_blocked(from: &str, own: &str, install: Cmd) -> String {
    refuse(
        format!("{} would move to {}, which exists", from, own),
        Next::agent(Act::Rewrites(own.to_string()))
            .then(Act::Removes(from.to_string()))
            .then(Act::Runs(install)),
    )
}

/// Install stops: two hooks of one name would move to the same file.
pub fn hook_move_twice(from: &str, other: &str, own: &str, install: Cmd) -> String {
    refuse(
        format!("{} and {} would both move to {}", from, other, own),
        Next::agent(Act::Writes(own.to_string()))
            .then(Act::Removes(from.to_string()))
            .then(Act::Removes(other.to_string()))
            .then(Act::Runs(install)),
    )
}

pub fn parse_failed(target: &str, e: impl Display, next: Next) -> String {
    refuse(format!("{}: {}", target, e), next.on(target))
}

pub fn stdin_read_failed(next: Next) -> String {
    refuse(
        "failed to read stdin".to_string(),
        next.on("what is piped to adr"),
    )
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
    refuse(
        format!("output: standard output cannot be written: {}", e),
        next,
    )
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::DeriveError;
    use crate::status::FileStatus;

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
            assert_eq!(
                ACTS.iter().filter(|a| text.starts_with(**a)).count(),
                1,
                "{}",
                text
            );
        }
        let mv = Next::agent(Act::Rewrites("the line".into()))
            .then(Act::Runs(Cmd::fixed("adr ask x.md")))
            .and_owner_runs(Cmd::fixed("adr approve x.md"));
        assert_eq!(
            mv.render(),
            "agent rewrites the line, then runs: adr ask x.md, owner runs: adr approve x.md"
        );
        assert_eq!(
            Next::not_yet("after it is approved").render(),
            "not yet: after it is approved"
        );
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
            moves: vec![crate::moves::Move::new(crate::moves::MoveKind::Approve {
                path: "x.md".to_string(),
            })],
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
            term_retired_but_carried(
                "x",
                "id-1: \"heading\"",
                RetiredUser::Decision("id-1", "heading"),
            )
            .text(),
            term_retired_but_carried("x", "x.md: \"heading\"", RetiredUser::Asked("x.md")).text(),
            term_retired_but_carried("x", "x.md: \"heading\"", RetiredUser::PlanHeading("x.md"))
                .text(),
            word_moved_off_term("word", "term", "id-1: \"heading\"", Some("id-1")).text(),
            definition_too_long("x", 3).text(),
            terms_banned_word(3, "").text(),
            shape_error("x.md", "missing h1").text(),
            section_shape_error("x.md", "heading", "stray line 4").text(),
            tell(
                "x.md",
                3,
                "context",
                "claude",
                crate::tells::TellKind::Provenance,
            )
            .text(),
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
            frozen_depends_edited("x.md", "heading", crate::status::QuestionStatus::Answered)
                .text(),
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
                    crate::moves::Waits::RefusedTarget {
                        heading: "refused".to_string(),
                    },
                    crate::moves::Waits::RefusedTier {
                        heading: "other".to_string(),
                    },
                ],
            )
            .0
            .text(),
            crate::moves::answer_waits(
                "heading",
                "x.md",
                vec![crate::moves::Waits::Asking {
                    heading: "target".to_string(),
                    report: approve_x.clone(),
                }],
            )
            .0
            .text(),
            stand_cap_exceeded("x.md", None, &["heading".to_string()], 0, 4).text(),
            stand_cap_exceeded("x.md", Some("parent?"), &["heading".to_string()], 3, 4).text(),
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
            #[cfg(not(feature = "model"))]
            model_missing(),
            #[cfg(feature = "model")]
            model_not_installed("/home/x/.cache/adrdom/model"),
            #[cfg(feature = "model")]
            onnxruntime_not_installed("/home/x/.cache/adrdom/libonnxruntime.so.1.28.0"),
            #[cfg(feature = "model")]
            cache_dir_missing(),
            #[cfg(feature = "model")]
            model_load_failed("/home/x/.cache/adrdom/model", "bad file"),
            #[cfg(feature = "model")]
            onnxruntime_load_failed("/home/x/.cache/adrdom/libonnxruntime.so.1.28.0", "bad file"),
            #[cfg(feature = "model")]
            onnxruntime_removed("/home/x/.cache/adrdom/libonnxruntime.so.1.28.0", "bad file"),
            #[cfg(feature = "model")]
            onnxruntime_unusable("/home/x/.cache/adrdom/libonnxruntime.so.1.28.0", "bad file"),
            #[cfg(feature = "model")]
            onnxruntime_no_build("macos-x86_64"),
            #[cfg(feature = "model")]
            model_fetch_failed("the onnxruntime library", "no network"),
            embed_failed("x"),
            reader_missing("claude"),
            reader_missing("codex"),
            reader_no_agent(),
            unrelated_names_nothing("x.md", "Does X?", "Y").text(),
            unrelated_ambiguous(
                "x.md",
                "Does X?",
                "Y",
                &["a.md".to_string(), "b.md".to_string()],
            )
            .text(),
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
            stale_test(
                "id-1",
                None,
                &["x.rs:3".to_string()],
                Next::agent(Act::Removes("the tie comment from the test".into())),
            )
            .text(),
            brief_line_unmarked(3, Next::agent(Act::Writes("the from-comment".into()))).text(),
            brief_comment_malformed(
                3,
                "is empty",
                Next::agent(Act::Rewrites("the from-comment".into())),
            )
            .text(),
            brief_comment_unknown(
                3,
                "x",
                false,
                Next::agent(Act::Rewrites("the from-comment".into())),
            )
            .text(),
            brief_comment_stale(
                3,
                "id-1",
                None,
                false,
                Next::agent(Act::Removes("the from-comment".into())),
            )
            .text(),
            duplicate_heading_in_file("heading", "x.md").text(),
            duplicate_heading_across_files("heading", "x.md", "y.md").text(),
            bad_topic_file_name(
                "adrdom/decisions/Bad_Name.md",
                "adrdom/decisions/bad-name.md",
            )
            .text(),
            topic_file_name_mismatch("adrdom/decisions/x.md", "Y").text(),
            topic_unnameable("adrdom/decisions/rule.md", "Rule 2").text(),
            bind_failed(7357, e),
            missing_answer_selection("id-1", "abcd1234"),
            blame_shape_error("adrdom/blame/x.md", "line 1: missing \"origin: <path>\"").text(),
            blame_row_value_error(
                "adrdom/blame/x.md",
                "line 4: row value \"term: pass\" is neither decision nor detail",
            )
            .text(),
            blame_id_error(
                "adrdom/blame/x.md",
                "line 3: id line holds \"x\", which is no identifier the tool assigns",
                None,
            )
            .text(),
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
            hooks_path_foreign(".husky", command(&["adr", "install"])),
            hook_move_blocked(
                ".git/hooks/pre-commit",
                ".githooks/pre-commit.own",
                command(&["adr", "install"]),
            ),
            hook_move_twice(
                ".git/hooks/pre-commit",
                ".githooks/pre-commit",
                ".githooks/pre-commit.own",
                command(&["adr", "install"]),
            ),
            hooks_path_unset().text(),
            hooks_path_not_ours(".husky").text(),
            heading_not_a_question("x.md", 3).text(),
            approve_not_a_file_approve_takes("README.md"),
            dep_names_decision_of_other_file("x.md", "H?", "id-1", None).text(),
            dep_names_heading_of_other_file("x.md", "H?", "E?").text(),
            depends_line_over_limit(
                "x.md",
                "H?",
                &["A?".to_string(), "B?".to_string(), "C?".to_string()],
                2,
            )
            .text(),
            id_not_found_some_unread("id-1", &["y.md".to_string()]),
            drop_without_id("H?", "x.md").text(),
            drop_of_closed("H?", true, "x.md").text(),
            drop_of_closed("H?", false, "x.md").text(),
            drop_not_approved("H?", "x.md").text(),
            set_on_term("x.md", "H?").text(),
            supersedes_leaves_out(
                "x.md",
                "H?",
                &[("y.md".to_string(), "G?".to_string(), "id-2".to_string())],
                SupersedesMoment::Approve,
            )
            .text(),
            supersedes_leaves_out(
                "x.md",
                "H?",
                &[("x.md".to_string(), "G?".to_string(), "id-2".to_string())],
                SupersedesMoment::Asked,
            )
            .text(),
            stands_on_what_it_supersedes("x.md", "H?", "G?").text(),
            stands_on_what_the_plan_supersedes("x.md", "H?", "G?", "F?").text(),
            ask_supersedes_ambiguous(
                "x.md",
                "H?",
                "G?",
                &["y.md".to_string(), "z.md".to_string()],
            )
            .text(),
            no_terms_file_for_term("widget"),
            removed_with_file(
                "id-1",
                "H?",
                "asked",
                Next::owner_runs(command(&["adr", "approve", "x.md"])),
            )
            .text(),
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
                o.contains("; agent ") || o.contains("; owner ") || o.contains("; not yet: "),
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
            for prose in [
                " again", " if ", " to ", " and ", " with ", " first", ";", ",",
            ] {
                assert!(
                    !command.contains(prose),
                    "{:?} after the command: {}",
                    prose,
                    o
                );
            }
        }
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
            (
                not_a_repo("unban", &[], Actor::Owner),
                "owner changes into a git repository, then runs: adr unban",
            ),
            (
                stdin_read_failed(after_fix_owner_runs(Cmd::fixed("adr unban"))),
                "owner runs: adr unban",
            ),
            (
                stdin_read_failed(after_fix_agent_runs(Cmd::fixed("adr ban"))),
                "then runs: adr ban",
            ),
            (
                update_failed(
                    "banned list",
                    e,
                    after_fix_agent_runs(Cmd::fixed("adr ban")),
                ),
                "then runs: adr ban",
            ),
            (
                update_failed("ledger", e, after_fix_agent_runs(Cmd::fixed("adr ban"))),
                "then runs: adr ban",
            ),
            (
                update_failed(
                    "banned list",
                    e,
                    after_fix_agent_runs(Cmd::fixed("adr ban --rebuild")),
                ),
                "then runs: adr ban --rebuild",
            ),
        ] {
            assert!(
                line.ends_with(command),
                "{} should end with {}",
                line,
                command
            );
        }
    }

    // adr: 50b31713-4fea-4aa3-842a-79c2750660ba
    //
    // A move that has two legitimate acts names both, and the one for a
    // current decision names the question by identifier and heading.
    #[test]
    fn moves_name_one_act_each() {
        let asked =
            term_retired_but_carried("x", "x.md: \"H?\"", RetiredUser::Asked("x.md")).text();
        assert!(
            asked.ends_with("agent rewrites the question, then runs: adr ask x.md"),
            "{}",
            asked
        );
        assert!(!asked.contains("strikes it"), "{}", asked);
        let heading =
            term_retired_but_carried("x", "x.md: \"H?\"", RetiredUser::PlanHeading("x.md")).text();
        assert!(
            heading.ends_with("agent rewrites the heading, owner runs: adr approve x.md"),
            "{}",
            heading
        );
        assert!(!heading.contains("strikes it"), "{}", heading);
        let decision =
            term_retired_but_carried("x", "id-1", RetiredUser::Decision("id-1", "H?")).text();
        assert!(
            decision.contains("then writes a question that supersedes id-1 (\"H?\")"),
            "{}",
            decision
        );
        let removed = term_removed_but_carried("u", Some("w"), "id-1", Some(("id-1", "H?"))).text();
        assert!(
            removed.contains("then writes a question that supersedes id-1 (\"H?\")"),
            "{}",
            removed
        );
        let moved = word_moved_off_term("w", "t", "id-1", Some("id-1")).text();
        assert!(
            moved.ends_with("agent rewrites the entry of \"t\" so \"w\" names it"),
            "{}",
            moved
        );
        assert!(!moved.contains("or writes a question"), "{}", moved);
        assert!(section_no_terms_line("x.md", "H?", true)
            .text()
            .contains("agent restores the terms line from git"));
        let no_term = dep_no_term("x.md", "H?", "w").text();
        assert!(
            no_term.ends_with("agent writes the term in adrdom/terms.md"),
            "{}",
            no_term
        );
        assert!(!no_term.contains("or names another"), "{}", no_term);
        let not_current = dep_not_current_decision("x.md", "H?", "id-1", None).text();
        assert!(
            not_current
                .ends_with("agent rewrites the entry to name a term or a question of this file"),
            "{}",
            not_current
        );
    }

    #[test]
    fn dropped_malformed_row_prints_its_detail_and_a_removal_names_its_move() {
        assert!(ledger_dropped_malformed("id-1", "x y z")
            .text()
            .contains("\"x y z\""));
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
}
