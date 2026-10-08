//! The move of every thing in the records: the step it takes next and who
//! takes it, computed once from its status and the findings of the rules
//! (per "Does a command compute a move of its own?", aac5c313 A). Every
//! command that prints a move reads it from here, and `Move::text` is the
//! one place a move becomes words.
use crate::ledger::Ledger;
use crate::msg::Next;
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
    /// An answered decision built once its file's tiers derive: no tier is
    /// known to wait on.
    BuildWaitsTiers { heading: String },
    /// The agent writes a question that supersedes one decision of each pair
    /// the reader marked same or inconsistent.
    SupersedeMarkedPairs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    pub kind: MoveKind,
}

impl Move {
    pub fn new(kind: MoveKind) -> Move {
        Move { kind }
    }

    /// Who takes the move, fixed by its kind.
    pub fn actor(&self) -> Mover {
        match &self.kind {
            MoveKind::Approve { .. } | MoveKind::Answer { .. } => Mover::Owner,
            MoveKind::AnswerWaits { .. }
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
            | MoveKind::SupersedeMarkedPairs => Mover::Agent,
        }
    }

    /// The clauses of the move as a refusal's tail: who does what. `None` for
    /// a move nobody takes yet.
    pub fn next(&self) -> Option<Next> {
        Some(match &self.kind {
            MoveKind::PlanUnresolved => {
                Next::all(vec![Next::agent_runs(Cmd::fixed("adr check")), Next::agent("clears what it names in the plan")])
            }
            MoveKind::Plan { rel } => Next::agent_runs(command(&["adr", "plan", rel])),
            MoveKind::WriteLines { headings } => Next::agent(format!("writes lines for {}", quoted(headings))),
            MoveKind::WriteBodies { headings } => Next::agent(format!("writes the body of {}", quoted(headings))),
            MoveKind::Approve { path } => Next::owner_runs(command(&["adr", "approve", path])),
            MoveKind::Ask { rel, .. } => Next::agent_runs(command(&["adr", "ask", rel])),
            MoveKind::Answer { id, version, .. } => {
                Next::owner_runs(command(&["adr", "answer", &format!("{}@{}", id, version)]).then("<letter>"))
            }
            MoveKind::Refused { heading } => Next::agent(format!(
                "changes the plan: \"{}\" is refused and holds the questions that come after it",
                heading
            )),
            MoveKind::Build { heading, id } => {
                Next::agent(format!("builds \"{}\" and puts \"adr: {}\" in a test that shows it", heading, id))
            }
            MoveKind::SupersedeMarkedPairs => {
                Next::agent("writes a question that supersedes one decision of each same or inconsistent pair")
            }
            MoveKind::AnswerWaits { .. }
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
            MoveKind::Answer { heading, .. } => format!("{}  # \"{}\"", self.clause_text(), heading),
            MoveKind::Ask { headings, .. } => format!("{}  # {}", self.clause_text(), quoted(headings)),
            _ => self.clause_text(),
        }
    }

    /// The move as one clause among others in a refusal's tail: no `#`
    /// comment, so the clauses after it are never read as part of one.
    pub fn clause_text(&self) -> String {
        match &self.kind {
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
            MoveKind::BuildWaitsTiers { heading } => {
                format!("not yet: \"{}\" is answered and is built once the tiers of its file derive", heading)
            }
            _ => self.next().expect("a move with a mover").render(),
        }
    }

}

/// What the rules find in one thing of the records and the moves that
/// follow from its status: what every command that reads the thing prints,
/// each finding whole with the move that clears it, then the moves (per
/// "Does a command compute a move of its own?", aac5c313 A, and "What does
/// a command that writes nothing print?", 848e1b76 A). A finding is never
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
    pub fn moves_tail(&self, fallback: Next) -> String {
        if self.moves.is_empty() {
            debug_assert!(!self.findings.is_empty(), "a refusal names at least one move");
            return fallback.render();
        }
        self.moves.iter().map(Move::clause_text).collect::<Vec<_>>().join(", ")
    }
}

/// The move of a pair of decisions the reader marks as depending on one
/// another: nobody moves on it.
pub const PAIR_WAIT: &str = "not yet, nobody moves on a pair that depends";

/// The tail of a finding whose only move is to wait for an answer
/// elsewhere: nobody moves on it yet.
pub const WAIT_TAIL: &str = "; not yet, after it is answered";

