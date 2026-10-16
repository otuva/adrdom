//! The move of every thing in the records: the step it takes next and who
//! takes it, computed once from its status and the findings of the rules
//! (per "Does a command compute a move of its own?", aac5c313 A). Every
//! command that prints a move reads it from here, and `Move::text` is the
//! one place a move becomes words.
mod command;
mod topic;
mod waits;

pub use command::{command, hold_words, removal_next, shell_word, Cmd};
pub use topic::{
    of_blame_file, of_marked_pairs, of_planning, of_removed_topic_file, of_terms_file,
    of_topic_file, of_topic_file_in, of_topic_file_judged, of_topic_file_with, of_unasked_question,
    of_unasked_question_in, of_untested, Judging,
};
pub use waits::{answer_waits, Waits};

use crate::ledger::Ledger;
use crate::msg::{Act, Next};
use crate::record::{self, RecordFile};
use crate::rules::Finding;
use crate::state;
use crate::status::{self, QuestionStatus};
use std::collections::HashMap;

/// Who takes a move: the agent, the owner, or nobody yet (the move waits on
/// another thing's move first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mover {
    Agent,
    /// The agent in a planner's session, for a build move (9a2ff21b).
    Builder,
    Owner,
    NotYet,
}

/// What keeps a removed topic file from being approved away: a question of
/// it whose status is a current decision (`Answered`), `Asked`, or a heading
/// of the approved plan never asked (`NotAsked`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalHold {
    pub id: String,
    pub heading: String,
    pub status: QuestionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveKind {
    /// The plan's tiers do not resolve and no finding names why.
    PlanUnresolved,
    /// The agent runs `adr plan` on the file.
    Plan { rel: String },
    /// The agent writes lines for some neighbours of these headings.
    WriteLines { headings: Vec<String> },
    /// The agent writes the bodies of these headings.
    WriteBodies { headings: Vec<String> },
    /// The owner approves the file at `path`.
    Approve { path: String },
    /// The owner approves one entry of the terms file that waits, by its
    /// name (da0dc8b1).
    ApproveEntry { name: String },
    /// The agent asks these questions of `rel`.
    Ask { rel: String, headings: Vec<String> },
    /// The owner answers this asked question.
    Answer {
        id: String,
        version: String,
        heading: String,
    },
    /// An asked question `answer` does not take until `on` are settled.
    AnswerWaits { heading: String, on: Vec<String> },
    /// A refused question holds the questions after it: the plan changes.
    Refused { heading: String },
    /// The agent builds a decision and ties a test to it.
    Build { heading: String, id: String },
    /// The agent moves a test's tie from a decision that is not current to
    /// the current decisions (heading, id) that took its place, or removes it
    /// when none did (2381d759).
    Retie { to: Vec<(String, String)> },
    /// The agent writes the from-comment a sentence of the brief lacks.
    BriefTieWrite,
    /// The agent rewrites a from-comment of the brief that names no
    /// decision or term, or is malformed.
    BriefTieFix,
    /// The agent moves a from-comment of the brief from a decision that is
    /// not current to the current decisions (heading, id) that took its
    /// place, or removes it when none did.
    BriefTieRetie { to: Vec<(String, String)> },
    /// An answered decision built once the question beside it is answered.
    BuildWaits { heading: String, beside: String },
    /// An answered decision built once the plan changes: the question
    /// beside it is refused.
    BuildWaitsRefused { heading: String, beside: String },
    /// An answered decision built once its file's tiers derive: no tier is
    /// known to wait on.
    BuildWaitsTiers { heading: String },
    /// The agent writes a question that supersedes one decision of each pair
    /// the reader marked same or inconsistent.
    SupersedeMarkedPairs,
    /// A commit waits while the terms file waits for the owner's approval:
    /// the hook's check refuses it until then.
    CommitWaits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    pub kind: MoveKind,
    /// The role of the session the move is computed in: a build move prints
    /// with builder in place of agent in a planner's session.
    pub session: crate::session::Role,
}

impl Move {
    pub fn new(kind: MoveKind) -> Move {
        Move {
            kind,
            session: crate::session::Role::Plain,
        }
    }

