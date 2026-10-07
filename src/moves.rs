//! The move of every thing in the records: the step it takes next and who
//! takes it, computed once from its status and the findings of the rules
//! (per "Does a command compute a move of its own?", aac5c313 A). Every
//! command that prints a move reads it from here, and `Move::text` is the
//! one place a move becomes words.
use crate::ledger::Ledger;
use crate::msg::{Actor, Next};
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
    /// The agent clears a finding, named whole.
    Clear { finding: String },
    /// A finding whose only move is an answer elsewhere first.
    WaitFor { what: String },
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
    /// The agent asks these questions of `rel`.
    Ask { rel: String, headings: Vec<String> },
    /// The owner answers this asked question.
    Answer { id: String, version: String, heading: String },
    /// An asked question `answer` does not take until `on` are settled.
    AnswerWaits { heading: String, on: Vec<String> },
    /// A refused question holds the questions after it: the plan changes.
    Refused { heading: String },
    /// The agent builds a decision and ties a test to it.
    Build { heading: String, id: String },
    /// An answered decision built once the question beside it is answered.
    BuildWaits { heading: String, beside: String },
    /// An answered decision built once the plan changes: the question
    /// beside it is refused.
    BuildWaitsRefused { heading: String, beside: String },
    /// A removed topic file: the agent restores it and settles each hold by a
    /// plan change the owner approves.
    Removal { rel: String, holds: Vec<RemovalHold> },
    /// The agent writes a question that supersedes one decision of each pair
    /// the reader marked same or inconsistent.
    SupersedeMarkedPairs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    pub actor: Mover,
    pub kind: MoveKind,
}

impl Move {
    /// The move of `kind`, its actor fixed by the kind.
    pub fn new(kind: MoveKind) -> Move {
        let actor = match &kind {
            MoveKind::Approve { .. } | MoveKind::Answer { .. } => Mover::Owner,
            MoveKind::Removal { holds, .. } if holds.is_empty() => Mover::Owner,
            MoveKind::WaitFor { .. }
            | MoveKind::AnswerWaits { .. }
            | MoveKind::BuildWaits { .. }
            | MoveKind::BuildWaitsRefused { .. } => Mover::NotYet,
            _ => Mover::Agent,
        };
        Move { actor, kind }
    }

    /// The clauses of the move as a refusal's tail: who does what. `None` for
    /// a move nobody takes yet.
    pub fn next(&self) -> Option<Next> {
        Some(match &self.kind {
            MoveKind::Clear { finding } => Next::agent(format!("clears: {}", finding)),
            MoveKind::PlanUnresolved => Next::agent("runs adr check and clears what it names in the plan"),
            MoveKind::Plan { rel } => Next::agent(format!("runs: adr plan {}", rel)),
            MoveKind::WriteLines { headings } => Next::agent(format!("writes lines for {}", quoted(headings))),
            MoveKind::WriteBodies { headings } => Next::agent(format!("writes the body of {}", quoted(headings))),
            MoveKind::Approve { path } => Next::owner_runs(format!("adr approve {}", path)),
            MoveKind::Ask { rel, headings } => Next::agent(format!("runs adr ask {}  # {}", rel, quoted(headings))),
            MoveKind::Answer { id, version, .. } => Next::owner_runs(format!("adr answer {}@{} <letter>", id, version)),
            MoveKind::Refused { heading } => Next::agent(format!(
                "changes the plan: \"{}\" is refused and holds the questions that come after it",
                heading
            )),
            MoveKind::Build { heading, id } => {
                Next::agent(format!("builds \"{}\" and puts \"adr: {}\" in a test that shows it", heading, id))
            }
            MoveKind::Removal { rel, holds } => removal_next(rel, holds),
            MoveKind::SupersedeMarkedPairs => {
                Next::agent("writes a question that supersedes one decision of each same or inconsistent pair")
            }
            MoveKind::WaitFor { .. }
            | MoveKind::AnswerWaits { .. }
            | MoveKind::BuildWaits { .. }
            | MoveKind::BuildWaitsRefused { .. } => return None,
        })
    }