/// The clauses of the move `kind` names, for a refusal that stops in a state
/// whose move this module computes: the refusal names the same move every
/// command names for that state (per "Does a command compute a move of its
/// own?", aac5c313 A).
pub fn clause(kind: MoveKind) -> Next {
    Move::new(kind).next().expect("a move with a mover")
}

/// A command line a move names, as the one who runs it pastes it into a
/// shell. Built from its words by `command`, which quotes every word that a
/// shell would read otherwise, so a path, an identifier or a term name
/// taken from the records runs as the word it is, whatever it holds (per
/// "Does a refusal name the command that clears it?", b0e865b4 A).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cmd(String);

impl Cmd {
    /// A command none of whose words come from the records: written once,
    /// in the code, as the shell reads it.
    pub fn fixed(line: &'static str) -> Cmd {
        Cmd(line.to_string())
    }

    /// The command with `placeholder` after it, a word the one who runs it
    /// writes in (`<letter>`), printed as it stands.
    pub fn then(mut self, placeholder: &'static str) -> Cmd {
        self.0.push(' ');
        self.0.push_str(placeholder);
        self
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The command line of `words`, each quoted as `shell_word` quotes it.
pub fn command(words: &[&str]) -> Cmd {
    Cmd(words.iter().map(|w| shell_word(w)).collect::<Vec<_>>().join(" "))
}

/// `word` as a POSIX shell reads it back: unchanged when it holds only
/// letters, digits and `_ . / @ : = , + -` (and is not empty), otherwise in
/// single quotes, a quote inside written `'\''`. Nothing inside single
/// quotes is expanded, so `$(..)`, a backtick, a space or a newline stay
/// part of the one word.
pub fn shell_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '@' | ':' | '=' | ',' | '+' | '-'));
    if plain {
        return word.to_string();
    }
    // A control character (a newline, an escape) would end or redraw the
    // printed line inside plain quotes, and a zero-width or bidirectional
    // one would be written `\u{..}` by the printer inside them: such a word
    // is written `$'..'`, each of those characters and every quote escaped,
    // so the move stays one line and runs as printed.
    if word.chars().any(|c| c == '\t' || crate::out::is_hidden(c)) {
        let mut s = String::from("$'");
        for c in word.chars() {
            match c {
                '\\' => s.push_str("\\\\"),
                '\'' => s.push_str("\\'"),
                '\n' => s.push_str("\\n"),
                '\t' => s.push_str("\\t"),
                '\r' => s.push_str("\\r"),
                c if (c as u32) < 0x80 && c.is_control() => s.push_str(&format!("\\x{:02x}", c as u32)),
                c if crate::out::is_hidden(c) => s.push_str(&format!("\\U{:08x}", c as u32)),
                c => s.push(c),
            }
        }
        s.push('\'');
        return s;
    }
    format!("'{}'", word.replace('\'', "'\\''"))
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
    // Nothing holds the removal (check's ids rule names a removed file whose
    // questions are refused, say): the owner approves it away.
    if holds.is_empty() {
        return Next::owner_runs(command(&["adr", "approve", rel]));
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
        .and_owner_runs(command(&["adr", "approve", rel]))
}