    /// The move as computed in a session of `session`.
    pub fn in_session(mut self, session: crate::session::Role) -> Move {
        self.session = session;
        self
    }

    /// A build move's words open with the mover `actor` names: builder in a
    /// planner's session, agent in any other.
    fn build_mover(&self, action: Act) -> Next {
        match self.actor() {
            Mover::Builder => Next::builder(action),
            _ => Next::agent(action),
        }
    }

    /// Who takes the move, fixed by its kind (and, for a build, the role of
    /// the session).
    pub fn actor(&self) -> Mover {
        if matches!(
            self.kind,
            MoveKind::Build { .. }
                | MoveKind::Retie { .. }
                | MoveKind::BriefTieWrite
                | MoveKind::BriefTieFix
                | MoveKind::BriefTieRetie { .. }
        ) && self.session == crate::session::Role::Planner
        {
            return Mover::Builder;
        }
        match &self.kind {
            MoveKind::Approve { .. } | MoveKind::ApproveEntry { .. } | MoveKind::Answer { .. } => {
                Mover::Owner
            }
            MoveKind::CommitWaits
            | MoveKind::AnswerWaits { .. }
            | MoveKind::BuildWaits { .. }
            | MoveKind::BuildWaitsRefused { .. }
            | MoveKind::BuildWaitsTiers { .. } => Mover::NotYet,
            MoveKind::PlanUnresolved
            | MoveKind::Plan { .. }
            | MoveKind::WriteLines { .. }
            | MoveKind::WriteBodies { .. }
            | MoveKind::Ask { .. }
            | MoveKind::Refused { .. }
            | MoveKind::Build { .. }
            | MoveKind::Retie { .. }
            | MoveKind::BriefTieWrite
            | MoveKind::BriefTieFix
            | MoveKind::BriefTieRetie { .. }
            | MoveKind::SupersedeMarkedPairs => Mover::Agent,
        }
    }

    /// The clauses of the move as a refusal's tail: who does what. `None` for
    /// a move nobody takes yet.
    pub fn next(&self) -> Option<Next> {
        Some(match &self.kind {
            MoveKind::PlanUnresolved => Next::all(vec![
                Next::agent_runs(Cmd::fixed("adr check")),
                Next::agent(Act::Rewrites(
                    "the headings and lines of the plan".to_string(),
                )),
            ]),
            MoveKind::Plan { rel } => Next::agent_runs(command(&["adr", "plan", rel])),
            MoveKind::WriteLines { headings } => {
                Next::agent(Act::Writes(format!("lines for {}", quoted(headings))))
            }
            MoveKind::WriteBodies { headings } => {
                Next::agent(Act::Writes(format!("the body of {}", quoted(headings))))
            }
            MoveKind::Approve { path } => Next::owner_runs(command(&["adr", "approve", path])),
            MoveKind::ApproveEntry { name } => {
                Next::owner_runs(crate::msg::approve_entry_cmd(name))
            }
            MoveKind::Ask { rel, .. } => Next::agent_runs(command(&["adr", "ask", rel])),
            MoveKind::Answer { id, version, .. } => Next::owner_runs(
                command(&["adr", "answer", &format!("{}@{}", id, version)]).then("<letter>"),
            ),
            MoveKind::Refused { heading } => Next::agent(Act::Writes(format!(
                "the plan change that drops or replaces \"{}\"",
                heading
            ))),
            MoveKind::Build { heading, id } => {
                let action = Act::Builds(format!(
                    "\"{}\" and puts \"adr: {}\" in a test that shows it",
                    heading, id
                ));
                self.build_mover(action)
            }
            MoveKind::Retie { to } => {
                let action = if to.is_empty() {
                    Act::Removes("the tie comment from the test".to_string())
                } else {
                    let list: Vec<String> = to
                        .iter()
                        .map(|(h, id)| format!("\"{}\" (\"adr: {}\")", h, id))
                        .collect();
                    Act::Rewrites(format!("the tie comment to {}", list.join(" and ")))
                };
                self.build_mover(action)
            }
            MoveKind::BriefTieWrite => self.build_mover(Act::Writes(
                "the from-comment naming the decision or term the sentence states".to_string(),
            )),
            MoveKind::BriefTieFix => self.build_mover(Act::Rewrites(
                "the from-comment to name a decision or term of the records".to_string(),
            )),
            MoveKind::BriefTieRetie { to } => {
                let action = if to.is_empty() {
                    Act::Removes("the from-comment from the brief".to_string())
                } else {
                    let list: Vec<String> = to
                        .iter()
                        .map(|(h, id)| format!("\"{}\" ({})", h, id))
                        .collect();
                    Act::Rewrites(format!("the from-comment to {}", list.join(" and ")))
                };
                self.build_mover(action)
            }
            MoveKind::SupersedeMarkedPairs => Next::agent(Act::Writes(
                "a question that supersedes one decision of each same or inconsistent pair"
                    .to_string(),
            )),
            MoveKind::CommitWaits
            | MoveKind::AnswerWaits { .. }
            | MoveKind::BuildWaits { .. }
            | MoveKind::BuildWaitsRefused { .. }
            | MoveKind::BuildWaitsTiers { .. } => return None,
        })
    }