    /// The move as one line: who does what, or `not yet: ...`.
    pub fn text(&self) -> String {
        match &self.kind {
            MoveKind::WaitFor { what } => format!("not yet: {}", what),
            MoveKind::AnswerWaits { heading, on } => {
                format!("not yet: \"{}\" is asked and waits for {}", heading, quoted(on))
            }
            MoveKind::BuildWaits { heading, beside } => {
                format!("not yet: \"{}\" is answered and is built once \"{}\" is answered", heading, beside)
            }
            MoveKind::BuildWaitsRefused { heading, beside } => format!(
                "not yet: \"{}\" is answered and is built once the plan changes, \"{}\" beside it is refused",
                heading, beside
            ),
            MoveKind::Answer { heading, .. } => {
                format!("{}  # \"{}\"", self.next().expect("an answer has a mover").render(), heading)
            }
            _ => self.next().expect("a move with a mover").render(),
        }
    }

    /// The move as the last line of a command: `next: <text>`.
    pub fn text_line(&self) -> String {
        format!("next: {}", self.text())
    }
}

fn quoted(headings: &[String]) -> String {
    headings.iter().map(|h| format!("\"{}\"", h)).collect::<Vec<_>>().join(", ")
}

/// The words of a hold in a refusal: what the question is that keeps the
/// file's removal from being approved.
pub fn hold_words(status: QuestionStatus) -> &'static str {
    match status {
        QuestionStatus::Answered => "a current decision",
        QuestionStatus::Asked => "asked and not answered",
        _ => "approved and not yet asked",
    }
}

/// The clauses that clear a removal held by `holds`, or the owner's
/// approval when nothing holds it. The agent restores the file, then
/// settles each hold by a plan change the owner approves: a current
/// decision is superseded, any other question struck from the plan.
pub fn removal_next(rel: &str, holds: &[RemovalHold]) -> Next {
    if holds.is_empty() {
        return Next::owner_runs(format!("adr approve {}", rel));
    }
    let steps: Vec<String> = holds
        .iter()
        .map(|h| {
            let q = crate::msg::named(&h.id, Some(&h.heading));
            if h.status == QuestionStatus::Answered {
                format!("supersedes {} with a new question", q)
            } else {
                format!("strikes {} from the plan", q)
            }
        })
        .collect();
    Next::agent(format!("restores {}, then {}, by a plan change", rel, steps.join(", ")))
        .and_owner_runs(format!("adr approve {}", rel))
}

/// The move of a removed topic file.
pub fn removal(rel: &str, holds: &[RemovalHold]) -> Move {
    if holds.is_empty() {
        return Move::new(MoveKind::Approve { path: rel.to_string() });
    }
    Move::new(MoveKind::Removal { rel: rel.to_string(), holds: holds.to_vec() })
}

/// The move a finding names: the agent clears it, or nobody moves yet when
/// it waits on an answer elsewhere.
pub fn of_finding(f: &Finding) -> Move {
    match f.waits_for() {
        Some(what) => Move::new(MoveKind::WaitFor { what: what.to_string() }),
        None => Move::new(MoveKind::Clear { finding: f.message.clone() }),
    }
}