/// The report of a removed topic file: each hold that keeps its removal
/// from being approved (`rules::judge_removed_topic_file`, each carrying
/// the move `removal_next` names), or, with none, the owner's approval.
pub fn of_removed_topic_file(rel: &str, findings: Vec<Finding>) -> Report {
    let moves = if findings.is_empty() { vec![Move::new(MoveKind::Approve { path: rel.to_string() })] } else { Vec::new() };
    Report { stops: Vec::new(), findings, moves }
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
/// rules find in the file (`rules::judge_topic_file_in`): each carries its
/// own move, so while one stands the planning steps wait on it.
pub fn of_topic_file(
    rf: &RecordFile,
    led: &Ledger,
    plan_ok: bool,
    findings: &[Finding],
    tests: &HashMap<String, Vec<String>>,
    planning: &crate::plan::Planning,
) -> Vec<Move> {
    if rf.tiers.is_none() {
        if findings.is_empty() {
            return vec![Move::new(MoveKind::PlanUnresolved)];
        }
        return Vec::new();
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
        match status::question_status_in(led, rf, sec) {
            QuestionStatus::NoBody | QuestionStatus::NotAsked if sec.is_skeleton => bare = true,
            QuestionStatus::NoBody | QuestionStatus::NotAsked => to_ask.push(heading.to_string()),
            // Edited since it was asked: `answer` takes only the text the
            // ledger holds, so the agent asks it again first.
            QuestionStatus::ChangedSinceAsked => to_ask.push(heading.to_string()),
            QuestionStatus::Asked => {
                let Some(id) = &sec.id else { continue };
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
                // Two standing answers for different options: no decision
                // stands to build until the owner answers it again, the
                // move its finding names (`rules::closed_findings`).
                if state::conflicting_rulings(led, id).is_some() {
                    continue;
                }
                let m = of_untested(rf, i, id);
                match m.kind {
                    MoveKind::Build { .. } => build.push(m),
                    _ => build_waits.push(m),
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
    let refused = !findings.is_empty();
    if plan_ok {
        // Approve refuses a heading with no body, so one in an approved file
        // is one approved before it did: its body is written, then planned.
        if bare {
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

/// What the moves of a file are judged against, read once for a run: the
/// repository root, every topic file as loaded, the terms, the ledger and
/// what the judgement shares across files (`rules::RepoIndex`).
pub struct Judging<'a> {
    pub root: &'a std::path::Path,
    pub all: &'a [RecordFile],
    pub terms_list: &'a [crate::terms::Term],
    pub led: &'a Ledger,
    pub index: &'a crate::rules::RepoIndex,
}

/// `of_unasked_question` of the question `heading` of the topic file `rf`
/// as loaded from disk, with the findings and planning records read the
/// way `of_topic_file_in` reads them: the move `answer` names for a
/// question it cannot take yet is the one `show` names for its file.
pub fn of_unasked_question_in(j: &Judging, rf: &RecordFile, heading: &str, unwritten: bool) -> Report {
    let plan_ok = state::plan_approved(j.led, rf, &rf.rel) && state::context_approved(j.led, rf, &rf.rel);
    let planning = crate::plan::load_with(rf, j.terms_list, j.led, &j.index.planning);
    let mut findings = crate::rules::judge_topic_file_in(j.root, rf, j.all, j.terms_list, j.led, j.index);
    findings.extend(j.index.records().iter().cloned());
    let mut moves = of_unasked_question(rf, plan_ok, &findings, &planning, heading, unwritten);
    // The terms file's gate, as `of_topic_file_with` reads it.
    let gate = crate::rules::terms_gate(j.root, j.led, &rf.rel);
    let mut stops = Vec::new();
    if !gate.is_empty() {
        moves.retain(|m| !matches!(m.kind, MoveKind::Approve { .. } | MoveKind::Ask { .. } | MoveKind::Answer { .. }));
        findings.splice(0..0, gate.findings);
        stops = gate.stops;
    }
    Report { stops, findings, moves }
}

/// Every move of the topic file `rf` as loaded from disk: the approval of
/// its plan and prose and its planning records read off the file as loaded,
/// then every finding `approve` refuses on (`rules::judge_topic_file_in`)
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
) -> Report {
    of_topic_file_with(root, rf, all, terms_list, led, tests, &crate::rules::RepoIndex::new(root, all, terms_list, led))
}

/// `of_topic_file_in` with what the run shares read once (`index`): what a
/// command that reports many files calls once per file.
pub fn of_topic_file_with(
    root: &std::path::Path,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[crate::terms::Term],
    led: &Ledger,
    tests: &HashMap<String, Vec<String>>,
    index: &crate::rules::RepoIndex,
) -> Report {
    of_topic_file_judged(root, rf, all, terms_list, led, tests, index).0
}

/// `of_topic_file_with` and the file's own judgement
/// (`rules::judge_topic_file_in`) that the report's findings start from,
/// before the findings on the records as a whole, the terms gate and the
/// struck questions join them: what `check` files by rule for the file.
pub fn of_topic_file_judged(
    root: &std::path::Path,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[crate::terms::Term],
    led: &Ledger,
    tests: &HashMap<String, Vec<String>>,
    index: &crate::rules::RepoIndex,
) -> (Report, Vec<Finding>) {
    let shared = &index.planning;
    let plan_ok = state::plan_approved(led, rf, &rf.rel) && state::context_approved(led, rf, &rf.rel);
    let planning = crate::plan::load_with(rf, terms_list, led, shared);
    let findings = crate::rules::judge_topic_file_in(root, rf, all, terms_list, led, index);
    let judged = findings.clone();
    let mut rf = rf.clone();
    crate::depends::normalize_cross_file_deps(&mut rf, terms_list, &shared.phrase_lookup, &shared.cross, led);
    // The rules on the topic files as a whole (an identifier twice, a
    // withdrawn one back, a current decision no file holds): approve
    // refuses any file on them, so the file's report carries them and its
    // planning moves wait on them.
    let mut findings = findings;
    findings.extend(index.records().iter().cloned());
    let mut moves = of_topic_file(&rf, led, plan_ok, &findings, tests, &planning);
    // While adrdom/terms.md is not approved, approve, ask and answer stop on
    // it: its gate is the file's finding, and their moves wait on it.
    let gate = crate::rules::terms_gate(root, led, &rf.rel);
    let mut stops = Vec::new();
    if !gate.is_empty() {
        moves.retain(|m| !matches!(m.kind, MoveKind::Approve { .. } | MoveKind::Ask { .. } | MoveKind::Answer { .. }));
        findings.splice(0..0, gate.findings);
        stops = gate.stops;
    }
    // A question asked and struck from the plan is cleared by the owner's
    // approve the moves above already name, so it joins the findings only
    // after the moves are computed from them.
    // A file is stale once approved and changed since (`rules::stale_files`,
    // the list check reads): a draft never approved strikes nothing.
    let sole = index.stale().len() == 1;
    let stale = index.stale().contains(&rf.rel);
    findings.extend(crate::rules::struck_asked(&rf.rel, stale, sole, all, led));
    (Report { stops, findings, moves }, judged)
}

/// The move of the answered decision `id`, section `i` of `rf`, that no
/// test names yet: the agent builds it, or nobody yet while a question
/// beside it waits for an answer or is refused (per "Does the agent build
/// on a decision before its tier is fully answered?"). The same move
/// `of_topic_file` names for it; what `check` prints beside an untested
/// decision.
pub fn of_untested(rf: &RecordFile, i: usize, id: &str) -> Move {
    let heading = rf.sections[i].heading_text.clone();
    // A question beside it that still waits for an answer is named ahead of
    // a refused one: the refused one has its own line, and the answer is
    // what the build waits for longest.
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
/// question waiting on it, by the rule `of_topic_file` applies: under an
/// approved plan, the agent's `adr ask` (or, for a question with no body,
/// the planning move); under a plan that waits for its approval and has no
/// finding, the planning move. The findings carry their own moves.
pub fn of_unasked_question(
    rf: &RecordFile,
    plan_ok: bool,
    findings: &[Finding],
    planning: &crate::plan::Planning,
    heading: &str,
    unwritten: bool,
) -> Vec<Move> {
    let mut out: Vec<Move> = Vec::new();
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
pub fn of_planning(rf: &RecordFile, p: &crate::plan::Planning) -> Vec<Move> {
    use crate::plan::{NextMove, Step};
    let approve = || vec![Move::new(MoveKind::Approve { path: rf.rel.clone() })];
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

/// The report of one blame file: its findings, and, for a draft with
/// nothing left to clear (no finding, no stop on reading its origin), the
/// owner's approval.
pub fn of_blame_file(rel: &str, stops: Vec<crate::out::Line>, findings: Vec<Finding>, awaiting: bool) -> Report {
    let mut moves = Vec::new();
    if findings.is_empty() && stops.is_empty() && awaiting {
        moves.push(Move::new(MoveKind::Approve { path: rel.to_string() }));
    }
    Report { stops, findings, moves }
}

/// The report of the terms file: the findings of the rules on it, and, for
/// a file that differs from its latest approval with nothing left to clear,
/// the owner's approval.
pub fn of_terms_file(findings: Vec<Finding>, status: status::TermsFileStatus) -> Report {
    let mut moves = Vec::new();
    if findings.is_empty() && status.awaits_approval() {
        moves.push(Move::new(MoveKind::Approve { path: crate::repo::TERMS_PATH.to_string() }));
    }
    Report { stops: Vec::new(), findings, moves }
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
    Asking { heading: String, report: Report },
    /// No decision yet and no question to answer first: the chain it waits
    /// on ends on `refused`, a refused question.
    Stuck { heading: String, refused: String },
    /// No decision yet and no body written.
    Unwritten { heading: String, report: Report },
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
pub fn answer_waits(heading: &str, rel: &str, waits: Vec<Waits>) -> (crate::msg::Said, Vec<Finding>) {
    let mut parts = Vec::new();
    let mut answer: Option<Move> = None;
    let mut refused: Vec<String> = Vec::new();
    let mut found: Vec<Finding> = Vec::new();
    let mut agent: Vec<Move> = Vec::new();
    let mut owner: Option<Move> = None;
    let add = |report: Report, found: &mut Vec<Finding>, agent: &mut Vec<Move>, owner: &mut Option<Move>| {
        for f in &report.findings {
            if !found.iter().any(|g| g.text() == f.text()) {
                found.push(f.clone());
            }
        }
        for m in report.moves {
            if m.actor() == Mover::Owner {
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
            Waits::Unwritten { heading: h, report } => {
                parts.push(format!("\"{}\" has no decision yet and is [{}]", h, QuestionStatus::NoBody.name()));
                add(report, &mut found, &mut agent, &mut owner);
            }
            Waits::Asking { heading: h, report } => {
                parts.push(format!("\"{}\" has no decision yet", h));
                add(report, &mut found, &mut agent, &mut owner);
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
    let mut clauses: Vec<String> = Vec::new();
    clauses.extend(agent.iter().map(Move::clause_text));
    if !refused.is_empty() {
        clauses.push(
            Next::agent(format!("writes the plan change that drops or replaces {}", quoted(&refused))).render(),
        );
        owner = Some(Move::new(MoveKind::Approve { path: rel.to_string() }));
    }
    if owner.is_none() {
        owner = answer;
    }
    clauses.extend(owner.map(|m| m.clause_text()));
    if clauses.is_empty() {
        clauses.push(Next::agent("clears the findings below").render());
    }
    // What the rules find in the files it waits on is returned apart: each
    // finding prints whole on a line of its own after the refusal's line.
    let rule = if refused.is_empty() {
        &crate::rule::ANSWER_NEEDS_LOWER_ANSWERS
    } else {
        &crate::rule::ANSWER_NEEDS_NO_REFUSED_BELOW
    };
    (crate::msg::said_raw(rule, format!("\"{}\" waits: {}", heading, parts.join(", ")), clauses.join(", ")), found)
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: 184b5bcb-fa72-436b-92cb-4d4605366c59
    #[test]
    fn a_report_with_no_move_names_the_fallback_act() {
        let mut empty = Report::default();
        empty.findings.push(Finding::new(&crate::rule::ASKED_TEXT_FROZEN, "f".into(), "agent m".into()));
        assert_eq!(empty.moves_tail(Next::agent("restores x.md from git")), "agent restores x.md from git");
        assert!(!crate::msg::edited_after_asking("x.md", "h", &empty).text().contains("each finding of the file names"));
        assert!(crate::msg::edited_after_asking("x.md", "h", &empty).text().ends_with("agent restores x.md from git"));
    }

    #[test]
    fn the_kind_fixes_who_moves() {
        assert_eq!(Move::new(MoveKind::Approve { path: "x.md".into() }).actor(), Mover::Owner);
        assert_eq!(Move::new(MoveKind::Plan { rel: "x.md".into() }).actor(), Mover::Agent);
        assert_eq!(Move::new(MoveKind::BuildWaits { heading: "h".into(), beside: "b".into() }).actor(), Mover::NotYet);
        assert_eq!(of_removed_topic_file("x.md", Vec::new()).moves[0].actor(), Mover::Owner);
    }

    // adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
    //
    // A word taken from the records runs as the one word it is: nothing in
    // it is expanded by the shell the move is pasted into, and the move
    // stays one line.
    #[test]
    fn a_command_quotes_every_word_a_shell_would_read_otherwise() {
        assert_eq!(command(&["adr", "approve", "adrdom/decisions/x-y.md"]).to_string(), "adr approve adrdom/decisions/x-y.md");
        assert_eq!(
            command(&["adr", "approve", "adrdom/blame/a$(touch PWNED).md"]).to_string(),
            "adr approve 'adrdom/blame/a$(touch PWNED).md'"
        );
        assert_eq!(shell_word("it's"), "'it'\\''s'");
        assert_eq!(shell_word(""), "''");
        assert_eq!(shell_word("a\nowner runs: x"), "$'a\\nowner runs: x'");
        assert_eq!(shell_word("`id`"), "'`id`'");
        let m = Move::new(MoveKind::Answer { id: "x$(touch P)".into(), version: "abcd".into(), heading: "H?".into() });
        assert_eq!(m.clause_text(), "owner runs: adr answer 'x$(touch P)@abcd' <letter>");
    }

    // adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
    //
    // A word holding a zero-width, bidirectional or control character is
    // quoted so the terminal line shows it escaped and the shell, reading the
    // move as printed, gets the word back as the file holds it.
    #[test]
    fn a_word_with_a_hidden_character_runs_as_printed() {
        for word in ["adrdom/decisions/z\u{200b}z.md", "a\u{202e}b.md", "x\u{85}y", "p\u{1b}[2Kq", "it's\u{feff}"] {
            let line = crate::out::text(&crate::out::Line::Move(Move::new(MoveKind::Approve { path: word.into() })));
            assert!(!line.contains("\\u{"), "{:?} printed as {}", word, line);
            let printed = line.strip_prefix("owner runs: adr approve ").unwrap();
            let out = std::process::Command::new("bash").arg("-c").arg(format!("printf %s {}", printed)).output().unwrap();
            assert_eq!(String::from_utf8(out.stdout).unwrap(), word, "{}", line);
        }
    }

    #[test]
    fn each_move_reads_as_one_line() {
        let m = Move::new(MoveKind::Answer { id: "id-1".into(), version: "abcd".into(), heading: "Does it?".into() });
        assert_eq!(m.text(), "owner runs: adr answer id-1@abcd <letter>  # \"Does it?\"");
        let m = Move::new(MoveKind::Ask { rel: "x.md".into(), headings: vec!["A?".into(), "B?".into()] });
        assert_eq!(m.text(), "agent runs: adr ask x.md  # \"A?\", \"B?\"");
        let m = Move::new(MoveKind::BuildWaits { heading: "A?".into(), beside: "B?".into() });
        assert_eq!(m.text(), "not yet: \"A?\" is answered and is built once \"B?\" is answered");
        assert_eq!(Move::new(MoveKind::Approve { path: "x.md".into() }).text(), "owner runs: adr approve x.md");
    }

    // adr: aac5c313-dedd-466b-a0c3-ead5e99ce913
    #[test]
    fn a_tail_of_clauses_holds_no_comment_before_a_later_clause() {
        let report = Report {
            stops: Vec::new(),
            findings: Vec::new(),
            moves: vec![
                Move::new(MoveKind::Ask { rel: "x.md".into(), headings: vec!["A?".into(), "B?".into()] }),
                Move::new(MoveKind::Approve { path: "x.md".into() }),
            ],
        };
        let tail = report.moves_tail(Next::agent("clears the findings below"));
        assert_eq!(tail, "agent runs: adr ask x.md, owner runs: adr approve x.md");
        assert!(!tail.contains('#'), "{}", tail);
    }

    #[test]
    fn answer_waits_merges_the_ask_moves_and_puts_the_owner_last() {
        let ask = |h: &str| Move::new(MoveKind::Ask { rel: "x.md".into(), headings: vec![h.into()] });
        let (said, _) = answer_waits(
            "heading",
            "x.md",
            vec![
                Waits::Asking { heading: "a".into(), report: Report { stops: Vec::new(), findings: Vec::new(), moves: vec![ask("a")] } },
                Waits::Asking {
                    heading: "b".into(),
                    report: Report {
            stops: Vec::new(),
                        findings: Vec::new(),
                        moves: vec![ask("b"), Move::new(MoveKind::Approve { path: "x.md".into() })],
                    },
                },
            ],
        );
        assert_eq!(said.found, "\"heading\" waits: \"a\" has no decision yet, \"b\" has no decision yet");
        assert_eq!(said.next, "agent runs: adr ask x.md, owner runs: adr approve x.md");
    }

    #[test]
    fn a_hold_is_superseded_when_current_and_struck_otherwise() {
        let holds = vec![
            RemovalHold { id: "id-1".into(), heading: "A?".into(), status: QuestionStatus::Answered },
            RemovalHold { id: "id-2".into(), heading: "B?".into(), status: QuestionStatus::Asked },
        ];
        let text = removal_next("x.md", &holds).render();
        assert!(text.contains("supersedes") && text.contains("strikes"), "{}", text);
        assert!(text.ends_with("owner runs: adr approve x.md"), "{}", text);
    }
}