    /// The move as one line of its own: who does what, or `not yet: ...`.
    /// A command line is followed by a `#` comment naming the questions it
    /// is about, so it is printed only where the move stands alone.
    pub fn text(&self) -> String {
        match &self.kind {
            MoveKind::Answer { heading, .. } => {
                format!("{}  # \"{}\"", self.clause_text(), heading)
            }
            MoveKind::Ask { headings, .. } => {
                format!("{}  # {}", self.clause_text(), quoted(headings))
            }
            _ => self.clause_text(),
        }
    }

    /// The move as one clause among others in a refusal's tail: no `#`
    /// comment, so the clauses after it are never read as part of one.
    pub fn clause_text(&self) -> String {
        self.clause().render()
    }

    /// The clauses of the move, a wait nobody takes yet among them.
    pub fn clause(&self) -> Next {
        match &self.kind {
            MoveKind::AnswerWaits { heading, on } => {
                Next::not_yet(format!("\"{}\" is asked and waits for {}", heading, quoted(on)))
            }
            MoveKind::BuildWaits { heading, beside } => {
                Next::not_yet(format!("\"{}\" is answered and is built once \"{}\" is answered", heading, beside))
            }
            MoveKind::BuildWaitsRefused { heading, beside } => Next::not_yet(format!(
                "\"{}\" is answered and is built once the plan changes, \"{}\" beside it is refused",
                heading, beside
            )),
            MoveKind::BuildWaitsTiers { heading } => {
                Next::not_yet(format!("\"{}\" is answered and is built once the tiers of its file derive", heading))
            }
            MoveKind::CommitWaits => {
                Next::not_yet(format!("commit, after the owner approves {}", crate::repo::TERMS_PATH))
            }
            _ => self.next().expect("a move with a mover"),
        }
    }
}

/// What the rules find in one thing of the records and the moves that
/// follow from its status: what every command that reads the thing prints,
/// each finding whole with the move that clears it, then the moves (per
/// "Does a command compute a move of its own?", aac5c313 A, and "What does
/// a command print when it writes nothing?", 1af76f91 A). A finding is never
/// turned into a move of its own: it carries its move.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// What stopped the terms file's gate on its own input or files
    /// (`rules::terms_gate`): a stop line, printed before the findings.
    pub stops: Vec<crate::out::Line>,
    pub findings: Vec<Finding>,
    pub moves: Vec<Move>,
}

impl Report {
    /// The moves alone, joined with `, `: the findings are printed apart,
    /// each on its own line. With no move, `fallback` names the act.
    pub fn moves_tail(&self, fallback: Next) -> Next {
        if self.moves.is_empty() {
            debug_assert!(
                !self.findings.is_empty(),
                "a refusal names at least one move"
            );
            return fallback;
        }
        Next::all(self.moves.iter().map(Move::clause).collect())
    }
}

/// The move of a finding whose only move is to wait for the approval of a
/// file elsewhere: nobody moves on it yet.
pub fn approval_wait() -> Next {
    Next::not_yet("after it is approved")
}