/// Every move of the topic file `rf`, in the order they are taken: what is
/// answered is built before anything new is written, then the findings of
/// its plan, the planning or ask moves, the owner's answers, and what waits.
/// `plan_ok` is read by the caller off the file as loaded, before
/// `depends::normalize_cross_file_deps`. `tests` is `check::test_index`: an
/// answered decision with a test tied to it is built. `planning` is
/// `plan::load` of the file: a plan that waits for its approval, or holds a
/// heading with no body, follows the planning steps from it ("What does the
/// agent do to plan a topic file?", 46fc51ff A). `findings` are what the
/// rules find in the plan (`rules::judge_record`).
pub fn of_topic_file(
    rf: &RecordFile,
    led: &Ledger,
    plan_ok: bool,
    findings: &[Finding],
    tests: &HashMap<String, Vec<String>>,
    planning: Option<&crate::plan::Planning>,
) -> Vec<Move> {
    let clears: Vec<Move> = findings.iter().map(of_finding).collect();
    if rf.tiers.is_none() {
        if clears.is_empty() {
            return vec![Move::new(MoveKind::PlanUnresolved)];
        }
        return clears;
    }
    let mut bare = false;
    let mut to_ask = Vec::new();
    let mut answers = Vec::new();
    let mut held = Vec::new();
    let mut build = Vec::new();
    let mut build_waits = Vec::new();
    let mut waiting = Vec::new();

    for (i, sec) in rf.sections.iter().enumerate() {
        let heading = sec.heading_text.as_str();
        match status::question_status(led, sec) {
            QuestionStatus::NoBody | QuestionStatus::NotAsked if sec.is_skeleton => bare = true,
            QuestionStatus::NoBody | QuestionStatus::NotAsked => to_ask.push(heading.to_string()),
            QuestionStatus::Asked => {
                let Some(id) = &sec.id else { continue };
                // Edited since it was asked: `answer` takes only the text
                // the ledger holds, so the agent asks it again first.
                if !crate::view::asked_text_stands(led, rf, sec) {
                    to_ask.push(heading.to_string());
                    continue;
                }
                // `answer` takes it only once what it waits on is settled
                // (the one test `answer` itself reads).
                let w = state::answer_waits(rf, i);
                if w.is_empty() {
                    answers.push(Move::new(MoveKind::Answer {
                        id: id.clone(),
                        version: record::version_of(&rf.lines, sec),
                        heading: heading.to_string(),
                    }));
                } else {
                    let on: Vec<String> = w
                        .unanswered
                        .iter()
                        .chain(&w.refused_target)
                        .chain(&w.refused_tier)
                        .map(|&j| rf.sections[j].heading_text.clone())
                        .collect();
                    waiting.push(Move::new(MoveKind::AnswerWaits { heading: heading.to_string(), on }));
                }
            }
            QuestionStatus::Refused => held.push(Move::new(MoveKind::Refused { heading: heading.to_string() })),
            QuestionStatus::Answered => {
                let Some(id) = &sec.id else { continue };
                if tests.get(id).is_some_and(|locations| !locations.is_empty()) {
                    continue;
                }
                // A question beside it that still waits for an answer is
                // named ahead of a refused one: the refused one has its own
                // line, and the answer is what the build waits for longest.
                let beside = |s: &record::Section| s.heading_text.clone();
                match state::unresolved_sibling(rf, i) {
                    Some(s) if s.is_refused() => build_waits.push(Move::new(MoveKind::BuildWaitsRefused {
                        heading: heading.to_string(),
                        beside: beside(s),
                    })),
                    Some(s) => build_waits
                        .push(Move::new(MoveKind::BuildWaits { heading: heading.to_string(), beside: beside(s) })),
                    None => build.push(Move::new(MoveKind::Build { heading: heading.to_string(), id: id.clone() })),
                }
            }
            QuestionStatus::Superseded | QuestionStatus::Withdrawn | QuestionStatus::Dropped => {}
        }
    }

    // What is answered is built before anything new is written. Answers are
    // committed as given and check reports an untested decision; the build
    // still waits until the tier is answered, per "Does the agent build on a
    // decision before its tier is fully answered?" (f8d92841 A).
    let mut out = build;
    let refused = !clears.is_empty();
    out.extend(clears);
    if plan_ok {
        // Approve refuses a heading with no body, so one in an approved file
        // is one approved before it did: its body is written, then planned.
        if bare && planning.is_some() {
            out.extend(of_planning(rf, planning));
        }
        if !to_ask.is_empty() {
            out.push(Move::new(MoveKind::Ask { rel: rf.rel.clone(), headings: to_ask }));
        }
    } else if !refused {
        // Nothing is asked under a plan that waits for its approval; what
        // the owner owes and what is answered stands. The planning steps
        // come first, the owner's command once `approve` would take the plan.
        out.extend(of_planning(rf, planning));
    }
    out.extend(answers);
    out.extend(held);
    out.extend(build_waits);
    out.extend(waiting);
    out
}

