//! The moves of a topic file and of the other files of the records: the order they are taken in.
use super::*;

/// The report of a removed topic file: each hold that keeps its removal
/// from being approved (`rules::judge_removed_topic_file`, each carrying
/// the move `removal_next` names), or, with none, the owner's approval.
pub fn of_removed_topic_file(rel: &str, findings: Vec<Finding>) -> Report {
    let moves = if findings.is_empty() {
        vec![Move::new(MoveKind::Approve {
            path: rel.to_string(),
        })]
    } else {
        Vec::new()
    };
    Report {
        stops: Vec::new(),
        findings,
        moves,
    }
}

/// Every move of the topic file `rf`, in the order they are taken: what is
/// answered is built before anything new is written, then the findings of
/// its plan, the planning or ask moves, the owner's answers, and what waits.
/// `plan_ok` is read by the caller off the file as loaded. `tests` is `check::test_index`: an
/// answered decision with a test tied to it is built. `planning` is
/// `plan::load` of the file: a plan that waits for its approval, or holds a
/// heading with no body, follows the planning steps from it ("What does the
/// agent do to plan a topic file?", 46fc51ff A). `findings` are what the
/// rules find in the file (`rules::judge_topic_file_in`): each carries its
/// own move, so while one stands the planning steps wait on it. With `builds`
/// false no untested decision has a build move (a satellite builds none, and
/// a primary none while a satellite is missing).
#[allow(clippy::too_many_arguments)]
pub fn of_topic_file(
    rf: &RecordFile,
    led: &Ledger,
    plan_ok: bool,
    findings: &[Finding],
    tests: &HashMap<String, Vec<String>>,
    planning: &crate::plan::Planning,
    role: crate::session::Role,
    builds: bool,
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
                    waiting.push(Move::new(MoveKind::AnswerWaits {
                        heading: heading.to_string(),
                        on,
                    }));
                }
            }
            QuestionStatus::Refused => held.push(Move::new(MoveKind::Refused {
                heading: heading.to_string(),
            })),
            QuestionStatus::Answered => {
                let Some(id) = &sec.id else { continue };
                if !builds {
                    continue;
                }
                if tests.get(id).is_some_and(|locations| !locations.is_empty()) {
                    continue;
                }
                // Two standing answers for different options: no decision
                // stands to build until the owner answers it again, the
                // move its finding names (`rules::closed_findings`).
                if state::conflicting_rulings(led, id).is_some() {
                    continue;
                }
                let m = of_untested(rf, i, id, role);
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
            out.push(Move::new(MoveKind::Ask {
                rel: rf.rel.clone(),
                headings: to_ask,
            }));
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
pub fn of_unasked_question_in(
    j: &Judging,
    rf: &RecordFile,
    heading: &str,
    unwritten: bool,
) -> Report {
    let plan_ok =
        state::plan_approved(j.led, rf, &rf.rel) && state::context_approved(j.led, rf, &rf.rel);
    let planning = crate::plan::load_with(rf, j.terms_list, j.led, &j.index.planning);
    let mut findings =
        crate::rules::judge_topic_file_in(j.root, rf, j.all, j.terms_list, j.led, j.index);
    findings.extend(j.index.records().iter().cloned());
    let mut moves = of_unasked_question(rf, plan_ok, &findings, &planning, heading, unwritten);
    // The terms file's gate, as `of_topic_file_with` reads it.
    let gate = crate::rules::terms_gate(j.root, j.led, &rf.rel);
    let mut stops = Vec::new();
    if !gate.is_empty() {
        moves.retain(|m| {
            !matches!(
                m.kind,
                MoveKind::Approve { .. } | MoveKind::Ask { .. } | MoveKind::Answer { .. }
            )
        });
        findings.splice(0..0, gate.findings);
        stops = gate.stops;
    }
    if gate.waits {
        moves.push(Move::new(MoveKind::CommitWaits));
    }
    Report {
        stops,
        findings,
        moves,
    }
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
    of_topic_file_with(
        root,
        rf,
        all,
        terms_list,
        led,
        tests,
        &crate::rules::RepoIndex::new(root, all, terms_list, led),
    )
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
    of_topic_file_judged(root, rf, all, terms_list, led, tests, index, false).0
}

/// `of_topic_file_with` and the file's own judgement
/// (`rules::judge_topic_file_in`) that the report's findings start from,
/// before the findings on the records as a whole, the terms gate and the
/// struck questions join them: what `check` files by rule for the file.
/// `in_satellite`: `root` is the record of a satellite's primary.
#[allow(clippy::too_many_arguments)]
pub fn of_topic_file_judged(
    root: &std::path::Path,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[crate::terms::Term],
    led: &Ledger,
    tests: &HashMap<String, Vec<String>>,
    index: &crate::rules::RepoIndex,
    in_satellite: bool,
) -> (Report, Vec<Finding>) {
    let shared = &index.planning;
    let plan_ok =
        state::plan_approved(led, rf, &rf.rel) && state::context_approved(led, rf, &rf.rel);
    let planning = crate::plan::load_with(rf, terms_list, led, shared);
    let findings = crate::rules::judge_topic_file_in(root, rf, all, terms_list, led, index);
    let judged = findings.clone();
    // The rules on the topic files as a whole (an identifier twice, a
    // withdrawn one back, a current decision no file holds): approve
    // refuses any file on them, so the file's report carries them and its
    // planning moves wait on them.
    let mut findings = findings;
    findings.extend(index.records().iter().cloned());
    let mut moves = of_topic_file(
        rf,
        led,
        plan_ok,
        &findings,
        tests,
        &planning,
        crate::session::role(root),
        !in_satellite && crate::satellite::missing(root).is_empty(),
    );
    // While adrdom/terms.md is not approved, approve, ask and answer stop on
    // it: its gate is the file's finding, and their moves wait on it.
    let gate = crate::rules::terms_gate(root, led, &rf.rel);
    let mut stops = Vec::new();
    if !gate.is_empty() {
        moves.retain(|m| {
            !matches!(
                m.kind,
                MoveKind::Approve { .. } | MoveKind::Ask { .. } | MoveKind::Answer { .. }
            )
        });
        findings.splice(0..0, gate.findings);
        stops = gate.stops;
    }
    if gate.waits {
        moves.push(Move::new(MoveKind::CommitWaits));
    }
    // A question asked and struck from the plan is cleared by the owner's
    // approve the moves above already name, so it joins the findings only
    // after the moves are computed from them.
    // A file is stale once approved and changed since (`rules::stale_files`,
    // the list check reads): a draft never approved strikes nothing.
    let sole = index.stale().len() == 1;
    let stale = index.stale().contains(&rf.rel);
    findings.extend(crate::rules::struck_asked(&rf.rel, stale, sole, all, led));
    (
        Report {
            stops,
            findings,
            moves,
        },
        judged,
    )
}

/// The move of the answered decision `id`, section `i` of `rf`, that no
/// test names yet: the agent builds it, or nobody yet while a question
/// beside it waits for an answer or is refused (per "Does the agent build
/// on a decision before its tier is fully answered?"). The same move
/// `of_topic_file` names for it; what `check` prints beside an untested
/// decision.
pub fn of_untested(rf: &RecordFile, i: usize, id: &str, role: crate::session::Role) -> Move {
    let heading = rf.sections[i].heading_text.clone();
    // A question beside it that still waits for an answer is named ahead of
    // a refused one: the refused one has its own line, and the answer is
    // what the build waits for longest.
    match state::unresolved_sibling(rf, i) {
        Some(s) if s.is_refused() => Move::new(MoveKind::BuildWaitsRefused {
            heading,
            beside: s.heading_text.clone(),
        }),
        Some(s) => Move::new(MoveKind::BuildWaits {
            heading,
            beside: s.heading_text.clone(),
        }),
        None => Move::new(MoveKind::Build {
            heading,
            id: id.to_string(),
        })
        .in_session(role),
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
            out.push(Move::new(MoveKind::Ask {
                rel: rf.rel.clone(),
                headings: vec![heading.to_string()],
            }));
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
    let approve = || {
        vec![Move::new(MoveKind::Approve {
            path: rf.rel.clone(),
        })]
    };
    match p.next() {
        NextMove::Plan => vec![Move::new(MoveKind::Plan {
            rel: rf.rel.clone(),
        })],
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

/// The report of the blame file `e` of `index`, read against its origin in
/// the working tree: its findings, and, for a draft with nothing left to
/// clear (no finding, no stop on reading its origin), the owner's approval.
/// The one report `show` prints for a blame file; `cmd` leads each line of a
/// stop.
pub fn of_blame_file(
    root: &std::path::Path,
    e: &crate::blame::Entry,
    index: &crate::blame::Index,
    cmd: crate::rules::Command,
) -> Report {
    let st = crate::blame::file_state(
        root,
        &crate::repo::Source::WorkingTree,
        &e.file,
        e.status,
        index,
    );
    let findings = crate::rules::judge_blame_file(&st, &e.file);
    let stops: Vec<crate::out::Line> = st
        .stops
        .iter()
        .flat_map(|m| crate::out::stop(cmd, m))
        .collect();
    let mut moves = Vec::new();
    if findings.is_empty() && stops.is_empty() && e.status.awaits_approval() {
        moves.push(Move::new(MoveKind::Approve {
            path: e.rel().to_string(),
        }));
    }
    Report {
        stops,
        findings,
        moves,
    }
}

/// The report of the terms file: the findings of the rules on it, and, for
/// a file that differs from its latest approval with nothing left to clear,
/// the owner's approval of each entry that waits (`waiting`, per "What does
/// approve take for a terms file holding several entries that wait?",
/// da0dc8b1 A), or of the file when no entry waits; while it differs, a
/// commit waits.
pub fn of_terms_file(
    findings: Vec<Finding>,
    status: status::TermsFileStatus,
    waiting: &[String],
) -> Report {
    let mut moves = Vec::new();
    if findings.is_empty() && status.awaits_approval() {
        if waiting.is_empty() {
            moves.push(Move::new(MoveKind::Approve {
                path: crate::repo::TERMS_PATH.to_string(),
            }));
        }
        moves.extend(
            waiting
                .iter()
                .map(|name| Move::new(MoveKind::ApproveEntry { name: name.clone() })),
        );
    }
    if status.awaits_approval() {
        moves.push(Move::new(MoveKind::CommitWaits));
    }
    Report {
        stops: Vec::new(),
        findings,
        moves,
    }
}

/// The move after a reading that marked a pair same or inconsistent.
pub fn of_marked_pairs() -> Move {
    Move::new(MoveKind::SupersedeMarkedPairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: 1af76f91-5644-42ef-97ea-8f70f934c26c
    //
    // A blame file's report is read once for `show`: a draft with
    // nothing to clear is the owner's approve, a draft with a finding has
    // none, and a file approved has no move at all, its origin gone or not.
    #[test]
    fn the_report_of_a_blame_file_is_the_owners_approve_only_for_a_clean_draft() {
        use crate::blame::{self, Index};
        use crate::ledger::{Ledger, Row as LedgerRow};
        use crate::rules::Command;
        let root = std::env::temp_dir().join(format!("adr-moves-blame-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/x.md"), "one\ntwo\n").unwrap();
        let blob = crate::repo::git_hash_object_stdin(&root, "one\ntwo\n").unwrap();
        let file = |rel: &str, origin: &str, rows: &str| {
            let text = format!("origin: {}\nblob: {}\n\n{}", origin, blob, rows);
            blame::parse(root.join(rel), rel.to_string(), record::split_lines(&text))
        };
        let report = |bf: blame::BlameFile, approved: bool| {
            let led = Ledger {
                rows: approved
                    .then(|| LedgerRow {
                        time: "2024-01-01T00:00:00Z".to_string(),
                        event: "blame".to_string(),
                        id: bf.id.clone().unwrap_or_else(|| "-".to_string()),
                        hash: blame::file_hash(&bf.lines),
                        detail: bf.rel.clone(),
                        version: "0.1.0".to_string(),
                    })
                    .into_iter()
                    .collect(),
                malformed: vec![],
            };
            let index = Index::of(vec![bf], &led);
            of_blame_file(&root, &index.entries()[0], &index, Command::Show)
        };
        let clean = report(
            file("adrdom/blame/a.md", "docs/x.md", "1-2\tdecision\n"),
            false,
        );
        assert!(clean.findings.is_empty() && clean.stops.is_empty());
        assert_eq!(
            clean.moves,
            vec![Move::new(MoveKind::Approve {
                path: "adrdom/blame/a.md".into()
            })]
        );
        let bad_row = report(
            file("adrdom/blame/a.md", "docs/x.md", "1-2\tmaybe\n"),
            false,
        );
        assert_eq!(bad_row.findings.len(), 1);
        assert!(bad_row.moves.is_empty());
        let approved = report(
            file("adrdom/blame/a.md", "docs/x.md", "1-2\tdecision\n"),
            true,
        );
        assert!(approved.findings.is_empty() && approved.moves.is_empty());
        let gone = report(
            file("adrdom/blame/a.md", "docs/gone.md", "1-2\tdecision\n"),
            true,
        );
        assert!(gone.findings.is_empty() && gone.stops.is_empty() && gone.moves.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