/// The clauses of the move `kind` names, for a refusal that stops in a state
/// whose move this module computes: the refusal names the same move every
/// command names for that state (per "Does a command compute a move of its
/// own?", aac5c313 A).
pub fn clause(kind: MoveKind) -> Next {
    Move::new(kind).next().expect("a move with a mover")
}

pub(super) fn quoted(headings: &[String]) -> String {
    headings
        .iter()
        .map(|h| format!("\"{}\"", h))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: 50b31713-4fea-4aa3-842a-79c2750660ba
    #[test]
    fn a_report_with_no_move_names_the_fallback_act() {
        let mut empty = Report::default();
        empty.findings.push(Finding::new(
            &crate::rule::ASKED_TEXT_FROZEN,
            "f".into(),
            Next::agent(Act::Writes("m".to_string())),
        ));
        assert_eq!(
            empty
                .moves_tail(Next::agent(Act::Restores("x.md".to_string())))
                .render(),
            "agent restores x.md from git"
        );
        assert!(!crate::msg::edited_after_asking("x.md", "h", &empty)
            .text()
            .contains("each finding of the file names"));
        assert!(crate::msg::edited_after_asking("x.md", "h", &empty)
            .text()
            .ends_with("agent restores x.md from git"));
    }

    #[test]
    fn the_kind_fixes_who_moves() {
        assert_eq!(
            Move::new(MoveKind::Approve {
                path: "x.md".into()
            })
            .actor(),
            Mover::Owner
        );
        assert_eq!(
            Move::new(MoveKind::Plan { rel: "x.md".into() }).actor(),
            Mover::Agent
        );
        assert_eq!(
            Move::new(MoveKind::BuildWaits {
                heading: "h".into(),
                beside: "b".into()
            })
            .actor(),
            Mover::NotYet
        );
        assert_eq!(
            of_removed_topic_file("x.md", Vec::new()).moves[0].actor(),
            Mover::Owner
        );
        let build = || {
            Move::new(MoveKind::Build {
                heading: "h".into(),
                id: "i".into(),
            })
        };
        assert_eq!(build().actor(), Mover::Agent);
        assert!(build().text().starts_with("agent builds"));
        let b = build().in_session(crate::session::Role::Planner);
        assert_eq!(b.actor(), Mover::Builder);
        assert!(b.text().starts_with("builder builds"), "{}", b.text());
    }

    #[test]
    fn each_move_reads_as_one_line() {
        let m = Move::new(MoveKind::Answer {
            id: "id-1".into(),
            version: "abcd".into(),
            heading: "Does it?".into(),
        });
        assert_eq!(
            m.text(),
            "owner runs: adr answer id-1@abcd <letter>  # \"Does it?\""
        );
        let m = Move::new(MoveKind::Ask {
            rel: "x.md".into(),
            headings: vec!["A?".into(), "B?".into()],
        });
        assert_eq!(m.text(), "agent runs: adr ask x.md  # \"A?\", \"B?\"");
        let m = Move::new(MoveKind::BuildWaits {
            heading: "A?".into(),
            beside: "B?".into(),
        });
        assert_eq!(
            m.text(),
            "not yet: \"A?\" is answered and is built once \"B?\" is answered"
        );
        assert_eq!(
            Move::new(MoveKind::Approve {
                path: "x.md".into()
            })
            .text(),
            "owner runs: adr approve x.md"
        );
    }

    // adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
    #[test]
    fn a_tail_of_clauses_holds_no_comment_before_a_later_clause() {
        let report = Report {
            stops: Vec::new(),
            findings: Vec::new(),
            moves: vec![
                Move::new(MoveKind::Ask {
                    rel: "x.md".into(),
                    headings: vec!["A?".into(), "B?".into()],
                }),
                Move::new(MoveKind::Approve {
                    path: "x.md".into(),
                }),
            ],
        };
        let tail = report.moves_tail(Next::agent(Act::Restores("x.md".to_string())));
        assert_eq!(
            tail.render(),
            "agent runs: adr ask x.md, owner runs: adr approve x.md"
        );
        assert!(!tail.render().contains('#'), "{}", tail);
    }
}