/// Every move of the topic file `rf` as loaded from disk: the approval of
/// its plan and prose and its planning records read off the file as loaded,
/// then every finding `approve` refuses on (`rules::judge_topic_file_at`)
/// that is no planning step, each cleared by its own move; the planning
/// steps come from the planning records (`of_planning`). `all` is every
/// topic file as loaded and `tests` is `check::test_index`. The one move a
/// command that names one file prints (per "Does a command compute a move
/// of its own?", aac5c313 A).
pub fn of_topic_file_in(
    root: &std::path::Path,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[crate::terms::Term],
    led: &Ledger,
    tests: &HashMap<String, Vec<String>>,
) -> Vec<Move> {
    let plan_ok = state::plan_approved(led, rf, &rf.rel) && state::context_approved(led, rf, &rf.rel);
    let planning = crate::plan::load(root, rf, all, terms_list, led);
    let (findings, _) = crate::rules::judge_topic_file_split_at(root, rf, all, terms_list, led);
    let lookup = crate::terms::phrase_lookup(terms_list);
    let cross = crate::depends::cross_file_heading_index(all, led);
    let mut rf = rf.clone();
    crate::depends::normalize_cross_file_deps(&mut rf, terms_list, &lookup, &cross, led);
    of_topic_file(&rf, led, plan_ok, &findings, tests, Some(&planning))
}

/// The move of the answered decision `id`, section `i` of `rf`, that no
/// test names yet: the agent builds it, or nobody yet while a question
/// beside it waits for an answer or is refused (per "Does the agent build
/// on a decision before its tier is fully answered?"). The same move
/// `of_topic_file` names for it; what `check` prints beside an untested
/// decision.
pub fn of_untested(rf: &RecordFile, i: usize, id: &str) -> Move {
    let heading = rf.sections[i].heading_text.clone();
    match state::unresolved_sibling(rf, i) {
        Some(s) if s.is_refused() => {
            Move::new(MoveKind::BuildWaitsRefused { heading, beside: s.heading_text.clone() })
        }
        Some(s) => Move::new(MoveKind::BuildWaits { heading, beside: s.heading_text.clone() }),
        None => Move::new(MoveKind::Build { heading, id: id.to_string() }),
    }
}

/// The moves for one question that is not asked or was edited since
/// (`unwritten`: it has no body), for the refusal `answer` makes of a
/// question waiting on it, by the rule `of_topic_file` applies: the plan's
/// findings first; then, under an approved plan, the agent's `adr ask` (or,
/// for a question with no body, the planning move); under a plan that waits
/// for its approval and has no finding, the planning move.
pub fn of_unasked_question(
    rf: &RecordFile,
    plan_ok: bool,
    findings: &[Finding],
    planning: Option<&crate::plan::Planning>,
    heading: &str,
    unwritten: bool,
) -> Vec<Move> {
    let mut out: Vec<Move> = findings.iter().map(of_finding).collect();
    if plan_ok {
        if unwritten {
            out.extend(of_planning(rf, planning));
        } else {
            out.push(Move::new(MoveKind::Ask { rel: rf.rel.clone(), headings: vec![heading.to_string()] }));
        }
    } else if findings.is_empty() {
        out.extend(of_planning(rf, planning));
    }
    out
}

/// The next move of the planning steps for the questions of `rf` not yet
/// asked: the owner's approve once every record is there.
pub fn of_planning(rf: &RecordFile, planning: Option<&crate::plan::Planning>) -> Vec<Move> {
    use crate::plan::{NextMove, Step};
    let approve = || vec![Move::new(MoveKind::Approve { path: rf.rel.clone() })];
    let Some(p) = planning else {
        return approve();
    };
    match p.next() {
        NextMove::Plan => vec![Move::new(MoveKind::Plan { rel: rf.rel.clone() })],
        NextMove::WriteLines => {
            let mut missing = p.lines_missing(Step::Heading);
            if missing.is_empty() {
                missing = p.lines_missing(Step::Body);
            }
            let mut headings: Vec<String> = Vec::new();
            for (q, _) in missing {
                if !headings.contains(&q.heading) {
                    headings.push(q.heading.clone());
                }
            }
            vec![Move::new(MoveKind::WriteLines { headings })]
        }
        NextMove::WriteBodies(headings) => vec![Move::new(MoveKind::WriteBodies { headings })],
        NextMove::Ready => approve(),
    }
}

/// The moves of one blame file: each finding of the file as the agent's
/// move, and, for a draft with nothing left to clear, the owner's approval.
pub fn of_blame_file(rel: &str, findings: &[Finding], awaiting: bool) -> Vec<Move> {
    let mut out: Vec<Move> = findings.iter().map(of_finding).collect();
    if out.is_empty() && awaiting {
        out.push(Move::new(MoveKind::Approve { path: rel.to_string() }));
    }
    out
}

/// The moves of the terms file: each finding of the rules on it as the
/// agent's move, and, for a file that differs from its latest approval with
/// nothing left to clear, the owner's approval.
pub fn of_terms_file(findings: &[Finding], status: status::TermsFileStatus) -> Vec<Move> {
    let mut out: Vec<Move> = findings.iter().map(of_finding).collect();
    if out.is_empty() && status.awaits_approval() {
        out.push(Move::new(MoveKind::Approve { path: crate::repo::TERMS_PATH.to_string() }));
    }
    out
}

/// The move after a reading that marked a pair same or inconsistent.
pub fn of_marked_pairs() -> Move {
    Move::new(MoveKind::SupersedeMarkedPairs)
}

/// What one question `answer` waits on, as the refusal of `answer` names it:
/// `Unanswered` with the owner's answer move once the question is asked and
/// stands as asked, `Asking` while it is not (its `moves` are what
/// `of_unasked_question` names for it), a question with no body, or a
/// refused question the plan has to drop or replace.
pub enum Waits {
    Unanswered { heading: String, answer: Move },
    Asking { heading: String, moves: Vec<Move> },
    /// No decision yet and no question to answer first: the chain it waits
    /// on ends on `refused`, a refused question.
    Stuck { heading: String, refused: String },
    /// No decision yet and no body written.
    Unwritten { heading: String, moves: Vec<Move> },
    RefusedTarget { heading: String },
    RefusedTier { heading: String },
}

/// `answer` (a letter or a refusal) of a question that waits: it stands on a
/// question of its file with no decision, or a question of a lower tier is
/// refused (per "Does answer take a question standing on a question with
/// no decision?" and "Does a refused question hold the tiers above it?").
/// Every question it waits on is named, each with the move that clears it:
/// the agent's moves first, one `adr ask` naming every heading, and the one
/// owner move last.
pub fn answer_waits(heading: &str, rel: &str, waits: Vec<Waits>) -> String {
    let mut parts = Vec::new();
    let mut answer: Option<Move> = None;
    let mut refused: Vec<String> = Vec::new();
    let mut agent: Vec<Move> = Vec::new();
    let mut owner: Option<Move> = None;
    let add = |moves: Vec<Move>, agent: &mut Vec<Move>, owner: &mut Option<Move>| {
        for m in moves {
            if m.actor == Mover::Owner {
                owner.get_or_insert(m);
            } else if let MoveKind::Ask { headings, .. } = &m.kind {
                let have = agent.iter_mut().find_map(|a| match &mut a.kind {
                    MoveKind::Ask { headings: h, .. } => Some(h),
                    _ => None,
                });
                match have {
                    Some(h) => {
                        for x in headings {
                            if !h.contains(x) {
                                h.push(x.clone());
                            }
                        }
                    }
                    None => agent.push(m),
                }
            } else if !agent.contains(&m) {
                agent.push(m);
            }
        }
    };
    let refuse_on = |h: String, refused: &mut Vec<String>| {
        if !refused.contains(&h) {
            refused.push(h);
        }
    };
    for w in waits {
        match w {
            Waits::Stuck { heading: h, refused: r } => {
                parts.push(format!("\"{}\" has no decision yet and waits on the refused \"{}\"", h, r));
                refuse_on(r, &mut refused);
            }
            Waits::Unwritten { heading: h, moves } => {
                parts.push(format!("\"{}\" has no decision yet and no body", h));
                add(moves, &mut agent, &mut owner);
            }
            Waits::Asking { heading: h, moves } => {
                parts.push(format!("\"{}\" has no decision yet", h));
                add(moves, &mut agent, &mut owner);
            }
            Waits::Unanswered { heading: h, answer: m } => {
                parts.push(format!("\"{}\" has no decision yet", h));
                answer = answer.or(Some(m));
            }
            Waits::RefusedTarget { heading: h } => {
                parts.push(format!("\"{}\" is refused and the question stands on it", h));
                refuse_on(h, &mut refused);
            }
            Waits::RefusedTier { heading: h } => {
                parts.push(format!("\"{}\" is refused and holds the tiers above it", h));
                refuse_on(h, &mut refused);
            }
        }
    }
    // One owner move, last: the plan's approval when a refused question has
    // to go or the plan waits for it, else the first answer owed.
    let mut clauses: Vec<String> = agent.iter().map(Move::text).collect();
    if !refused.is_empty() {
        clauses.push(
            Next::agent(format!("writes the plan change that drops or replaces {}", quoted(&refused))).render(),
        );
        owner = Some(Move::new(MoveKind::Approve { path: rel.to_string() }));
    }
    if owner.is_none() {
        owner = answer;
    }
    // The answer move in a refusal is the command alone, without the heading
    // comment `show` prints after it.
    clauses.extend(owner.and_then(|m| m.next()).map(|n| n.render()));
    format!("\"{}\" waits: {}; {}", heading, parts.join("; "), clauses.join(", "))
}

/// The actor of a `msg::Next` clause, for a caller holding a `Mover`.
pub fn actor(m: Mover) -> Option<Actor> {
    match m {
        Mover::Agent => Some(Actor::Agent),
        Mover::Owner => Some(Actor::Owner),
        Mover::NotYet => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kind_fixes_who_moves() {
        assert_eq!(Move::new(MoveKind::Approve { path: "x.md".into() }).actor, Mover::Owner);
        assert_eq!(Move::new(MoveKind::Plan { rel: "x.md".into() }).actor, Mover::Agent);
        assert_eq!(Move::new(MoveKind::WaitFor { what: "w".into() }).actor, Mover::NotYet);
        assert_eq!(removal("x.md", &[]).actor, Mover::Owner);
    }

    #[test]
    fn each_move_reads_as_one_line() {
        let m = Move::new(MoveKind::Answer { id: "id-1".into(), version: "abcd".into(), heading: "Does it?".into() });
        assert_eq!(m.text(), "owner runs: adr answer id-1@abcd <letter>  # \"Does it?\"");
        let m = Move::new(MoveKind::Ask { rel: "x.md".into(), headings: vec!["A?".into(), "B?".into()] });
        assert_eq!(m.text(), "agent runs adr ask x.md  # \"A?\", \"B?\"");
        assert_eq!(Move::new(MoveKind::WaitFor { what: "w".into() }).text(), "not yet: w");
        assert_eq!(Move::new(MoveKind::Approve { path: "x.md".into() }).text_line(), "next: owner runs: adr approve x.md");
    }

    #[test]
    fn answer_waits_merges_the_ask_moves_and_puts_the_owner_last() {
        let ask = |h: &str| Move::new(MoveKind::Ask { rel: "x.md".into(), headings: vec![h.into()] });
        let line = answer_waits(
            "heading",
            "x.md",
            vec![
                Waits::Asking { heading: "a".into(), moves: vec![ask("a")] },
                Waits::Asking {
                    heading: "b".into(),
                    moves: vec![ask("b"), Move::new(MoveKind::Approve { path: "x.md".into() })],
                },
            ],
        );
        assert_eq!(
            line,
            "\"heading\" waits: \"a\" has no decision yet; \"b\" has no decision yet; agent runs adr ask x.md  # \"a\", \"b\", owner runs: adr approve x.md"
        );
    }

    #[test]
    fn a_hold_is_superseded_when_current_and_struck_otherwise() {
        let holds = vec![
            RemovalHold { id: "id-1".into(), heading: "A?".into(), status: QuestionStatus::Answered },
            RemovalHold { id: "id-2".into(), heading: "B?".into(), status: QuestionStatus::Asked },
        ];
        let text = removal("x.md", &holds).text();
        assert!(text.contains("supersedes") && text.contains("strikes"), "{}", text);
        assert!(text.ends_with("owner runs: adr approve x.md"), "{}", text);
    }
}
